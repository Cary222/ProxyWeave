use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use proxyweave_gateway::{Gateway, GatewayHandle};
use proxyweave_plugin::{Manifest, PluginRecord, RuntimeManager, RuntimeStatus};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

const LOCAL_KEY: &str = "test-local-gateway-key-1234567890abcdef";
const UPSTREAM_KEY: &str = "test-upstream-backend-key-0987654321fedcba";

#[derive(Clone, Default)]
struct MockState {
    last_request_path: Arc<Mutex<String>>,
    last_request_headers: Arc<Mutex<Vec<(String, String)>>>,
    last_request_body: Arc<Mutex<serde_json::Value>>,
}

async fn healthz() -> Response {
    (
        StatusCode::OK,
        [
            ("X-Fixture", "gateway-fixture"),
            ("Content-Type", "text/plain"),
        ],
        "ok",
    )
        .into_response()
}

async fn mock_chat_completions(
    State(state): State<MockState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    *state.last_request_path.lock().await = "/v1/chat/completions".to_string();
    let header_list: Vec<(String, String)> = headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    *state.last_request_headers.lock().await = header_list;

    let body_json: serde_json::Value =
        serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    *state.last_request_body.lock().await = body_json.clone();

    let is_stream = body_json["stream"].as_bool().unwrap_or(false);
    let model = body_json["model"].as_str().unwrap_or("gpt-4o");

    if is_stream {
        let stream_body = format!(
            "data: {{\"id\":\"chatcmpl-stream-1\",\"object\":\"chat.completion.chunk\",\"model\":\"{model}\",\"choices\":[{{\"index\":0,\"delta\":{{\"role\":\"assistant\",\"content\":\"\"}},\"finish_reason\":null}}]}}\n\n\
             data: {{\"id\":\"chatcmpl-stream-1\",\"object\":\"chat.completion.chunk\",\"model\":\"{model}\",\"choices\":[{{\"index\":0,\"delta\":{{\"content\":\"Hello\"}},\"finish_reason\":null}}]}}\n\n\
             data: {{\"id\":\"chatcmpl-stream-1\",\"object\":\"chat.completion.chunk\",\"model\":\"{model}\",\"choices\":[{{\"index\":0,\"delta\":{{\"content\":\" world!\"}},\"finish_reason\":null}}]}}\n\n\
             data: {{\"id\":\"chatcmpl-stream-1\",\"object\":\"chat.completion.chunk\",\"model\":\"{model}\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}]}}\n\n\
             data: [DONE]\n\n"
        );
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
            .body(axum::body::Body::from(stream_body))
            .unwrap()
    } else {
        let resp_json = json!({
            "id": "chatcmpl-test-123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": model,
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello from mock upstream OpenAI chat!"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 7,
                "total_tokens": 19
            }
        });
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            serde_json::to_vec(&resp_json).unwrap(),
        )
            .into_response()
    }
}

struct TestEnv {
    _mock_task: tokio::task::JoinHandle<()>,
    _mock_state: MockState,
    _gateway_handle: GatewayHandle,
    client: reqwest::Client,
    base_url: String,
}

impl TestEnv {
    async fn setup() -> Self {
        let mock_state = MockState::default();
        let app = Router::new()
            .route("/healthz", get(healthz))
            .route("/v1/chat/completions", post(mock_chat_completions))
            .with_state(mock_state.clone());

        let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mock_addr = mock_listener.local_addr().unwrap();

        let mock_task = tokio::spawn(async move {
            axum::serve(mock_listener, app).await.unwrap();
        });

        let manifest_yaml = format!(
            r#"schema_version: '1.0'
metadata: {{id: mock-upstream-plugin, name: Mock Upstream, version: 1.0.0, license: MIT}}
runtime:
  ownership: external
  endpoint: http://{mock_addr}
network:
  health_check: {{path: /healthz, expected_status: 200, identity_header: X-Fixture, identity_value: gateway-fixture, timeout_ms: 1000}}
lifecycle: {{startup_timeout_ms: 2000, idle_timeout_ms: null, max_port_retries: 0}}
capabilities: {{protocol: openai_chat, models_path: /v1/models, inference_path: /v1/chat/completions, streaming: true}}
configuration: {{}}
"#
        );

        let manifest = Manifest::parse(manifest_yaml.as_bytes()).unwrap();
        let record = PluginRecord {
            manifest: manifest.clone(),
            fingerprint: "mock".into(),
            settings: Default::default(),
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let runtime = RuntimeManager::new();
        let status = runtime
            .start_plugin(&record, temp_dir.path(), None, temp_dir.path(), None)
            .await
            .expect("start mock external plugin");

        assert!(matches!(status, RuntimeStatus::Ready { .. }));

        let gateway = Gateway::new(runtime, LOCAL_KEY, Duration::from_secs(5)).unwrap();
        gateway.set_backend(&manifest, UPSTREAM_KEY).await.unwrap();

        let gateway_handle = gateway.listen(0).await.unwrap();
        let base_url = format!("http://{}", gateway_handle.address());
        let client = reqwest::Client::builder().no_proxy().build().unwrap();

        Self {
            _mock_task: mock_task,
            _mock_state: mock_state,
            _gateway_handle: gateway_handle,
            client,
            base_url,
        }
    }
}

#[tokio::test]
async fn test_messages_non_stream_transcoding() {
    let env = TestEnv::setup().await;

    // 1. Anthropic client request using x-api-key
    let req_body = json!({
        "model": "claude-3-5-sonnet",
        "messages": [
            {
                "role": "user",
                "content": "Hello Claude"
            }
        ],
        "max_tokens": 1024
    });

    let resp = env
        .client
        .post(format!("{}/v1/messages", env.base_url))
        .header("x-api-key", LOCAL_KEY)
        .header("content-type", "application/json")
        .json(&req_body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );

    let resp_json: serde_json::Value = resp.json().await.unwrap();

    // Anthropic response structure assertions
    assert_eq!(resp_json["type"], "message");
    assert_eq!(resp_json["role"], "assistant");
    assert_eq!(resp_json["model"], "claude-3-5-sonnet");
    assert_eq!(resp_json["stop_reason"], "end_turn");

    let content = resp_json["content"].as_array().expect("content array");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[0]["text"], "Hello from mock upstream OpenAI chat!");

    assert_eq!(resp_json["usage"]["input_tokens"], 12);
    assert_eq!(resp_json["usage"]["output_tokens"], 7);

    // Verify upstream received OpenAI format
    let upstream_body = env._mock_state.last_request_body.lock().await;
    assert_eq!(upstream_body["model"], "claude-3-5-sonnet");
    assert_eq!(upstream_body["messages"][0]["role"], "user");
    assert_eq!(upstream_body["messages"][0]["content"], "Hello Claude");

    // Verify upstream received the upstream auth key
    let upstream_headers = env._mock_state.last_request_headers.lock().await;
    let auth_header = upstream_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
        .map(|(_, v)| v.as_str());
    assert_eq!(auth_header, Some(format!("Bearer {UPSTREAM_KEY}").as_str()));
}

#[tokio::test]
async fn test_messages_streaming_transcoding() {
    let env = TestEnv::setup().await;

    let req_body = json!({
        "model": "claude-3-5-sonnet",
        "messages": [
            {
                "role": "user",
                "content": "Tell me a short poem"
            }
        ],
        "max_tokens": 512,
        "stream": true
    });

    let resp = env
        .client
        .post(format!("{}/v1/messages", env.base_url))
        .header("Authorization", format!("Bearer {LOCAL_KEY}"))
        .header("content-type", "application/json")
        .json(&req_body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.starts_with("text/event-stream"));

    let body_text = resp.text().await.unwrap();

    // Anthropic SSE events assertion
    assert!(body_text.contains("event: message_start"));
    assert!(body_text.contains("event: content_block_start"));
    assert!(body_text.contains("event: content_block_delta"));
    assert!(body_text.contains("event: content_block_stop"));
    assert!(body_text.contains("event: message_delta"));
    assert!(body_text.contains("event: message_stop"));
    assert!(body_text.contains("Hello"));
    assert!(body_text.contains(" world!"));
}

#[tokio::test]
async fn test_responses_transcoding() {
    let env = TestEnv::setup().await;

    for path in ["/v1/responses", "/responses"] {
        let req_body = json!({
            "model": "gpt-4o",
            "input": "Write tests"
        });

        let resp = env
            .client
            .post(format!("{}{path}", env.base_url))
            .header("Authorization", format!("Bearer {LOCAL_KEY}"))
            .header("content-type", "application/json")
            .json(&req_body)
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let resp_json: serde_json::Value = resp.json().await.unwrap();

        assert_eq!(resp_json["object"], "response");
        assert_eq!(resp_json["status"], "completed");
        assert_eq!(resp_json["model"], "gpt-4o");

        let output = resp_json["output"].as_array().expect("output array");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0]["type"], "message");
        assert_eq!(
            output[0]["content"][0]["text"],
            "Hello from mock upstream OpenAI chat!"
        );
    }
}

#[tokio::test]
async fn test_chat_completions_passthrough() {
    let env = TestEnv::setup().await;

    let req_body = json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "user",
                "content": "Direct chat pass-through"
            }
        ]
    });

    let resp = env
        .client
        .post(format!("{}/v1/chat/completions", env.base_url))
        .header("Authorization", format!("Bearer {LOCAL_KEY}"))
        .header("content-type", "application/json")
        .json(&req_body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let resp_json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(resp_json["object"], "chat.completion");
    assert_eq!(
        resp_json["choices"][0]["message"]["content"],
        "Hello from mock upstream OpenAI chat!"
    );
}

#[tokio::test]
async fn test_auth_rejection_on_messages_and_responses() {
    let env = TestEnv::setup().await;

    // No auth
    let resp = env
        .client
        .post(format!("{}/v1/messages", env.base_url))
        .json(&json!({"model": "test", "messages": []}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Wrong x-api-key
    let resp = env
        .client
        .post(format!("{}/v1/messages", env.base_url))
        .header("x-api-key", "wrong-key")
        .json(&json!({"model": "test", "messages": []}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Wrong bearer
    let resp = env
        .client
        .post(format!("{}/v1/responses", env.base_url))
        .header("Authorization", "Bearer wrong-key")
        .json(&json!({"model": "test", "input": "test"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
// ============================================================================
// DEF-020 to DEF-024 Regression Tests
// ============================================================================

#[tokio::test]
async fn test_def_020_header_leak_and_nominees_stripped() {
    let env = TestEnv::setup().await;

    let req_body = json!({
        "model": "claude-3-5-sonnet",
        "messages": [{"role": "user", "content": "hi"}],
        "max_tokens": 100
    });

    let resp = env
        .client
        .post(format!("{}/v1/messages", env.base_url))
        .header("x-api-key", LOCAL_KEY)
        .header("Authorization", "Bearer client-secret-token")
        .header("Proxy-Authorization", "Basic secret")
        .header("Cookie", "session=secret")
        .header("Connection", "keep-alive, X-Custom-Nominated")
        .header("X-Custom-Nominated", "leak-attempt")
        .header("X-Safe-Custom", "safe-value")
        .json(&req_body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let recorded_headers = env._mock_state.last_request_headers.lock().await.clone();

    for (k, _v) in &recorded_headers {
        let lower = k.to_ascii_lowercase();
        assert_ne!(lower, "x-api-key", "Leaked x-api-key to upstream!");
        assert_ne!(
            lower, "proxy-authorization",
            "Leaked proxy-authorization to upstream!"
        );
        assert_ne!(lower, "cookie", "Leaked cookie to upstream!");
        assert_ne!(
            lower, "x-custom-nominated",
            "Leaked connection-nominated header to upstream!"
        );
    }

    let auth_header = recorded_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
        .map(|(_, v)| v.as_str());

    assert_eq!(
        auth_header,
        Some(format!("Bearer {UPSTREAM_KEY}").as_str()),
        "Upstream authorization header mismatch or client token leaked!"
    );
}

#[tokio::test]
async fn test_def_021_abnormal_eof_never_synthesizes_done() {
    let mock_state = MockState::default();
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route(
            "/v1/chat/completions",
            post(|| async {
                // Emits only an initial chunk, then abruptly terminates stream without finish_reason or [DONE]
                let stream = async_stream::stream! {
                    let chunk = json!({
                        "id": "chatcmpl-cut",
                        "object": "chat.completion.chunk",
                        "created": 1700000000,
                        "model": "gpt-4o",
                        "choices": [{
                            "index": 0,
                            "delta": { "content": "Partially sent" },
                            "finish_reason": null
                        }]
                    });
                    yield Ok::<_, std::io::Error>(Bytes::from(format!("data: {}\n\n", serde_json::to_string(&chunk).unwrap())));
                };
                Response::builder()
                    .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
                    .body(axum::body::Body::from_stream(stream))
                    .unwrap()
            }),
        )
        .with_state(mock_state.clone());

    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();
    let _mock_task = tokio::spawn(async move {
        axum::serve(mock_listener, app).await.unwrap();
    });

    let manifest_yaml = format!(
        r#"schema_version: '1.0'
metadata: {{id: mock-upstream-def021, name: Mock Def021, version: 1.0.0, license: MIT}}
runtime:
  ownership: external
  endpoint: http://{mock_addr}
network:
  health_check: {{path: /healthz, expected_status: 200, identity_header: X-Fixture, identity_value: gateway-fixture, timeout_ms: 1000}}
lifecycle: {{startup_timeout_ms: 2000, idle_timeout_ms: null, max_port_retries: 0}}
capabilities: {{protocol: openai_chat, models_path: /v1/models, inference_path: /v1/chat/completions, streaming: true}}
configuration: {{}}
"#
    );

    let manifest = Manifest::parse(manifest_yaml.as_bytes()).unwrap();
    let record = PluginRecord {
        manifest: manifest.clone(),
        fingerprint: "mock".into(),
        settings: Default::default(),
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let runtime = RuntimeManager::new();
    runtime
        .start_plugin(&record, temp_dir.path(), None, temp_dir.path(), None)
        .await
        .unwrap();

    let gateway = Gateway::new(runtime, LOCAL_KEY, Duration::from_secs(5)).unwrap();
    gateway.set_backend(&manifest, UPSTREAM_KEY).await.unwrap();

    let gateway_handle = gateway.listen(0).await.unwrap();
    let base_url = format!("http://{}", gateway_handle.address());
    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    let resp = client
        .post(format!("{base_url}/v1/messages"))
        .header("x-api-key", LOCAL_KEY)
        .json(&json!({
            "model": "claude-3-5-sonnet",
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 100,
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut body = Vec::new();
    let mut failed = false;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => body.extend_from_slice(&bytes),
            Err(_) => {
                failed = true;
                break;
            }
        }
    }
    assert!(failed, "Abnormal EOF must be visible to the client");
    let body_text = String::from_utf8(body).unwrap();
    // DEF-021: MUST NOT contain synthesized message_stop or stop_reason!
    assert!(
        !body_text.contains("event: message_stop"),
        "Abnormal EOF synthesized message_stop!"
    );
    assert!(
        !body_text.contains("\"stop_reason\":\"end_turn\""),
        "Abnormal EOF synthesized stop_reason!"
    );
}

#[tokio::test]
async fn test_def_022_multibyte_utf8_split_across_chunks_intact() {
    let mock_state = MockState::default();
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route(
            "/v1/chat/completions",
            post(|| async {
                // "Hello 🦀 世界！"
                // 🦀 is 4 bytes: [0xF0, 0x9F, 0xA6, 0x80]
                // 世 is 3 bytes: [0xE4, 0xB8, 0x96]
                let chunk1 = json!({
                    "id": "chatcmpl-utf8",
                    "object": "chat.completion.chunk",
                    "created": 1700000000,
                    "model": "gpt-4o",
                    "choices": [{
                        "index": 0,
                        "delta": { "content": "Hello 🦀 世界！" },
                        "finish_reason": "stop"
                    }]
                });
                let sse_line = format!(
                    "data: {}\n\ndata: [DONE]\n\n",
                    serde_json::to_string(&chunk1).unwrap()
                );
                let full_bytes = sse_line.into_bytes();

                // Deliberately split full_bytes across chunk boundaries
                let split1 = &full_bytes[..full_bytes.len() / 2];
                let split2 = &full_bytes[full_bytes.len() / 2..];

                let b1 = Bytes::copy_from_slice(split1);
                let b2 = Bytes::copy_from_slice(split2);

                let stream = async_stream::stream! {
                    yield Ok::<_, std::io::Error>(b1);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    yield Ok::<_, std::io::Error>(b2);
                };

                Response::builder()
                    .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
                    .body(axum::body::Body::from_stream(stream))
                    .unwrap()
            }),
        )
        .with_state(mock_state.clone());

    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();
    let _mock_task = tokio::spawn(async move {
        axum::serve(mock_listener, app).await.unwrap();
    });

    let manifest_yaml = format!(
        r#"schema_version: '1.0'
metadata: {{id: mock-upstream-def022, name: Mock Def022, version: 1.0.0, license: MIT}}
runtime:
  ownership: external
  endpoint: http://{mock_addr}
network:
  health_check: {{path: /healthz, expected_status: 200, identity_header: X-Fixture, identity_value: gateway-fixture, timeout_ms: 1000}}
lifecycle: {{startup_timeout_ms: 2000, idle_timeout_ms: null, max_port_retries: 0}}
capabilities: {{protocol: openai_chat, models_path: /v1/models, inference_path: /v1/chat/completions, streaming: true}}
configuration: {{}}
"#
    );

    let manifest = Manifest::parse(manifest_yaml.as_bytes()).unwrap();
    let record = PluginRecord {
        manifest: manifest.clone(),
        fingerprint: "mock".into(),
        settings: Default::default(),
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let runtime = RuntimeManager::new();
    runtime
        .start_plugin(&record, temp_dir.path(), None, temp_dir.path(), None)
        .await
        .unwrap();

    let gateway = Gateway::new(runtime, LOCAL_KEY, Duration::from_secs(5)).unwrap();
    gateway.set_backend(&manifest, UPSTREAM_KEY).await.unwrap();

    let gateway_handle = gateway.listen(0).await.unwrap();
    let base_url = format!("http://{}", gateway_handle.address());
    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    let resp = client
        .post(format!("{base_url}/v1/messages"))
        .header("x-api-key", LOCAL_KEY)
        .json(&json!({
            "model": "claude-3-5-sonnet",
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 100,
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body_text = resp.text().await.unwrap();

    // Verify multibyte emojis and characters decoded completely without replacement character (U+FFFD)
    assert!(
        body_text.contains("Hello 🦀 世界！"),
        "Multibyte characters were corrupted: {body_text}"
    );
    assert!(
        !body_text.contains('\u{FFFD}'),
        "Found replacement character U+FFFD!"
    );
    assert!(body_text.contains("event: message_stop"));
}

#[tokio::test]
async fn test_def_023_header_timeout_returns_504() {
    let mock_state = MockState::default();
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route(
            "/v1/chat/completions",
            post(|| async {
                // Sleep longer than header_timeout
                tokio::time::sleep(Duration::from_millis(250)).await;
                (StatusCode::OK, "{}")
            }),
        )
        .with_state(mock_state.clone());

    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();
    let _mock_task = tokio::spawn(async move {
        axum::serve(mock_listener, app).await.unwrap();
    });

    let manifest_yaml = format!(
        r#"schema_version: '1.0'
metadata: {{id: mock-upstream-def023, name: Mock Def023, version: 1.0.0, license: MIT}}
runtime:
  ownership: external
  endpoint: http://{mock_addr}
network:
  health_check: {{path: /healthz, expected_status: 200, identity_header: X-Fixture, identity_value: gateway-fixture, timeout_ms: 1000}}
lifecycle: {{startup_timeout_ms: 2000, idle_timeout_ms: null, max_port_retries: 0}}
capabilities: {{protocol: openai_chat, models_path: /v1/models, inference_path: /v1/chat/completions, streaming: true}}
configuration: {{}}
"#
    );

    let manifest = Manifest::parse(manifest_yaml.as_bytes()).unwrap();
    let record = PluginRecord {
        manifest: manifest.clone(),
        fingerprint: "mock".into(),
        settings: Default::default(),
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let runtime = RuntimeManager::new();
    runtime
        .start_plugin(&record, temp_dir.path(), None, temp_dir.path(), None)
        .await
        .unwrap();

    let gateway = Gateway::new(runtime, LOCAL_KEY, Duration::from_millis(50)).unwrap();
    gateway.set_backend(&manifest, UPSTREAM_KEY).await.unwrap();

    let gateway_handle = gateway.listen(0).await.unwrap();
    let base_url = format!("http://{}", gateway_handle.address());
    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    let resp = client
        .post(format!("{base_url}/v1/messages"))
        .header("x-api-key", LOCAL_KEY)
        .json(&json!({
            "model": "claude-3-5-sonnet",
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 100
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::GATEWAY_TIMEOUT);
    let body_json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body_json["error"]["type"], "timeout_error");
}

#[tokio::test]
async fn test_def_024_lossy_conversion_rejection_returns_400() {
    let env = TestEnv::setup().await;

    // 1. Responses request with hosted tools (web_search) targeting OpenAI Chat (which lacks HostedTools cap)
    let resp_req_body = json!({
        "model": "gpt-5",
        "input": "Search the web",
        "tools": [
            {
                "type": "web_search"
            }
        ]
    });

    let resp = env
        .client
        .post(format!("{}/v1/responses", env.base_url))
        .header("Authorization", format!("Bearer {LOCAL_KEY}"))
        .json(&resp_req_body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body_json: serde_json::Value = resp.json().await.unwrap();
    let err_msg = body_json["error"]["message"].as_str().unwrap_or("");
    assert!(
        err_msg.contains("Lossy conversion rejected"),
        "Unexpected error message: {err_msg}"
    );
    assert!(
        err_msg.contains("HostedTools"),
        "Expected HostedTools dimension mentioned in: {err_msg}"
    );

    // 2. Claude Messages with reasoning thinking block targeting OpenAI Chat (which lacks ExtendedReasoning cap)
    let claude_req_body = json!({
        "model": "claude-3-5-sonnet",
        "messages": [
            {
                "role": "assistant",
                "content": [
                    {
                        "type": "thinking",
                        "thinking": "internal CoT thoughts",
                        "signature": "sig123"
                    }
                ]
            }
        ],
        "max_tokens": 100
    });

    let resp2 = env
        .client
        .post(format!("{}/v1/messages", env.base_url))
        .header("x-api-key", LOCAL_KEY)
        .json(&claude_req_body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);
    let body_json2: serde_json::Value = resp2.json().await.unwrap();
    let err_msg2 = body_json2["error"]["message"].as_str().unwrap_or("");
    assert!(
        err_msg2.contains("Lossy conversion rejected"),
        "Unexpected error message: {err_msg2}"
    );
    assert!(
        err_msg2.contains("ExtendedReasoning"),
        "Expected ExtendedReasoning dimension mentioned in: {err_msg2}"
    );
}
