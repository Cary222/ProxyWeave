//! Owned HTTP fixtures, not native-client or vendor acceptance.
use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use eventsource_stream::Eventsource;
use futures_util::{StreamExt, TryStreamExt};
use proxyweave_gateway::{Gateway, GatewayHandle};
use proxyweave_plugin::{Manifest, PluginRecord, RuntimeManager, RuntimeStatus};
use proxyweave_protocols::{
    chat_completions::ChatCompletionsCodec, messages::MessagesCodec, responses::ResponsesCodec,
    Content, EndpointCodec, StreamPart,
};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

const LOCAL: &str = "owned-matrix-local-key-0123456789abcdef";
const UPSTREAM: &str = "owned-matrix-upstream-key-fedcba9876543210";
const TEXT: &str = "你好 🌍 protocol matrix";
const PROTOCOLS: [&str; 3] = ["openai_chat", "openai_responses", "anthropic_messages"];
fn codec(protocol: &str) -> Box<dyn EndpointCodec> {
    match protocol {
        "openai_chat" => Box::new(ChatCompletionsCodec::new()),
        "openai_responses" => Box::new(ResponsesCodec::new()),
        "anthropic_messages" => Box::new(MessagesCodec::new()),
        _ => panic!("fixture protocol"),
    }
}
fn path(protocol: &str) -> &'static str {
    match protocol {
        "openai_chat" => "/v1/chat/completions",
        "openai_responses" => "/v1/responses",
        _ => "/v1/messages",
    }
}
fn request(protocol: &str, stream: bool) -> Vec<u8> {
    let mut body = match protocol {
        "openai_chat" => {
            json!({"model":"owned-model","messages":[{"role":"user","content":"owned prompt"}],"tools":[{"type":"function","function":{"name":"lookup","parameters":{"type":"object"}}}]})
        }
        "openai_responses" => {
            json!({"model":"owned-model","input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"owned prompt"}]}],"tools":[{"type":"function","name":"lookup","parameters":{"type":"object"}}]})
        }
        _ => {
            json!({"model":"owned-model","max_tokens":128,"messages":[{"role":"user","content":"owned prompt"}],"tools":[{"name":"lookup","input_schema":{"type":"object"}}]})
        }
    };
    body["stream"] = json!(stream);
    body["unknown_fixture"] = json!({"private":"must preserve same-protocol bytes"});
    serde_json::to_vec_pretty(&body).unwrap()
}
fn response(protocol: &str) -> Vec<u8> {
    let body = match protocol {
        "openai_chat" => {
            json!({"id":"owned-response","object":"chat.completion","model":"owned-model","choices":[{"index":0,"message":{"role":"assistant","content":TEXT,"tool_calls":[{"id":"call_owned","type":"function","function":{"name":"lookup","arguments":"{\"q\":\"owned\"}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":11,"completion_tokens":7,"total_tokens":18}})
        }
        "openai_responses" => {
            json!({"id":"owned-response","object":"response","model":"owned-model","status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":TEXT}]},{"type":"function_call","id":"fc_owned","call_id":"call_owned","name":"lookup","arguments":"{\"q\":\"owned\"}"}],"usage":{"input_tokens":11,"output_tokens":7,"total_tokens":18}})
        }
        _ => {
            json!({"id":"owned-response","type":"message","role":"assistant","model":"owned-model","content":[{"type":"text","text":TEXT},{"type":"tool_use","id":"call_owned","name":"lookup","input":{"q":"owned"}}],"stop_reason":"tool_use","usage":{"input_tokens":11,"output_tokens":7}})
        }
    };
    let mut body = body;
    body["unknown_fixture"] = json!({"wire":"response passthrough"});
    serde_json::to_vec_pretty(&body).unwrap()
}
fn sse(protocol: &str) -> Vec<u8> {
    let events = match protocol {
        "openai_chat" => vec![
            json!({"id":"owned-response","choices":[{"index":0,"delta":{"role":"assistant","content":TEXT},"finish_reason":null}]}),
            json!({"id":"owned-response","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_owned","type":"function","function":{"name":"lookup","arguments":"{\"q\":\"owned\"}"}}]},"finish_reason":null}]}),
            json!({"id":"owned-response","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":11,"completion_tokens":7,"total_tokens":18}}),
        ],
        "openai_responses" => vec![
            json!({"type":"response.created","response":{"id":"owned-response"}}),
            json!({"type":"response.output_text.delta","delta":TEXT}),
            json!({"type":"response.output_item.added","item":{"id":"fc_owned","type":"function_call","call_id":"call_owned","name":"lookup","arguments":""}}),
            json!({"type":"response.function_call_arguments.delta","item_id":"fc_owned","delta":"{\"q\":\"owned\"}"}),
            json!({"type":"response.completed","response":{"id":"owned-response","status":"completed","usage":{"input_tokens":11,"output_tokens":7,"total_tokens":18}}}),
        ],
        _ => vec![
            json!({"type":"message_start","message":{"id":"owned-response","type":"message","role":"assistant","model":"owned-model","content":[],"usage":{"input_tokens":11,"output_tokens":0}}}),
            json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":TEXT}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"call_owned","name":"lookup","input":{}}}),
            json!({"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"q\":\"owned\"}"}}),
            json!({"type":"content_block_stop","index":1}),
            json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":7}}),
            json!({"type":"message_stop"}),
        ],
    };
    let mut wire = String::from(": owned keepalive\r\n\r\n");
    for event in events {
        wire.push_str(&format!("data: {event}\r\n\r\n"));
    }
    if protocol == "openai_chat" {
        wire.push_str("data: [DONE]\r\n\r\n");
    }
    wire.into_bytes()
}
type CapturedRequest = (HeaderMap, Vec<u8>);
#[derive(Clone)]
struct Fixture {
    protocol: &'static str,
    capture: Arc<Mutex<Vec<CapturedRequest>>>,
    stream_override: Option<Vec<u8>>,
}
async fn inference(State(f): State<Fixture>, headers: HeaderMap, body: Bytes) -> impl IntoResponse {
    f.capture.lock().await.push((headers, body.to_vec()));
    let value: Value = serde_json::from_slice(&body).unwrap();
    if value["stream"] == true {
        let bytes = f.stream_override.unwrap_or_else(|| sse(f.protocol));
        // Split every UTF-8 code point/JSON/SSE delimiter across HTTP frames.
        let stream = futures_util::stream::iter(
            bytes
                .into_iter()
                .map(|b| Ok::<_, std::io::Error>(Bytes::from(vec![b]))),
        );
        (
            [("content-type", "text/event-stream")],
            Body::from_stream(stream),
        )
            .into_response()
    } else {
        ([("content-type", "application/json")], response(f.protocol)).into_response()
    }
}
struct Env {
    upstream: tokio::task::JoinHandle<()>,
    gateway: Gateway,
    handle: GatewayHandle,
    runtime: RuntimeManager,
    fixture: Fixture,
    manifest: Manifest,
    _root: tempfile::TempDir,
    client: reqwest::Client,
}
impl Drop for Env {
    fn drop(&mut self) {
        self.upstream.abort();
    }
}
impl Env {
    async fn new(protocol: &'static str, stream_override: Option<Vec<u8>>) -> Self {
        let fixture = Fixture {
            protocol,
            capture: Default::default(),
            stream_override,
        };
        let app = Router::new()
            .route(
                "/owned-health",
                get(|| async { ([("X-Fixture", "owned")], "ok") }),
            )
            .route(
                "/catalog/owned",
                get(|| async {
                    (
                        [("content-type", "application/json")],
                        r#"{"data":[{"id":"owned-model"}]}"#,
                    )
                }),
            )
            .route("/inference/owned", post(inference))
            .with_state(fixture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let upstream = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let manifest = Manifest::parse(format!("schema_version: '1.0'\nmetadata: {{id: owned-matrix, name: Owned, version: 1.0.0, license: MIT}}\nruntime: {{ownership: external, endpoint: 'http://{addr}'}}\nnetwork:\n  health_check: {{path: /owned-health, expected_status: 200, identity_header: X-Fixture, identity_value: owned, timeout_ms: 1000}}\nlifecycle: {{startup_timeout_ms: 1000, idle_timeout_ms: null, max_port_retries: 0}}\ncapabilities: {{protocol: {protocol}, models_path: /catalog/owned, inference_path: /inference/owned, streaming: true}}\n").as_bytes()).unwrap();
        let runtime = RuntimeManager::new();
        let root = tempfile::tempdir().unwrap();
        let record = PluginRecord {
            manifest: manifest.clone(),
            fingerprint: "owned".into(),
            settings: Default::default(),
        };
        assert!(matches!(
            runtime
                .start_plugin(&record, root.path(), None, root.path(), None)
                .await
                .unwrap(),
            RuntimeStatus::Ready { .. }
        ));
        let gateway = Gateway::new(runtime.clone(), LOCAL, Duration::from_secs(2)).unwrap();
        gateway.set_backend(&manifest, UPSTREAM).await.unwrap();
        let handle = gateway.listen(0).await.unwrap();
        Self {
            upstream,
            gateway,
            handle,
            runtime,
            fixture,
            manifest,
            _root: root,
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
        }
    }
    async fn send(&self, protocol: &str, stream: bool, alias: bool) -> reqwest::Response {
        let path = if alias { "/responses" } else { path(protocol) };
        self.client
            .post(format!("http://{}{path}", self.handle.address()))
            .header("x-api-key", LOCAL)
            .header("anthropic-version", "client-version-must-not-leak")
            .header("content-type", "application/json")
            .header("connection", "X-Local, Authorization")
            .header("x-local", "must-strip")
            .header("authorization", "Bearer client-credential-must-not-leak")
            .body(request(protocol, stream))
            .send()
            .await
            .unwrap()
    }
    async fn check_capture(&self, ingress: &str, stream: bool) {
        let captures = self.fixture.capture.lock().await;
        let (headers, bytes) = captures.last().unwrap();
        if self.fixture.protocol == "anthropic_messages" {
            assert_eq!(headers["x-api-key"], UPSTREAM);
            assert_eq!(headers["anthropic-version"], "2023-06-01");
            assert!(!headers.contains_key("authorization"));
        } else {
            assert_eq!(headers["authorization"], format!("Bearer {UPSTREAM}"));
            assert!(!headers.contains_key("x-api-key"));
            assert!(!headers.contains_key("anthropic-version"));
        }
        assert!(!headers.contains_key("x-local"));
        assert_eq!(headers["content-type"], "application/json");
        if ingress == self.fixture.protocol {
            assert_eq!(*bytes, request(ingress, stream));
        }
        let ir = codec(self.fixture.protocol)
            .decode_request(serde_json::from_slice(bytes).unwrap(), &Default::default())
            .unwrap();
        assert_eq!(ir.model, "owned-model");
        assert_eq!(ir.stream, stream);
        assert_eq!(ir.tools.len(), 1);
        assert_eq!(ir.tools[0].name, "lookup");
        assert!(format!("{:?}", ir.messages).contains("owned prompt"));
    }
    async fn check_lease(&self) {
        for _ in 0..100 {
            if self.runtime.active_request_count("owned-matrix").await == 0 {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("lease not released");
    }
}
#[tokio::test]
async fn owned_http_nonstream_three_by_three_and_models_manifest_paths() {
    for backend in PROTOCOLS {
        let env = Env::new(backend, None).await;
        for ingress in PROTOCOLS {
            let resp = env.send(ingress, false, false).await;
            assert_eq!(resp.status(), StatusCode::OK, "{ingress} -> {backend}");
            let bytes = resp.bytes().await.unwrap();
            if ingress == backend {
                assert_eq!(bytes.as_ref(), response(backend));
            }
            let ir = codec(ingress)
                .decode_response(serde_json::from_slice(&bytes).unwrap())
                .unwrap();
            assert!(ir
                .content
                .iter()
                .any(|c| matches!(c, Content::Text { text, .. } if text == TEXT)));
            assert!(format!("{:?}", ir.content).contains("lookup"));
            assert!(format!("{:?}", ir.content).contains("call_owned"));
            assert_eq!(ir.usage.as_ref().unwrap().prompt_tokens, 11);
            assert_eq!(ir.usage.as_ref().unwrap().completion_tokens, 7);
            env.check_capture(ingress, false).await;
            env.check_lease().await;
            println!("PASS nonstream {ingress} -> {backend}");
        }
        let resp = env
            .client
            .get(format!("http://{}/v1/models", env.handle.address()))
            .header("x-api-key", LOCAL)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.text().await.unwrap(),
            r#"{"data":[{"id":"owned-model"}]}"#
        );
        let alias = env.send("openai_responses", false, true).await;
        assert_eq!(alias.status(), StatusCode::OK);
        alias.bytes().await.unwrap();
        let mut unknown = env.manifest.clone();
        unknown.capabilities.protocol = "responses".into();
        assert!(env.gateway.set_backend(&unknown, UPSTREAM).await.is_err());
        env.check_lease().await;
    }
}
#[tokio::test]
async fn owned_http_stream_three_by_three_split_utf8_and_exact_passthrough() {
    for backend in PROTOCOLS {
        let env = Env::new(backend, None).await;
        for ingress in PROTOCOLS {
            let resp = env.send(ingress, true, false).await;
            assert_eq!(resp.status(), StatusCode::OK, "{ingress} -> {backend}");
            let bytes = resp.bytes().await.unwrap();
            if ingress == backend {
                assert_eq!(bytes.as_ref(), sse(backend));
            }
            assert!(!String::from_utf8(bytes.to_vec())
                .unwrap()
                .contains('\u{fffd}'));
            let wire = futures_util::stream::once(async { Ok::<_, std::io::Error>(bytes) });
            let events = wire.eventsource().try_collect::<Vec<_>>().await.unwrap();
            let mut decoder = codec(ingress).stream_decoder();
            let parts: Vec<_> = events
                .iter()
                .flat_map(|e| decoder.feed(&format!("data: {}", e.data)).unwrap())
                .collect();
            let text: String = parts
                .iter()
                .filter_map(|p| match p {
                    StreamPart::TextDelta { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(text, TEXT, "{ingress} -> {backend}");
            assert!(parts.iter().any(|p| matches!(p, StreamPart::ToolCallDelta { id, name: Some(name), .. } if id == "call_owned" && name == "lookup")));
            assert!(parts.iter().any(|p| matches!(p, StreamPart::Usage { usage } if usage.prompt_tokens == 11 && usage.completion_tokens == 7)));
            assert!(parts.iter().any(|p| matches!(p, StreamPart::Finish { .. })));
            assert!(
                parts
                    .iter()
                    .any(|p| matches!(p, StreamPart::ResponseCompleted { .. })),
                "{ingress} -> {backend}: no terminal"
            );
            env.check_capture(ingress, true).await;
            env.check_lease().await;
            println!("PASS stream {ingress} -> {backend}");
        }
    }
}
#[tokio::test]
async fn malformed_stream_transport_is_visible_and_same_protocol_is_opaque() {
    for bytes in [
        b"data: {invalid json}\n\n".to_vec(),
        b"data: \xff\n\n".to_vec(),
        b"data: {\"choices\":[]}\n\n".to_vec(),
    ] {
        let env = Env::new("openai_chat", Some(bytes.clone())).await;
        for ingress in ["anthropic_messages", "openai_responses"] {
            let mut resp = env.send(ingress, true, false).await.bytes_stream();
            let mut failed = false;
            while let Some(chunk) = resp.next().await {
                if chunk.is_err() {
                    failed = true;
                    break;
                }
            }
            assert!(failed, "malformed stream falsely succeeded: {ingress}");
            drop(resp);
            env.check_lease().await;
        }
        // Same-protocol clients own their decoding, even for private or invalid wire fields.
        let resp = env.send("openai_chat", true, false).await;
        assert_eq!(resp.bytes().await.unwrap().as_ref(), bytes);
        env.check_lease().await;
    }
}

#[tokio::test]
async fn lossy_request_is_rejected_before_any_backend_inference_and_releases_lease() {
    for backend in ["openai_chat", "anthropic_messages"] {
        let env = Env::new(backend, None).await;
        let body =
            json!({"model":"owned-model","input":"owned prompt","tools":[{"type":"web_search"}]});
        let resp = env
            .client
            .post(format!("http://{}/v1/responses", env.handle.address()))
            .header("x-api-key", LOCAL)
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&body).unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "backend {backend}");
        assert!(resp.text().await.unwrap().contains("hosted_tools"));
        assert!(env.fixture.capture.lock().await.is_empty());
        env.check_lease().await;
    }
}

#[tokio::test]
async fn protocol_error_frames_never_become_success_terminals() {
    for backend in PROTOCOLS {
        let native_error = match backend {
            "openai_chat" => json!({"error":{"message":"owned upstream failure","code":"429"}}),
            "anthropic_messages" => {
                json!({"type":"error","error":{"type":"overloaded_error","message":"owned upstream failure"}})
            }
            _ => {
                json!({"type":"response.failed","response":{"error":{"type":"server_error","message":"owned upstream failure"}}})
            }
        };
        // A later DONE must not convert an earlier error into successful completion.
        let wire = format!("data: {native_error}\n\ndata: [DONE]\n\n").into_bytes();
        let env = Env::new(backend, Some(wire)).await;
        for ingress in PROTOCOLS.into_iter().filter(|p| *p != backend) {
            let resp = env.send(ingress, true, false).await;
            let bytes = resp.bytes().await.unwrap();
            let events = futures_util::stream::once(async { Ok::<_, std::io::Error>(bytes) })
                .eventsource()
                .try_collect::<Vec<_>>()
                .await
                .unwrap();
            let mut decoder = codec(ingress).stream_decoder();
            let parts: Vec<_> = events
                .iter()
                .flat_map(|e| decoder.feed(&format!("data: {}", e.data)).unwrap())
                .collect();
            assert!(
                parts.iter().any(|p| matches!(p, StreamPart::Error { .. })),
                "{ingress} <- {backend}"
            );
            assert!(
                !parts.iter().any(|p| matches!(
                    p,
                    StreamPart::ResponseCompleted { .. } | StreamPart::Finish { .. }
                )),
                "error became success: {ingress} <- {backend}"
            );
            env.check_lease().await;
        }
    }
}
