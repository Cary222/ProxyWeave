//! Safe desktop Flight transport seam.
//!
//! Provides typed model discovery, chat completion (streaming via eventsource-stream
//! and non-streaming fallback), and safe cancellation via active session tracking.

use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::ipc::Channel;
use tokio::sync::{oneshot, Mutex, RwLock};

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightDiscoveredModel {
    pub id: String,
    pub name: String,
    pub created: Option<i64>,
    pub owned_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightModelListResult {
    pub models: Vec<FlightDiscoveredModel>,
    pub active_backend: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightContentPart {
    #[serde(rename = "type")]
    pub part_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<FlightImageUrl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightMessageInput {
    pub role: String,
    pub content: String,
    /// Optional multimodal content parts (only forwarded when supplied by the caller;
    /// the desktop UI only enables this when the backend declares multimodal support).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_parts: Option<Vec<FlightContentPart>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightToolCallFragment {
    pub index: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments_delta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightChatRequestInput {
    pub request_id: Option<String>,
    pub model: String,
    pub messages: Vec<FlightMessageInput>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
    /// OpenAI-compatible tool definitions, forwarded verbatim after bounded validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    /// "none" | "auto" | "required" or an object; forwarded verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightChatStartResult {
    pub request_id: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlightStreamEvent {
    Started {
        request_id: String,
        model: String,
    },
    Delta {
        request_id: String,
        text: Option<String>,
        reasoning: Option<String>,
        first_content_ms: Option<u64>,
        elapsed_ms: u64,
    },
    ToolCallDelta {
        request_id: String,
        fragments: Vec<FlightToolCallFragment>,
        elapsed_ms: u64,
    },
    Completed {
        request_id: String,
        total_text: String,
        total_reasoning: Option<String>,
        first_content_ms: Option<u64>,
        elapsed_ms: u64,
        usage: Option<FlightUsage>,
        finish_reason: Option<String>,
        saw_done: bool,
        total_tool_calls: Option<Vec<FlightToolCall>>,
    },
    Cancelled {
        request_id: String,
        elapsed_ms: u64,
    },
    Failed {
        request_id: String,
        error_message: String,
        elapsed_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightChatNonstreamResult {
    pub request_id: String,
    pub model: String,
    pub text: String,
    pub reasoning: Option<String>,
    pub first_content_ms: Option<u64>,
    pub elapsed_ms: u64,
    pub usage: Option<FlightUsage>,
    pub finish_reason: Option<String>,
    pub tool_calls: Option<Vec<FlightToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightCancelResult {
    pub cancelled: bool,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Session Manager
// ---------------------------------------------------------------------------

struct ActiveSession {
    request_id: String,
    #[allow(dead_code)]
    window_label: String,
    cancel_tx: Option<oneshot::Sender<()>>,
    #[allow(dead_code)]
    start_time: Instant,
}

#[derive(Clone, Default)]
pub struct FlightManager {
    counter: Arc<AtomicU64>,
    // Keyed by window_label (one active session per window)
    sessions: Arc<RwLock<HashMap<String, Arc<Mutex<ActiveSession>>>>>,
}

impl FlightManager {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(AtomicU64::new(1)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn next_request_id(&self) -> String {
        let seq = self.counter.fetch_add(1, Ordering::SeqCst);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        format!("flt-{ts}-{seq}")
    }

    pub async fn register_session(
        &self,
        window_label: &str,
        request_id: &str,
        cancel_tx: oneshot::Sender<()>,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        if let Some(existing) = sessions.get(window_label) {
            let session = existing.lock().await;
            if session.cancel_tx.is_some() {
                return Err("当前已有进行中的飞行会话，请等待完成或取消后再试。".to_string());
            }
        }
        let active = ActiveSession {
            request_id: request_id.to_string(),
            window_label: window_label.to_string(),
            cancel_tx: Some(cancel_tx),
            start_time: Instant::now(),
        };
        sessions.insert(window_label.to_string(), Arc::new(Mutex::new(active)));
        Ok(())
    }

    pub async fn unregister_session(&self, window_label: &str, request_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(existing) = sessions.get(window_label) {
            let session = existing.lock().await;
            if session.request_id == request_id {
                drop(session);
                sessions.remove(window_label);
            }
        }
    }

    pub async fn cancel_session(
        &self,
        window_label: &str,
        request_id: &str,
    ) -> Result<FlightCancelResult, String> {
        let sessions = self.sessions.read().await;
        let session_arc = sessions.get(window_label).cloned();
        drop(sessions);

        if let Some(session_arc) = session_arc {
            let mut session = session_arc.lock().await;
            if session.request_id != request_id {
                return Ok(FlightCancelResult {
                    cancelled: false,
                    message: "请求 ID 不匹配或该会话已结束".to_string(),
                });
            }
            if let Some(tx) = session.cancel_tx.take() {
                let _ = tx.send(());
                Ok(FlightCancelResult {
                    cancelled: true,
                    message: "飞行请求已触发取消".to_string(),
                })
            } else {
                Ok(FlightCancelResult {
                    cancelled: false,
                    message: "会话已在结束流程中".to_string(),
                })
            }
        } else {
            Ok(FlightCancelResult {
                cancelled: false,
                message: "未找到活跃的飞行会话".to_string(),
            })
        }
    }

    pub async fn cancel_all_for_window(&self, window_label: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session_arc) = sessions.remove(window_label) {
            let mut session = session_arc.lock().await;
            if let Some(tx) = session.cancel_tx.take() {
                let _ = tx.send(());
            }
        }
    }

    pub async fn cancel_all(&self) {
        let mut sessions = self.sessions.write().await;
        for (_w, session_arc) in sessions.drain() {
            let mut session = session_arc.lock().await;
            if let Some(tx) = session.cancel_tx.take() {
                let _ = tx.send(());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Security & Validation Helpers
// ---------------------------------------------------------------------------

pub fn caller_allowed(label: &str, origin: &str, dev_origin: Option<&str>) -> bool {
    label == "main"
        && (matches!(
            origin,
            "http://tauri.localhost" | "https://tauri.localhost" | "tauri://localhost"
        ) || dev_origin == Some(origin))
}

pub fn check_flight_caller(window: &tauri::WebviewWindow) -> Result<(), String> {
    use tauri::Manager;
    let url = window.url().map_err(|_| "flight caller denied")?;
    let origin = url.origin().ascii_serialization();
    let dev_origin = if cfg!(debug_assertions) {
        window
            .app_handle()
            .config()
            .build
            .dev_url
            .as_ref()
            .map(|url| url.origin().ascii_serialization())
    } else {
        None
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || !caller_allowed(window.label(), &origin, dev_origin.as_deref())
    {
        return Err("flight caller denied".into());
    }
    Ok(())
}

fn validate_chat_input(input: &FlightChatRequestInput) -> Result<(), String> {
    let model = input.model.trim();
    if model.is_empty() {
        return Err("模型名称不能为空".to_string());
    }
    if model.len() > 128 {
        return Err("模型名称超出允许长度 (128)".to_string());
    }
    if input.messages.is_empty() {
        return Err("消息列表不能为空".to_string());
    }
    if input.messages.len() > 200 {
        return Err("消息条数超出限制 (200)".to_string());
    }
    let mut total_chars = 0usize;
    for m in &input.messages {
        let role = m.role.trim();
        if !matches!(role, "user" | "assistant" | "system" | "tool") {
            return Err(format!("不支持的消息角色: {role}"));
        }
        total_chars += m.content.len();
        if total_chars > 256 * 1024 {
            return Err("消息总长度超出上限 (256KB)".to_string());
        }
        if let Some(parts) = m.content_parts.as_ref() {
            let mut images = 0usize;
            for part in parts {
                match part.part_type.as_str() {
                    "text" => {
                        let text = part.text.as_deref().unwrap_or_default();
                        total_chars += text.len();
                        if total_chars > 256 * 1024 {
                            return Err("消息总长度超出上限 (256KB)".to_string());
                        }
                    }
                    "image_url" => {
                        if role != "user" {
                            return Err("仅 user 消息可携带图片内容".to_string());
                        }
                        images += 1;
                        if images > 4 {
                            return Err("单条消息图片数量超出上限 (4)".to_string());
                        }
                        let url = part
                            .image_url
                            .as_ref()
                            .map(|i| i.url.as_str())
                            .unwrap_or_default();
                        if url.is_empty() {
                            return Err("图片内容缺少 URL".to_string());
                        }
                        if url.len() > 512 * 1024 {
                            return Err("单张图片数据超出上限 (512KB)".to_string());
                        }
                    }
                    other => return Err(format!("不支持的内容类型: {other}")),
                }
            }
        }
    }
    if let Some(tools) = input.tools.as_ref() {
        if tools.is_empty() {
            return Err("工具定义列表不能为空".to_string());
        }
        if tools.len() > 32 {
            return Err("工具定义数量超出上限 (32)".to_string());
        }
        for tool in tools {
            let raw = serde_json::to_string(tool).map_err(|_| "工具定义无法序列化".to_string())?;
            if raw.len() > 64 * 1024 {
                return Err("工具定义总体积超出上限 (64KB)".to_string());
            }
        }
    }
    if let Some(tc) = input.tool_choice.as_ref() {
        match tc {
            serde_json::Value::String(v) => {
                if !matches!(v.as_str(), "none" | "auto" | "required") {
                    return Err("tool_choice 字符串仅支持 none/auto/required".to_string());
                }
            }
            serde_json::Value::Object(_) => {
                let raw =
                    serde_json::to_string(tc).map_err(|_| "tool_choice 无法序列化".to_string())?;
                if raw.len() > 1024 {
                    return Err("tool_choice 对象体积超出上限 (1KB)".to_string());
                }
            }
            _ => return Err("tool_choice 必须是字符串或对象".to_string()),
        }
    }
    if let Some(t) = input.temperature {
        if !(0.0..=2.0).contains(&t) {
            return Err("温度参数须在 0.0 至 2.0 之间".to_string());
        }
    }
    Ok(())
}

fn truncate_error_message(msg: &str) -> String {
    let trimmed = msg.trim();
    if trimmed.chars().count() > 200 {
        let truncated: String = trimmed.chars().take(200).collect();
        format!("{truncated}...")
    } else {
        trimmed.to_string()
    }
}

fn redact_pattern(mut s: String, prefix: &str) -> String {
    let mut search_from = 0;
    while let Some(rel_idx) = s[search_from..].find(prefix) {
        let idx = search_from + rel_idx;
        let rest = &s[idx + prefix.len()..];
        let len = rest
            .find(|c: char| {
                c.is_whitespace() || matches!(c, '"' | '\'' | '}' | ')' | ']' | ',' | ';')
            })
            .unwrap_or(rest.len());
        s.replace_range(idx..idx + prefix.len() + len, "[REDACTED]");
        search_from = idx + "[REDACTED]".len();
    }
    s
}

fn build_wire_messages(input: &FlightChatRequestInput) -> Vec<serde_json::Value> {
    input
        .messages
        .iter()
        .map(|m| {
            let mut obj = serde_json::Map::new();
            obj.insert("role".into(), serde_json::Value::String(m.role.clone()));
            if let Some(parts) = m.content_parts.as_ref() {
                let parts_json = serde_json::to_value(parts)
                    .unwrap_or_else(|_| serde_json::Value::Array(vec![]));
                obj.insert("content".into(), parts_json);
            } else {
                obj.insert(
                    "content".into(),
                    serde_json::Value::String(m.content.clone()),
                );
            }
            serde_json::Value::Object(obj)
        })
        .collect()
}

fn build_chat_payload(input: &FlightChatRequestInput, stream: bool) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "model": input.model,
        "messages": build_wire_messages(input),
        "temperature": input.temperature.unwrap_or(0.7),
        "stream": stream,
    });
    let obj = payload.as_object_mut();
    if let Some(obj) = obj {
        if stream {
            obj.insert(
                "stream_options".to_string(),
                serde_json::json!({ "include_usage": true }),
            );
        }
        if let Some(tools) = input.tools.as_ref() {
            obj.insert("tools".to_string(), serde_json::Value::Array(tools.clone()));
        }
        if let Some(tc) = input.tool_choice.as_ref() {
            obj.insert("tool_choice".to_string(), tc.clone());
        }
    }
    payload
}

fn sanitize_error_message(msg: &str, master_key: &str) -> String {
    let mut s = msg.to_string();
    if !master_key.is_empty() {
        s = s.replace(master_key, "[REDACTED]");
    }
    s = redact_pattern(s, "Bearer ");
    s = redact_pattern(s, "pw_up_");
    s = redact_pattern(s, "sk-");
    s = redact_pattern(s, "http://");
    s = redact_pattern(s, "https://");
    truncate_error_message(&s)
}

async fn read_bounded_bytes(resp: reqwest::Response, max_bytes: usize) -> Result<Vec<u8>, String> {
    let mut stream = resp.bytes_stream();
    let mut buf = Vec::new();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|_| "读取网关响应数据失败".to_string())?;
        if buf.len() + bytes.len() > max_bytes {
            return Err("上游响应体积超出安全上限 (2MB)".to_string());
        }
        buf.extend_from_slice(&bytes);
    }
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Upstream OpenAI wire types (bounded)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct WireModelItem {
    id: String,
    created: Option<i64>,
    owned_by: Option<String>,
}

#[derive(Deserialize)]
struct WireModelList {
    data: Option<Vec<WireModelItem>>,
}

#[derive(Deserialize)]
struct WireUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct WireDelta {
    content: Option<String>,
    #[serde(alias = "reasoning")]
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<WireToolCallDelta>>,
}

#[derive(Deserialize)]
struct WireToolCallFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct WireToolCallDelta {
    index: Option<u64>,
    id: Option<String>,
    #[serde(rename = "type")]
    _call_type: Option<String>,
    function: Option<WireToolCallFunctionDelta>,
}

#[derive(Deserialize)]
struct WireStreamChoice {
    delta: Option<WireDelta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct WireStreamChunk {
    choices: Option<Vec<WireStreamChoice>>,
    usage: Option<WireUsage>,
}

#[derive(Deserialize)]
struct WireErrorDetail {
    message: Option<String>,
}

#[derive(Deserialize)]
struct WireErrorResponse {
    error: Option<WireErrorDetail>,
}

#[derive(Deserialize)]
struct WireNonstreamMessage {
    content: Option<String>,
    #[serde(alias = "reasoning")]
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<WireNonstreamToolCall>>,
}

#[derive(Deserialize)]
struct WireNonstreamToolCallFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct WireNonstreamToolCall {
    id: Option<String>,
    #[serde(rename = "type")]
    call_type: Option<String>,
    function: Option<WireNonstreamToolCallFunction>,
}

#[derive(Deserialize)]
struct WireNonstreamChoice {
    message: Option<WireNonstreamMessage>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct WireNonstreamResponse {
    choices: Option<Vec<WireNonstreamChoice>>,
    usage: Option<WireUsage>,
}

// ---------------------------------------------------------------------------
// Core Execution Logic
// ---------------------------------------------------------------------------

pub async fn list_models_core(
    port: u16,
    master_key: &str,
    active_backend: Option<String>,
) -> Result<FlightModelListResult, String> {
    let url = format!("http://127.0.0.1:{port}/v1/models");
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "无法初始化网络客户端".to_string())?;

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {master_key}"))
        .send()
        .await
        .map_err(|_| "请求本地网关失败，网关可能未启动或连接超时".to_string())?;

    let status = resp.status();
    if !status.is_success() {
        let code = status.as_u16();
        if code == 401 {
            return Err("网关鉴权失败 (401 Unauthorized)".to_string());
        }
        let body_bytes = read_bounded_bytes(resp, 64 * 1024)
            .await
            .unwrap_or_default();
        let custom_err = serde_json::from_slice::<WireErrorResponse>(&body_bytes)
            .ok()
            .and_then(|e| e.error)
            .and_then(|d| d.message)
            .map(|m| sanitize_error_message(&m, master_key));

        if let Some(detail) = custom_err {
            return Err(format!("网关请求失败 ({code}): {detail}"));
        }

        return Err(match code {
            429 => "上游请求过于频繁 (429 Rate Limit)".to_string(),
            502..=504 => "网关后端服务不可用，插件可能未正常运行".to_string(),
            _ => format!("网关返回错误状态: {code}"),
        });
    }

    let body_bytes = read_bounded_bytes(resp, 2 * 1024 * 1024).await?;
    let wire: WireModelList = serde_json::from_slice(&body_bytes)
        .map_err(|_| "网关返回的模型列表格式无法解析".to_string())?;

    let models = wire
        .data
        .unwrap_or_default()
        .into_iter()
        .map(|m| FlightDiscoveredModel {
            id: m.id.clone(),
            name: m.id,
            created: m.created,
            owned_by: m.owned_by,
        })
        .collect();

    Ok(FlightModelListResult {
        models,
        active_backend,
    })
}

pub async fn run_flight_chat_stream_core(
    port: u16,
    master_key: &str,
    input: FlightChatRequestInput,
    request_id: String,
    channel: Channel<FlightStreamEvent>,
    mut cancel_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    validate_chat_input(&input)?;

    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "无法初始化网络客户端".to_string())?;

    let payload = build_chat_payload(&input, true);

    let start_time = Instant::now();
    if channel
        .send(FlightStreamEvent::Started {
            request_id: request_id.clone(),
            model: input.model.clone(),
        })
        .is_err()
    {
        return Ok(());
    }

    let req_future = client
        .post(&url)
        .header("Authorization", format!("Bearer {master_key}"))
        .header("Accept", "text/event-stream")
        .json(&payload)
        .send();

    // Select on headers arrival or cancel
    let resp = tokio::select! {
        _ = &mut cancel_rx => {
            let elapsed_ms = start_time.elapsed().as_millis() as u64;
            let _ = channel.send(FlightStreamEvent::Cancelled {
                request_id: request_id.clone(),
                elapsed_ms,
            });
            return Ok(());
        }
        res = req_future => {
            match res {
                Ok(r) => r,
                Err(_) => {
                    let elapsed_ms = start_time.elapsed().as_millis() as u64;
                    let _ = channel.send(FlightStreamEvent::Failed {
                        request_id: request_id.clone(),
                        error_message: "连接网关失败，网关可能未启动或断开".to_string(),
                        elapsed_ms,
                    });
                    return Err("连接网关失败".to_string());
                }
            }
        }
    };

    let status = resp.status();
    if !status.is_success() {
        let code = status.as_u16();
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        let err_msg = if code == 401 {
            "网关鉴权失败 (401 Unauthorized)".to_string()
        } else {
            let body_bytes = read_bounded_bytes(resp, 64 * 1024)
                .await
                .unwrap_or_default();
            let custom_err = serde_json::from_slice::<WireErrorResponse>(&body_bytes)
                .ok()
                .and_then(|e| e.error)
                .and_then(|d| d.message)
                .map(|m| sanitize_error_message(&m, master_key));

            if let Some(detail) = custom_err {
                format!("网关请求失败 ({code}): {detail}")
            } else {
                match code {
                    429 => "上游请求过于频繁 (429 Rate Limit)".to_string(),
                    502..=504 => "网关后端服务不可用，插件未响应".to_string(),
                    _ => format!("网关请求失败 ({code})"),
                }
            }
        };

        let _ = channel.send(FlightStreamEvent::Failed {
            request_id: request_id.clone(),
            error_message: err_msg.clone(),
            elapsed_ms,
        });
        return Err(err_msg);
    }

    let mut stream = resp.bytes_stream().eventsource();
    let mut total_text = String::new();
    let mut total_reasoning = String::new();
    let mut first_content_ms = None;
    let mut last_usage = None;
    let mut finish_reason = None;
    let mut saw_terminal_done = false;
    // Accumulate streaming tool_call fragments keyed by their OpenAI index.
    let mut tool_calls_acc: std::collections::BTreeMap<u64, (String, String, String)> =
        std::collections::BTreeMap::new();

    loop {
        tokio::select! {
            _ = &mut cancel_rx => {
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                let _ = channel.send(FlightStreamEvent::Cancelled {
                    request_id: request_id.clone(),
                    elapsed_ms,
                });
                return Ok(());
            }
            timed_item = tokio::time::timeout(Duration::from_secs(60), stream.next()) => {
                let item = match timed_item {
                    Ok(item) => item,
                    Err(_) => {
                        let elapsed_ms = start_time.elapsed().as_millis() as u64;
                        let _ = channel.send(FlightStreamEvent::Failed {
                            request_id: request_id.clone(),
                            error_message: "上游数据流响应超时 (60s 无数据)".to_string(),
                            elapsed_ms,
                        });
                        return Err("上游数据流超时".to_string());
                    }
                };
                match item {
                    Some(Ok(event)) => {
                        let data = event.data.trim();
                        if data == "[DONE]" {
                            saw_terminal_done = true;
                            break;
                        }
                        if data.len() > 512 * 1024 {
                            let elapsed_ms = start_time.elapsed().as_millis() as u64;
                            let _ = channel.send(FlightStreamEvent::Failed {
                                request_id: request_id.clone(),
                                error_message: "上游数据流单帧体积超出安全上限 (512KB)".to_string(),
                                elapsed_ms,
                            });
                            return Err("上游数据流单帧过大".to_string());
                        }
                        if event.event == "error" {
                            let elapsed_ms = start_time.elapsed().as_millis() as u64;
                            let detail = serde_json::from_str::<WireErrorResponse>(data)
                                .ok()
                                .and_then(|e| e.error)
                                .and_then(|d| d.message)
                                .map(|m| sanitize_error_message(&m, master_key))
                                .unwrap_or_else(|| "上游数据流返回错误事件".to_string());
                            let _ = channel.send(FlightStreamEvent::Failed {
                                request_id: request_id.clone(),
                                error_message: format!("上游错误: {detail}"),
                                elapsed_ms,
                            });
                            return Err("上游数据流返回错误事件".to_string());
                        }
                        match serde_json::from_str::<WireStreamChunk>(data) {
                            Ok(chunk) => {
                                if chunk.choices.is_none() && chunk.usage.is_none() {
                                    // Check if it's an inline error object
                                    if let Ok(err_resp) = serde_json::from_str::<WireErrorResponse>(data) {
                                        if let Some(err_detail) = err_resp.error {
                                            let elapsed_ms = start_time.elapsed().as_millis() as u64;
                                            let detail = err_detail
                                                .message
                                                .map(|m| sanitize_error_message(&m, master_key))
                                                .unwrap_or_else(|| "上游返回错误对象".to_string());
                                            let _ = channel.send(FlightStreamEvent::Failed {
                                                request_id: request_id.clone(),
                                                error_message: format!("上游错误: {detail}"),
                                                elapsed_ms,
                                            });
                                            return Err("上游返回错误对象".to_string());
                                        }
                                    }
                                }
                                if let Some(u) = chunk.usage {
                                    last_usage = Some(FlightUsage {
                                        prompt_tokens: u.prompt_tokens.unwrap_or(0),
                                        completion_tokens: u.completion_tokens.unwrap_or(0),
                                        total_tokens: u.total_tokens.unwrap_or(0),
                                    });
                                }
                                if let Some(choices) = chunk.choices {
                                    for choice in choices {
                                        if let Some(r) = choice.finish_reason {
                                            if !r.is_empty() {
                                                finish_reason = Some(r);
                                            }
                                        }
                                        if let Some(delta) = choice.delta {
                                            let text_delta = delta.content.filter(|s| !s.is_empty());
                                            let reasoning_delta =
                                                delta.reasoning_content.filter(|s| !s.is_empty());
                                            // Streaming tool_call fragments: accumulate by index
                                            // and forward each raw fragment for real-time display.
                                            if let Some(delta_calls) = delta.tool_calls {
                                                if !delta_calls.is_empty() {
                                                    let now_elapsed =
                                                        start_time.elapsed().as_millis() as u64;
                                                    if first_content_ms.is_none() {
                                                        first_content_ms = Some(now_elapsed);
                                                    }
                                                    let mut fragments = Vec::new();
                                                    for tc in delta_calls {
                                                        let idx = tc.index.unwrap_or(0);
                                                        let slot = tool_calls_acc
                                                            .entry(idx)
                                                            .or_insert_with(|| {
                                                                (
                                                                    String::new(),
                                                                    String::new(),
                                                                    String::new(),
                                                                )
                                                            });
                                                        if let Some(id) = tc.id.as_ref() {
                                                            if !id.is_empty() {
                                                                slot.0 = id.clone();
                                                            }
                                                        }
                                                        let mut arg_delta: Option<String> = None;
                                                        if let Some(f) = tc.function.as_ref() {
                                                            if let Some(name) = f.name.as_ref() {
                                                                if !name.is_empty() {
                                                                    slot.1 = name.clone();
                                                                }
                                                            }
                                                            if let Some(args) = f.arguments.as_ref() {
                                                                slot.2.push_str(args);
                                                                arg_delta = Some(args.clone());
                                                            }
                                                        }
                                                        fragments.push(FlightToolCallFragment {
                                                            index: idx,
                                                            id: tc.id.clone(),
                                                            name: if slot.1.is_empty() {
                                                                None
                                                            } else {
                                                                Some(slot.1.clone())
                                                            },
                                                            arguments_delta: arg_delta,
                                                        });
                                                    }
                                                    let _ = channel.send(
                                                        FlightStreamEvent::ToolCallDelta {
                                                            request_id: request_id.clone(),
                                                            fragments,
                                                            elapsed_ms: now_elapsed,
                                                        },
                                                    );
                                                }
                                            }
                                            if text_delta.is_some() || reasoning_delta.is_some() {
                                                let now_elapsed =
                                                    start_time.elapsed().as_millis() as u64;
                                                if first_content_ms.is_none() {
                                                    first_content_ms = Some(now_elapsed);
                                                }
                                                if let Some(ref t) = text_delta {
                                                    total_text.push_str(t);
                                                }
                                                if let Some(ref r) = reasoning_delta {
                                                    total_reasoning.push_str(r);
                                                }
                                                if total_text.len() + total_reasoning.len()
                                                    > 2 * 1024 * 1024
                                                {
                                                    let _ = channel.send(FlightStreamEvent::Failed {
                                                        request_id: request_id.clone(),
                                                        error_message:
                                                            "会话累计输出超出上限 (2MB)".to_string(),
                                                        elapsed_ms: now_elapsed,
                                                    });
                                                    return Err("会话输出超出上限".to_string());
                                                }
                                                let send_res =
                                                    channel.send(FlightStreamEvent::Delta {
                                                        request_id: request_id.clone(),
                                                        text: text_delta,
                                                        reasoning: reasoning_delta,
                                                        first_content_ms,
                                                        elapsed_ms: now_elapsed,
                                                    });
                                                if send_res.is_err() {
                                                    // Channel dropped by frontend
                                                    return Ok(());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                                let _ = channel.send(FlightStreamEvent::Failed {
                                    request_id: request_id.clone(),
                                    error_message: "上游数据流包含格式损坏的 JSON 帧".to_string(),
                                    elapsed_ms,
                                });
                                return Err("上游数据流格式损坏".to_string());
                            }
                        }
                    }
                    Some(Err(_parse_or_transport_err)) => {
                        let elapsed_ms = start_time.elapsed().as_millis() as u64;
                        let _ = channel.send(FlightStreamEvent::Failed {
                            request_id: request_id.clone(),
                            error_message: "数据流解析或传输中断".to_string(),
                            elapsed_ms,
                        });
                        return Err("流传输中断".to_string());
                    }
                    None => {
                        // EOF
                        break;
                    }
                }
            }
        }
    }

    let elapsed_ms = start_time.elapsed().as_millis() as u64;
    // An EOF without [DONE] and without finish_reason is an unexpected truncation
    if !saw_terminal_done && finish_reason.is_none() {
        let _ = channel.send(FlightStreamEvent::Failed {
            request_id: request_id.clone(),
            error_message: "上游数据流未完成便异常断开 (EOF without completion)".to_string(),
            elapsed_ms,
        });
        return Err("上游数据流异常断开".to_string());
    }

    let reasoning_opt = if total_reasoning.is_empty() {
        None
    } else {
        Some(total_reasoning)
    };

    let total_tool_calls = if tool_calls_acc.is_empty() {
        None
    } else {
        Some(
            tool_calls_acc
                .into_iter()
                .map(|(_idx, (id, name, arguments))| FlightToolCall {
                    id,
                    call_type: "function".to_string(),
                    name,
                    arguments,
                })
                .collect(),
        )
    };
    let _ = channel.send(FlightStreamEvent::Completed {
        request_id: request_id.clone(),
        total_text,
        total_reasoning: reasoning_opt,
        first_content_ms,
        elapsed_ms,
        usage: last_usage,
        finish_reason,
        saw_done: saw_terminal_done,
        total_tool_calls,
    });

    Ok(())
}

pub async fn run_flight_chat_nonstream_core(
    port: u16,
    master_key: &str,
    input: FlightChatRequestInput,
    request_id: String,
    mut cancel_rx: oneshot::Receiver<()>,
) -> Result<FlightChatNonstreamResult, String> {
    validate_chat_input(&input)?;

    let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| "无法初始化网络客户端".to_string())?;

    let payload = build_chat_payload(&input, false);

    let start_time = Instant::now();
    let req_future = client
        .post(&url)
        .header("Authorization", format!("Bearer {master_key}"))
        .json(&payload)
        .send();

    let resp = tokio::select! {
        _ = &mut cancel_rx => {
            return Err("飞行请求已取消".to_string());
        }
        res = req_future => {
            res.map_err(|_| "请求本地网关失败，网关可能未启动或连接超时".to_string())?
        }
    };

    let status = resp.status();
    if !status.is_success() {
        let code = status.as_u16();
        if code == 401 {
            return Err("网关鉴权失败 (401 Unauthorized)".to_string());
        }
        let body_bytes = read_bounded_bytes(resp, 64 * 1024)
            .await
            .unwrap_or_default();
        let custom_err = serde_json::from_slice::<WireErrorResponse>(&body_bytes)
            .ok()
            .and_then(|e| e.error)
            .and_then(|d| d.message)
            .map(|m| sanitize_error_message(&m, master_key));

        if let Some(detail) = custom_err {
            return Err(format!("网关请求失败 ({code}): {detail}"));
        }

        return Err(match code {
            429 => "上游请求过于频繁 (429 Rate Limit)".to_string(),
            502..=504 => "网关后端服务不可用，插件未响应".to_string(),
            _ => format!("网关请求失败 ({code})"),
        });
    }

    let body_bytes = tokio::select! {
        _ = &mut cancel_rx => {
            return Err("飞行请求已取消".to_string());
        }
        res = read_bounded_bytes(resp, 2 * 1024 * 1024) => res?,
    };
    let wire: WireNonstreamResponse = serde_json::from_slice(&body_bytes)
        .map_err(|_| "网关返回的非流式响应格式无法解析".to_string())?;

    let elapsed_ms = start_time.elapsed().as_millis() as u64;
    let choice = wire
        .choices
        .and_then(|c| c.into_iter().next())
        .ok_or_else(|| "网关响应中未包含有效选择项".to_string())?;

    let msg = choice.message.unwrap_or(WireNonstreamMessage {
        content: None,
        reasoning_content: None,
        tool_calls: None,
    });

    let text = msg.content.unwrap_or_default();
    let reasoning = msg.reasoning_content.filter(|s| !s.is_empty());
    let first_content_ms = if !text.is_empty() || reasoning.is_some() {
        Some(elapsed_ms)
    } else {
        None
    };

    let usage = wire.usage.map(|u| FlightUsage {
        prompt_tokens: u.prompt_tokens.unwrap_or(0),
        completion_tokens: u.completion_tokens.unwrap_or(0),
        total_tokens: u.total_tokens.unwrap_or(0),
    });

    let tool_calls = msg.tool_calls.and_then(|calls| {
        if calls.is_empty() {
            None
        } else {
            Some(
                calls
                    .into_iter()
                    .map(|c| FlightToolCall {
                        id: c.id.unwrap_or_default(),
                        call_type: c.call_type.unwrap_or_else(|| "function".to_string()),
                        name: c
                            .function
                            .as_ref()
                            .and_then(|f| f.name.clone())
                            .unwrap_or_default(),
                        arguments: c
                            .function
                            .as_ref()
                            .and_then(|f| f.arguments.clone())
                            .unwrap_or_default(),
                    })
                    .collect(),
            )
        }
    });

    Ok(FlightChatNonstreamResult {
        request_id,
        model: input.model,
        text,
        reasoning,
        first_content_ms,
        elapsed_ms,
        usage,
        finish_reason: choice.finish_reason,
        tool_calls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tauri::ipc::InvokeResponseBody;
    use tokio::sync::mpsc;

    fn make_test_channel() -> (
        Channel<FlightStreamEvent>,
        mpsc::UnboundedReceiver<FlightStreamEvent>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel::<FlightStreamEvent>();
        let ch = Channel::<FlightStreamEvent>::new(move |body| {
            if let InvokeResponseBody::Json(s) = body {
                if let Ok(ev) = serde_json::from_str::<FlightStreamEvent>(&s) {
                    if tx.send(ev).is_err() {
                        return Err(tauri::Error::Io(std::io::Error::new(
                            std::io::ErrorKind::BrokenPipe,
                            "channel closed",
                        )));
                    }
                }
            }
            Ok(())
        });
        (ch, rx)
    }

    #[test]
    fn test_validate_chat_input() {
        let valid = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "hello".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };
        assert!(validate_chat_input(&valid).is_ok());

        let mut invalid = valid.clone();
        invalid.model = "   ".to_string();
        assert_eq!(
            validate_chat_input(&invalid).unwrap_err(),
            "模型名称不能为空"
        );

        let mut invalid = valid.clone();
        invalid.messages.clear();
        assert_eq!(
            validate_chat_input(&invalid).unwrap_err(),
            "消息列表不能为空"
        );

        let mut invalid = valid.clone();
        invalid.messages = (0..201)
            .map(|i| FlightMessageInput {
                role: "user".to_string(),
                content: format!("msg {i}"),
                content_parts: None,
            })
            .collect();
        assert_eq!(
            validate_chat_input(&invalid).unwrap_err(),
            "消息条数超出限制 (200)"
        );

        let mut invalid = valid.clone();
        invalid.messages = vec![FlightMessageInput {
            role: "user".to_string(),
            content: "x".repeat(300 * 1024),
            content_parts: None,
        }];
        assert_eq!(
            validate_chat_input(&invalid).unwrap_err(),
            "消息总长度超出上限 (256KB)"
        );

        let mut invalid = valid.clone();
        invalid.temperature = Some(-0.1);
        assert_eq!(
            validate_chat_input(&invalid).unwrap_err(),
            "温度参数须在 0.0 至 2.0 之间"
        );

        let mut invalid = valid.clone();
        invalid.temperature = Some(2.5);
        assert_eq!(
            validate_chat_input(&invalid).unwrap_err(),
            "温度参数须在 0.0 至 2.0 之间"
        );
    }

    #[test]
    fn test_truncate_error_message() {
        let short = "Normal error message";
        assert_eq!(truncate_error_message(short), short);

        let long = "a".repeat(300);
        let truncated = truncate_error_message(&long);
        assert_eq!(truncated.len(), 203); // 200 + "..."
        assert!(truncated.ends_with("..."));
    }

    #[tokio::test]
    async fn test_list_models_core_success() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = r#"{"data":[{"id":"gpt-4o","created":1700000000,"owned_by":"system"}]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });

        let res = list_models_core(port, "secret-key", Some("wb2api".to_string()))
            .await
            .unwrap();

        assert_eq!(res.models.len(), 1);
        assert_eq!(res.models[0].id, "gpt-4o");
        assert_eq!(res.active_backend, Some("wb2api".to_string()));
    }

    #[tokio::test]
    async fn test_list_models_core_401_scrubbing() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = r#"{"error":{"message":"Invalid Bearer token master-secret-12345"}}"#;
                let resp = format!(
                    "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });

        let err = list_models_core(port, "master-secret-12345", None)
            .await
            .unwrap_err();

        assert!(err.contains("401"));
        assert!(!err.contains("master-secret-12345"));
        assert!(!err.contains("Bearer"));
    }

    #[tokio::test]
    async fn test_chat_nonstream_core_success() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let body = r#"{
                    "choices": [{
                        "message": {
                            "content": "飞行测试响应文本",
                            "reasoning_content": "这是思考过程"
                        },
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": 10,
                        "completion_tokens": 20,
                        "total_tokens": 30
                    }
                }"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });

        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "ping".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(false),
            tools: None,
            tool_choice: None,
        };

        let (_tx, cancel_rx) = oneshot::channel();
        let res = run_flight_chat_nonstream_core(
            port,
            "master-key",
            input,
            "req-nonstream-1".to_string(),
            cancel_rx,
        )
        .await
        .unwrap();

        assert_eq!(res.request_id, "req-nonstream-1");
        assert_eq!(res.text, "飞行测试响应文本");
        assert_eq!(res.reasoning, Some("这是思考过程".to_string()));
        assert_eq!(res.finish_reason, Some("stop".to_string()));
        assert!(res.first_content_ms.is_some());
        let usage = res.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }

    #[tokio::test]
    async fn test_chat_nonstream_core_cancel_preheaders() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                // Delay 2000ms before headers to simulate slow upstream header arrival
                std::thread::sleep(Duration::from_millis(2000));
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
            }
        });

        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "wait".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(false),
            tools: None,
            tool_choice: None,
        };

        let (cancel_tx, cancel_rx) = oneshot::channel();
        let start = Instant::now();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(40)).await;
            let _ = cancel_tx.send(());
        });

        let err = run_flight_chat_nonstream_core(
            port,
            "master-key",
            input,
            "req-cancel-preheader".to_string(),
            cancel_rx,
        )
        .await
        .unwrap_err();

        let elapsed = start.elapsed();
        assert_eq!(err, "飞行请求已取消");
        // Must complete promptly upon cancel (< 300ms), far before the 2000ms server sleep!
        assert!(
            elapsed < Duration::from_millis(500),
            "Cancellation took too long: {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn test_chat_stream_core_sse_success_with_chunks_utf8_crlf_and_comments() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(header.as_bytes());

                // 1. Comment
                let _ = stream.write_all(b": keepalive ping\r\n\r\n");
                std::thread::sleep(Duration::from_millis(5));

                // 2. Reasoning delta
                let r_chunk = "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"初步思考中\"}}]}\r\n\r\n";
                let _ = stream.write_all(r_chunk.as_bytes());
                std::thread::sleep(Duration::from_millis(5));

                // 3. Multi-byte UTF8 text split into 3-byte slices to test parser UTF-8 resilience
                let text_chunk = "data: {\"choices\":[{\"delta\":{\"content\":\"你好，ProxyWeave飞行！🌍\"}}]}\r\n\r\n";
                for slice in text_chunk.as_bytes().chunks(3) {
                    let _ = stream.write_all(slice);
                    std::thread::sleep(Duration::from_millis(2));
                }

                // 4. Usage chunk with finish_reason
                let usage_chunk = "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":8,\"completion_tokens\":12,\"total_tokens\":20}}\r\n\r\n";
                let _ = stream.write_all(usage_chunk.as_bytes());
                std::thread::sleep(Duration::from_millis(5));

                // 5. [DONE]
                let _ = stream.write_all(b"data: [DONE]\r\n\r\n");
            }
        });

        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "hi".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };

        let (ch, mut rx) = make_test_channel();
        let (_cancel_tx, cancel_rx) = oneshot::channel();

        let core_handle = tokio::spawn(async move {
            run_flight_chat_stream_core(
                port,
                "master-key",
                input,
                "req-sse-1".to_string(),
                ch,
                cancel_rx,
            )
            .await
        });

        let mut events = Vec::new();
        while let Some(ev) = rx.recv().await {
            let is_terminal = matches!(
                ev,
                FlightStreamEvent::Completed { .. }
                    | FlightStreamEvent::Failed { .. }
                    | FlightStreamEvent::Cancelled { .. }
            );
            events.push(ev);
            if is_terminal {
                break;
            }
        }

        let res = core_handle.await.unwrap();
        assert!(res.is_ok());

        assert!(matches!(events[0], FlightStreamEvent::Started { .. }));
        assert!(events.iter().any(|e| matches!(e, FlightStreamEvent::Delta { reasoning: Some(r), .. } if r == "初步思考中")));
        assert!(events.iter().any(|e| matches!(e, FlightStreamEvent::Delta { text: Some(t), .. } if t.contains("你好，ProxyWeave飞行！🌍"))));

        let last = events.last().unwrap();
        if let FlightStreamEvent::Completed {
            total_text,
            total_reasoning,
            finish_reason,
            usage,
            saw_done,
            ..
        } = last
        {
            assert_eq!(total_text, "你好，ProxyWeave飞行！🌍");
            assert_eq!(total_reasoning.as_deref(), Some("初步思考中"));
            assert_eq!(finish_reason.as_deref(), Some("stop"));
            assert!(saw_done);
            let u = usage.as_ref().unwrap();
            assert_eq!(u.prompt_tokens, 8);
            assert_eq!(u.completion_tokens, 12);
            assert_eq!(u.total_tokens, 20);
        } else {
            panic!("Expected Completed event as final event, got {last:?}");
        }
    }

    #[tokio::test]
    async fn test_chat_stream_core_eof_without_done_fails() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(header.as_bytes());
                // Send partial delta then close immediately without [DONE] and without finish_reason
                let _ = stream.write_all(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"未完成便断开\"}}]}\r\n\r\n"
                        .as_bytes(),
                );
            }
        });

        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "abort".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };

        let (ch, mut rx) = make_test_channel();
        let (_cancel_tx, cancel_rx) = oneshot::channel();

        let core_handle = tokio::spawn(async move {
            run_flight_chat_stream_core(
                port,
                "master-key",
                input,
                "req-eof-fail".to_string(),
                ch,
                cancel_rx,
            )
            .await
        });

        let mut events = Vec::new();
        while let Some(ev) = rx.recv().await {
            let is_terminal = matches!(
                ev,
                FlightStreamEvent::Completed { .. }
                    | FlightStreamEvent::Failed { .. }
                    | FlightStreamEvent::Cancelled { .. }
            );
            events.push(ev);
            if is_terminal {
                break;
            }
        }

        let res = core_handle.await.unwrap();
        assert!(
            res.is_err(),
            "Expected core execution to return Err on EOF truncation"
        );

        let last = events.last().unwrap();
        match last {
            FlightStreamEvent::Failed { error_message, .. } => {
                assert!(error_message.contains("EOF without completion"));
            }
            _ => panic!("Expected Failed event, got {last:?}"),
        }
    }

    #[tokio::test]
    async fn test_chat_stream_core_cancel_midstream() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server_closed = Arc::new(AtomicBool::new(false));
        let server_closed_clone = server_closed.clone();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(header.as_bytes());

                let _ = stream.write_all(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"流式进行中\"}}]}\r\n\r\n"
                        .as_bytes(),
                );

                // Stream indefinitely until client drops connection
                loop {
                    std::thread::sleep(Duration::from_millis(30));
                    if stream.write_all(b": keepalive\r\n\r\n").is_err() {
                        server_closed_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }
        });

        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "stream".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };

        let (ch, mut rx) = make_test_channel();
        let (cancel_tx, cancel_rx) = oneshot::channel();

        let core_handle = tokio::spawn(async move {
            run_flight_chat_stream_core(
                port,
                "master-key",
                input,
                "req-midstream-cancel".to_string(),
                ch,
                cancel_rx,
            )
            .await
        });

        // Wait until first delta is received, then cancel
        while let Some(ev) = rx.recv().await {
            if matches!(ev, FlightStreamEvent::Delta { .. }) {
                let _ = cancel_tx.send(());
                break;
            }
        }

        // Wait for terminal Cancelled event
        let mut saw_cancelled = false;
        while let Some(ev) = rx.recv().await {
            if matches!(ev, FlightStreamEvent::Cancelled { .. }) {
                saw_cancelled = true;
                break;
            }
        }

        let res = core_handle.await.unwrap();
        assert!(res.is_ok());
        assert!(saw_cancelled, "Expected Cancelled stream event");

        // Ensure upstream server socket closes promptly
        let deadline = Instant::now() + Duration::from_millis(500);
        while !server_closed.load(Ordering::SeqCst) && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            server_closed.load(Ordering::SeqCst),
            "Server socket should be closed after stream cancellation"
        );
    }

    #[tokio::test]
    async fn test_chat_stream_core_wire_error_upstream() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = r#"{"error":{"message":"Upstream rate limited or overloaded"}}"#;
                let resp = format!(
                    "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });

        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "error".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };

        let (ch, mut rx) = make_test_channel();
        let (_cancel_tx, cancel_rx) = oneshot::channel();

        let res = run_flight_chat_stream_core(
            port,
            "master-key",
            input,
            "req-wire-err".to_string(),
            ch,
            cancel_rx,
        )
        .await;

        assert!(res.is_err());
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }

        let failed_ev = events
            .into_iter()
            .find(|e| matches!(e, FlightStreamEvent::Failed { .. }))
            .expect("Must emit Failed event");
        if let FlightStreamEvent::Failed { error_message, .. } = failed_ev {
            assert!(error_message.contains("429"));
            assert!(error_message.contains("Upstream rate limited or overloaded"));
        }
    }

    #[tokio::test]
    async fn test_real_loopback_gateway_integration_states() {
        use proxyweave_gateway::Gateway;
        use proxyweave_plugin::RuntimeManager;

        let local_key = "local-master-key-must-be-at-least-32-chars-long";
        let gateway = Gateway::new(RuntimeManager::new(), local_key, Duration::from_secs(5))
            .expect("gateway init");
        let handle = gateway.listen(0).await.expect("listen");
        let port = handle.address().port();

        // 1. Wrong local master key -> 401 Unauthorized scrubbed
        let err_401 = list_models_core(port, "wrong-key-that-should-be-rejected", None)
            .await
            .unwrap_err();
        assert!(err_401.contains("401 Unauthorized"));
        assert!(!err_401.contains("wrong-key"));

        // 2. Correct master key but unattached backend -> 503 Service Unavailable
        let err_503 = list_models_core(port, local_key, None).await.unwrap_err();
        assert!(err_503.contains("网关后端服务不可用"));

        // 3. Chat stream with unattached backend -> Failed event with 503
        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "ping".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };
        let (ch, mut rx) = make_test_channel();
        let (_cancel_tx, cancel_rx) = oneshot::channel();
        let res_stream = run_flight_chat_stream_core(
            port,
            local_key,
            input.clone(),
            "req-gw-503".to_string(),
            ch,
            cancel_rx,
        )
        .await;
        assert!(res_stream.is_err());
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }
        let failed_ev = events
            .iter()
            .find(|e| matches!(e, FlightStreamEvent::Failed { .. }));
        assert!(failed_ev.is_some(), "Must emit Failed event on 503");

        // 4. Stopped / unmounted gateway (unused port) -> Connection refused
        let unused_port = proxyweave_plugin::port::pick_unused_port().unwrap();
        let err_stopped = list_models_core(unused_port, local_key, None)
            .await
            .unwrap_err();
        assert!(err_stopped.contains("请求本地网关失败"));
    }

    #[test]
    fn test_caller_allowed_trust_boundary() {
        assert!(caller_allowed("main", "http://tauri.localhost", None));
        assert!(caller_allowed("main", "https://tauri.localhost", None));
        assert!(caller_allowed("main", "tauri://localhost", None));
        assert!(caller_allowed(
            "main",
            "http://localhost:1420",
            Some("http://localhost:1420")
        ));

        // Non-main window labels denied
        assert!(!caller_allowed("devtools", "http://tauri.localhost", None));
        assert!(!caller_allowed("settings", "http://tauri.localhost", None));
        assert!(!caller_allowed("", "http://tauri.localhost", None));

        // Untrusted origins denied even if main window
        assert!(!caller_allowed("main", "http://evil.com", None));
        assert!(!caller_allowed(
            "main",
            "https://attacker.org",
            Some("http://localhost:1420")
        ));
    }

    #[tokio::test]
    async fn test_chat_nonstream_core_reasoning_only() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = r#"{
                    "choices": [{
                        "message": {
                            "content": "",
                            "reasoning_content": "仅思考内容而无最终文本"
                        },
                        "finish_reason": "stop"
                    }]
                }"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });

        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "think".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(false),
            tools: None,
            tool_choice: None,
        };

        let (_tx, cancel_rx) = oneshot::channel();
        let res = run_flight_chat_nonstream_core(
            port,
            "master-key",
            input,
            "req-reasoning-only".to_string(),
            cancel_rx,
        )
        .await
        .unwrap();

        assert_eq!(res.text, "");
        assert_eq!(res.reasoning, Some("仅思考内容而无最终文本".to_string()));
        assert!(res.first_content_ms.is_some());
    }

    #[tokio::test]
    async fn test_chat_stream_core_finish_reason_stop_without_done() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(header.as_bytes());
                // Valid protocol stop finish reason, then EOF without explicit [DONE] marker
                let delta = "data: {\"choices\":[{\"delta\":{\"content\":\"正常输出完结\"},\"finish_reason\":\"stop\"}]}\r\n\r\n";
                let _ = stream.write_all(delta.as_bytes());
            }
        });

        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "go".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };

        let (ch, mut rx) = make_test_channel();
        let (_cancel_tx, cancel_rx) = oneshot::channel();

        let res = run_flight_chat_stream_core(
            port,
            "master-key",
            input,
            "req-stop-nodone".to_string(),
            ch,
            cancel_rx,
        )
        .await;

        assert!(res.is_ok());
        let mut events = Vec::new();
        while let Some(ev) = rx.recv().await {
            let is_term = matches!(
                ev,
                FlightStreamEvent::Completed { .. } | FlightStreamEvent::Failed { .. }
            );
            events.push(ev);
            if is_term {
                break;
            }
        }

        let last = events.last().unwrap();
        if let FlightStreamEvent::Completed {
            total_text,
            finish_reason,
            saw_done,
            ..
        } = last
        {
            assert_eq!(total_text, "正常输出完结");
            assert_eq!(finish_reason.as_deref(), Some("stop"));
            assert!(
                !saw_done,
                "saw_done must accurately reflect false when no [DONE]"
            );
        } else {
            panic!("Expected Completed, got {last:?}");
        }
    }

    #[tokio::test]
    async fn test_chat_stream_core_oversized_frame_rejected() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(header.as_bytes());
                // Single SSE frame > 512KB
                let huge = "a".repeat(600 * 1024);
                let _ = stream.write_all(format!("data: {huge}\r\n\r\n").as_bytes());
            }
        });

        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "huge".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };

        let (ch, mut rx) = make_test_channel();
        let (_cancel_tx, cancel_rx) = oneshot::channel();

        let res = run_flight_chat_stream_core(
            port,
            "master-key",
            input,
            "req-oversized-frame".to_string(),
            ch,
            cancel_rx,
        )
        .await;

        assert!(res.is_err());
        let mut events = Vec::new();
        while let Some(ev) = rx.recv().await {
            let is_term = matches!(
                ev,
                FlightStreamEvent::Completed { .. } | FlightStreamEvent::Failed { .. }
            );
            events.push(ev);
            if is_term {
                break;
            }
        }
        let last = events.last().unwrap();
        if let FlightStreamEvent::Failed { error_message, .. } = last {
            assert!(error_message.contains("512KB"));
        } else {
            panic!("Expected Failed, got {last:?}");
        }
    }

    #[tokio::test]
    async fn test_chat_stream_core_channel_drop_early_exit() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(header.as_bytes());

                loop {
                    if stream
                        .write_all(
                            b"data: {\"choices\":[{\"delta\":{\"content\":\"ping\"}}]}\r\n\r\n",
                        )
                        .is_err()
                    {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        });

        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "drop".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };

        let (ch, rx) = make_test_channel();
        let (_cancel_tx, cancel_rx) = oneshot::channel();

        // Drop the receiver immediately
        drop(rx);

        let res = run_flight_chat_stream_core(
            port,
            "master-key",
            input,
            "req-channel-drop".to_string(),
            ch,
            cancel_rx,
        )
        .await;

        // When channel drops, stream execution should exit cleanly without hanging
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_real_gateway_runtime_lease_and_cancellation_lifecycle() {
        use proxyweave_gateway::Gateway;
        use proxyweave_plugin::{Manifest, PluginRecord, RuntimeManager};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend_port = listener.local_addr().unwrap().port();

        // Spawn mock external backend server supporting healthz, models, normal chat, headerwait, and endless streaming
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                std::thread::spawn(move || {
                    let mut buf = [0u8; 4096];
                    let mut n = 0;
                    while n < buf.len() {
                        match stream.read(&mut buf[n..]) {
                            Ok(0) | Err(_) => return,
                            Ok(read_bytes) => n += read_bytes,
                        }
                        if buf[..n].windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let req_str = String::from_utf8_lossy(&buf[..n]);
                    let first_line = req_str.lines().next().unwrap_or("");
                    let path = first_line.split_whitespace().nth(1).unwrap_or("");

                    if path == "/healthz" {
                        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nX-Fixture: flight-fixture\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
                        return;
                    }

                    if path == "/v1/models" {
                        let body = r#"{"data":[{"id":"qwen-2.5-72b","created":1700000000,"owned_by":"owned-backend"}]}"#;
                        let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                        let _ = stream.write_all(resp.as_bytes());
                        return;
                    }

                    if path == "/v1/chat/completions" {
                        if req_str.contains("headerwait") {
                            // Simulate slow upstream: wait 3000ms before headers to allow testing cancellation preheaders
                            std::thread::sleep(Duration::from_millis(3000));
                            let _ =
                                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
                            return;
                        }

                        if req_str.contains("endless") {
                            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n");
                            let _ = stream.write_all(b"data: {\"choices\":[{\"delta\":{\"content\":\"start\"}}]}\r\n\r\n");
                            loop {
                                std::thread::sleep(Duration::from_millis(25));
                                if stream.write_all(b"data: {\"choices\":[{\"delta\":{\"content\":\".\"}}]}\r\n\r\n").is_err() {
                                        break;
                                    }
                            }
                            return;
                        }

                        // Normal chat
                        if req_str.contains("\"stream\":true") {
                            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n");
                            let _ = stream.write_all(b"data: {\"choices\":[{\"delta\":{\"content\":\"hello from real gateway\"},\"finish_reason\":\"stop\"}]}\r\n\r\ndata: [DONE]\r\n\r\n");
                        } else {
                            let body = r#"{"choices":[{"message":{"content":"nonstream from gateway"},"finish_reason":"stop"}]}"#;
                            let resp = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                    body.len(),
                                    body
                                );
                            let _ = stream.write_all(resp.as_bytes());
                        }
                    }
                });
            }
        });

        let manifest_yaml = format!(
            r#"schema_version: '1.0'
metadata: {{id: owned-flight-backend, name: Owned Flight Backend Fixture, version: 1.0.0, license: MIT}}
runtime:
  ownership: external
  endpoint: 'http://127.0.0.1:{backend_port}'
network:
  health_check: {{path: /healthz, expected_status: 200, identity_header: X-Fixture, identity_value: flight-fixture, timeout_ms: 1000}}
lifecycle: {{startup_timeout_ms: 1000, idle_timeout_ms: null, max_port_retries: 0}}
capabilities: {{protocol: openai_chat, models_path: /v1/models, inference_path: /v1/chat/completions, streaming: true}}
"#
        );

        let manifest = Manifest::parse(manifest_yaml.as_bytes()).expect("manifest parse");
        let record = PluginRecord {
            manifest,
            fingerprint: "test-fingerprint".into(),
            settings: Default::default(),
            source: None,
        };

        let runtime = RuntimeManager::new();
        let status = runtime
            .start_plugin(
                &record,
                std::path::Path::new("."),
                Some(backend_port),
                std::path::Path::new("."),
                None,
            )
            .await
            .expect("start plugin");
        assert!(matches!(
            status,
            proxyweave_plugin::RuntimeStatus::Ready { .. }
        ));

        let local_key = "local-master-key-must-be-at-least-32-chars-long";
        let gateway =
            Gateway::new(runtime.clone(), local_key, Duration::from_secs(5)).expect("gateway init");
        gateway
            .set_backend(&record.manifest, "upstream-test-key")
            .await
            .expect("set backend");
        let handle = gateway.listen(0).await.expect("gateway listen");
        let gw_port = handle.address().port();

        // Phase 1: Model Discovery through Gateway
        let model_res =
            list_models_core(gw_port, local_key, Some("owned-flight-backend".to_string()))
                .await
                .expect("list models");
        assert_eq!(model_res.models.len(), 1);
        assert_eq!(model_res.models[0].id, "qwen-2.5-72b");
        assert_eq!(
            model_res.active_backend.as_deref(),
            Some("owned-flight-backend")
        );

        // Phase 2: Nonstream Chat through Gateway
        let nonstream_input = FlightChatRequestInput {
            request_id: None,
            model: "qwen-2.5-72b".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "ping".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(false),
            tools: None,
            tool_choice: None,
        };
        let (_tx, cancel_rx) = oneshot::channel();
        let nonstream_res = run_flight_chat_nonstream_core(
            gw_port,
            local_key,
            nonstream_input,
            "req-ns-gw".to_string(),
            cancel_rx,
        )
        .await
        .expect("nonstream chat");
        assert_eq!(nonstream_res.text, "nonstream from gateway");
        assert_eq!(nonstream_res.finish_reason.as_deref(), Some("stop"));

        // Phase 3: Stream Chat through Gateway
        let stream_input = FlightChatRequestInput {
            request_id: None,
            model: "qwen-2.5-72b".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "ping".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };
        let (ch, mut rx) = make_test_channel();
        let (_tx, cancel_rx) = oneshot::channel();
        let stream_res = run_flight_chat_stream_core(
            gw_port,
            local_key,
            stream_input,
            "req-stream-gw".to_string(),
            ch,
            cancel_rx,
        )
        .await;
        assert!(stream_res.is_ok());
        let mut events = Vec::new();
        while let Some(ev) = rx.recv().await {
            let is_term = matches!(
                ev,
                FlightStreamEvent::Completed { .. } | FlightStreamEvent::Failed { .. }
            );
            events.push(ev);
            if is_term {
                break;
            }
        }
        let last = events.last().unwrap();
        if let FlightStreamEvent::Completed {
            total_text,
            saw_done,
            ..
        } = last
        {
            assert_eq!(total_text, "hello from real gateway");
            assert!(saw_done);
        } else {
            panic!("Expected Completed event, got {last:?}");
        }

        // Phase 4: Cancellation Preheaders proves disconnect and lease zero before deadline
        let cancel_pre_input = FlightChatRequestInput {
            request_id: None,
            model: "qwen-2.5-72b".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "headerwait".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };
        let (ch, mut rx) = make_test_channel();
        let (cancel_tx, cancel_rx) = oneshot::channel();

        let cancel_start = Instant::now();
        let task = tokio::spawn(async move {
            run_flight_chat_stream_core(
                gw_port,
                local_key,
                cancel_pre_input,
                "req-cancel-pre".to_string(),
                ch,
                cancel_rx,
            )
            .await
        });

        // Wait until request lease is actively acquired by Gateway
        let lease_deadline = Instant::now() + Duration::from_millis(500);
        while runtime.active_request_count("owned-flight-backend").await == 0
            && Instant::now() < lease_deadline
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            runtime.active_request_count("owned-flight-backend").await,
            1,
            "Gateway must have active lease"
        );

        // Trigger cancellation promptly (< 100ms)
        let _ = cancel_tx.send(());

        // Stream event must be Cancelled
        let mut saw_cancelled = false;
        while let Some(ev) = rx.recv().await {
            if matches!(ev, FlightStreamEvent::Cancelled { .. }) {
                saw_cancelled = true;
                break;
            }
        }
        assert!(saw_cancelled, "Must emit Cancelled event");

        let task_res = task.await.unwrap();
        assert!(task_res.is_ok());

        // Lease must drop to zero before 500ms deadline (far before the 5000ms Gateway timeout)
        let zero_deadline = Instant::now() + Duration::from_millis(500);
        while runtime.active_request_count("owned-flight-backend").await > 0
            && Instant::now() < zero_deadline
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            runtime.active_request_count("owned-flight-backend").await,
            0,
            "Active lease must be 0 after preheader cancellation"
        );
        assert!(
            cancel_start.elapsed() < Duration::from_millis(1000),
            "Cancellation must complete far faster than header timeout"
        );

        // Phase 5: Cancellation Midstream proves disconnect and lease zero before deadline
        let cancel_mid_input = FlightChatRequestInput {
            request_id: None,
            model: "qwen-2.5-72b".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "endless".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: None,
            tool_choice: None,
        };
        let (ch, mut rx) = make_test_channel();
        let (cancel_tx, cancel_rx) = oneshot::channel();

        let mid_task = tokio::spawn(async move {
            run_flight_chat_stream_core(
                gw_port,
                local_key,
                cancel_mid_input,
                "req-cancel-mid".to_string(),
                ch,
                cancel_rx,
            )
            .await
        });

        // Wait until first delta is received
        while let Some(ev) = rx.recv().await {
            if matches!(ev, FlightStreamEvent::Delta { .. }) {
                break;
            }
        }
        assert_eq!(
            runtime.active_request_count("owned-flight-backend").await,
            1,
            "Gateway must have active lease midstream"
        );

        // Cancel midstream
        let _ = cancel_tx.send(());

        let mut saw_mid_cancelled = false;
        while let Some(ev) = rx.recv().await {
            if matches!(ev, FlightStreamEvent::Cancelled { .. }) {
                saw_mid_cancelled = true;
                break;
            }
        }
        assert!(saw_mid_cancelled, "Must emit Cancelled event midstream");

        let mid_res = mid_task.await.unwrap();
        assert!(mid_res.is_ok());

        // Lease must drop to zero promptly
        let zero_deadline_mid = Instant::now() + Duration::from_millis(500);
        while runtime.active_request_count("owned-flight-backend").await > 0
            && Instant::now() < zero_deadline_mid
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            runtime.active_request_count("owned-flight-backend").await,
            0,
            "Active lease must be 0 after midstream cancellation"
        );
    }

    #[tokio::test]
    async fn test_flight_session_manager_concurrency_and_cleanup() {
        let manager = FlightManager::new();

        let (tx1, rx1) = oneshot::channel();
        manager
            .register_session("main", "req-1", tx1)
            .await
            .unwrap();

        // 1. Second request on same window is rejected
        let (tx2, _rx2) = oneshot::channel();
        let err2 = manager
            .register_session("main", "req-2", tx2)
            .await
            .unwrap_err();
        assert!(err2.contains("当前已有进行中的飞行会话"));

        // 2. Different window is allowed
        let (tx_sec, _rx_sec) = oneshot::channel();
        manager
            .register_session("secondary", "req-sec", tx_sec)
            .await
            .unwrap();

        // 3. Stale request_id cannot cancel session
        let res_stale = manager.cancel_session("main", "req-stale").await.unwrap();
        assert!(!res_stale.cancelled);

        // 4. Correct request_id cancels session
        let res_true = manager.cancel_session("main", "req-1").await.unwrap();
        assert!(res_true.cancelled);

        // Verify receiver gets cancellation trigger
        assert!(rx1.await.is_ok());

        // 5. Unregister / new request on same window succeeds
        manager.unregister_session("main", "req-1").await;
        let (tx3, _rx3) = oneshot::channel();
        manager
            .register_session("main", "req-3", tx3)
            .await
            .unwrap();

        // 6. cancel_all_for_window cleans up
        manager.cancel_all_for_window("main").await;
        let (tx4, _rx4) = oneshot::channel();
        manager
            .register_session("main", "req-4", tx4)
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn test_chat_stream_core_tool_calls_fragments_and_completion() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(header.as_bytes());
                // Fragment 1: id + name + partial arguments
                let f1 = "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_abc\",\"type\":\"function\",\"function\":{\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\":\\\"\"}}]}}]}\n\n";
                let _ = stream.write_all(f1.as_bytes());
                std::thread::sleep(Duration::from_millis(2));
                // Fragment 2: remaining arguments
                let f2 = "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"Beijing\\\"}\"}}]}}]}\n\n";
                let _ = stream.write_all(f2.as_bytes());
                std::thread::sleep(Duration::from_millis(2));
                let fin = r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":5,"completion_tokens":3,"total_tokens":8}}

"#;
                let _ = stream.write_all(fin.as_bytes());
                let _ = stream.write_all(
                    b"data: [DONE]

",
                );
            }
        });
        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "what is the weather in Beijing?".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(true),
            tools: Some(vec![serde_json::json!({
                "type": "function",
                "function": {"name": "get_weather", "parameters": {"type": "object"}}
            })]),
            tool_choice: Some(serde_json::Value::String("auto".to_string())),
        };
        let (channel, mut rx) = make_test_channel();
        let (_tx, cancel_rx) = oneshot::channel();
        run_flight_chat_stream_core(
            port,
            "master-key",
            input,
            "req-tools-1".to_string(),
            channel,
            cancel_rx,
        )
        .await
        .unwrap();
        let mut saw_delta = false;
        let mut completed: Option<FlightStreamEvent> = None;
        while let Some(ev) = rx.recv().await {
            match ev {
                FlightStreamEvent::ToolCallDelta { fragments, .. } => {
                    assert!(!fragments.is_empty());
                    saw_delta = true;
                }
                FlightStreamEvent::Completed {
                    total_tool_calls,
                    finish_reason,
                    saw_done,
                    ..
                } => {
                    completed = Some(FlightStreamEvent::Completed {
                        request_id: String::new(),
                        total_text: String::new(),
                        total_reasoning: None,
                        first_content_ms: None,
                        elapsed_ms: 0,
                        usage: None,
                        finish_reason,
                        saw_done,
                        total_tool_calls,
                    });
                    break;
                }
                _ => {}
            }
        }
        assert!(saw_delta, "must emit ToolCallDelta fragments");
        let completed = completed.expect("must complete");
        if let FlightStreamEvent::Completed {
            total_tool_calls,
            finish_reason,
            saw_done,
            ..
        } = completed
        {
            let calls = total_tool_calls.expect("must include assembled tool_calls");
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].id, "call_abc");
            assert_eq!(calls[0].name, "get_weather");
            assert!(calls[0].arguments.contains("Beijing"));
            assert_eq!(finish_reason.as_deref(), Some("tool_calls"));
            assert!(saw_done);
        } else {
            panic!("unreachable");
        }
    }

    #[tokio::test]
    async fn test_chat_nonstream_core_tool_calls() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let body = r#"{"choices":[{"message":{"content":null,"tool_calls":[{"id":"call_xyz","type":"function","function":{"name":"get_weather","arguments":"{\"city\":\"Shanghai\"}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5}}"#;
                let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                let _ = stream.write_all(resp.as_bytes());
            }
        });
        let input = FlightChatRequestInput {
            request_id: None,
            model: "gpt-4o".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".to_string(),
                content: "weather in Shanghai?".to_string(),
                content_parts: None,
            }],
            temperature: Some(0.7),
            stream: Some(false),
            tools: Some(vec![
                serde_json::json!({"type":"function","function":{"name":"get_weather"}}),
            ]),
            tool_choice: None,
        };
        let (_tx, cancel_rx) = oneshot::channel();
        let res = run_flight_chat_nonstream_core(
            port,
            "master-key",
            input,
            "req-tools-ns".to_string(),
            cancel_rx,
        )
        .await
        .unwrap();
        let calls = res.tool_calls.expect("must parse tool_calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_xyz");
        assert_eq!(calls[0].name, "get_weather");
        assert!(calls[0].arguments.contains("Shanghai"));
        assert_eq!(res.finish_reason.as_deref(), Some("tool_calls"));
    }

    #[test]
    fn test_validate_chat_input_tools_bounds() {
        let mut ok = FlightChatRequestInput {
            request_id: None,
            model: "m".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".into(),
                content: "x".into(),
                content_parts: None,
            }],
            temperature: None,
            stream: None,
            tools: Some(vec![
                serde_json::json!({"type":"function","function":{"name":"f"}}),
            ]),
            tool_choice: Some(serde_json::Value::String("auto".into())),
        };
        assert!(validate_chat_input(&ok).is_ok());
        ok.tools = Some(
            (0..33)
                .map(|_| serde_json::json!({"type":"function"}))
                .collect(),
        );
        assert_eq!(
            validate_chat_input(&ok).unwrap_err(),
            "工具定义数量超出上限 (32)"
        );
        ok.tools = Some(vec![
            serde_json::json!({"type":"function","function":{"name":"f"}}),
        ]);
        ok.tool_choice = Some(serde_json::Value::String("sometimes".into()));
        assert_eq!(
            validate_chat_input(&ok).unwrap_err(),
            "tool_choice 字符串仅支持 none/auto/required"
        );
        ok.tool_choice = Some(serde_json::json!({"type":"function","function":{"name":"f"}}));
        assert!(validate_chat_input(&ok).is_ok());
    }

    #[test]
    fn test_validate_chat_input_multimodal_parts_bounds() {
        let mut ok = FlightChatRequestInput {
            request_id: None,
            model: "m".to_string(),
            messages: vec![FlightMessageInput {
                role: "user".into(),
                content: "describe".into(),
                content_parts: Some(vec![
                    FlightContentPart {
                        part_type: "text".into(),
                        text: Some("describe".into()),
                        image_url: None,
                    },
                    FlightContentPart {
                        part_type: "image_url".into(),
                        text: None,
                        image_url: Some(FlightImageUrl {
                            url: "https://example.com/i.png".into(),
                        }),
                    },
                ]),
            }],
            temperature: None,
            stream: None,
            tools: None,
            tool_choice: None,
        };
        assert!(validate_chat_input(&ok).is_ok());
        ok.messages[0].content_parts = Some(
            (0..5)
                .map(|_| FlightContentPart {
                    part_type: "image_url".into(),
                    text: None,
                    image_url: Some(FlightImageUrl {
                        url: "https://e/x".into(),
                    }),
                })
                .collect(),
        );
        assert_eq!(
            validate_chat_input(&ok).unwrap_err(),
            "单条消息图片数量超出上限 (4)"
        );
    }
}
