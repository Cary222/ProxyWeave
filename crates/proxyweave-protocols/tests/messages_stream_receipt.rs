//! Wire-level regression receipt for `docs/verification/PHASE1_REVIEW_STATUS.md` §4.
//!
//! §4 freezes four historical Inspector failures, all with a **Messages client
//! ingress** and a Chat-Completions / Responses backend:
//!
//! 1. `messages->chat parallel tool SSE ID/name/fragment association`  (tool_ids empty, tools_intact false)
//! 2. `messages->chat SSE usage preserved`                            (usage empty object)
//! 3. `messages->responses parallel tool SSE ID/name/fragment association`
//! 4. `messages->responses SSE usage preserved`
//!
//! Those records were taken against a build where `MessagesStreamEncoder`
//! routed `name=None` argument fragments through a single positional cursor.
//! Both backends stream parallel calls *interleaved* (a fragment for call A
//! arrives after the opener of call B), so fragments were bound to the wrong
//! block, the reassembled `partial_json` was invalid, and the Inspector parser
//! then reported `tool_ids` and `usage` as empty for the whole frame.
//!
//! This file locks the repaired behaviour at the wire the Inspector reads — the
//! client-facing Anthropic SSE — for both backends, and asserts the two
//! properties §4 tracks (`tool_ids` / `tools_intact`, `usage preserved`) plus
//! the Anthropic block-nesting contract.
//!
//! Run: `cargo test -p proxyweave-protocols --test messages_stream_receipt`

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
            unimplemented!("gemini codecs are out of scope (Round 11 non-goal)")
        }
    }
}

/// The exact transform the gateway runs when ingress and egress differ:
/// upstream SSE -> egress decoder -> IR -> ingress encoder -> client SSE.
fn transcode(src: ProtocolSuite, dst: ProtocolSuite, upstream: &str) -> String {
    let src_codec = codec(src);
    let dst_codec = codec(dst);
    let mut dec = src_codec.stream_decoder();
    let mut enc = dst_codec.stream_encoder();
    let mut out: Vec<u8> = Vec::new();
    for line in upstream.split('\n') {
        for part in dec.feed(line).expect("egress decode") {
            out.extend_from_slice(&enc.encode_part(&part).expect("ingress encode"));
        }
    }
    for part in dec.finish().expect("egress decode finish") {
        out.extend_from_slice(&enc.encode_part(&part).expect("ingress encode finish"));
    }
    out.extend_from_slice(&enc.encode_done());
    String::from_utf8(out).expect("client SSE is utf-8")
}

fn client_events(client: &str) -> Vec<Value> {
    client
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .filter_map(|data| serde_json::from_str::<Value>(data).ok())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolBlock {
    index: u64,
    id: String,
    name: String,
    /// Concatenated `input_json_delta.partial_json` for this block index.
    partial_json: String,
}

/// What the Inspector's `messages->X` checks read off the client SSE.
#[derive(Debug)]
struct Receipt {
    blocks: Vec<ToolBlock>,
    usage: Option<Value>,
    terminal_deltas: usize,
    has_message_stop: bool,
}

fn receipt(client: &str) -> Receipt {
    let events = client_events(client);
    let mut blocks: Vec<ToolBlock> = Vec::new();
    let mut usage: Option<Value> = None;
    let mut terminal_deltas = 0usize;
    let mut has_message_stop = false;

    for event in &events {
        match event["type"].as_str() {
            Some("content_block_start") => {
                if event["content_block"]["type"] == "tool_use" {
                    blocks.push(ToolBlock {
                        index: event["index"].as_u64().expect("block index"),
                        id: event["content_block"]["id"]
                            .as_str()
                            .expect("tool_use id")
                            .to_string(),
                        name: event["content_block"]["name"]
                            .as_str()
                            .expect("tool_use name")
                            .to_string(),
                        partial_json: String::new(),
                    });
                }
            }
            Some("content_block_delta") => {
                if event["delta"]["type"] == "input_json_delta" {
                    let index = event["index"].as_u64().expect("delta index");
                    let block = blocks
                        .iter_mut()
                        .find(|block| block.index == index)
                        .unwrap_or_else(|| panic!("input_json_delta for unopened block {index}"));
                    block
                        .partial_json
                        .push_str(event["delta"]["partial_json"].as_str().unwrap_or(""));
                }
            }
            Some("message_delta") => {
                if event["delta"]["stop_reason"].is_string() {
                    terminal_deltas += 1;
                    usage = Some(event["usage"].clone());
                }
            }
            Some("message_stop") => has_message_stop = true,
            _ => {}
        }
    }

    Receipt {
        blocks,
        usage,
        terminal_deltas,
        has_message_stop,
    }
}

/// The Anthropic streaming contract: a `content_block_delta` may only appear
/// inside an open block, `content_block_stop` must match the open index, and
/// the terminal `message_delta` / `message_stop` may only appear after every
/// block is closed.
fn assert_anthropic_block_contract(client: &str, label: &str) {
    let events = client_events(client);
    let mut open: Option<u64> = None;
    let mut closed_terminal = false;
    for (position, event) in events.iter().enumerate() {
        match event["type"].as_str() {
            Some("content_block_start") => {
                assert!(
                    open.is_none(),
                    "{label}: nested content_block_start at #{position}"
                );
                assert!(
                    !closed_terminal,
                    "{label}: content_block_start after terminal message_delta at #{position}"
                );
                open = Some(event["index"].as_u64().expect("start index"));
            }
            Some("content_block_delta") => {
                assert_eq!(
                    open,
                    Some(event["index"].as_u64().expect("delta index")),
                    "{label}: content_block_delta outside its block at #{position}"
                );
            }
            Some("content_block_stop") => {
                assert_eq!(
                    open,
                    Some(event["index"].as_u64().expect("stop index")),
                    "{label}: content_block_stop without matching open block at #{position}"
                );
                open = None;
            }
            Some("message_delta") => {
                assert!(
                    open.is_none(),
                    "{label}: message_delta while block {open:?} is still open at #{position}"
                );
                if event["delta"]["stop_reason"].is_string() {
                    closed_terminal = true;
                }
            }
            Some("message_stop") => {
                assert!(
                    open.is_none(),
                    "{label}: message_stop while block {open:?} is still open at #{position}"
                );
            }
            _ => {}
        }
    }
    assert!(
        open.is_none(),
        "{label}: stream ended with an open content block"
    );
}

/// Chat-Completions backend: two parallel calls whose argument fragments are
/// interleaved by `index`, as OpenAI and compatible providers stream them.
const CHAT_INTERLEAVED: &str = concat!(
    "data: {\"id\":\"chatcmpl-receipt\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}]}\n\n",
    "data: {\"id\":\"chatcmpl-receipt\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[",
    "{\"index\":0,\"id\":\"call_weather\",\"type\":\"function\",\"function\":{\"name\":\"get_weather\",\"arguments\":\"\"}},",
    "{\"index\":1,\"id\":\"call_time\",\"type\":\"function\",\"function\":{\"name\":\"get_time\",\"arguments\":\"\"}}",
    "]}}]}\n\n",
    "data: {\"id\":\"chatcmpl-receipt\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":1,\"function\":{\"arguments\":\"{\\\"tz\\\":\"}}]}}]}\n\n",
    "data: {\"id\":\"chatcmpl-receipt\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"city\\\":\"}}]}}]}\n\n",
    "data: {\"id\":\"chatcmpl-receipt\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":1,\"function\":{\"arguments\":\"\\\"UTC\\\"}\"}}]}}]}\n\n",
    "data: {\"id\":\"chatcmpl-receipt\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"SF\\\"}\"}}]}}]}\n\n",
    "data: {\"id\":\"chatcmpl-receipt\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
    "data: {\"id\":\"chatcmpl-receipt\",\"choices\":[],\"usage\":{\"prompt_tokens\":100,\"completion_tokens\":20,\"total_tokens\":120}}\n\n",
    "data: [DONE]\n\n",
);

/// Responses backend: the same two calls, interleaved through `item_id`.
const RESPONSES_INTERLEAVED: &str = concat!(
    "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_receipt\",\"status\":\"in_progress\"}}\n\n",
    "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"id\":\"fc_weather\",\"type\":\"function_call\",\"call_id\":\"call_weather\",\"name\":\"get_weather\",\"arguments\":\"\"}}\n\n",
    "data: {\"type\":\"response.output_item.added\",\"output_index\":1,\"item\":{\"id\":\"fc_time\",\"type\":\"function_call\",\"call_id\":\"call_time\",\"name\":\"get_time\",\"arguments\":\"\"}}\n\n",
    "data: {\"type\":\"response.function_call_arguments.delta\",\"item_id\":\"fc_time\",\"output_index\":1,\"delta\":\"{\\\"tz\\\":\"}\n\n",
    "data: {\"type\":\"response.function_call_arguments.delta\",\"item_id\":\"fc_weather\",\"output_index\":0,\"delta\":\"{\\\"city\\\":\"}\n\n",
    "data: {\"type\":\"response.function_call_arguments.delta\",\"item_id\":\"fc_time\",\"output_index\":1,\"delta\":\"\\\"UTC\\\"}\"}\n\n",
    "data: {\"type\":\"response.function_call_arguments.delta\",\"item_id\":\"fc_weather\",\"output_index\":0,\"delta\":\"\\\"SF\\\"}\"}\n\n",
    "data: {\"type\":\"response.function_call_arguments.done\",\"item_id\":\"fc_weather\",\"output_index\":0,\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\":\\\"SF\\\"}\"}\n\n",
    "data: {\"type\":\"response.function_call_arguments.done\",\"item_id\":\"fc_time\",\"output_index\":1,\"name\":\"get_time\",\"arguments\":\"{\\\"tz\\\":\\\"UTC\\\"}\"}\n\n",
    "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"id\":\"fc_weather\",\"type\":\"function_call\",\"call_id\":\"call_weather\",\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\":\\\"SF\\\"}\"}}\n\n",
    "data: {\"type\":\"response.output_item.done\",\"output_index\":1,\"item\":{\"id\":\"fc_time\",\"type\":\"function_call\",\"call_id\":\"call_time\",\"name\":\"get_time\",\"arguments\":\"{\\\"tz\\\":\\\"UTC\\\"}\"}}\n\n",
    "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_receipt\",\"status\":\"completed\",\"usage\":{\"input_tokens\":100,\"output_tokens\":20,\"total_tokens\":120}}}\n\n",
);

/// §4 items 1 and 3: `parallel tool SSE ID/name/fragment association`.
fn assert_parallel_association(src: ProtocolSuite, upstream: &str, label: &str) {
    let client = transcode(src, ProtocolSuite::AnthropicMessages, upstream);
    assert_anthropic_block_contract(&client, label);
    let receipt = receipt(&client);

    // tool_ids: one non-empty, distinct id per call — never empty, never shared.
    assert_eq!(receipt.blocks.len(), 2, "{label}: client blocks {client}");
    let ids: Vec<&str> = receipt
        .blocks
        .iter()
        .map(|block| block.id.as_str())
        .collect();
    assert!(
        ids.iter().all(|id| !id.is_empty()),
        "{label}: empty tool_use id in {ids:?}"
    );
    assert_ne!(ids[0], ids[1], "{label}: parallel calls shared id {ids:?}");

    // tools_intact: every reassembled partial_json is the exact call argument
    // and stays bound to its own name — the interleaving must not cross over.
    let mut seen: Vec<(String, Value)> = Vec::new();
    for block in &receipt.blocks {
        let parsed: Value = serde_json::from_str(&block.partial_json).unwrap_or_else(|error| {
            panic!(
                "{label}: block {} ({}) reassembled invalid JSON: {error}; raw {:?}",
                block.index, block.name, block.partial_json
            )
        });
        seen.push((block.name.clone(), parsed));
    }
    seen.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        seen,
        vec![
            ("get_time".to_string(), json!({"tz": "UTC"})),
            ("get_weather".to_string(), json!({"city": "SF"})),
        ],
        "{label}: tool name/argument association broken; client {client}"
    );
}

/// §4 items 2 and 4: `SSE usage preserved`.
fn assert_usage_preserved(src: ProtocolSuite, upstream: &str, label: &str) {
    let client = transcode(src, ProtocolSuite::AnthropicMessages, upstream);
    assert_anthropic_block_contract(&client, label);
    let receipt = receipt(&client);

    assert!(
        receipt.has_message_stop,
        "{label}: stream has no message_stop; client {client}"
    );
    assert_eq!(
        receipt.terminal_deltas, 1,
        "{label}: exactly one terminal message_delta expected; client {client}"
    );
    let usage = receipt
        .usage
        .unwrap_or_else(|| panic!("{label}: terminal message_delta carries no usage object"));
    assert!(
        !usage
            .as_object()
            .map(|object| object.is_empty())
            .unwrap_or(true),
        "{label}: usage object is empty"
    );
    assert_eq!(
        usage["output_tokens"], 20,
        "{label}: output_tokens not preserved; usage {usage}"
    );
    assert_eq!(
        usage["input_tokens"], 100,
        "{label}: input_tokens not preserved; usage {usage}"
    );
}

#[test]
fn messages_chat_parallel_tool_sse_association() {
    assert_parallel_association(
        ProtocolSuite::OpenAiCompatible,
        CHAT_INTERLEAVED,
        "messages->chat parallel tool SSE ID/name/fragment association",
    );
}

#[test]
fn messages_chat_sse_usage_preserved() {
    assert_usage_preserved(
        ProtocolSuite::OpenAiCompatible,
        CHAT_INTERLEAVED,
        "messages->chat SSE usage preserved",
    );
}

#[test]
fn messages_responses_parallel_tool_sse_association() {
    assert_parallel_association(
        ProtocolSuite::OpenAiResponses,
        RESPONSES_INTERLEAVED,
        "messages->responses parallel tool SSE ID/name/fragment association",
    );
}

#[test]
fn messages_responses_sse_usage_preserved() {
    assert_usage_preserved(
        ProtocolSuite::OpenAiResponses,
        RESPONSES_INTERLEAVED,
        "messages->responses SSE usage preserved",
    );
}

#[test]
fn messages_chat_parallel_tools_survive_idless_fragments() {
    // Same stream with the openers' ids omitted (some compatible providers),
    // still interleaved: the client must not inherit one shared anonymous id
    // and must not lose either call's arguments.
    let idless = CHAT_INTERLEAVED
        .replace("\"id\":\"call_weather\",\"type\":\"function\",", "")
        .replace("\"id\":\"call_time\",\"type\":\"function\",", "");
    assert_parallel_association(
        ProtocolSuite::OpenAiCompatible,
        &idless,
        "messages->chat idless parallel tool SSE association",
    );
}

#[test]
fn messages_chat_without_upstream_usage_still_terminates_with_usage_object() {
    // A backend that ignores `include_usage` sends no usage chunk. The client
    // must still receive a syntactically complete terminal message_delta with
    // an integer `output_tokens` (never a missing/empty usage object), so a
    // strict Anthropic client can finish the turn.
    let no_usage = CHAT_INTERLEAVED
        .replace("data: {\"id\":\"chatcmpl-receipt\",\"choices\":[],\"usage\":{\"prompt_tokens\":100,\"completion_tokens\":20,\"total_tokens\":120}}\n\n", "");
    let client = transcode(
        ProtocolSuite::OpenAiCompatible,
        ProtocolSuite::AnthropicMessages,
        &no_usage,
    );
    assert_anthropic_block_contract(&client, "messages->chat without upstream usage");
    let receipt = receipt(&client);
    assert!(receipt.has_message_stop, "client {client}");
    assert_eq!(receipt.terminal_deltas, 1, "client {client}");
    let usage = receipt.usage.expect("terminal usage object");
    assert!(
        usage["output_tokens"].is_u64(),
        "terminal usage must carry an integer output_tokens; got {usage}"
    );
}
