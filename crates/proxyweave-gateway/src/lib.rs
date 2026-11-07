//! Unified Chat / Responses / Messages ingress and backend protocol routing.
//! Same-protocol bytes use axum-reverse-proxy; cross-protocol codecs are extracted TiyGate IR.
use axum::{
    body::{Body, Bytes},
    http::{header, HeaderValue, Request, Response, StatusCode},
    routing::{get, post},
    Router,
};
use axum_reverse_proxy::{ProxyPolicy, ReverseProxy, XForwardedFor};
use http_body::{Body as HttpBody, Frame, SizeHint};
use http_body_util::BodyExt;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use proxyweave_plugin::{Manifest, RequestLease, RuntimeError, RuntimeManager, RuntimeStatus};
use proxyweave_protocols::{
    chat_completions::ChatCompletionsCodec, error_body::encode_error_body_for_suite,
    messages::MessagesCodec, protocol::lossy::check_lossy_conversion, responses::ResponsesCodec,
    EndpointCodec, ErrorClass, ProtocolSuite, RawEnvelope, StreamPart,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    io,
    net::{Ipv4Addr, SocketAddr},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    net::TcpListener,
    sync::RwLock,
    task::{JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tower_http::validate_request::{ValidateRequest, ValidateRequestHeaderLayer};
use tracing::instrument::WithSubscriber;

/// Compiled ingress contracts, available even while the listener is stopped.
pub const INGRESS_PROTOCOL_ENDPOINTS: &[(&str, &[&str])] = &[
    ("openai_chat", &["/v1/chat/completions"]),
    ("openai_responses", &["/v1/responses", "/responses"]),
    ("anthropic_messages", &["/v1/messages"]),
];
pub use proxyweave_plugin::manifest::SUPPORTED_BACKEND_PROTOCOLS;
pub mod routing;
pub use routing::{
    extract_session_key, Backend, BackendMetrics, RoutePool, RouteStrategy, SessionStickiness,
};
// Thin custom validator on the official Tower HTTP validation layer, following
// its Bearer implementation. Unlike that type's Debug, this holds a sensitive
// HeaderValue without Debug, and rejects duplicate Authorization fields.
#[derive(Clone)]
struct LocalAuth(Arc<std::sync::RwLock<HeaderValue>>);
impl ValidateRequest<Body> for LocalAuth {
    type ResponseBody = Body;
    fn validate(&mut self, request: &mut Request<Body>) -> Result<(), Response<Body>> {
        let guard = match self.0.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let expected_key = guard.to_str().ok().and_then(|s| s.strip_prefix("Bearer "));

        // Check x-api-key first (useful for Anthropic clients sending custom client Authorization downstream)
        let mut x_api_key = request.headers().get_all("x-api-key").iter();
        if let Some(key_val) = x_api_key.next() {
            if x_api_key.next().is_none() {
                if let (Some(expected), Ok(actual)) = (expected_key, key_val.to_str()) {
                    if expected == actual {
                        return Ok(());
                    }
                }
            }
            return Err(error(StatusCode::UNAUTHORIZED, "invalid_api_key"));
        }

        let mut auth = request.headers().get_all(header::AUTHORIZATION).iter();
        if let Some(auth_val) = auth.next() {
            if auth.next().is_none() && auth_val == *guard {
                return Ok(());
            }
            return Err(error(StatusCode::UNAUTHORIZED, "invalid_api_key"));
        }
        Err(error(StatusCode::UNAUTHORIZED, "invalid_api_key"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestTrace {
    pub id: String,
    pub timestamp_ms: u64,
    pub client: String,
    pub client_ip: Option<String>,
    pub method: String,
    pub path: String,
    pub model: Option<String>,
    pub ingress_protocol: String,
    pub route_strategy: String,
    pub target_plugin: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub first_token_ms: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_hit: Option<bool>,
    pub fallback_trail: Vec<String>,
    pub error: Option<String>,
}

pub fn detect_client_name(headers: &axum::http::HeaderMap) -> String {
    if let Some(ua) = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
    {
        let ua_lower = ua.to_ascii_lowercase();
        if ua_lower.contains("cursor") {
            return "Cursor".to_string();
        }
        if ua_lower.contains("windsurf") {
            return "Windsurf".to_string();
        }
        if ua_lower.contains("claude") || ua_lower.contains("anthropic") {
            return "Claude Code".to_string();
        }
        if ua_lower.contains("aider") {
            return "Aider".to_string();
        }
        if ua_lower.contains("cline") {
            return "Cline".to_string();
        }
        if ua_lower.contains("roo") {
            return "Roo Code".to_string();
        }
        if ua_lower.contains("opencode") {
            return "OpenCode".to_string();
        }
        if ua_lower.contains("continue") {
            return "Continue".to_string();
        }
        if ua_lower.contains("python") {
            return "Python SDK".to_string();
        }
        if ua_lower.contains("curl") {
            return "cURL".to_string();
        }
        let trimmed = ua.trim();
        if !trimmed.is_empty() && trimmed.len() <= 32 {
            return trimmed.to_string();
        }
    }
    "HTTP Client".to_string()
}

pub fn extract_client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
}

/// Live route-table entry: the exact dynamic port the gateway forwards to right now.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouteBinding {
    pub plugin_id: String,
    pub protocol: String,
    /// Dynamic port reported by the runtime; `None` while no instance is serving.
    pub port: Option<u16>,
    pub endpoint: Option<String>,
    /// Generation this route entry is bound to; `None` means "any generation".
    pub route_generation: Option<u64>,
    pub runtime_generation: Option<u64>,
    pub ready: bool,
    pub state: String,
}

#[derive(Clone)]
pub struct Gateway {
    runtime: RuntimeManager,
    backend: Arc<RwLock<Option<Backend>>>,
    route_pool: Arc<RoutePool>,
    auth: LocalAuth,
    client: Client<hyper_tls::HttpsConnector<HttpConnector>, Body>,
    header_timeout: Duration,
    traces: Arc<tokio::sync::Mutex<VecDeque<RequestTrace>>>,
    active_requests: Arc<AtomicU64>,
    total_routed: Arc<AtomicU64>,
}

impl Gateway {
    pub fn new(
        runtime: RuntimeManager,
        local_key: &str,
        header_timeout: Duration,
    ) -> Result<Self, &'static str> {
        if local_key.len() < 32
            || local_key.len() > 512
            || header_timeout.is_zero()
            || !local_key.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err("Invalid gateway key or timeout");
        }
        let auth = sensitive_bearer(local_key)?;
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        http.set_nodelay(true);
        http.set_connect_timeout(Some(header_timeout));
        // Direct Hyper connector: does not consult system HTTP_PROXY/HTTPS_PROXY.
        let client = Client::builder(TokioExecutor::new())
            .retry_canceled_requests(false)
            .build(hyper_tls::HttpsConnector::new_with_connector(http));
        Ok(Self {
            runtime,
            backend: Arc::new(RwLock::new(None)),
            route_pool: Arc::new(RoutePool::new()),
            auth: LocalAuth(Arc::new(std::sync::RwLock::new(auth))),
            client,
            header_timeout,
            traces: Arc::new(tokio::sync::Mutex::new(VecDeque::with_capacity(100))),
            active_requests: Arc::new(AtomicU64::new(0)),
            total_routed: Arc::new(AtomicU64::new(0)),
        })
    }

    pub fn set_local_key(&self, local_key: &str) -> Result<(), &'static str> {
        if local_key.len() < 32
            || local_key.len() > 512
            || !local_key.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err("Invalid gateway key");
        }
        let auth = sensitive_bearer(local_key)?;
        let mut guard = match self.auth.0.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = auth;
        Ok(())
    }

    /// Only select a validated registered Manifest; upstream key is separate from local key.
    /// Dynamic Managed port is taken from the lease, never cached from the Manifest.
    pub async fn set_backend(
        &self,
        manifest: &Manifest,
        upstream_key: &str,
    ) -> Result<(), &'static str> {
        manifest
            .validate()
            .map_err(|_| "Invalid backend manifest")?;
        let protocol = IngressProtocol::from_backend(&manifest.capabilities.protocol)
            .ok_or("Unsupported protocol")?;
        let credential = if protocol == IngressProtocol::Messages {
            sensitive_api_key(upstream_key)?
        } else {
            sensitive_bearer(upstream_key)?
        };
        let generation = self.runtime.current_generation(&manifest.metadata.id).await;
        let backend = Backend {
            plugin_id: manifest.metadata.id.clone(),
            protocol: manifest.capabilities.protocol.clone(),
            models_path: manifest.capabilities.models_path.clone(),
            inference_path: manifest.capabilities.inference_path.clone(),
            credential,
            instance_generation: generation,
        };
        *self.backend.write().await = Some(backend.clone());
        self.route_pool.set_single_backend(backend).await;
        Ok(())
    }

    pub async fn clear_backend(&self) {
        *self.backend.write().await = None;
        self.route_pool.clear_backends().await;
    }

    pub async fn set_route_strategy(&self, strategy: RouteStrategy) {
        self.route_pool.set_strategy(strategy).await;
    }

    pub async fn get_route_strategy(&self) -> RouteStrategy {
        self.route_pool.get_strategy().await
    }

    pub async fn add_backend_target(
        &self,
        manifest: &Manifest,
        upstream_key: &str,
    ) -> Result<(), &'static str> {
        manifest
            .validate()
            .map_err(|_| "Invalid backend manifest")?;
        let protocol = IngressProtocol::from_backend(&manifest.capabilities.protocol)
            .ok_or("Unsupported protocol")?;
        let credential = if protocol == IngressProtocol::Messages {
            sensitive_api_key(upstream_key)?
        } else {
            sensitive_bearer(upstream_key)?
        };
        let generation = self.runtime.current_generation(&manifest.metadata.id).await;
        let backend = Backend {
            plugin_id: manifest.metadata.id.clone(),
            protocol: manifest.capabilities.protocol.clone(),
            models_path: manifest.capabilities.models_path.clone(),
            inference_path: manifest.capabilities.inference_path.clone(),
            credential,
            instance_generation: generation,
        };
        self.route_pool.add_backend(backend.clone()).await;
        if self.backend.read().await.is_none() {
            *self.backend.write().await = Some(backend);
        }
        Ok(())
    }

    pub async fn remove_backend_target(&self, plugin_id: &str) {
        self.route_pool.remove_backend(plugin_id).await;
        let mut guard = self.backend.write().await;
        if guard.as_ref().map(|b| b.plugin_id.as_str()) == Some(plugin_id) {
            *guard = self.route_pool.get_primary_backend().await;
        }
    }

    pub fn route_pool(&self) -> Arc<RoutePool> {
        self.route_pool.clone()
    }

    /// Re-binds one plugin's route entry to the generation the runtime reports now.
    ///
    /// A Managed plugin that restarted gets a new dynamic port and a new generation; the
    /// route table is refreshed here so the gateway never forwards to a retired port.
    pub async fn refresh_route(&self, plugin_id: &str) -> Option<u64> {
        let generation = self.runtime.current_generation(plugin_id).await;
        // A plugin without a live instance has no generation to follow. `None` in a route
        // entry means "any generation", so writing it here would silently switch the
        // stale-instance check off for that entry; the binding is left untouched instead and
        // admission refuses the request explicitly.
        if let Some(generation) = generation {
            self.route_pool
                .update_generation(plugin_id, Some(generation))
                .await;
            let mut guard = self.backend.write().await;
            if let Some(active) = guard.as_mut() {
                if active.plugin_id == plugin_id {
                    active.instance_generation = Some(generation);
                }
            }
        }
        generation
    }

    /// Snapshot of every route entry with its live dynamic port and generation.
    pub async fn route_snapshot(&self) -> Vec<RouteBinding> {
        let mut bindings = Vec::new();
        for backend in self.route_pool.list_backends().await {
            let status = self.runtime.get_status(&backend.plugin_id).await;
            let (port, ready, state) = match &status {
                RuntimeStatus::Ready { port, .. } => (Some(*port), true, "ready".to_string()),
                // Plan.md §6.3 B: a saturated 503 is not Ready and never a conflict signal.
                RuntimeStatus::Busy { port, reason, .. } => {
                    (Some(*port), false, format!("busy: {reason}"))
                }
                RuntimeStatus::Starting => (None, false, "starting".to_string()),
                RuntimeStatus::Stopped => (None, false, "stopped".to_string()),
                RuntimeStatus::Failed { error } => (None, false, format!("failed: {error}")),
            };
            bindings.push(RouteBinding {
                plugin_id: backend.plugin_id.clone(),
                protocol: backend.protocol.clone(),
                port,
                endpoint: port.map(|p| format!("http://127.0.0.1:{p}")),
                route_generation: backend.instance_generation,
                runtime_generation: self.runtime.current_generation(&backend.plugin_id).await,
                ready,
                state,
            });
        }
        bindings
    }

    /// Admits one request, following the runtime when the instance generation moved.
    ///
    /// One refresh is attempted; a second mismatch is reported as-is instead of looping.
    pub async fn acquire_lease_following_route(
        &self,
        backend: &mut Backend,
    ) -> Result<RequestLease, RuntimeError> {
        match self
            .runtime
            .acquire_request_for(&backend.plugin_id, backend.instance_generation)
            .await
        {
            Err(RuntimeError::StaleGeneration { .. }) => {
                // Exactly one refresh, then one retry: if the runtime cannot name a live
                // generation the request is refused here rather than retried, and a second
                // mismatch is returned as-is (no loop, no silent fallback).
                let Some(generation) = self.refresh_route(&backend.plugin_id).await else {
                    return Err(RuntimeError::HealthCheckFailed(format!(
                        "Plugin '{}' has no live instance after the route refresh",
                        backend.plugin_id
                    )));
                };
                backend.instance_generation = Some(generation);
                self.runtime
                    .acquire_request_for(&backend.plugin_id, Some(generation))
                    .await
            }
            other => other,
        }
    }

    /// Actual successful attachment, not the desktop's persisted preference.
    pub async fn backend_plugin_id(&self) -> Option<String> {
        self.backend
            .read()
            .await
            .as_ref()
            .map(|backend| backend.plugin_id.clone())
    }

    #[cfg(test)]
    pub async fn set_backend_for_test(&self, backend: Option<Backend>) {
        *self.backend.write().await = backend.clone();
        if let Some(b) = backend {
            self.route_pool.set_single_backend(b).await;
        }
    }

    pub async fn record_trace(&self, trace: RequestTrace) {
        let mut list = self.traces.lock().await;
        list.push_front(trace);
        while list.len() > 100 {
            list.pop_back();
        }
    }

    pub async fn get_traces(&self, limit: Option<usize>) -> Vec<RequestTrace> {
        let list = self.traces.lock().await;
        let cap = limit.unwrap_or(50).min(list.len());
        list.iter().rev().take(cap).cloned().collect()
    }

    pub async fn clear_traces(&self) {
        let mut list = self.traces.lock().await;
        list.clear();
    }

    pub fn active_requests(&self) -> u64 {
        self.active_requests.load(Ordering::Relaxed)
    }

    pub fn total_routed(&self) -> u64 {
        self.total_routed.load(Ordering::Relaxed)
    }

    fn router(&self) -> Router {
        let models = self.clone();
        let chat = self.clone();
        let messages = self.clone();
        let responses1 = self.clone();
        let responses2 = self.clone();
        Router::new()
            .route(
                "/v1/models",
                get(move |req| {
                    let g = models.clone();
                    async move { g.forward(req, true).await }
                }),
            )
            .route(
                "/v1/chat/completions",
                post(move |req| {
                    let g = chat.clone();
                    async move { g.handle_chat_completions(req).await }
                }),
            )
            .route(
                "/v1/messages",
                post(move |req| {
                    let g = messages.clone();
                    async move { g.handle_messages(req).await }
                }),
            )
            .route(
                "/v1/responses",
                post(move |req| {
                    let g = responses1.clone();
                    async move { g.handle_responses(req).await }
                }),
            )
            .route(
                "/responses",
                post(move |req| {
                    let g = responses2.clone();
                    async move { g.handle_responses(req).await }
                }),
            )
            .layer(ValidateRequestHeaderLayer::custom(self.auth.clone()))
    }

    async fn handle_chat_completions(&self, req: Request<Body>) -> Response<Body> {
        self.dispatch_protocol(req, IngressProtocol::ChatCompletions)
            .await
    }

    async fn handle_messages(&self, req: Request<Body>) -> Response<Body> {
        self.dispatch_protocol(req, IngressProtocol::Messages).await
    }

    async fn handle_responses(&self, req: Request<Body>) -> Response<Body> {
        self.dispatch_protocol(req, IngressProtocol::Responses)
            .await
    }

    async fn dispatch_protocol(
        &self,
        req: Request<Body>,
        ingress: IngressProtocol,
    ) -> Response<Body> {
        self.total_routed.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_add(1, Ordering::Relaxed);
        struct ActiveGuard(Arc<AtomicU64>);
        impl Drop for ActiveGuard {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::Relaxed);
            }
        }
        let _active_guard = ActiveGuard(self.active_requests.clone());

        let client_ip = extract_client_ip(req.headers());
        let session_key = extract_session_key(req.headers(), client_ip.as_deref(), None);

        // Header-based routing target
        let header_target = req
            .headers()
            .get("x-proxyweave-target")
            .or_else(|| req.headers().get("x-target-plugin"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.trim().to_string());

        // Extract body bytes to inspect model prefix if header is absent
        let (parts, body) = req.into_parts();
        let body_bytes = match axum::body::to_bytes(body, 16 * 1024 * 1024).await {
            Ok(b) => b,
            Err(_) => {
                return error_with_message(
                    StatusCode::BAD_REQUEST,
                    "bad_request",
                    "Failed to read request body payload",
                );
            }
        };

        let mut model_target = None;
        if header_target.is_none() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                if let Some(m) = val.get("model").and_then(|m| m.as_str()) {
                    if let Some((prefix, _)) = m.split_once('/') {
                        if !prefix.is_empty() {
                            model_target = Some(prefix.to_string());
                        }
                    }
                }
            }
        }

        let target_plugin = header_target.or(model_target);

        let (candidates, strategy, is_sticky) = self
            .route_pool
            .select_candidates_for_target(session_key.as_deref(), target_plugin.as_deref())
            .await;

        let (backend, route_strategy) = if let Some(first) = candidates.first() {
            let label = if is_sticky {
                format!("{} (Sticky)", strategy.as_str())
            } else {
                strategy.as_str().to_string()
            };
            (first.clone(), label)
        } else if let Some(target) = target_plugin {
            // Explicit target requested but not found or ready in route pool -> 502 Bad Gateway
            return error_with_message(
                StatusCode::BAD_GATEWAY,
                "bad_gateway",
                &format!("Target plugin '{}' is not registered or unavailable", target),
            );
        } else {
            let guard = self.backend.read().await;
            match guard.as_ref() {
                Some(b) => (b.clone(), "Direct".to_string()),
                None => {
                    return error_with_message(
                        StatusCode::BAD_GATEWAY,
                        "bad_gateway",
                        "No active plugin backend available",
                    );
                }
            }
        };

        let req = Request::from_parts(parts, Body::from(body_bytes));

        if ingress.matches_backend_protocol(&backend.protocol) {
            self.forward_with_backend(req, false, backend, route_strategy, session_key)
                .await
        } else {
            self.transcode(req, ingress, backend, route_strategy, session_key)
                .await
        }
    }

    async fn forward(&self, req: Request<Body>, models: bool) -> Response<Body> {
        self.total_routed.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_add(1, Ordering::Relaxed);
        struct ActiveGuard(Arc<AtomicU64>);
        impl Drop for ActiveGuard {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::Relaxed);
            }
        }
        let _active_guard = ActiveGuard(self.active_requests.clone());

        let target_header = req
            .headers()
            .get("x-proxyweave-target")
            .or_else(|| req.headers().get("x-target-plugin"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.trim().to_string());

        let backend = {
            let (candidates, _, _) = self
                .route_pool
                .select_candidates_for_target(None, target_header.as_deref())
                .await;
            if let Some(first) = candidates.first() {
                first.clone()
            } else if let Some(b) = self.backend.read().await.clone() {
                b
            } else {
                let timestamp_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let client_name = detect_client_name(req.headers());
                let client_ip = extract_client_ip(req.headers());
                let method = req.method().to_string();
                let original_path = req.uri().path().to_string();
                let ingress_protocol = if models {
                    "models".to_string()
                } else {
                    "passthrough".to_string()
                };
                self.record_trace(RequestTrace {
                    id: format!("trc_{}", timestamp_ms),
                    timestamp_ms,
                    client: client_name,
                    client_ip,
                    method,
                    path: original_path,
                    model: None,
                    ingress_protocol,
                    route_strategy: "Direct".to_string(),
                    target_plugin: target_header.unwrap_or_else(|| "none".to_string()),
                    status_code: 502,
                    duration_ms: 0,
                    first_token_ms: None,
                    input_tokens: None,
                    output_tokens: None,
                    cache_hit: None,
                    fallback_trail: Vec::new(),
                    error: Some("backend_unavailable".to_string()),
                })
                .await;
                return error_with_message(
                    StatusCode::BAD_GATEWAY,
                    "bad_gateway",
                    "No active plugin backend available",
                );
            }
        };

        let strategy_str = if models {
            "Direct (Models)".to_string()
        } else {
            "Direct (Passthrough)".to_string()
        };
        self.forward_with_backend(req, models, backend, strategy_str, None)
            .await
    }

    async fn forward_with_backend(
        &self,
        mut req: Request<Body>,
        models: bool,
        mut backend: Backend,
        route_strategy: String,
        session_key: Option<String>,
    ) -> Response<Body> {
        let start_time = Instant::now();
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let client_name = detect_client_name(req.headers());
        let client_ip = extract_client_ip(req.headers());
        let method = req.method().to_string();
        let original_path = req.uri().path().to_string();
        let ingress_protocol = if models {
            "models".to_string()
        } else {
            "passthrough".to_string()
        };

        let lease = match self.acquire_lease_following_route(&mut backend).await {
            Ok(lease) => {
                if let Some(expected_gen) = backend.instance_generation {
                    if lease.instance_generation() != expected_gen {
                        let duration_ms = start_time.elapsed().as_millis() as u64;
                        self.record_trace(RequestTrace {
                            id: format!("trc_{}", timestamp_ms),
                            timestamp_ms,
                            client: client_name,
                            client_ip,
                            method,
                            path: original_path,
                            model: None,
                            ingress_protocol,
                            route_strategy: route_strategy.clone(),
                            target_plugin: backend.plugin_id.clone(),
                            status_code: 503,
                            duration_ms,
                            first_token_ms: None,
                            input_tokens: None,
                            output_tokens: None,
                            cache_hit: None,
                            fallback_trail: vec![format!(
                                "Stale generation for {}: expected {}, got {}",
                                backend.plugin_id,
                                expected_gen,
                                lease.instance_generation()
                            )],
                            error: Some("stale_backend_generation".to_string()),
                        })
                        .await;
                        self.route_pool
                            .record_outcome(
                                &backend.plugin_id,
                                duration_ms,
                                503,
                                session_key.as_deref(),
                            )
                            .await;
                        return error(StatusCode::SERVICE_UNAVAILABLE, "stale_backend_generation");
                    }
                }
                lease
            }
            Err(e) => {
                let err_str = match &e {
                    RuntimeError::StaleGeneration { .. } => "stale_backend_generation",
                    RuntimeError::ServiceUnderMaintenance => "service_under_maintenance",
                    _ => "backend_unavailable",
                };
                let duration_ms = start_time.elapsed().as_millis() as u64;
                self.record_trace(RequestTrace {
                    id: format!("trc_{}", timestamp_ms),
                    timestamp_ms,
                    client: client_name,
                    client_ip,
                    method,
                    path: original_path,
                    model: None,
                    ingress_protocol,
                    route_strategy: route_strategy.clone(),
                    target_plugin: backend.plugin_id.clone(),
                    status_code: 503,
                    duration_ms,
                    first_token_ms: None,
                    input_tokens: None,
                    output_tokens: None,
                    cache_hit: None,
                    fallback_trail: vec![format!(
                        "Failed to acquire lease for {}: {}",
                        backend.plugin_id, e
                    )],
                    error: Some(err_str.to_string()),
                })
                .await;
                self.route_pool
                    .record_outcome(&backend.plugin_id, duration_ms, 503, session_key.as_deref())
                    .await;
                return error(StatusCode::SERVICE_UNAVAILABLE, err_str);
            }
        };
        // No secrets/userinfo in target constructors or their panic messages.
        let endpoint = match url::Url::parse(lease.endpoint()) {
            Ok(url)
                if matches!(url.scheme(), "http" | "https")
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none() =>
            {
                url
            }
            _ => return error(StatusCode::BAD_GATEWAY, "invalid_backend"),
        };
        let path = if models {
            &backend.models_path
        } else {
            &backend.inference_path
        };
        let path_query = match req.uri().query() {
            Some(query) => format!("{path}?{query}"),
            None => path.clone(),
        };
        req.extensions_mut().clear();
        strip_client_headers(req.headers_mut());
        apply_upstream_headers(req.headers_mut(), &backend);
        *req.uri_mut() = match path_query.parse() {
            Ok(uri) => uri,
            Err(_) => return error(StatusCode::BAD_REQUEST, "invalid_request_uri"),
        };
        let proxy = ReverseProxy::new_with_client("", endpoint.as_str(), self.client.clone())
            .with_policy(
                ProxyPolicy::new()
                    .with_x_forwarded_for(XForwardedFor::Preserve)
                    .with_upstream_timeout(self.header_timeout),
            );
        // Upstream trace records raw request URI/query and error chains. Do not
        // attach the application's subscriber to this data-plane future.
        let response = match proxy
            .proxy_request(req)
            .with_subscriber(tracing::subscriber::NoSubscriber::default())
            .await
        {
            Ok(r) => r,
            Err(e) => match e {},
        };
        let status_code = response.status().as_u16();
        let duration_ms = start_time.elapsed().as_millis() as u64;
        self.record_trace(RequestTrace {
            id: format!("trc_{}", timestamp_ms),
            timestamp_ms,
            client: client_name,
            client_ip,
            method,
            path: original_path,
            model: None,
            ingress_protocol,
            route_strategy,
            target_plugin: backend.plugin_id.clone(),
            status_code,
            duration_ms,
            first_token_ms: None,
            input_tokens: None,
            output_tokens: None,
            cache_hit: None,
            fallback_trail: Vec::new(),
            error: if status_code >= 400 {
                Some(format!("HTTP {}", status_code))
            } else {
                None
            },
        })
        .await;
        self.route_pool
            .record_outcome(
                &backend.plugin_id,
                duration_ms,
                status_code,
                session_key.as_deref(),
            )
            .await;
        let (mut parts, body) = response.into_parts();
        parts.extensions.clear(); // do not expose/log raw ProxyError chains through other middleware
                                  // Hold admission through connect, header wait, every body frame and body Drop.
        Response::from_parts(parts, Body::new(LeasedBody::new(body, lease)))
    }

    async fn transcode(
        &self,
        req: Request<Body>,
        ingress: IngressProtocol,
        mut backend: Backend,
        route_strategy: String,
        session_key: Option<String>,
    ) -> Response<Body> {
        let start_time = Instant::now();
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let client_name = detect_client_name(req.headers());
        let client_ip = extract_client_ip(req.headers());
        let method = req.method().to_string();
        let original_path = req.uri().path().to_string();
        let ingress_protocol_str = format!("{:?}", ingress);

        let lease = match self.acquire_lease_following_route(&mut backend).await {
            Ok(lease) => {
                if let Some(expected_gen) = backend.instance_generation {
                    if lease.instance_generation() != expected_gen {
                        let duration_ms = start_time.elapsed().as_millis() as u64;
                        self.record_trace(RequestTrace {
                            id: format!("trc_{}", timestamp_ms),
                            timestamp_ms,
                            client: client_name,
                            client_ip,
                            method,
                            path: original_path,
                            model: None,
                            ingress_protocol: ingress_protocol_str,
                            route_strategy: route_strategy.clone(),
                            target_plugin: backend.plugin_id.clone(),
                            status_code: 503,
                            duration_ms,
                            first_token_ms: None,
                            input_tokens: None,
                            output_tokens: None,
                            cache_hit: None,
                            fallback_trail: vec![format!(
                                "Stale generation for {}: expected {}, got {}",
                                backend.plugin_id,
                                expected_gen,
                                lease.instance_generation()
                            )],
                            error: Some("stale_backend_generation".to_string()),
                        })
                        .await;
                        self.route_pool
                            .record_outcome(
                                &backend.plugin_id,
                                duration_ms,
                                503,
                                session_key.as_deref(),
                            )
                            .await;
                        return error(StatusCode::SERVICE_UNAVAILABLE, "stale_backend_generation");
                    }
                }
                lease
            }
            Err(e) => {
                let err_str = match &e {
                    RuntimeError::StaleGeneration { .. } => "stale_backend_generation",
                    RuntimeError::ServiceUnderMaintenance => "service_under_maintenance",
                    _ => "backend_unavailable",
                };
                let duration_ms = start_time.elapsed().as_millis() as u64;
                self.record_trace(RequestTrace {
                    id: format!("trc_{}", timestamp_ms),
                    timestamp_ms,
                    client: client_name,
                    client_ip,
                    method,
                    path: original_path,
                    model: None,
                    ingress_protocol: ingress_protocol_str,
                    route_strategy: route_strategy.clone(),
                    target_plugin: backend.plugin_id.clone(),
                    status_code: 503,
                    duration_ms,
                    first_token_ms: None,
                    input_tokens: None,
                    output_tokens: None,
                    cache_hit: None,
                    fallback_trail: vec![format!(
                        "Failed to acquire lease for {}: {}",
                        backend.plugin_id, e
                    )],
                    error: Some(err_str.to_string()),
                })
                .await;
                self.route_pool
                    .record_outcome(&backend.plugin_id, duration_ms, 503, session_key.as_deref())
                    .await;
                return error(StatusCode::SERVICE_UNAVAILABLE, err_str);
            }
        };

        let endpoint = match url::Url::parse(lease.endpoint()) {
            Ok(url)
                if matches!(url.scheme(), "http" | "https")
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none() =>
            {
                url
            }
            _ => return error(StatusCode::BAD_GATEWAY, "invalid_backend"),
        };

        let (ingress_codec, ingress_suite): (Box<dyn EndpointCodec>, ProtocolSuite) = match ingress
        {
            IngressProtocol::ChatCompletions => (
                Box::new(ChatCompletionsCodec::new()),
                ProtocolSuite::OpenAiCompatible,
            ),
            IngressProtocol::Messages => (
                Box::new(MessagesCodec::new()),
                ProtocolSuite::AnthropicMessages,
            ),
            IngressProtocol::Responses => (
                Box::new(ResponsesCodec::new()),
                ProtocolSuite::OpenAiResponses,
            ),
        };

        let egress_codec = IngressProtocol::from_backend(&backend.protocol)
            .expect("validated canonical backend protocol")
            .codec();

        let query = req.uri().query().map(|q| q.to_string());
        let mut headers = req.headers().clone();
        strip_client_headers(&mut headers);

        let body_bytes = match req.into_body().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => return error(StatusCode::BAD_REQUEST, "invalid_request_body"),
        };

        let body_json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
            Ok(v) => v,
            Err(_) => {
                let err_body = encode_error_body_for_suite(
                    ingress_suite,
                    "Request body must be valid JSON",
                    ErrorClass::BadRequest,
                    400,
                    None,
                );
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&err_body).unwrap_or_default(),
                    ))
                    .expect("static response");
            }
        };

        let raw_env = RawEnvelope::default();
        let ir_request = match ingress_codec.decode_request(body_json, &raw_env) {
            Ok(ir) => ir,
            Err(e) => {
                let err_body = encode_error_body_for_suite(
                    ingress_suite,
                    &format!("Decode request error: {e}"),
                    ErrorClass::BadRequest,
                    400,
                    None,
                );
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&err_body).unwrap_or_default(),
                    ))
                    .expect("static response");
            }
        };

        let is_stream = ir_request.stream;
        let requested_model = ir_request.model.clone();

        if let Err((dimension, err)) =
            check_lossy_conversion(&ir_request, egress_codec.id(), egress_codec.capabilities())
        {
            drop(lease);
            let err_body = encode_error_body_for_suite(
                ingress_suite,
                &format!("Lossy conversion rejected on {dimension:?}: {err}"),
                ErrorClass::LossyOrCapability,
                400,
                Some("unsupported_protocol_feature"),
            );
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&err_body).unwrap_or_default(),
                ))
                .expect("static response");
        }

        let (upstream_json, extra_headers) = match egress_codec.encode_request(&ir_request) {
            Ok(res) => res,
            Err(e) => {
                let err_body = encode_error_body_for_suite(
                    ingress_suite,
                    &format!("Encode request error: {e}"),
                    ErrorClass::BadRequest,
                    400,
                    None,
                );
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&err_body).unwrap_or_default(),
                    ))
                    .expect("static response");
            }
        };

        let path = &backend.inference_path;
        let path_query = match query {
            Some(q) => format!("{path}?{q}"),
            None => path.clone(),
        };
        let upstream_url = format!("{}{}", endpoint.as_str().trim_end_matches('/'), path_query);

        let mut req_builder = Request::builder()
            .method("POST")
            .uri(&upstream_url)
            .header(header::CONTENT_TYPE, "application/json");

        for (k, v) in extra_headers.iter() {
            req_builder = req_builder.header(k, v);
        }

        if is_stream {
            req_builder = req_builder.header(header::ACCEPT, "text/event-stream");
        }

        for (name, val) in headers.iter() {
            if !req_builder
                .headers_ref()
                .is_some_and(|h| h.contains_key(name))
            {
                req_builder = req_builder.header(name, val);
            }
        }
        apply_upstream_headers(req_builder.headers_mut().expect("valid headers"), &backend);
        let upstream_body_bytes = match serde_json::to_vec(&upstream_json) {
            Ok(b) => b,
            Err(_) => {
                drop(lease);
                return error(StatusCode::INTERNAL_SERVER_ERROR, "serialization_error");
            }
        };
        let upstream_req = match req_builder.body(Body::from(upstream_body_bytes)) {
            Ok(r) => r,
            Err(_) => {
                drop(lease);
                return error(StatusCode::INTERNAL_SERVER_ERROR, "request_build_error");
            }
        };

        let upstream_resp = match tokio::time::timeout(
            self.header_timeout,
            self.client.request(upstream_req),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                drop(lease);
                let err_body = encode_error_body_for_suite(
                    ingress_suite,
                    &format!("Upstream error: {e}"),
                    ErrorClass::Transient,
                    502,
                    None,
                );
                return Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&err_body).unwrap_or_default(),
                    ))
                    .expect("static response");
            }
            Err(_) => {
                drop(lease);
                let err_body = encode_error_body_for_suite(
                    ingress_suite,
                    "Gateway timeout waiting for upstream headers",
                    ErrorClass::DeadlineExceeded,
                    504,
                    Some("gateway_timeout"),
                );
                return Response::builder()
                    .status(StatusCode::GATEWAY_TIMEOUT)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&err_body).unwrap_or_default(),
                    ))
                    .expect("static response");
            }
        };

        let status = upstream_resp.status();
        if !status.is_success() {
            let body_bytes = match upstream_resp.into_body().collect().await {
                Ok(c) => c.to_bytes(),
                Err(_) => Bytes::new(),
            };
            let err_val: serde_json::Value =
                serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);
            let err_msg = err_val["error"]["message"]
                .as_str()
                .or_else(|| err_val["message"].as_str())
                .unwrap_or("Upstream error");
            let err_code = err_val["error"]["code"]
                .as_str()
                .or_else(|| err_val["error"]["type"].as_str());

            let error_class =
                proxyweave_protocols::classify_upstream_error(Some(status.as_u16()), err_code);
            let client_err_body = encode_error_body_for_suite(
                ingress_suite,
                err_msg,
                error_class,
                status.as_u16(),
                err_code,
            );
            let duration_ms = start_time.elapsed().as_millis() as u64;
            self.record_trace(RequestTrace {
                id: format!("trc_{}", timestamp_ms),
                timestamp_ms,
                client: client_name.clone(),
                client_ip: client_ip.clone(),
                method: method.clone(),
                path: original_path.clone(),
                model: Some(requested_model.clone()),
                ingress_protocol: ingress_protocol_str.clone(),
                route_strategy: route_strategy.clone(),
                target_plugin: backend.plugin_id.clone(),
                status_code: status.as_u16(),
                duration_ms,
                first_token_ms: None,
                input_tokens: None,
                output_tokens: None,
                cache_hit: None,
                fallback_trail: if status.as_u16() == 429 || status.as_u16() >= 500 {
                    vec![format!(
                        "Upstream {} returned {}: {}",
                        backend.plugin_id,
                        status.as_u16(),
                        err_msg
                    )]
                } else {
                    Vec::new()
                },
                error: Some(err_msg.to_string()),
            })
            .await;
            self.route_pool
                .record_outcome(
                    &backend.plugin_id,
                    duration_ms,
                    status.as_u16(),
                    session_key.as_deref(),
                )
                .await;
            return Response::builder()
                .status(status)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&client_err_body).unwrap_or_default(),
                ))
                .expect("static response");
        }

        if !is_stream {
            let body_bytes = match upstream_resp.into_body().collect().await {
                Ok(c) => c.to_bytes(),
                Err(_) => return error(StatusCode::BAD_GATEWAY, "upstream_body_error"),
            };
            let upstream_json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
                Ok(v) => v,
                Err(_) => return error(StatusCode::BAD_GATEWAY, "invalid_upstream_json"),
            };
            let ir_resp = match egress_codec.decode_response(upstream_json) {
                Ok(ir) => ir,
                Err(e) => {
                    let err_body = encode_error_body_for_suite(
                        ingress_suite,
                        &format!("Decode upstream response error: {e}"),
                        ErrorClass::Transient,
                        502,
                        None,
                    );
                    return Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(
                            serde_json::to_vec(&err_body).unwrap_or_default(),
                        ))
                        .expect("static response");
                }
            };
            let mut client_json = match ingress_codec.encode_response(&ir_resp) {
                Ok(val) => val,
                Err(e) => {
                    let err_body = encode_error_body_for_suite(
                        ingress_suite,
                        &format!("Encode client response error: {e}"),
                        ErrorClass::Transient,
                        500,
                        None,
                    );
                    return Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(
                            serde_json::to_vec(&err_body).unwrap_or_default(),
                        ))
                        .expect("static response");
                }
            };
            if client_json.is_object() {
                if let Some(m) = client_json.get_mut("model") {
                    if m.as_str().is_some_and(|s| s.is_empty()) && !requested_model.is_empty() {
                        *m = serde_json::Value::String(requested_model.clone());
                    }
                }
            }
            let resp_bytes = match serde_json::to_vec(&client_json) {
                Ok(b) => b,
                Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "serialization_error"),
            };
            let input_tokens = ir_resp.usage.as_ref().map(|u| u.prompt_tokens);
            let output_tokens = ir_resp.usage.as_ref().map(|u| u.completion_tokens);
            let cache_hit = ir_resp
                .usage
                .as_ref()
                .and_then(|u| u.cache_read_tokens.map(|c| c > 0));
            let duration_ms = start_time.elapsed().as_millis() as u64;
            self.record_trace(RequestTrace {
                id: format!("trc_{}", timestamp_ms),
                timestamp_ms,
                client: client_name.clone(),
                client_ip: client_ip.clone(),
                method: method.clone(),
                path: original_path.clone(),
                model: Some(requested_model.clone()),
                ingress_protocol: ingress_protocol_str.clone(),
                route_strategy: route_strategy.clone(),
                target_plugin: backend.plugin_id.clone(),
                status_code: 200,
                duration_ms,
                first_token_ms: None,
                input_tokens,
                output_tokens,
                cache_hit,
                fallback_trail: Vec::new(),
                error: None,
            })
            .await;
            self.route_pool
                .record_outcome(&backend.plugin_id, duration_ms, 200, session_key.as_deref())
                .await;
            let mut resp_builder = Response::builder().status(StatusCode::OK);
            resp_builder = resp_builder.header(header::CONTENT_TYPE, "application/json");
            resp_builder
                .body(Body::new(LeasedBody::new(Body::from(resp_bytes), lease)))
                .expect("static response")
        } else {
            use eventsource_stream::Eventsource;
            use futures_util::StreamExt;

            let mut decoder = egress_codec.stream_decoder();
            let mut encoder = ingress_codec.stream_encoder();
            let mut events = upstream_resp.into_body().into_data_stream().eventsource();
            let traces_ref = self.traces.clone();
            let stream_client_name = client_name.clone();
            let stream_client_ip = client_ip.clone();
            let stream_method = method.clone();
            let stream_path = original_path.clone();
            let stream_model = requested_model.clone();
            let stream_ingress = ingress_protocol_str.clone();
            let stream_plugin = backend.plugin_id.clone();
            let stream_route_strategy = route_strategy.clone();
            let stream_session_key = session_key.clone();
            let route_pool_ref = self.route_pool.clone();
            let stream = async_stream::stream! {
                let _lease = lease;
                let mut saw_terminal = false;
                let mut first_token_ms: Option<u64> = None;
                let mut stream_input_tokens: Option<u64> = None;
                let mut stream_output_tokens: Option<u64> = None;
                let mut stream_cache_hit: Option<bool> = None;
                let mut stream_error: Option<String> = None;
                let mut stream_status: u16 = 200;

                while let Some(event) = events.next().await {
                    let event = match event {
                        Ok(event) => event,
                        Err(_) => {
                            stream_status = 502;
                            stream_error = Some("invalid upstream SSE transport or UTF-8".to_string());
                            yield Err::<Bytes, io::Error>(io::Error::other("invalid upstream SSE transport or UTF-8"));
                            break;
                        }
                    };
                    let parts = match decoder.feed(&format!("data: {}", event.data)) {
                        Ok(parts) => parts,
                        Err(_) => {
                            stream_status = 502;
                            stream_error = Some("invalid upstream protocol event".to_string());
                            yield Err::<Bytes, io::Error>(io::Error::other("invalid upstream protocol event"));
                            break;
                        }
                    };
                    for part in &parts {
                        match part {
                            StreamPart::TextDelta { .. } | StreamPart::ReasoningDelta { .. } => {
                                if first_token_ms.is_none() {
                                    first_token_ms = Some(start_time.elapsed().as_millis() as u64);
                                }
                            }
                            StreamPart::Usage { usage } => {
                                stream_input_tokens = Some(usage.prompt_tokens);
                                stream_output_tokens = Some(usage.completion_tokens);
                                if let Some(c) = usage.cache_read_tokens {
                                    stream_cache_hit = Some(c > 0);
                                }
                            }
                            StreamPart::ResponseCompleted { usage, .. } => {
                                saw_terminal = true;
                                if let Some(u) = usage {
                                    stream_input_tokens = Some(u.prompt_tokens);
                                    stream_output_tokens = Some(u.completion_tokens);
                                    if let Some(c) = u.cache_read_tokens {
                                        stream_cache_hit = Some(c > 0);
                                    }
                                }
                            }
                            StreamPart::Finish { .. } => {
                                saw_terminal = true;
                            }
                            StreamPart::Error { message, .. } => {
                                stream_status = 502;
                                stream_error = Some(message.clone());
                            }
                            _ => {}
                        }
                        match encoder.encode_part(part) {
                            Ok(bytes) if !bytes.is_empty() => yield Ok(Bytes::from(bytes)),
                            Ok(_) => {},
                            Err(_) => {
                                stream_status = 502;
                                stream_error = Some("upstream protocol conversion failed".to_string());
                                yield Err(io::Error::other("upstream protocol conversion failed"));
                                break;
                            }
                        }
                        if matches!(part, StreamPart::Error { .. }) {
                            break;
                        }
                    }
                    if stream_error.is_some() {
                        break;
                    }
                }
                // HTTP EOF never proves protocol completion. Fail transport, not a synthetic DONE.
                if !saw_terminal && stream_error.is_none() {
                    stream_status = 502;
                    stream_error = Some("upstream SSE ended without a protocol terminal event".to_string());
                    yield Err(io::Error::new(io::ErrorKind::UnexpectedEof, "upstream SSE ended without a protocol terminal event"));
                }

                let duration_ms = start_time.elapsed().as_millis() as u64;
                let mut lock = traces_ref.lock().await;
                if lock.len() >= 100 {
                    lock.pop_front();
                }
                lock.push_back(RequestTrace {
                    id: format!("trc_{}", timestamp_ms),
                    timestamp_ms,
                    client: stream_client_name,
                    client_ip: stream_client_ip,
                    method: stream_method,
                    path: stream_path,
                    model: Some(stream_model),
                    ingress_protocol: stream_ingress,
                    route_strategy: stream_route_strategy,
                    target_plugin: stream_plugin.clone(),
                    status_code: stream_status,
                    duration_ms,
                    first_token_ms,
                    input_tokens: stream_input_tokens,
                    output_tokens: stream_output_tokens,
                    cache_hit: stream_cache_hit,
                    fallback_trail: Vec::new(),
                    error: stream_error,
                });
                route_pool_ref
                    .record_outcome(&stream_plugin, duration_ms, stream_status, stream_session_key.as_deref())
                    .await;
            };

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
                .header(header::CACHE_CONTROL, "no-cache")
                .header(header::CONNECTION, "keep-alive")
                .body(Body::from_stream(stream))
                .expect("static response")
        }
    }

    /// F3 serves loopback HTTP/1.1; remote access/HTTP2/upgrades are not in this acceptance.
    /// The connection task owns every handler future. A TCP disconnect drops the connection
    /// and handler; shutdown aborts/drains owned connections rather than waiting on infinite SSE.
    /// This is a bounded adapter of axum::serve's official TokioIo/TowerToHyperService loop.
    pub async fn listen(&self, port: u16) -> io::Result<GatewayHandle> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
        let address = listener.local_addr()?;
        let router = self.router();
        let cancel = CancellationToken::new();
        let shutdown = cancel.clone();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => break,
                    joined = connections.join_next(), if !connections.is_empty() => { let _ = joined; },
                    accepted = listener.accept() => {
                        let (stream, _) = accepted?;
                        let service = TowerToHyperService::new(router.clone());
                        connections.spawn(async move {
                            let mut builder = hyper::server::conn::http1::Builder::new();
                            builder.half_close(false);
                            let _ = builder.serve_connection(TokioIo::new(stream), service).await;
                        });
                    }
                }
            }
            connections.abort_all();
            while connections.join_next().await.is_some() {}
            Ok(())
        });
        Ok(GatewayHandle {
            address,
            cancel,
            task,
        })
    }
}

pub struct GatewayHandle {
    address: SocketAddr,
    cancel: CancellationToken,
    task: JoinHandle<io::Result<()>>,
}
impl GatewayHandle {
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub async fn shutdown(mut self) -> io::Result<()> {
        self.cancel.cancel();
        (&mut self.task)
            .await
            .map_err(|_| io::Error::other("Gateway task failed"))?
    }
}
impl Drop for GatewayHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.task.abort();
    }
}

fn sensitive_bearer(key: &str) -> Result<HeaderValue, &'static str> {
    if key.is_empty() || key.len() > 4096 || !key.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("Invalid API key");
    }
    let mut value =
        HeaderValue::from_str(&format!("Bearer {key}")).map_err(|_| "Invalid API key")?;
    value.set_sensitive(true);
    Ok(value)
}
fn error(status: StatusCode, code: &'static str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(r#"{{"error":{{"code":"{code}"}}}}"#)))
        .expect("static response")
}

fn error_with_message(status: StatusCode, code: &'static str, message: &str) -> Response<Body> {
    let payload = serde_json::json!({
        "error": {
            "message": message,
            "type": code,
            "code": status.as_u16(),
        }
    });
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap_or_default()))
        .expect("static response")
}
struct LeasedBody {
    inner: Body,
    lease: Option<RequestLease>,
}
impl LeasedBody {
    fn new(inner: Body, lease: RequestLease) -> Self {
        let lease = if inner.is_end_stream() {
            None
        } else {
            Some(lease)
        };
        Self { inner, lease }
    }
}
impl HttpBody for LeasedBody {
    type Data = Bytes;
    type Error = axum::Error;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        let frame = Pin::new(&mut self.inner).poll_frame(cx);
        if matches!(frame, Poll::Ready(None) | Poll::Ready(Some(Err(_))))
            || self.inner.is_end_stream()
        {
            self.lease.take();
        }
        frame
    }
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IngressProtocol {
    ChatCompletions,
    Messages,
    Responses,
}

impl IngressProtocol {
    fn from_backend(protocol: &str) -> Option<Self> {
        match protocol {
            "openai_chat" => Some(Self::ChatCompletions),
            "openai_responses" => Some(Self::Responses),
            "anthropic_messages" => Some(Self::Messages),
            _ => None,
        }
    }

    fn matches_backend_protocol(&self, backend_protocol: &str) -> bool {
        Self::from_backend(backend_protocol) == Some(*self)
    }

    fn codec(self) -> Box<dyn EndpointCodec> {
        match self {
            Self::ChatCompletions => Box::new(ChatCompletionsCodec::new()),
            Self::Responses => Box::new(ResponsesCodec::new()),
            Self::Messages => Box::new(MessagesCodec::new()),
        }
    }
}
fn is_hop_by_hop_or_sensitive(name: &header::HeaderName) -> bool {
    let lower = name.as_str().to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "authorization"
            | "proxy-authorization"
            | "x-api-key"
            | "anthropic-version"
            | "cookie"
            | "connection"
            | "host"
            | "content-length"
            | "keep-alive"
            | "proxy-connection"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    ) || lower.starts_with("sec-websocket-")
}

/// Strip Connection nominations before adding independent backend credentials.
fn strip_client_headers(headers: &mut axum::http::HeaderMap) {
    let nominated: Vec<_> = headers
        .get_all(header::CONNECTION)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|name| header::HeaderName::from_bytes(name.trim().as_bytes()).ok())
        .collect();
    let stripped: Vec<_> = headers
        .keys()
        .filter(|name| is_hop_by_hop_or_sensitive(name) || nominated.contains(name))
        .cloned()
        .collect();
    for name in stripped {
        headers.remove(name);
    }
}

fn apply_upstream_headers(headers: &mut axum::http::HeaderMap, backend: &Backend) {
    // TiyGate auth/{api_key,bearer}.rs; MessagesCodec's canonical version contract.
    if backend.protocol == "anthropic_messages" {
        headers.insert("x-api-key", backend.credential.clone());
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    } else {
        headers.insert(header::AUTHORIZATION, backend.credential.clone());
    }
}

fn sensitive_api_key(key: &str) -> Result<HeaderValue, &'static str> {
    if key.is_empty() || key.len() > 4096 || !key.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("Invalid API key");
    }
    let mut value = HeaderValue::from_str(key).map_err(|_| "Invalid API key")?;
    value.set_sensitive(true);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_local_key_updates_gateway_auth_and_rejects_invalid_keys() {
        let initial_key = "initial-local-gateway-key-0123456789abcdef";
        let rotated_key = "rotated-local-gateway-key-fedcba9876543210";
        let gateway = Gateway::new(RuntimeManager::new(), initial_key, Duration::from_secs(5))
            .expect("gateway init");

        let handle = gateway.listen(0).await.expect("listen");
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let url = format!("http://{}/v1/models", handle.address());

        assert_eq!(
            client
                .get(&url)
                .bearer_auth(initial_key)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            client
                .get(&url)
                .bearer_auth(rotated_key)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );

        for invalid in [
            "too-short-key",
            "invalid key with spaces 0123456789abcdef",
            "invalid-key-with-null\x000123456789abcdef",
            "invalid-key-non-ascii-中文-0123456789abcdef",
            &"x".repeat(513),
        ] {
            assert!(gateway.set_local_key(invalid).is_err());
        }

        gateway
            .set_local_key(rotated_key)
            .expect("valid rotated key");

        assert_eq!(
            client
                .get(&url)
                .bearer_auth(initial_key)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            client
                .get(&url)
                .bearer_auth(rotated_key)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );

        handle.shutdown().await.expect("shutdown");

        let handle2 = gateway.listen(0).await.expect("re-listen");
        let url2 = format!("http://{}/v1/models", handle2.address());
        assert_eq!(
            client
                .get(&url2)
                .bearer_auth(initial_key)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            client
                .get(&url2)
                .bearer_auth(rotated_key)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        handle2.shutdown().await.expect("shutdown 2");
    }

    #[tokio::test]
    async fn gateway_rejects_stale_backend_generation_and_maintenance() {
        let key = "test-gateway-key-0123456789abcdef";
        let runtime = RuntimeManager::new();
        let gateway =
            Gateway::new(runtime.clone(), key, Duration::from_secs(5)).expect("gateway init");

        let handle = gateway.listen(0).await.expect("listen");
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let models_url = format!("http://{}/v1/models", handle.address());

        // Configure a backend with expected instance_generation = Some(42)
        let backend = Backend {
            plugin_id: "test-plug".to_string(),
            protocol: "openai_chat".to_string(),
            models_path: "/v1/models".to_string(),
            inference_path: "/v1/chat/completions".to_string(),
            credential: HeaderValue::from_static("Bearer test"),
            instance_generation: Some(42),
        };
        gateway.set_backend_for_test(Some(backend)).await;

        // When runtime has no such plugin running, acquire_request_for fails with PluginNotRunning -> 503 backend_unavailable
        let resp = client
            .get(&models_url)
            .bearer_auth(key)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = resp.text().await.unwrap();
        assert!(body.contains("backend_unavailable"));

        handle.shutdown().await.expect("shutdown");
    }
}