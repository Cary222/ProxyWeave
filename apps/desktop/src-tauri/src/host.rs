use getrandom::fill;
use proxyweave_gateway::{Gateway, GatewayHandle};
use proxyweave_plugin::{
    ImportResult, Ownership, PluginRecord, Registry, RuntimeManager, RuntimeStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, State};
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopConfig {
    pub gateway_port: u16,
    pub selected_plugin_id: Option<String>,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            gateway_port: 28319,
            selected_plugin_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEvent {
    pub id: u64,
    pub timestamp_ms: u64,
    pub category: String,
    pub action: String,
    pub target: String,
    pub success: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStatusInfo {
    pub status: String,
    pub port: Option<u16>,
    pub pid: Option<u32>,
    pub reason: Option<String>,
    pub error: Option<String>,
    pub active_requests: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretStatusInfo {
    pub name: String,
    pub is_set: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStatusInfo {
    pub is_running: bool,
    pub port: u16,
    pub listen_address: Option<String>,
    pub active_backend: Option<String>,
    pub master_key_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCallResult {
    pub success: bool,
    pub status: u16,
    pub latency_ms: u64,
    pub response_body: String,
}

pub struct DesktopState {
    pub profile_dir: PathBuf,
    pub registry: Registry,
    pub runtime: RuntimeManager,
    pub gateway: Gateway,
    pub gateway_handle: Arc<Mutex<Option<GatewayHandle>>>,
    pub gateway_starting: Arc<AtomicBool>,
    pub gateway_port: Arc<RwLock<u16>>,
    pub gateway_master_key: Arc<RwLock<String>>,
    pub selected_plugin_id: Arc<RwLock<Option<String>>>,
    pub events: Arc<Mutex<VecDeque<HostEvent>>>,
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

pub fn generate_master_key() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    fill(&mut bytes).map_err(|e| format!("cannot generate secure random key: {e}"))?;
    Ok(format!("pw_{}", hex_encode(&bytes)))
}

impl DesktopState {
    pub fn new(profile_dir: PathBuf, registry: Registry) -> Result<Self, String> {
        let runtime = RuntimeManager::new();

        // Load or generate Master Key (ProxyWeave local Bearer token)
        let master_key = match quotio_platform::get_credential("gateway:master_key") {
            Ok(Some(key)) if key.len() >= 32 && key.len() <= 512 => key,
            _ => {
                let generated = generate_master_key()?;
                let _ = quotio_platform::set_credential("gateway:master_key", &generated);
                generated
            }
        };

        let gateway = Gateway::new(runtime.clone(), &master_key, Duration::from_secs(30))
            .map_err(|e| format!("cannot initialize gateway: {e}"))?;

        let config_path = profile_dir.join("desktop_config.json");
        let config: DesktopConfig = if config_path.exists() {
            std::fs::read(&config_path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or_default()
        } else {
            DesktopConfig::default()
        };

        Ok(Self {
            profile_dir,
            registry,
            runtime,
            gateway,
            gateway_handle: Arc::new(Mutex::new(None)),
            gateway_starting: Arc::new(AtomicBool::new(false)),
            gateway_port: Arc::new(RwLock::new(config.gateway_port)),
            gateway_master_key: Arc::new(RwLock::new(master_key)),
            selected_plugin_id: Arc::new(RwLock::new(config.selected_plugin_id)),
            events: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    pub fn save_desktop_config(&self, config: &DesktopConfig) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?;
        quotio_platform::atomic_write(&self.profile_dir.join("desktop_config.json"), &bytes, false)
            .map_err(|e| format!("cannot save desktop config: {e}"))
    }

    pub async fn record_event(
        &self,
        category: &str,
        action: &str,
        target: &str,
        success: bool,
        summary: &str,
    ) {
        let id;
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        {
            let mut events = self.events.lock().await;
            id = events.back().map(|e| e.id + 1).unwrap_or(1);
            events.push_back(HostEvent {
                id,
                timestamp_ms,
                category: category.to_string(),
                action: action.to_string(),
                target: target.to_string(),
                success,
                summary: summary.to_string(),
            });
            while events.len() > 50 {
                events.pop_front();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn import_local_plugin(
    state: State<'_, DesktopState>,
    manifest_path: String,
) -> Result<ImportResult, String> {
    let registry = state.registry.clone();
    let path = manifest_path.clone();
    let res = tauri::async_runtime::spawn_blocking(move || registry.import(path))
        .await
        .map_err(|_| "import task panicked".to_string())??;

    state
        .record_event(
            "plugin",
            "import",
            &res.record.manifest.metadata.id,
            true,
            if res.already_registered {
                "Plugin verified (already registered)"
            } else {
                "Plugin package successfully imported"
            },
        )
        .await;

    // Auto-select if no plugin selected yet
    let current_selected = {
        let r = state.selected_plugin_id.read().await;
        r.clone()
    };
    if current_selected.is_none() {
        let new_sel = {
            let mut sel = state.selected_plugin_id.write().await;
            *sel = Some(res.record.manifest.metadata.id.clone());
            sel.clone()
        };
        let current_port = *state.gateway_port.read().await;
        let _ = state.save_desktop_config(&DesktopConfig {
            gateway_port: current_port,
            selected_plugin_id: new_sel,
        });
    }

    Ok(res)
}

#[tauri::command]
pub async fn list_local_plugins(
    state: State<'_, DesktopState>,
) -> Result<Vec<PluginRecord>, String> {
    let registry = state.registry.clone();
    tauri::async_runtime::spawn_blocking(move || registry.list())
        .await
        .map_err(|_| "list task panicked".to_string())?
}

#[tauri::command]
pub async fn save_plugin_settings(
    state: State<'_, DesktopState>,
    id: String,
    settings: BTreeMap<String, Value>,
) -> Result<PluginRecord, String> {
    let registry = state.registry.clone();
    let id_clone = id.clone();
    let res =
        tauri::async_runtime::spawn_blocking(move || registry.save_settings(&id_clone, settings))
            .await
            .map_err(|_| "save settings task panicked".to_string())??;

    state
        .record_event(
            "settings",
            "save",
            &id,
            true,
            "Non-sensitive settings updated",
        )
        .await;

    Ok(res)
}

#[tauri::command]
pub async fn get_plugin_runtime_status(
    state: State<'_, DesktopState>,
    id: String,
) -> Result<PluginStatusInfo, String> {
    let status = state.runtime.get_status(&id).await;
    let active_requests = state.runtime.active_request_count(&id).await;

    let (status_str, port, pid, reason, error) = match status {
        RuntimeStatus::Stopped => ("stopped".to_string(), None, None, None, None),
        RuntimeStatus::Starting => ("starting".to_string(), None, None, None, None),
        RuntimeStatus::Ready { port, pid } => ("ready".to_string(), Some(port), pid, None, None),
        RuntimeStatus::Busy { port, pid, reason } => {
            ("busy".to_string(), Some(port), pid, Some(reason), None)
        }
        RuntimeStatus::Failed { error } => ("failed".to_string(), None, None, None, Some(error)),
    };

    Ok(PluginStatusInfo {
        status: status_str,
        port,
        pid,
        reason,
        error,
        active_requests,
    })
}

#[tauri::command]
pub async fn start_plugin(
    state: State<'_, DesktopState>,
    id: String,
) -> Result<PluginStatusInfo, String> {
    let record = state.registry.load(&id)?;
    let package_dir = state.registry.package_dir(&id)?;
    let data_dir = state.registry.data_dir(&id)?;

    // Get or generate plugin API key (upstream key)
    let api_key = match quotio_platform::get_plugin_credential(&id, "api_key") {
        Ok(Some(k)) if !k.is_empty() => k,
        _ => {
            let mut bytes = [0u8; 16];
            let _ = fill(&mut bytes);
            let gen = format!("pw_up_{}", hex_encode(&bytes));
            let _ = quotio_platform::set_plugin_credential(&id, "api_key", &gen);
            gen
        }
    };

    let start_res = state
        .runtime
        .start_plugin(&record, &package_dir, None, &data_dir, Some(&api_key))
        .await;

    match start_res {
        Ok(status) => {
            let is_ready_or_busy = matches!(
                status,
                RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }
            );

            if is_ready_or_busy {
                // If selected plugin, connect to Gateway backend
                let selected = {
                    let r = state.selected_plugin_id.read().await;
                    r.clone()
                };
                if selected.as_deref() == Some(&id) {
                    let _ = state.gateway.set_backend(&record.manifest, &api_key).await;
                }

                let (port, pid) = match status {
                    RuntimeStatus::Ready { port, pid } => (port, pid),
                    RuntimeStatus::Busy { port, pid, .. } => (port, pid),
                    _ => (0, None),
                };
                state
                    .record_event(
                        "plugin",
                        "start",
                        &id,
                        true,
                        &format!("Plugin started on port {port} (PID: {:?})", pid),
                    )
                    .await;
            } else {
                state
                    .record_event(
                        "plugin",
                        "start",
                        &id,
                        false,
                        "Startup entered failed state",
                    )
                    .await;
            }

            get_plugin_runtime_status(state, id).await
        }
        Err(e) => {
            state
                .record_event(
                    "plugin",
                    "start",
                    &id,
                    false,
                    &format!("Startup error: {e}"),
                )
                .await;
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn stop_plugin(state: State<'_, DesktopState>, id: String) -> Result<(), String> {
    match state.runtime.stop_plugin(&id).await {
        Ok(()) => {
            let selected = {
                let r = state.selected_plugin_id.read().await;
                r.clone()
            };
            if selected.as_deref() == Some(&id) {
                state.gateway.clear_backend().await;
            }
            state
                .record_event("plugin", "stop", &id, true, "Plugin stopped successfully")
                .await;
            Ok(())
        }
        Err(proxyweave_plugin::RuntimeError::ActiveRequests) => {
            let active = state.runtime.active_request_count(&id).await;
            state
                .record_event(
                    "plugin",
                    "stop",
                    &id,
                    false,
                    &format!("Stop refused: {active} active requests in-flight"),
                )
                .await;
            Err(format!(
                "插件当前有正在进行的网络请求（活跃连接数：{}），拒绝停止以防止中断活跃会话。请等待请求完成或取消下游连接后再尝试停止。",
                active
            ))
        }
        Err(e) => {
            state
                .record_event("plugin", "stop", &id, false, &format!("Stop failed: {e}"))
                .await;
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn restart_plugin(
    state: State<'_, DesktopState>,
    id: String,
) -> Result<PluginStatusInfo, String> {
    // 1. Stop
    if let Err(e) = state.runtime.stop_plugin(&id).await {
        if matches!(e, proxyweave_plugin::RuntimeError::ActiveRequests) {
            let active = state.runtime.active_request_count(&id).await;
            return Err(format!(
                "插件当前有正在进行的网络请求（活跃连接数：{}），无法重启。请等待请求完成后再试。",
                active
            ));
        }
    }

    // 2. Start
    let record = state.registry.load(&id)?;
    let package_dir = state.registry.package_dir(&id)?;
    let data_dir = state.registry.data_dir(&id)?;
    let api_key = match quotio_platform::get_plugin_credential(&id, "api_key") {
        Ok(Some(k)) if !k.is_empty() => k,
        _ => {
            let mut bytes = [0u8; 16];
            let _ = fill(&mut bytes);
            let gen = format!("pw_up_{}", hex_encode(&bytes));
            let _ = quotio_platform::set_plugin_credential(&id, "api_key", &gen);
            gen
        }
    };

    let start_res = state
        .runtime
        .start_plugin(&record, &package_dir, None, &data_dir, Some(&api_key))
        .await;

    match start_res {
        Ok(status) => {
            let is_ready = matches!(
                status,
                RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }
            );
            if is_ready {
                let selected = {
                    let r = state.selected_plugin_id.read().await;
                    r.clone()
                };
                if selected.as_deref() == Some(&id) {
                    let _ = state.gateway.set_backend(&record.manifest, &api_key).await;
                }
            }
            state
                .record_event("plugin", "restart", &id, is_ready, "Plugin restart settled")
                .await;
            get_plugin_runtime_status(state, id).await
        }
        Err(e) => {
            state
                .record_event(
                    "plugin",
                    "restart",
                    &id,
                    false,
                    &format!("Restart failed: {e}"),
                )
                .await;
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_plugin_secrets_status(
    state: State<'_, DesktopState>,
    id: String,
) -> Result<Vec<SecretStatusInfo>, String> {
    let record = state.registry.load(&id)?;
    let mut list = Vec::new();

    for name in &record.manifest.configuration.required_secrets {
        let is_set = match quotio_platform::get_plugin_credential(&id, name) {
            Ok(Some(v)) => !v.is_empty(),
            _ => false,
        };
        list.push(SecretStatusInfo {
            name: name.clone(),
            is_set,
        });
    }

    Ok(list)
}

#[tauri::command]
pub async fn save_plugin_secret(
    state: State<'_, DesktopState>,
    id: String,
    name: String,
    value: String,
) -> Result<(), String> {
    let record = state.registry.load(&id)?;
    if !record
        .manifest
        .configuration
        .required_secrets
        .contains(&name)
        && name != "api_key"
    {
        return Err(format!("密钥名称 '{name}' 未在插件 Manifest 中声明"));
    }

    if value.trim().is_empty() {
        return Err("密钥内容不可为空；如需移除请点击删除按钮".to_string());
    }

    // Require stopped status before mutating credentials to avoid tearing
    let status = state.runtime.get_status(&id).await;
    if matches!(
        status,
        RuntimeStatus::Ready { .. } | RuntimeStatus::Starting
    ) {
        return Err("无法在插件运行中修改敏感凭据，请先停止插件。".to_string());
    }

    quotio_platform::set_plugin_credential(&id, &name, value.trim())
        .map_err(|e| format!("保存凭据到系统 Keyring 失败：{e}"))?;

    state
        .record_event(
            "credential",
            "save_secret",
            &id,
            true,
            &format!("Secret '{name}' safely persisted to OS Keyring"),
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn delete_plugin_secret(
    state: State<'_, DesktopState>,
    id: String,
    name: String,
) -> Result<(), String> {
    let status = state.runtime.get_status(&id).await;
    if matches!(
        status,
        RuntimeStatus::Ready { .. } | RuntimeStatus::Starting
    ) {
        return Err("无法在插件运行中删除敏感凭据，请先停止插件。".to_string());
    }

    quotio_platform::delete_plugin_credential(&id, &name)
        .map_err(|e| format!("从系统 Keyring 删除凭据失败：{e}"))?;

    state
        .record_event(
            "credential",
            "delete_secret",
            &id,
            true,
            &format!("Secret '{name}' removed from OS Keyring"),
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn get_gateway_status(
    state: State<'_, DesktopState>,
) -> Result<GatewayStatusInfo, String> {
    let (is_running, port, listen_address) = {
        let gh = state.gateway_handle.lock().await;
        if let Some(h) = gh.as_ref() {
            (
                true,
                h.address().port(),
                Some(format!("127.0.0.1:{}", h.address().port())),
            )
        } else {
            (false, *state.gateway_port.read().await, None)
        }
    };

    let active_backend = {
        let r = state.selected_plugin_id.read().await;
        r.clone()
    };
    let master_key = {
        let r = state.gateway_master_key.read().await;
        r.clone()
    };
    let master_key_preview = if master_key.len() >= 8 {
        format!("{}••••••••", &master_key[..6])
    } else {
        "••••••••".to_string()
    };

    Ok(GatewayStatusInfo {
        is_running,
        port,
        listen_address,
        active_backend,
        master_key_preview,
    })
}

#[tauri::command]
pub async fn get_gateway_master_key(state: State<'_, DesktopState>) -> Result<String, String> {
    let r = state.gateway_master_key.read().await;
    Ok(r.clone())
}

#[tauri::command]
pub async fn regenerate_gateway_master_key(
    state: State<'_, DesktopState>,
) -> Result<String, String> {
    {
        let gh = state.gateway_handle.lock().await;
        if gh.is_some() {
            return Err("网关正在运行中，无法重新生成 Master Key；请先停止网关。".to_string());
        }
    }

    let new_key = generate_master_key()?;
    let _ = quotio_platform::set_credential("gateway:master_key", &new_key);
    {
        let mut w = state.gateway_master_key.write().await;
        *w = new_key.clone();
    }

    state
        .record_event(
            "gateway",
            "regenerate_key",
            "gateway",
            true,
            "Master Key regenerated and saved to OS Keyring",
        )
        .await;

    Ok(new_key)
}

#[tauri::command]
pub async fn start_gateway(state: State<'_, DesktopState>) -> Result<GatewayStatusInfo, String> {
    if state
        .gateway_starting
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return get_gateway_status(state).await;
    }

    let is_already_running = {
        let gh = state.gateway_handle.lock().await;
        gh.is_some()
    };
    if is_already_running {
        state.gateway_starting.store(false, Ordering::SeqCst);
        return get_gateway_status(state).await;
    }

    let port = *state.gateway_port.read().await;

    // Attach selected backend if it's currently running
    let selected_id = {
        let r = state.selected_plugin_id.read().await;
        r.clone()
    };
    if let Some(id) = selected_id {
        if let Ok(record) = state.registry.load(&id) {
            let status = state.runtime.get_status(&id).await;
            if matches!(
                status,
                RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }
            ) {
                let api_key = quotio_platform::get_plugin_credential(&id, "api_key")
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                let _ = state.gateway.set_backend(&record.manifest, &api_key).await;
            }
        }
    }

    let listen_res = state.gateway.listen(port).await;
    state.gateway_starting.store(false, Ordering::SeqCst);

    match listen_res {
        Ok(handle) => {
            let actual_port = handle.address().port();
            {
                let mut gh = state.gateway_handle.lock().await;
                *gh = Some(handle);
            }
            state
                .record_event(
                    "gateway",
                    "start",
                    "gateway",
                    true,
                    &format!("Unified gateway listening on 127.0.0.1:{actual_port}"),
                )
                .await;
            get_gateway_status(state).await
        }
        Err(e) => {
            state
                .record_event(
                    "gateway",
                    "start",
                    "gateway",
                    false,
                    &format!("Failed to bind port {port}: {e}"),
                )
                .await;
            Err(format!("启动网关服务失败（端口 {port}）：{e}"))
        }
    }
}

#[tauri::command]
pub async fn stop_gateway(state: State<'_, DesktopState>) -> Result<GatewayStatusInfo, String> {
    let handle = {
        let mut gh = state.gateway_handle.lock().await;
        gh.take()
    };

    if let Some(h) = handle {
        let _ = h.shutdown().await;
        state
            .record_event(
                "gateway",
                "stop",
                "gateway",
                true,
                "Gateway stopped and connections drained",
            )
            .await;
    }

    get_gateway_status(state).await
}

#[tauri::command]
pub async fn set_gateway_port(state: State<'_, DesktopState>, port: u16) -> Result<(), String> {
    {
        let gh = state.gateway_handle.lock().await;
        if gh.is_some() {
            return Err("网关正在运行中，无法修改端口；请先停止网关。".to_string());
        }
    }

    {
        let mut w = state.gateway_port.write().await;
        *w = port;
    }

    let selected_id = {
        let r = state.selected_plugin_id.read().await;
        r.clone()
    };
    let _ = state.save_desktop_config(&DesktopConfig {
        gateway_port: port,
        selected_plugin_id: selected_id,
    });

    state
        .record_event(
            "gateway",
            "set_port",
            "gateway",
            true,
            &format!("Configured gateway port: {port}"),
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn select_plugin(state: State<'_, DesktopState>, id: String) -> Result<(), String> {
    let record = state.registry.load(&id)?;
    {
        let mut w = state.selected_plugin_id.write().await;
        *w = Some(id.clone());
    }

    let current_port = *state.gateway_port.read().await;
    let _ = state.save_desktop_config(&DesktopConfig {
        gateway_port: current_port,
        selected_plugin_id: Some(id.clone()),
    });

    // If plugin is ready, update gateway backend immediately
    let status = state.runtime.get_status(&id).await;
    if matches!(
        status,
        RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }
    ) {
        let api_key = quotio_platform::get_plugin_credential(&id, "api_key")
            .ok()
            .flatten()
            .unwrap_or_default();
        let _ = state.gateway.set_backend(&record.manifest, &api_key).await;
    } else {
        state.gateway.clear_backend().await;
    }

    state
        .record_event(
            "gateway",
            "select_backend",
            &id,
            true,
            &format!("Selected plugin '{id}' as active backend"),
        )
        .await;

    Ok(())
}

#[tauri::command]
pub async fn get_host_events(state: State<'_, DesktopState>) -> Result<Vec<HostEvent>, String> {
    let events = state.events.lock().await;
    Ok(events.iter().cloned().collect())
}

#[tauri::command]
pub async fn test_gateway_models(state: State<'_, DesktopState>) -> Result<TestCallResult, String> {
    let port = {
        let gh = state.gateway_handle.lock().await;
        if let Some(h) = gh.as_ref() {
            h.address().port()
        } else {
            return Err("本地网关尚未启动，请先开启网关服务后再进行调用测试。".into());
        }
    };

    let master_key = {
        let r = state.gateway_master_key.read().await;
        r.clone()
    };
    let url = format!("http://127.0.0.1:{port}/v1/models");

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("cannot create HTTP client: {e}"))?;

    let start = std::time::Instant::now();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {master_key}"))
        .send()
        .await
        .map_err(|e| format!("请求网关失败：{e}"))?;

    let status = resp.status().as_u16();
    let latency_ms = start.elapsed().as_millis() as u64;
    let body = resp.text().await.unwrap_or_default();
    let success = status == 200;

    Ok(TestCallResult {
        success,
        status,
        latency_ms,
        response_body: body,
    })
}

pub async fn graceful_exit(state: &DesktopState, app: &AppHandle) {
    // 1. Drain & shutdown gateway connections first
    let handle = {
        let mut gh = state.gateway_handle.lock().await;
        gh.take()
    };
    if let Some(h) = handle {
        let _ = h.shutdown().await;
    }

    // 2. Clear backend
    state.gateway.clear_backend().await;

    // 3. Stop ONLY Managed plugins owned by ProxyWeave; External plugins are NEVER stopped or killed
    if let Ok(records) = state.registry.list() {
        for record in records {
            if record.manifest.ownership() == Ownership::Managed {
                let _ = state
                    .runtime
                    .stop_plugin(&record.manifest.metadata.id)
                    .await;
            }
        }
    }

    app.exit(0);
}
