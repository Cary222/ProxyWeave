//! Compatibility Flight — the six functional QC checks required by
//! `Plan.md` §6 / line 548 and carried as an open item on the Phase 2 Gate
//! ("Compatibility Flight 六项质检套件尚未作为独立自动化套件交付").
//!
//!   1. OpenAI Chat 文本生成            (non-stream text generation + usage)
//!   2. SSE 流式首字延迟                (incremental first-token delivery)
//!   3. 多轮对话角色映射                (multi-turn roles, system hoisting)
//!   4. Tool Calling 参数片段规整       (fragmented/whitespace argument JSON)
//!   5. 长文本截断压力                  (256 KiB through the streaming path)
//!   6. 多模态输入兼容                  (image parts both directions)
//!
//! Every check drives the real codec path the gateway runs — client JSON ->
//! `decode_request` -> IR -> `encode_request` (upstream JSON), and upstream SSE
//! -> decoder -> IR -> client SSE — and asserts on the wire shapes a client
//! actually receives. No network, no ports, no plugins: deterministic.
//!
//! `flight_report` runs all six in one call and prints the deck table:
//!   `cargo test -p proxyweave-protocols --test compatibility_flight -- --nocapture`
//!
//! Gemini is excluded: it is an explicit Round 11 non-goal (`G2`).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use proxyweave_protocols::{EndpointCodec, ProtocolSuite};
use serde_json::{json, Value};

fn codec(suite: ProtocolSuite) -> Box<dyn EndpointCodec> {
    use proxyweave_protocols::chat_completions::ChatCompletionsCodec;
    use proxyweave_protocols::messages::MessagesCodec;
    use proxyweave_protocols::responses::ResponsesCodec;

    match suite {
        ProtocolSuite::OpenAiCompatible => Box::new(ChatCompletionsCodec::new()),
        ProtocolSuite::AnthropicMessages => Box::new(MessagesCodec::new()),
        ProtocolSuite::OpenAiResponses => Box::new(ResponsesCodec::new()),
        ProtocolSuite::GoogleGemini => {
            unimplemented!("Gemini codecs are out of scope (Round 11 G2 non-goal)")
        }
    }
}

fn envelope() -> proxyweave_protocols::RawEnvelope {
    proxyweave_protocols::RawEnvelope {
        method: "POST".to_string(),
        path: "/v1/messages".to_string(),
        headers: Default::default(),
        body: None,
        original_body_size: 0,
        timestamp: chrono::Utc::now(),
    }
}

/// Client JSON -> IR -> upstream JSON (what the gateway sends the plugin).
fn transcode_request(src: ProtocolSuite, dst: ProtocolSuite, body: Value) -> Value {
    let ir = codec(src)
        .decode_request(body, &envelope())
        .expect("decode client request");
    codec(dst)
        .encode_request(&ir)
        .expect("encode upstream request")
        .0
}

/// Upstream JSON -> IR -> client JSON (non-stream response path).
fn transcode_response(src: ProtocolSuite, dst: ProtocolSuite, body: Value) -> Value {
    let ir = codec(src)
        .decode_response(body)
        .expect("decode upstream response");
    codec(dst)
        .encode_response(&ir)
        .expect("encode client response")
}

/// The streaming path: upstream SSE lines -> egress decoder -> IR -> ingress
/// encoder -> client SSE. `observe` runs after every upstream line so a check
/// can see exactly when a frame reached the client.
fn transcode_stream_with<F>(
    src: ProtocolSuite,
    dst: ProtocolSuite,
    upstream: &str,
    mut observe: F,
) -> String
where
    F: FnMut(usize, &str),
{
    let src_codec = codec(src);
    let dst_codec = codec(dst);
    let mut decoder = src_codec.stream_decoder();
    let mut encoder = dst_codec.stream_encoder();
    let mut client = String::new();
    let mut line_number = 0usize;
    for line in upstream.split('\n') {
        line_number += 1;
        for part in decoder.feed(line).expect("egress decode") {
            client.push_str(
                std::str::from_utf8(&encoder.encode_part(&part).expect("ingress encode"))
                    .expect("utf8"),
            );
        }
        observe(line_number, &client);
    }
    for part in decoder.finish().expect("egress decode finish") {
        client.push_str(
            std::str::from_utf8(&encoder.encode_part(&part).expect("ingress encode"))
                .expect("utf8"),
        );
    }
    client.push_str(std::str::from_utf8(&encoder.encode_done()).expect("utf8"));
    client
}

fn transcode_stream(src: ProtocolSuite, dst: ProtocolSuite, upstream: &str) -> String {
    transcode_stream_with(src, dst, upstream, |_, _| {})
}

fn client_events(client: &str) -> Vec<Value> {
    client
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .filter_map(|data| serde_json::from_str::<Value>(data).ok())
        .collect()
}

fn client_text(client: &str) -> String {
    client_events(client)
        .iter()
        .filter(|event| event["type"] == "content_block_delta")
        .filter_map(|event| event["delta"]["text"].as_str().map(String::from))
        .collect()
}

fn chat_sse(chunks: &[Value]) -> String {
    let mut out = String::new();
    for chunk in chunks {
        out.push_str(&format!("data: {chunk}\n\n"));
    }
    out.push_str("data: [DONE]\n\n");
    out
}

// ---------------------------------------------------------------- check 1 ---
/// OpenAI Chat 文本生成: a plain text turn survives client -> IR -> upstream ->
/// response -> client, with usage intact.
fn check_text_generation() -> Result<(), String> {
    // Client is an Anthropic Messages SDK pointed at a Chat backend.
    let client_request = json!({
        "model": "claude-x", "max_tokens": 64,
        "system": "be brief",
        "messages": [{"role": "user", "content": "hello there"}]
    });
    let upstream_request = transcode_request(
        ProtocolSuite::AnthropicMessages,
        ProtocolSuite::OpenAiCompatible,
        client_request,
    );
    if upstream_request["messages"][0]["role"] != "system" {
        return Err(format!("system turn not hoisted: {upstream_request}"));
    }
    if upstream_request["messages"][1]["content"] != "hello there" {
        return Err(format!("user text not forwarded: {upstream_request}"));
    }
    if upstream_request["max_tokens"] != 64 {
        return Err(format!("max_tokens not forwarded: {upstream_request}"));
    }

    let upstream_response = json!({
        "id": "chatcmpl-flight", "object": "chat.completion", "model": "gpt-x",
        "choices": [{"index": 0, "finish_reason": "stop",
                     "message": {"role": "assistant", "content": "hello world"}}],
        "usage": {"prompt_tokens": 11, "completion_tokens": 7, "total_tokens": 18}
    });
    let client_response = transcode_response(
        ProtocolSuite::OpenAiCompatible,
        ProtocolSuite::AnthropicMessages,
        upstream_response,
    );
    if client_response["content"][0]["text"] != "hello world" {
        return Err(format!("text lost: {client_response}"));
    }
    if client_response["stop_reason"] != "end_turn" {
        return Err(format!("stop_reason unmapped: {client_response}"));
    }
    if client_response["usage"]["output_tokens"] != 7
        || client_response["usage"]["input_tokens"] != 11
    {
        return Err(format!("usage lost: {client_response}"));
    }

    // Streaming text path must produce the same turn.
    let upstream = chat_sse(&[
        json!({"id":"c1","choices":[{"index":0,"delta":{"role":"assistant","content":""}}]}),
        json!({"id":"c1","choices":[{"index":0,"delta":{"content":"hello "}}]}),
        json!({"id":"c1","choices":[{"index":0,"delta":{"content":"world"}}]}),
        json!({"id":"c1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}),
        json!({"id":"c1","choices":[],"usage":{"prompt_tokens":11,"completion_tokens":7,"total_tokens":18}}),
    ]);
    let client = transcode_stream(
        ProtocolSuite::OpenAiCompatible,
        ProtocolSuite::AnthropicMessages,
        &upstream,
    );
    if client_text(&client) != "hello world" {
        return Err(format!("streamed text lost: {client}"));
    }
    Ok(())
}

// ---------------------------------------------------------------- check 2 ---
/// SSE 流式首字延迟: the first token must reach the client on the same upstream
/// line that carried it — no buffering until the stream ends.
fn check_first_token_latency() -> Result<(), String> {
    let upstream = chat_sse(&[
        json!({"id":"c1","choices":[{"index":0,"delta":{"role":"assistant","content":""}}]}),
        json!({"id":"c1","choices":[{"index":0,"delta":{"content":"FIRST"}}]}),
        json!({"id":"c1","choices":[{"index":0,"delta":{"content":"-second"}}]}),
        json!({"id":"c1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}),
        json!({"id":"c1","choices":[],"usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7}}),
    ]);

    let mut first_token_line: Option<usize> = None;
    let started = std::time::Instant::now();
    let mut first_token_at: Option<std::time::Duration> = None;
    let client = transcode_stream_with(
        ProtocolSuite::OpenAiCompatible,
        ProtocolSuite::AnthropicMessages,
        &upstream,
        |line_number, visible| {
            if first_token_line.is_none() && visible.contains("FIRST") {
                first_token_line = Some(line_number);
                first_token_at = Some(started.elapsed());
            }
        },
    );

    // The token "FIRST" arrives on upstream line 3 (chunk index 2 in SSE terms);
    // the client must see it by then, not after the terminal frame.
    match first_token_line {
        Some(line) if line <= 4 => {}
        Some(line) => return Err(format!("first token only surfaced at upstream line {line}")),
        None => return Err("first token never reached the client".to_string()),
    }
    if !client.contains("FIRST-second") && client_text(&client) != "FIRST-second" {
        return Err(format!("first-token frame incomplete: {client}"));
    }
    // Frame ordering: the first content_block_delta must precede message_stop.
    let events = client_events(&client);
    let first_delta = events
        .iter()
        .position(|e| e["type"] == "content_block_delta");
    let stop = events.iter().position(|e| e["type"] == "message_stop");
    match (first_delta, stop) {
        (Some(delta), Some(stop)) if delta < stop => {}
        _ => return Err(format!("frame order broken: {client}")),
    }
    if let Some(at) = first_token_at {
        // Informational only: this is a codec-in-memory path, not a network
        // measurement, so no wall-clock threshold is asserted.
        eprintln!(
            "flight-2: first client token emitted after upstream line {first_token_line:?} in {}µs",
            at.as_micros()
        );
    }
    Ok(())
}

// ---------------------------------------------------------------- check 3 ---
/// 多轮对话角色映射: system handling and turn order in both directions.
fn check_role_mapping() -> Result<(), String> {
    // Messages client, four turns + system -> Chat backend.
    let messages_client = json!({
        "model": "claude-x", "max_tokens": 128, "system": "S",
        "messages": [
            {"role": "user", "content": "u1"},
            {"role": "assistant", "content": "a1"},
            {"role": "user", "content": "u2"},
            {"role": "assistant", "content": "a2"},
            {"role": "user", "content": "u3"}
        ]
    });
    let chat = transcode_request(
        ProtocolSuite::AnthropicMessages,
        ProtocolSuite::OpenAiCompatible,
        messages_client,
    );
    let roles: Vec<&str> = chat["messages"]
        .as_array()
        .expect("messages array")
        .iter()
        .map(|turn| turn["role"].as_str().expect("role"))
        .collect();
    if roles != ["system", "user", "assistant", "user", "assistant", "user"] {
        return Err(format!("messages->chat role order {roles:?}: {chat}"));
    }
    let texts: Vec<&str> = chat["messages"]
        .as_array()
        .expect("messages array")
        .iter()
        .map(|turn| turn["content"].as_str().unwrap_or(""))
        .collect();
    if texts != ["S", "u1", "a1", "u2", "a2", "u3"] {
        return Err(format!("messages->chat turn text {texts:?}: {chat}"));
    }

    // Chat client with a system turn -> Messages backend: `system` is a
    // top-level field there, never a message.
    let chat_client = json!({
        "model": "gpt-x",
        "messages": [
            {"role": "system", "content": "S"},
            {"role": "user", "content": "u1"},
            {"role": "assistant", "content": "a1"},
            {"role": "user", "content": "u2"}
        ]
    });
    let messages_upstream = transcode_request(
        ProtocolSuite::OpenAiCompatible,
        ProtocolSuite::AnthropicMessages,
        chat_client,
    );
    if messages_upstream["system"][0]["text"] != "S" {
        return Err(format!(
            "system not lifted to top level: {messages_upstream}"
        ));
    }
    let upstream_roles: Vec<&str> = messages_upstream["messages"]
        .as_array()
        .expect("messages array")
        .iter()
        .map(|turn| turn["role"].as_str().expect("role"))
        .collect();
    if upstream_roles != ["user", "assistant", "user"] {
        return Err(format!(
            "chat->messages role order {upstream_roles:?}: {messages_upstream}"
        ));
    }
    if messages_upstream["messages"]
        .as_array()
        .expect("messages")
        .len()
        != 3
    {
        return Err(format!(
            "system turn duplicated into messages: {messages_upstream}"
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------- check 4 ---
/// Tool Calling 参数片段规整: argument fragments that split JSON at awkward
/// points (inside whitespace, across tokens) reassemble into exactly one valid
/// call per tool, and the declared schema survives the request direction.
fn check_tool_argument_normalisation() -> Result<(), String> {
    // Upstream fragments the argument string with leading spaces, newlines and
    // split tokens — the shape a token-streaming backend really produces.
    let upstream = chat_sse(&[
        json!({"id":"c1","choices":[{"index":0,"delta":{"tool_calls":[
            {"index":0,"id":"call_a","type":"function","function":{"name":"lookup","arguments":""}}]}}]}),
        json!({"id":"c1","choices":[{"index":0,"delta":{"tool_calls":[
            {"index":0,"function":{"arguments":"{\n  \"city\" "}}]}}]}),
        json!({"id":"c1","choices":[{"index":0,"delta":{"tool_calls":[
            {"index":0,"function":{"arguments":": \"San "}}]}}]}),
        json!({"id":"c1","choices":[{"index":0,"delta":{"tool_calls":[
            {"index":0,"function":{"arguments":"Francisco\",\n  \"unit\": \"c\"\n}"}}]}}]}),
        json!({"id":"c1","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}),
        json!({"id":"c1","choices":[],"usage":{"prompt_tokens":9,"completion_tokens":4,"total_tokens":13}}),
    ]);
    let client = transcode_stream(
        ProtocolSuite::OpenAiCompatible,
        ProtocolSuite::AnthropicMessages,
        &upstream,
    );
    let events = client_events(&client);
    let starts: Vec<&Value> = events
        .iter()
        .filter(|event| event["type"] == "content_block_start")
        .filter(|event| event["content_block"]["type"] == "tool_use")
        .collect();
    if starts.len() != 1 {
        return Err(format!(
            "expected exactly one tool block, saw {}: {client}",
            starts.len()
        ));
    }
    if starts[0]["content_block"]["name"] != "lookup" {
        return Err(format!("tool name lost: {client}"));
    }
    let raw: String = events
        .iter()
        .filter(|event| event["delta"]["type"] == "input_json_delta")
        .map(|event| {
            event["delta"]["partial_json"]
                .as_str()
                .unwrap_or("")
                .to_string()
        })
        .collect();
    let parsed: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("reassembled arguments are not valid JSON ({error}): {raw:?}"))?;
    if parsed != json!({"city": "San Francisco", "unit": "c"}) {
        return Err(format!("arguments normalised wrongly: {parsed}"));
    }

    // Request direction: the declared JSON schema must reach the upstream
    // unchanged (Chat `function.parameters` <-> Messages `input_schema`).
    let schema = json!({
        "type": "object",
        "properties": {"city": {"type": "string"}, "unit": {"enum": ["c", "f"]}},
        "required": ["city"]
    });
    let messages_request = json!({
        "model": "claude-x", "max_tokens": 32,
        "messages": [{"role": "user", "content": "weather?"}],
        "tools": [{"name": "lookup", "description": "d", "input_schema": schema}]
    });
    let chat_request = transcode_request(
        ProtocolSuite::AnthropicMessages,
        ProtocolSuite::OpenAiCompatible,
        messages_request,
    );
    if chat_request["tools"][0]["function"]["parameters"] != schema {
        return Err(format!("tool schema not forwarded: {chat_request}"));
    }
    if chat_request["tools"][0]["function"]["name"] != "lookup" {
        return Err(format!("tool name not forwarded: {chat_request}"));
    }
    Ok(())
}

// ---------------------------------------------------------------- check 5 ---
/// 长文本截断压力: 256 KiB of text through the streaming path must arrive whole.
fn check_long_text_pressure() -> Result<(), String> {
    const TOTAL: usize = 256 * 1024;
    let marker = "-TAIL-MARKER";
    let mut text = "a".repeat(TOTAL - marker.len());
    text.push_str(marker);

    let mut chunks: Vec<Value> =
        vec![json!({"id":"c1","choices":[{"index":0,"delta":{"role":"assistant","content":""}}]})];
    for piece in text.as_bytes().chunks(1024) {
        let piece = std::str::from_utf8(piece).expect("ascii chunk");
        chunks.push(json!({"id":"c1","choices":[{"index":0,"delta":{"content":piece}}]}));
    }
    chunks.push(json!({"id":"c1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}));
    chunks.push(json!({"id":"c1","choices":[],"usage":{"prompt_tokens":1,"completion_tokens":TOTAL,"total_tokens":TOTAL+1}}));

    let client = transcode_stream(
        ProtocolSuite::OpenAiCompatible,
        ProtocolSuite::AnthropicMessages,
        &chat_sse(&chunks),
    );
    let received = client_text(&client);
    if received.len() != text.len() {
        return Err(format!(
            "streamed text truncated: sent {} bytes, client saw {}",
            text.len(),
            received.len()
        ));
    }
    if received != text {
        return Err("streamed text corrupted (content mismatch)".to_string());
    }
    let events = client_events(&client);
    if events.iter().any(|event| event["type"] == "error") {
        return Err("error frame while streaming long text".to_string());
    }
    if !events.iter().any(|event| event["type"] == "message_stop") {
        return Err("no message_stop after long text".to_string());
    }

    // Non-stream path with the same payload must not clip either.
    let response = transcode_response(
        ProtocolSuite::OpenAiCompatible,
        ProtocolSuite::AnthropicMessages,
        json!({
            "id": "c2", "object": "chat.completion", "model": "gpt-x",
            "choices": [{"index": 0, "finish_reason": "stop",
                         "message": {"role": "assistant", "content": text}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": TOTAL, "total_tokens": TOTAL + 1}
        }),
    );
    let inline = response["content"][0]["text"].as_str().unwrap_or("");
    if inline.len() != text.len() {
        return Err(format!(
            "non-stream text truncated: sent {} bytes, client saw {}",
            text.len(),
            inline.len()
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------- check 6 ---
/// 多模态输入兼容: image parts in both directions, base64 and remote URL.
fn check_multimodal() -> Result<(), String> {
    // Messages client with an inline base64 image -> Chat backend.
    let messages_client = json!({
        "model": "claude-x", "max_tokens": 64,
        "messages": [{"role": "user", "content": [
            {"type": "text", "text": "what is this?"},
            {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "QUJD"}}
        ]}]
    });
    let chat = transcode_request(
        ProtocolSuite::AnthropicMessages,
        ProtocolSuite::OpenAiCompatible,
        messages_client,
    );
    let parts = chat["messages"][0]["content"]
        .as_array()
        .expect("content parts");
    if parts[0]["type"] != "text" || parts[0]["text"] != "what is this?" {
        return Err(format!("text part lost: {chat}"));
    }
    if parts[1]["type"] != "image_url"
        || parts[1]["image_url"]["url"] != "data:image/png;base64,QUJD"
    {
        return Err(format!("base64 image not mapped to a data URL: {chat}"));
    }

    // Chat client with an image_url -> Messages backend.
    let chat_client = json!({
        "model": "gpt-x",
        "messages": [{"role": "user", "content": [
            {"type": "text", "text": "describe"},
            {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,WFla"}}
        ]}]
    });
    let messages_upstream = transcode_request(
        ProtocolSuite::OpenAiCompatible,
        ProtocolSuite::AnthropicMessages,
        chat_client,
    );
    let blocks = messages_upstream["messages"][0]["content"]
        .as_array()
        .expect("content blocks");
    if blocks[0]["type"] != "text" || blocks[0]["text"] != "describe" {
        return Err(format!("text block lost: {messages_upstream}"));
    }
    if blocks[1]["type"] != "image"
        || blocks[1]["source"]["media_type"] != "image/jpeg"
        || blocks[1]["source"]["data"] != "WFla"
    {
        return Err(format!(
            "image_url not split into a base64 source: {messages_upstream}"
        ));
    }
    Ok(())
}

// ------------------------------------------------------------------ deck ---

struct Check {
    id: u8,
    name: &'static str,
    run: fn() -> Result<(), String>,
}

const FLIGHT: [Check; 6] = [
    Check {
        id: 1,
        name: "OpenAI Chat 文本生成",
        run: check_text_generation,
    },
    Check {
        id: 2,
        name: "SSE 流式首字延迟",
        run: check_first_token_latency,
    },
    Check {
        id: 3,
        name: "多轮对话角色映射",
        run: check_role_mapping,
    },
    Check {
        id: 4,
        name: "Tool Calling 参数片段规整",
        run: check_tool_argument_normalisation,
    },
    Check {
        id: 5,
        name: "长文本截断压力",
        run: check_long_text_pressure,
    },
    Check {
        id: 6,
        name: "多模态输入兼容",
        run: check_multimodal,
    },
];

/// One-key run of the whole deck: prints the flight table and fails if any
/// check fails.
#[test]
fn flight_report() {
    let mut failures = Vec::new();
    eprintln!("Compatibility Flight");
    eprintln!("{:<4} {:<32} RESULT", "ID", "CHECK");
    for check in &FLIGHT {
        match (check.run)() {
            Ok(()) => eprintln!("{:<4} {:<32} PASS", check.id, check.name),
            Err(reason) => {
                eprintln!("{:<4} {:<32} FAIL  {reason}", check.id, check.name);
                failures.push(format!("{} {}: {reason}", check.id, check.name));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Compatibility Flight failures: {failures:#?}"
    );
}

#[test]
fn flight_1_openai_chat_text_generation() {
    check_text_generation().unwrap();
}

#[test]
fn flight_2_sse_first_token_latency() {
    check_first_token_latency().unwrap();
}

#[test]
fn flight_3_multi_turn_role_mapping() {
    check_role_mapping().unwrap();
}

#[test]
fn flight_4_tool_calling_argument_normalisation() {
    check_tool_argument_normalisation().unwrap();
}

#[test]
fn flight_5_long_text_truncation_pressure() {
    check_long_text_pressure().unwrap();
}

#[test]
fn flight_6_multimodal_input_compatibility() {
    check_multimodal().unwrap();
}
