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
use proxyweave_plugin::{Manifest, RequestLease, RuntimeManager};
use proxyweave_protocols::{
    chat_completions::ChatCompletionsCodec, error_body::encode_error_body_for_suite,
    messages::MessagesCodec, protocol::lossy::check_lossy_conversion, responses::ResponsesCodec,
    EndpointCodec, ErrorClass, ProtocolSuite, RawEnvelope, StreamPart,
};
use std::{
    io,
    net::{Ipv4Addr, SocketAddr},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
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

#[derive(Clone)]
struct Backend {
    plugin_id: String,
    protocol: String,
    models_path: String,
    inference_path: String,
    credential: HeaderValue,
}

#[derive(Clone)]
pub struct Gateway {
    runtime: RuntimeManager,
    backend: Arc<RwLock<Option<Backend>>>,
    auth: LocalAuth,
    client: Client<hyper_tls::HttpsConnector<HttpConnector>, Body>,
    header_timeout: Duration,
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
            auth: LocalAuth(Arc::new(std::sync::RwLock::new(auth))),
            client,
            header_timeout,
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
        *self.backend.write().await = Some(Backend {
            plugin_id: manifest.metadata.id.clone(),
            protocol: manifest.capabilities.protocol.clone(),
            models_path: manifest.capabilities.models_path.clone(),
            inference_path: manifest.capabilities.inference_path.clone(),
            credential,
        });
        Ok(())
    }

    pub async fn clear_backend(&self) {
        *self.backend.write().await = None;
    }

    /// Actual successful attachment, not the desktop's persisted preference.
    pub async fn backend_plugin_id(&self) -> Option<String> {
        self.backend
            .read()
            .await
            .as_ref()
            .map(|backend| backend.plugin_id.clone())
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
        let backend = {
            let guard = self.backend.read().await;
            match guard.as_ref() {
                Some(b) => b.clone(),
                None => return error(StatusCode::SERVICE_UNAVAILABLE, "no_backend"),
            }
        };

        if ingress.matches_backend_protocol(&backend.protocol) {
            self.forward(req, false).await
        } else {
            self.transcode(req, ingress, backend).await
        }
    }

    async fn forward(&self, mut req: Request<Body>, models: bool) -> Response<Body> {
        let Some(backend) = self.backend.read().await.clone() else {
            return error(StatusCode::SERVICE_UNAVAILABLE, "no_backend");
        };
        let lease = match self.runtime.acquire_request(&backend.plugin_id).await {
            Ok(lease) => lease,
            Err(_) => return error(StatusCode::SERVICE_UNAVAILABLE, "backend_unavailable"),
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
        let (mut parts, body) = response.into_parts();
        parts.extensions.clear(); // do not expose/log raw ProxyError chains through other middleware
                                  // Hold admission through connect, header wait, every body frame and body Drop.
        Response::from_parts(parts, Body::new(LeasedBody::new(body, lease)))
    }

    async fn transcode(
        &self,
        req: Request<Body>,
        ingress: IngressProtocol,
        backend: Backend,
    ) -> Response<Body> {
        let lease = match self.runtime.acquire_request(&backend.plugin_id).await {
            Ok(lease) => lease,
            Err(_) => return error(StatusCode::SERVICE_UNAVAILABLE, "backend_unavailable"),
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
                        *m = serde_json::Value::String(requested_model);
                    }
                }
            }
            let resp_bytes = match serde_json::to_vec(&client_json) {
                Ok(b) => b,
                Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "serialization_error"),
            };
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
            let stream = async_stream::stream! {
                let _lease = lease;
                let mut saw_terminal = false;
                while let Some(event) = events.next().await {
                    let event = match event {
                        Ok(event) => event,
                        Err(_) => {
                            yield Err::<Bytes, io::Error>(io::Error::other("invalid upstream SSE transport or UTF-8"));
                            return;
                        }
                    };
                    let parts = match decoder.feed(&format!("data: {}", event.data)) {
                        Ok(parts) => parts,
                        Err(_) => {
                            yield Err::<Bytes, io::Error>(io::Error::other("invalid upstream protocol event"));
                            return;
                        }
                    };
                    for part in &parts {
                        saw_terminal |= matches!(part, StreamPart::Finish { .. } | StreamPart::ResponseCompleted { .. });
                        match encoder.encode_part(part) {
                            Ok(bytes) if !bytes.is_empty() => yield Ok(Bytes::from(bytes)),
                            Ok(_) => {},
                            Err(_) => {
                                yield Err(io::Error::other("upstream protocol conversion failed"));
                                return;
                            }
                        }
                        // A protocol error is final failure, not a successful completion.
                        // Ignore any later upstream DONE instead of inventing a success frame.
                        if matches!(part, StreamPart::Error { .. }) {
                            return;
                        }
                    }
                }
                // HTTP EOF never proves protocol completion. Fail transport, not a synthetic DONE.
                if !saw_terminal {
                    yield Err(io::Error::new(io::ErrorKind::UnexpectedEof, "upstream SSE ended without a protocol terminal event"));
                }
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
}
