//! OpenAI-compatible byte passthrough, directly using axum-reverse-proxy.
//! No protocol conversion, terminal synthesis, provider-specific implementation or database.
use axum::{
    body::{Body, Bytes},
    http::{header, HeaderValue, Request, Response, StatusCode},
    routing::{get, post},
    Router,
};
use axum_reverse_proxy::{ProxyPolicy, ReverseProxy, XForwardedFor};
use http_body::{Body as HttpBody, Frame, SizeHint};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use proxyweave_plugin::{Manifest, RequestLease, RuntimeManager};
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

// Thin custom validator on the official Tower HTTP validation layer, following
// its Bearer implementation. Unlike that type's Debug, this holds a sensitive
// HeaderValue without Debug, and rejects duplicate Authorization fields.
#[derive(Clone)]
struct LocalAuth(HeaderValue);
impl ValidateRequest<Body> for LocalAuth {
    type ResponseBody = Body;
    fn validate(&mut self, request: &mut Request<Body>) -> Result<(), Response<Body>> {
        let mut auth = request.headers().get_all(header::AUTHORIZATION).iter();
        if auth.next() == Some(&self.0) && auth.next().is_none() {
            Ok(())
        } else {
            Err(error(StatusCode::UNAUTHORIZED, "invalid_api_key"))
        }
    }
}

#[derive(Clone)]
struct Backend {
    plugin_id: String,
    models_path: String,
    inference_path: String,
    authorization: HeaderValue,
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
            auth: LocalAuth(auth),
            client,
            header_timeout,
        })
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
        if manifest.capabilities.protocol != "openai_chat" {
            return Err("Unsupported protocol");
        }
        let authorization = sensitive_bearer(upstream_key)?;
        *self.backend.write().await = Some(Backend {
            plugin_id: manifest.metadata.id.clone(),
            models_path: manifest.capabilities.models_path.clone(),
            inference_path: manifest.capabilities.inference_path.clone(),
            authorization,
        });
        Ok(())
    }

    pub async fn clear_backend(&self) {
        *self.backend.write().await = None;
    }

    fn router(&self) -> Router {
        let models = self.clone();
        let chat = self.clone();
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
                    async move { g.forward(req, false).await }
                }),
            )
            .layer(ValidateRequestHeaderLayer::custom(self.auth.clone()))
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
        req.headers_mut().remove(header::AUTHORIZATION);
        req.headers_mut().remove(header::PROXY_AUTHORIZATION);
        req.headers_mut().remove(header::COOKIE);
        // Connection-nominated Authorization must not remove our own upstream credential.
        // The upstream module strips nominated headers; add the credential after clearing
        // Connection nominations (its standard hop-by-hop removal still runs).
        let nominated: Vec<_> = req
            .headers()
            .get_all(header::CONNECTION)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(','))
            .filter_map(|name| header::HeaderName::from_bytes(name.trim().as_bytes()).ok())
            .collect();
        for name in nominated {
            req.headers_mut().remove(name);
        }
        req.headers_mut().remove(header::CONNECTION);
        req.headers_mut()
            .insert(header::AUTHORIZATION, backend.authorization);
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
