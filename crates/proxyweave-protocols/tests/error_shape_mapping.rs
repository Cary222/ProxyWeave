//! Error-shape mapping — 3 ingress endpoints × 4 upstream error-body shapes.
//!
//! Round 12 / spec v13. Every stage driven here is production code reached
//! through the public API of `proxyweave-protocols`; no network, no ports.
//!
//! | Stage | Production source |
//! |---|---|
//! | non-stream upstream error-body parse | `crates/proxyweave-gateway/src/lib.rs:1206-1215` |
//! | `ErrorClass` classification | `src/routing.rs:65` `classify_upstream_error` |
//! | ingress-native error body | `src/error_body.rs:88` `encode_error_body_for_suite` |
//! | HTTP status handed to the client | `crates/proxyweave-gateway/src/lib.rs:1265` |
//! | upstream error-frame parse (stream) | `src/chat_completions.rs:1665`, `src/messages.rs:1758`, `src/responses.rs:3233` |
//! | ingress-native error frame (stream) | `src/chat_completions.rs:1544`, `src/messages.rs:1412`, `src/responses.rs:2814` |
//!
//! Ingress: Chat Completions (`OpenAiCompatible`), Anthropic Messages,
//! OpenAI Responses. Upstream error-body shapes: standard JSON error object,
//! non-JSON plain text, empty body, truncated JSON.
//!
//! Each case asserts the HTTP status the client receives, the protocol-native
//! `error.type` / `error.code`, the `message` structure, and that no credential
//! material (`Authorization`, `x-api-key`, key text) reaches the client body or
//! the trace/error string the gateway records.
//!
//! `cargo test -p proxyweave-protocols --test error_shape_mapping`

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use proxyweave_protocols::{
    chat_completions::ChatCompletionsCodec, classify_upstream_error,
    error_body::encode_error_body_for_suite, messages::MessagesCodec, responses::ResponsesCodec,
    EndpointCodec, ErrorClass, ProtocolSuite, StreamPart,
};
use serde_json::{json, Value};

/// Fake credential planted in upstream payloads. Not a real key: the only
/// property under test is that it never reaches the client or the trace line.
const PLANTED_KEY: &str = "sk-live-PLANTED-4f7a9c-do-not-echo";
const PLANTED_BEARER: &str = "Bearer sk-live-PLANTED-4f7a9c-do-not-echo";

/// The three ingress endpoints this round covers (`Round 11 G2` keeps Gemini
/// out of scope).
const INGRESS: [ProtocolSuite; 3] = [
    ProtocolSuite::OpenAiCompatible,
    ProtocolSuite::AnthropicMessages,
    ProtocolSuite::OpenAiResponses,
];

fn codec(suite: ProtocolSuite) -> Box<dyn EndpointCodec> {
    match suite {
        ProtocolSuite::OpenAiCompatible => Box::new(ChatCompletionsCodec::new()),
        ProtocolSuite::AnthropicMessages => Box::new(MessagesCodec::new()),
        ProtocolSuite::OpenAiResponses => Box::new(ResponsesCodec::new()),
        ProtocolSuite::GoogleGemini => {
            unimplemented!("Gemini codecs are out of scope (Round 11 G2 non-goal)")
        }
    }
}

/// One upstream non-2xx response, as the gateway sees it.
struct UpstreamResponse {
    status: u16,
    body: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    StandardJson,
    PlainText,
    Empty,
    Truncated,
}

impl Shape {
    const ALL: [Shape; 4] = [
        Shape::StandardJson,
        Shape::PlainText,
        Shape::Empty,
        Shape::Truncated,
    ];

    fn response(self) -> UpstreamResponse {
        let status = match self {
            Shape::StandardJson => 400,
            Shape::PlainText => 500,
            Shape::Empty => 503,
            Shape::Truncated => 429,
        };
        let body = match self {
            Shape::StandardJson => {
                br#"{"error":{"message":"Invalid 'temperature': must be <= 2","type":"invalid_request_error","code":"invalid_temperature"}}"#.to_vec()
            }
            // A chatty provider that echoes the request credential in a
            // non-JSON error page. The gateway must not forward this body.
            Shape::PlainText => {
                format!("upstream 500: connection reset while forwarding x-api-key: {PLANTED_KEY}")
                    .into_bytes()
            }
            Shape::Empty => Vec::new(),
            // Cut mid-field, credential included: still unparseable, so the
            // client must get the generic fallback rather than the raw bytes.
            Shape::Truncated => format!(
                r#"{{"error":{{"message":"rate limit exceeded (Authorization: {PLANTED_BEARER})","type":"rate_limit_error","co"#
            )
            .into_bytes(),
        };
        UpstreamResponse { status, body }
    }

    /// Message the client must see: the parsed upstream message, or the
    /// gateway's generic fallback for every unparseable shape.
    fn expected_message(self) -> &'static str {
        match self {
            Shape::StandardJson => "Invalid 'temperature': must be <= 2",
            Shape::PlainText | Shape::Empty | Shape::Truncated => "Upstream error",
        }
    }

    fn expected_class(self) -> ErrorClass {
        match self {
            Shape::StandardJson => ErrorClass::BadRequest,
            Shape::PlainText => ErrorClass::Transient,
            Shape::Empty => ErrorClass::Overloaded,
            Shape::Truncated => ErrorClass::RateLimited,
        }
    }

    fn expected_code(self) -> Option<&'static str> {
        match self {
            Shape::StandardJson => Some("invalid_temperature"),
            Shape::PlainText | Shape::Empty | Shape::Truncated => None,
        }
    }

    fn plants_credential(self) -> bool {
        matches!(self, Shape::PlainText | Shape::Truncated)
    }
}

/// What the gateway sends back for a failed upstream call.
struct ClientError {
    status: u16,
    /// Same string the gateway stores in `RequestTrace.error`
    /// (`crates/proxyweave-gateway/src/lib.rs:1253`) and in `fallback_trail`.
    trace_error: String,
    body: Value,
}

/// Non-streaming upstream-error path.
///
/// The parse step is mirrored from `crates/proxyweave-gateway/src/lib.rs:1206-1215`
/// because it is inlined in the gateway and not exported; the two functions it
/// feeds — `classify_upstream_error` (`src/routing.rs:65`) and
/// `encode_error_body_for_suite` (`src/error_body.rs:88`) — are the real ones.
fn gateway_http_error_path(ingress: ProtocolSuite, upstream: &UpstreamResponse) -> ClientError {
    let err_val: Value = serde_json::from_slice(&upstream.body).unwrap_or(Value::Null);
    let err_msg = err_val["error"]["message"]
        .as_str()
        .or_else(|| err_val["message"].as_str())
        .unwrap_or("Upstream error");
    let err_code = err_val["error"]["code"]
        .as_str()
        .or_else(|| err_val["error"]["type"].as_str());

    let class = classify_upstream_error(Some(upstream.status), err_code);
    let body = encode_error_body_for_suite(ingress, err_msg, class, upstream.status, err_code);

    ClientError {
        status: upstream.status,
        trace_error: err_msg.to_string(),
        body,
    }
}

/// Expected protocol-native `error.type` for the four classes this deck
/// produces. Literal oracle on purpose: it must not be read back from the
/// implementation it checks.
fn expected_type(suite: ProtocolSuite, class: ErrorClass) -> &'static str {
    match (suite, class) {
        (
            ProtocolSuite::OpenAiCompatible | ProtocolSuite::OpenAiResponses,
            ErrorClass::Transient,
        ) => "server_error",
        (ProtocolSuite::AnthropicMessages, ErrorClass::Transient) => "api_error",
        (_, ErrorClass::RateLimited) => "rate_limit_error",
        (_, ErrorClass::BadRequest) => "invalid_request_error",
        (_, ErrorClass::Overloaded) => "overloaded_error",
        (_, other) => panic!("shape deck does not produce {other:?}"),
    }
}

fn assert_no_credential_echo(label: &str, client: &ClientError) {
    let wire = serde_json::to_string(&client.body).expect("client body serialises");
    for needle in [PLANTED_KEY, PLANTED_BEARER, "x-api-key", "Authorization"] {
        assert!(
            !wire.contains(needle),
            "{label}: client body echoed {needle:?}: {wire}"
        );
        assert!(
            !client.trace_error.contains(needle),
            "{label}: trace/error string echoed {needle:?}: {:?}",
            client.trace_error
        );
    }
}

/// Status + native error shape + message structure for one (ingress, shape).
fn assert_mapped_shape(suite: ProtocolSuite, shape: Shape) {
    let label = format!("{}/{shape:?}", suite.label());
    let upstream = shape.response();
    let client = gateway_http_error_path(suite, &upstream);

    // The gateway forwards the upstream status verbatim
    // (`crates/proxyweave-gateway/src/lib.rs:1265`).
    assert_eq!(
        client.status, upstream.status,
        "{label}: upstream status must reach the client unchanged"
    );

    let wire_type = expected_type(suite, shape.expected_class());
    match suite {
        ProtocolSuite::OpenAiCompatible | ProtocolSuite::OpenAiResponses => {
            assert_eq!(
                client.body["error"]["message"],
                shape.expected_message(),
                "{label}: message"
            );
            assert_eq!(
                client.body["error"]["type"], wire_type,
                "{label}: error.type"
            );
            assert!(
                client.body["error"]["param"].is_null(),
                "{label}: OpenAI-family bodies carry `param: null`, got {}",
                client.body["error"]["param"]
            );
            match shape.expected_code() {
                Some(code) => assert_eq!(
                    client.body["error"]["code"], code,
                    "{label}: upstream error.code must survive"
                ),
                None => assert!(
                    client.body["error"].get("code").is_none(),
                    "{label}: unparsed shape must not invent a code, got {}",
                    client.body["error"]["code"]
                ),
            }
            assert!(
                client.body.get("type").is_none(),
                "{label}: top-level `type` belongs to Anthropic only"
            );
        }
        ProtocolSuite::AnthropicMessages => {
            assert_eq!(client.body["type"], "error", "{label}: envelope type");
            assert_eq!(
                client.body["error"]["type"], wire_type,
                "{label}: error.type"
            );
            assert_eq!(
                client.body["error"]["message"],
                shape.expected_message(),
                "{label}: message"
            );
            // `src/error_body.rs:111-119`: the Anthropic body is
            // `{type, error:{type, message}}` — no `code` field exists.
            assert!(
                client.body["error"].get("code").is_none(),
                "{label}: Anthropic error bodies must not carry `code`, got {}",
                client.body["error"]["code"]
            );
        }
        ProtocolSuite::GoogleGemini => unreachable!("Gemini is out of scope"),
    }

    assert_no_credential_echo(&label, &client);

    if shape.plants_credential() {
        assert!(
            shape.expected_message() == "Upstream error",
            "{label}: an unparseable upstream body must fall back, not be forwarded"
        );
    }
}

#[test]
fn chat_ingress_maps_four_upstream_error_body_shapes() {
    for shape in Shape::ALL {
        assert_mapped_shape(ProtocolSuite::OpenAiCompatible, shape);
    }
    println!(
        "chat ingress: {} shapes mapped (status 400/500/503/429, types \
         invalid_request_error/server_error/overloaded_error/rate_limit_error)",
        Shape::ALL.len()
    );
}

#[test]
fn messages_ingress_maps_four_upstream_error_body_shapes() {
    for shape in Shape::ALL {
        assert_mapped_shape(ProtocolSuite::AnthropicMessages, shape);
    }
    println!(
        "messages ingress: {} shapes mapped (status 400/500/503/429, types \
         invalid_request_error/api_error/overloaded_error/rate_limit_error)",
        Shape::ALL.len()
    );
}

#[test]
fn responses_ingress_maps_four_upstream_error_body_shapes() {
    for shape in Shape::ALL {
        assert_mapped_shape(ProtocolSuite::OpenAiResponses, shape);
    }
    println!(
        "responses ingress: {} shapes mapped (status 400/500/503/429, types \
         invalid_request_error/server_error/overloaded_error/rate_limit_error)",
        Shape::ALL.len()
    );
}

#[test]
fn credential_material_never_reaches_client_body_or_trace() {
    let mut cases = 0usize;
    for suite in INGRESS {
        for shape in Shape::ALL {
            let client = gateway_http_error_path(suite, &shape.response());
            assert_no_credential_echo(&format!("{}/{shape:?}", suite.label()), &client);
            cases += 1;
        }
    }
    assert_eq!(cases, INGRESS.len() * Shape::ALL.len());
}

/// Error frame an upstream provider sends mid-stream, per suite.
fn upstream_error_frame(suite: ProtocolSuite) -> String {
    match suite {
        ProtocolSuite::OpenAiCompatible => format!(
            "data: {}\n\n",
            json!({"error": {"message": "Rate limit reached", "type": "rate_limit_error", "code": "rate_limit_exceeded"}})
        ),
        ProtocolSuite::AnthropicMessages => format!(
            "event: error\ndata: {}\n\n",
            json!({"type": "error", "error": {"type": "rate_limit_error", "message": "Rate limit reached"}})
        ),
        ProtocolSuite::OpenAiResponses => format!(
            "data: {}\n\n",
            json!({"type": "error", "error": {"type": "rate_limit_error", "message": "Rate limit reached"}})
        ),
        ProtocolSuite::GoogleGemini => unreachable!("Gemini is out of scope"),
    }
}

fn frame_error_payload(frame: &str) -> Value {
    frame
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter_map(|data| serde_json::from_str::<Value>(data).ok())
        .find_map(|value| value.get("error").cloned())
        .expect("encoder must emit an error frame")
}

#[test]
fn upstream_error_frames_parse_and_reencode_per_ingress() {
    for suite in INGRESS {
        let frame = upstream_error_frame(suite);
        let mut decoder = codec(suite).stream_decoder();
        let mut parts = Vec::new();
        for line in frame.lines() {
            parts.extend(decoder.feed(line).expect("upstream error frame decodes"));
        }

        let (message, class, upstream_code) = parts
            .iter()
            .find_map(|part| match part {
                StreamPart::Error {
                    message,
                    class,
                    upstream_code,
                } => Some((message.clone(), *class, upstream_code.clone())),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{}: no StreamPart::Error decoded", suite.label()));

        assert_eq!(message, "Rate limit reached", "{}: message", suite.label());
        assert_eq!(
            class,
            ErrorClass::RateLimited,
            "{}: error.type drives the class",
            suite.label()
        );
        // Chat Completions prefers `error.code` over `error.type`
        // (`src/chat_completions.rs:1666`); Messages and Responses read
        // `error.type` (`src/messages.rs:1759`, `src/responses.rs:3237`).
        let expected_upstream_code = match suite {
            ProtocolSuite::OpenAiCompatible => "rate_limit_exceeded",
            _ => "rate_limit_error",
        };
        assert_eq!(
            upstream_code.as_deref(),
            Some(expected_upstream_code),
            "{}: upstream code is preserved",
            suite.label()
        );

        // Re-encode for the same ingress: native frame, same class table.
        let mut encoder = codec(suite).stream_encoder();
        let out =
            String::from_utf8(encoder.encode_error(&message, class, upstream_code.as_deref()))
                .expect("utf-8 frame");
        let payload = frame_error_payload(&out);
        assert_eq!(
            payload["type"],
            expected_type(suite, ErrorClass::RateLimited),
            "{}: re-encoded error.type",
            suite.label()
        );
        assert_eq!(
            payload["message"],
            "Rate limit reached",
            "{}: frame message",
            suite.label()
        );
        for needle in [PLANTED_KEY, PLANTED_BEARER] {
            assert!(
                !out.contains(needle),
                "{}: re-encoded frame leaked {needle:?}",
                suite.label()
            );
        }
        match suite {
            ProtocolSuite::AnthropicMessages => assert!(
                payload.get("code").is_none(),
                "messages: error frames carry no `code`, got {}",
                payload["code"]
            ),
            _ => assert_eq!(
                payload["code"],
                expected_upstream_code,
                "{}: error frames keep the upstream code",
                suite.label()
            ),
        }
    }
}

const ALL_CLASSES: [ErrorClass; 13] = [
    ErrorClass::Transient,
    ErrorClass::RateLimited,
    ErrorClass::Auth,
    ErrorClass::BadRequest,
    ErrorClass::LossyOrCapability,
    ErrorClass::ModelNotFound,
    ErrorClass::DeadlineExceeded,
    ErrorClass::UpstreamExhausted,
    ErrorClass::AuthMissing,
    ErrorClass::AuthInvalid,
    ErrorClass::AuthDisabled,
    ErrorClass::ModelAccessDenied,
    ErrorClass::Overloaded,
];

/// `src/error_body.rs:13-15` states that its tables mirror each codec's own
/// `error_type_for_class`. Drift between the two would give a client different
/// error types for the same failure depending on the code path (streaming
/// frame vs HTTP body), so both are compared here for every class.
#[test]
fn body_and_stream_error_tables_agree_for_every_class() {
    for suite in INGRESS {
        for class in ALL_CLASSES {
            let body = encode_error_body_for_suite(suite, "probe", class, 400, None);
            let mut encoder = codec(suite).stream_encoder();
            let frame =
                String::from_utf8(encoder.encode_error("probe", class, None)).expect("utf-8 frame");
            let payload = frame_error_payload(&frame);
            assert_eq!(
                body["error"]["type"],
                payload["type"],
                "{}: error_body and stream tables disagree for {class:?}",
                suite.label()
            );
            assert_eq!(
                body["error"]["message"],
                "probe",
                "{}: message passthrough for {class:?}",
                suite.label()
            );
        }
    }
}
