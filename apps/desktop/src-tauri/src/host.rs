use getrandom::fill;
use proxyweave_gateway::{Gateway, GatewayHandle, RequestTrace, RouteStrategy};
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
use tauri::{AppHandle, Emitter, State};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<proxyweave_plugin::registry::InstalledSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginUpdateStatusDto {
    pub status: String,
    pub owned_update_allowed: bool,
    pub latest_tag: Option<String>,
    pub http_status: Option<u16>,
    pub rate_limit_remaining: Option<u64>,
    pub rate_limit_reset: Option<u64>,
    pub detail: String,
}

impl From<proxyweave_plugin::updater::UpdateCheck> for PluginUpdateStatusDto {
    fn from(check: proxyweave_plugin::updater::UpdateCheck) -> Self {
        let status = match check.status {
            proxyweave_plugin::updater::UpdateStatus::UpdateAvailable => "update_available",
            proxyweave_plugin::updater::UpdateStatus::UpToDate => "up_to_date",
            proxyweave_plugin::updater::UpdateStatus::ReleaseChanged => "release_changed",
            proxyweave_plugin::updater::UpdateStatus::InstalledNewer => "installed_newer",
            proxyweave_plugin::updater::UpdateStatus::NotCheckable => "not_checkable",
            proxyweave_plugin::updater::UpdateStatus::Forbidden => "forbidden",
            proxyweave_plugin::updater::UpdateStatus::NotFound => "not_found",
            proxyweave_plugin::updater::UpdateStatus::InvalidVersion => "error",
            proxyweave_plugin::updater::UpdateStatus::Error => "error",
        };
        Self {
            status: status.to_string(),
            owned_update_allowed: check.owned_update_allowed,
            latest_tag: check.latest_tag,
            http_status: check.http_status,
            rate_limit_remaining: check.rate_limit_remaining,
            rate_limit_reset: check.rate_limit_reset,
            detail: check.detail.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretStatusInfo {
    pub name: String,
    pub is_set: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayProtocolCapabilities {
    pub ingress: Vec<GatewayIngressCapability>,
    pub backend_protocols: Vec<String>,
    pub ir_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayIngressCapability {
    pub protocol: String,
    pub endpoints: Vec<String>,
}

impl GatewayProtocolCapabilities {
    fn compiled() -> Self {
        Self {
            ingress: proxyweave_gateway::INGRESS_PROTOCOL_ENDPOINTS
                .iter()
                .map(|(protocol, endpoints)| GatewayIngressCapability {
                    protocol: (*protocol).into(),
                    endpoints: endpoints.iter().map(|path| (*path).into()).collect(),
                })
                .collect(),
            backend_protocols: proxyweave_gateway::SUPPORTED_BACKEND_PROTOCOLS
                .iter()
                .map(|protocol| (*protocol).into())
                .collect(),
            ir_source: "TiyGate 6db40e54663455fb99d675bd8985e4dc50f68a14 / Apache-2.0".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStatusInfo {
    pub is_running: bool,
    pub port: u16,
    pub listen_address: Option<String>,
    pub active_backend: Option<String>,
    pub master_key_preview: String,
    pub protocol_capabilities: GatewayProtocolCapabilities,
    pub route_strategy: String,
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
    pub watcher: Arc<proxyweave_plugin::watcher::DirectoryWatcher>,
    plugin_operations: Mutex<BTreeMap<String, Arc<Mutex<()>>>>,
}

static FLIGHT_MANAGERS: std::sync::Mutex<
    Option<BTreeMap<PathBuf, Arc<crate::flight::FlightManager>>>,
> = std::sync::Mutex::new(None);

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn valid_master_key(value: &str) -> bool {
    value.len() >= 32 && value.len() <= 512 && value.bytes().all(|b| b.is_ascii_graphic())
}

pub fn generate_master_key() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    fill(&mut bytes).map_err(|e| format!("cannot generate secure random key: {e}"))?;
    Ok(format!("pw_{}", hex_encode(&bytes)))
}

impl DesktopState {
    pub fn new(profile_dir: PathBuf, registry: Registry) -> Result<Self, String> {
        Self::new_with_credential_store(
            profile_dir,
            registry,
            || quotio_platform::get_credential("gateway:master_key"),
            |key| quotio_platform::set_credential("gateway:master_key", key),
        )
    }

    pub fn new_with_credential_store<G, S>(
        profile_dir: PathBuf,
        registry: Registry,
        get_master_key: G,
        set_master_key: S,
    ) -> Result<Self, String>
    where
        G: FnOnce() -> Result<Option<String>, String>,
        S: FnOnce(&str) -> Result<(), String>,
    {
        let runtime = RuntimeManager::new();

        // Load or generate Master Key (ProxyWeave local Bearer token)
        let master_key =
            match get_master_key().map_err(|e| format!("cannot read gateway master key: {e}"))? {
                Some(key) if valid_master_key(&key) => key,
                Some(_) => {
                    return Err(
                        "stored gateway master key is invalid; regenerate or configure a valid key"
                            .to_string(),
                    )
                }
                None => {
                    let generated = generate_master_key()?;
                    set_master_key(&generated)
                        .map_err(|e| format!("cannot persist gateway master key: {e}"))?;
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

        let watcher = Arc::new(proxyweave_plugin::watcher::DirectoryWatcher::new());
        watcher.add_watch_dir(profile_dir.join("plugins"));
        watcher.add_watch_dir(PathBuf::from("plugins"));
        let _ = watcher.scan_all();

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
            watcher,
            plugin_operations: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn flight(&self) -> Arc<crate::flight::FlightManager> {
        let mut guard = FLIGHT_MANAGERS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let map = guard.get_or_insert_with(BTreeMap::new);
        map.entry(self.profile_dir.clone())
            .or_insert_with(|| Arc::new(crate::flight::FlightManager::new()))
            .clone()
    }

    async fn plugin_operation(&self, id: &str) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = self
            .plugin_operations
            .lock()
            .await
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        lock.lock_owned().await
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
/// Only the bundled main view (or the exact configured debug dev origin) may open links.
fn management_caller_allowed(label: &str, origin: &str, dev_origin: Option<&str>) -> bool {
    label == "main"
        && (matches!(
            origin,
            "http://tauri.localhost" | "https://tauri.localhost" | "tauri://localhost"
        ) || dev_origin == Some(origin))
}

fn check_management_caller(window: &tauri::WebviewWindow) -> Result<(), String> {
    use tauri::Manager;
    let url = window.url().map_err(|_| "management caller denied")?;
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
        || !management_caller_allowed(window.label(), &origin, dev_origin.as_deref())
    {
        return Err("management caller denied".into());
    }
    Ok(())
}

async fn management_link(
    state: &DesktopState,
    id: String,
) -> Result<proxyweave_plugin::management::ManagementLink, String> {
    let registry = state.registry.clone();
    let load_id = id.clone();
    let record = tauri::async_runtime::spawn_blocking(move || registry.load(&load_id))
        .await
        .map_err(|_| "management entry unavailable")?
        .map_err(|_| "management entry unavailable")?;
    let status = state.runtime.get_status(&id).await;
    proxyweave_plugin::management::resolve_management(&record.manifest, &status)
        .map_err(|_| "management entry unavailable".into())
}

#[tauri::command]
pub async fn get_plugin_management(
    window: tauri::WebviewWindow,
    state: State<'_, DesktopState>,
    id: String,
) -> Result<proxyweave_plugin::management::ManagementLink, String> {
    check_management_caller(&window)?;
    management_link(&state, id).await
}

#[tauri::command]
pub async fn open_plugin_management(
    window: tauri::WebviewWindow,
    state: State<'_, DesktopState>,
    id: String,
) -> Result<(), String> {
    check_management_caller(&window)?;
    // Never accept a UI URL or cached port. Reload/validate and reconcile again on each click.
    let link = management_link(&state, id).await?;
    let url = link.url.ok_or("management entry unavailable")?;
    tauri::async_runtime::spawn_blocking(move || tauri_plugin_opener::open_url(url, None::<&str>))
        .await
        .map_err(|_| "cannot open management page")?
        .map_err(|_| "cannot open management page".into())
}

#[cfg(test)]
mod management_tests {
    use super::management_caller_allowed;
    #[test]
    fn only_exact_local_main_caller() {
        assert!(management_caller_allowed(
            "main",
            "http://tauri.localhost",
            None
        ));
        assert!(management_caller_allowed(
            "main",
            "http://localhost:1420",
            Some("http://localhost:1420")
        ));
        for (label, origin) in [
            ("menubar", "http://tauri.localhost"),
            ("main", "http://127.0.0.1:9999"),
            ("main", "https://evil.invalid"),
            ("main", "http://tauri.localhost.evil.invalid"),
            ("main", "http://tauri.localhost:9999"),
        ] {
            assert!(!management_caller_allowed(label, origin, None));
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
        let new_sel = Some(res.record.manifest.metadata.id.clone());
        let current_port = *state.gateway_port.read().await;
        state
            .save_desktop_config(&DesktopConfig {
                gateway_port: current_port,
                selected_plugin_id: new_sel.clone(),
            })
            .map_err(|e| format!("自动选中插件保存配置失败: {e}"))?;
        let mut sel = state.selected_plugin_id.write().await;
        *sel = new_sel;
    }

    Ok(res)
}

#[tauri::command]
pub async fn import_project_from_directory(
    state: State<'_, DesktopState>,
    path: String,
) -> Result<ImportResult, String> {
    let raw_path = path.trim().to_string();
    if raw_path.is_empty() {
        return Err("项目目录路径不能为空".to_string());
    }
    let dir = PathBuf::from(&raw_path);
    if !dir.exists() {
        return Err(format!("指定路径不存在: {}", raw_path));
    }
    if !dir.is_dir() {
        return Err(format!("指定路径不是有效文件夹: {}", raw_path));
    }

    let registry = state.registry.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        proxyweave_plugin::stage_and_import_project(&registry, dir, None)
    })
    .await
    .map_err(|_| "directory import task panicked".to_string())??;

    state
        .record_event(
            "plugin",
            "import_directory",
            &res.record.manifest.metadata.id,
            true,
            &format!(
                "已探测并导入项目: {} (v{})",
                res.record.manifest.metadata.name, res.record.manifest.metadata.version
            ),
        )
        .await;

    // Auto-select if no plugin selected yet
    let current_selected = {
        let r = state.selected_plugin_id.read().await;
        r.clone()
    };
    if current_selected.is_none() {
        let new_sel = Some(res.record.manifest.metadata.id.clone());
        let current_port = *state.gateway_port.read().await;
        state
            .save_desktop_config(&DesktopConfig {
                gateway_port: current_port,
                selected_plugin_id: new_sel.clone(),
            })
            .map_err(|e| format!("自动选中插件保存配置失败: {e}"))?;
        let mut sel = state.selected_plugin_id.write().await;
        *sel = new_sel;
    }

    Ok(res)
}

#[tauri::command]
pub async fn import_project_from_github(
    state: State<'_, DesktopState>,
    url: String,
) -> Result<ImportResult, String> {
    let raw_url = url.trim().to_string();
    if raw_url.is_empty() {
        return Err("GitHub 仓库 URL 不能为空".to_string());
    }
    if raw_url.starts_with('-')
        || raw_url.contains('\0')
        || raw_url.contains('\n')
        || raw_url.contains('\r')
    {
        return Err("仓库地址格式非法".to_string());
    }
    if !raw_url.starts_with("https://github.com/")
        && !raw_url.starts_with("http://github.com/")
        && !raw_url.starts_with("git@github.com:")
    {
        return Err("当前仅支持标准 GitHub 仓库 URL (如 https://github.com/...)".to_string());
    }

    let registry = state.registry.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        let staging_clone_dir = tempfile::Builder::new()
            .prefix(".pw-git-clone-")
            .tempdir()
            .map_err(|e| format!("创建临时克隆目录失败: {e}"))?;

        let output = std::process::Command::new("git")
            .args(["clone", "--depth", "1", "--quiet", &raw_url, "."])
            .current_dir(staging_clone_dir.path())
            .output()
            .map_err(|e| format!("执行 git 命令失败 (请确认系统已安装 git): {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Git 克隆失败: {}", stderr.trim()));
        }

        proxyweave_plugin::stage_and_import_project(&registry, staging_clone_dir.path(), None)
    })
    .await
    .map_err(|_| "GitHub import task panicked".to_string())??;

    state
        .record_event(
            "plugin",
            "import_github",
            &res.record.manifest.metadata.id,
            true,
            &format!(
                "已克隆并导入 GitHub 项目: {} (v{})",
                res.record.manifest.metadata.name, res.record.manifest.metadata.version
            ),
        )
        .await;

    // Auto-select if no plugin selected yet
    let current_selected = {
        let r = state.selected_plugin_id.read().await;
        r.clone()
    };
    if current_selected.is_none() {
        let new_sel = Some(res.record.manifest.metadata.id.clone());
        let current_port = *state.gateway_port.read().await;
        state
            .save_desktop_config(&DesktopConfig {
                gateway_port: current_port,
                selected_plugin_id: new_sel.clone(),
            })
            .map_err(|e| format!("自动选中插件保存配置失败: {e}"))?;
        let mut sel = state.selected_plugin_id.write().await;
        *sel = new_sel;
    }

    Ok(res)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectProjectResult {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub homepage: String,
    pub license: String,
    pub project_type: String,
    pub runtime_kind: String, // "managed" | "external"
    pub entrypoint: String,
    pub sha256: String,
    pub protocols: Vec<String>,
    pub exported_models: Vec<String>,
    pub requires_secrets: Vec<String>,
    pub permissions: Vec<String>,
    pub notes: Vec<String>,
    pub raw_manifest: serde_json::Value,
}

pub fn extract_github_author(url: &str) -> String {
    let stripped = url
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/")
        .trim_start_matches("git@github.com:");
    if let Some((owner, _)) = stripped.split_once('/') {
        return owner.to_string();
    }
    String::new()
}

pub fn validate_github_repo_url(url: &str) -> Result<String, String> {
    let raw_url = url.trim().to_string();
    if raw_url.is_empty() {
        return Err("GitHub 仓库 URL 不能为空".to_string());
    }
    if raw_url.starts_with('-')
        || raw_url.contains('\0')
        || raw_url.contains('\n')
        || raw_url.contains('\r')
    {
        return Err("仓库地址格式非法".to_string());
    }
    if !raw_url.starts_with("https://github.com/")
        && !raw_url.starts_with("http://github.com/")
        && !raw_url.starts_with("git@github.com:")
    {
        return Err("当前仅支持标准 GitHub 仓库 URL (如 https://github.com/...)".to_string());
    }
    Ok(raw_url)
}

#[tauri::command]
pub async fn inspect_project_from_github(
    url: String,
) -> Result<InspectProjectResult, String> {
    let raw_url = validate_github_repo_url(&url)?;

    tauri::async_runtime::spawn_blocking(move || {
        let staging_clone_dir = tempfile::Builder::new()
            .prefix(".pw-git-inspect-")
            .tempdir()
            .map_err(|e| format!("创建临时预检目录失败: {e}"))?;

        let output = std::process::Command::new("git")
            .args(["clone", "--depth", "1", "--quiet", &raw_url, "."])
            .current_dir(staging_clone_dir.path())
            .output()
            .map_err(|e| format!("执行 git 命令失败 (请确认系统已安装 git): {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Git 克隆失败: {}", stderr.trim()));
        }

        let report = proxyweave_plugin::detect_project(staging_clone_dir.path())
            .map_err(|e| format!("解析项目清单失败: {e}"))?;

        let m = &report.manifest;
        let runtime_kind = match &m.runtime {
            proxyweave_plugin::manifest::Runtime::Managed { .. } => "managed".to_string(),
            proxyweave_plugin::manifest::Runtime::External { .. } => "external".to_string(),
        };
        let entrypoint = match &m.runtime {
            proxyweave_plugin::manifest::Runtime::Managed { executable, .. } => executable.clone(),
            proxyweave_plugin::manifest::Runtime::External { endpoint } => endpoint.clone(),
        };

        let mut protocols = vec![m.capabilities.protocol.clone()];
        if m.capabilities.streaming && !protocols.contains(&"streaming".to_string()) {
            protocols.push("streaming".to_string());
        }

        let exported_models = Vec::new();

        let catalog_item = proxyweave_plugin::get_builtin_catalog()
            .into_iter()
            .find(|item| item.id == m.metadata.id);

        let (description, author, homepage) = if let Some(item) = catalog_item {
            let author = if item.author.is_empty() {
                extract_github_author(&raw_url)
            } else {
                item.author
            };
            let homepage = if item.repo_url.is_empty() {
                raw_url.clone()
            } else {
                item.repo_url
            };
            (item.description, author, homepage)
        } else {
            let author = extract_github_author(&raw_url);
            (String::new(), author, raw_url.clone())
        };

        let mut permissions = Vec::new();
        if m.ownership() == proxyweave_plugin::Ownership::Managed {
            permissions.push("独立子进程执行 (managed_process)".to_string());
        }
        permissions.push("本地网络端口监听与通信 (local_network)".to_string());
        if !m.configuration.required_secrets.is_empty() {
            permissions.push("系统安全凭据托管 (keyring_secrets)".to_string());
        }

        let raw_manifest = serde_json::to_value(m).unwrap_or(serde_json::Value::Null);

        Ok(InspectProjectResult {
            id: m.metadata.id.clone(),
            name: m.metadata.name.clone(),
            version: m.metadata.version.clone(),
            description,
            author,
            homepage,
            license: m.metadata.license.clone(),
            project_type: report.project_type,
            runtime_kind,
            entrypoint,
            sha256: report.executable_sha256,
            protocols,
            exported_models,
            requires_secrets: m.configuration.required_secrets.clone(),
            permissions,
            notes: report.notes,
            raw_manifest,
        })
    })
    .await
    .map_err(|_| "GitHub inspect task panicked".to_string())?
}

#[tauri::command]
pub async fn install_plugin_from_release(
    app_handle: tauri::AppHandle,
    state: State<'_, DesktopState>,
    repo_url: String,
    tag: Option<String>,
    plugin_id: String,
    expected_sha256: Option<String>,
) -> Result<ImportResult, String> {
    let raw_url = repo_url.trim().to_string();
    if raw_url.is_empty() {
        return Err("GitHub 仓库 URL 不能为空".to_string());
    }
    let clean_id = plugin_id.trim().to_string();
    if clean_id.is_empty() {
        return Err("插件 ID 不能为空".to_string());
    }

    let handle = app_handle.clone();
    let on_progress = move |progress: proxyweave_plugin::downloader::DownloadProgress| {
        let _ = handle.emit("plugin_download_progress", &progress);
    };

    let registry = state.registry.clone();
    let res = proxyweave_plugin::downloader::download_and_install_release(
        &registry,
        &raw_url,
        tag.as_deref(),
        &clean_id,
        expected_sha256.as_deref(),
        None,
        on_progress,
    )
    .await?;

    state
        .record_event(
            "plugin",
            "install_release",
            &res.record.manifest.metadata.id,
            true,
            &format!(
                "已下载并安全安装 GitHub Release 资产: {} (v{})",
                res.record.manifest.metadata.name, res.record.manifest.metadata.version
            ),
        )
        .await;

    // Auto-select if no plugin selected yet
    let current_selected = {
        let r = state.selected_plugin_id.read().await;
        r.clone()
    };
    if current_selected.is_none() {
        let new_sel = Some(res.record.manifest.metadata.id.clone());
        let current_port = *state.gateway_port.read().await;
        state
            .save_desktop_config(&DesktopConfig {
                gateway_port: current_port,
                selected_plugin_id: new_sel.clone(),
            })
            .map_err(|e| format!("自动选中插件保存配置失败: {e}"))?;
        let mut sel = state.selected_plugin_id.write().await;
        *sel = new_sel;
    }

    Ok(res)
}

#[tauri::command]
pub async fn check_plugin_update(
    app_handle: tauri::AppHandle,
    state: State<'_, DesktopState>,
    plugin_id: String,
) -> Result<PluginUpdateStatusDto, String> {
    let _ = app_handle;
    let clean_id = plugin_id.trim();
    if clean_id.is_empty() {
        return Err("插件 ID 不能为空".to_string());
    }
    let record = state.registry.load(clean_id)?;
    let check = proxyweave_plugin::updater::check_plugin_update(&record).await;
    Ok(PluginUpdateStatusDto::from(check))
}

pub type UpdateResultDto = proxyweave_plugin::updater::UpdateResult;

#[tauri::command]
pub async fn update_plugin_to_release(
    app_handle: AppHandle,
    state: State<'_, DesktopState>,
    id: String,
    expected_sha256: Option<String>,
) -> Result<UpdateResultDto, String> {
    let clean_id = id.trim().to_string();
    if clean_id.is_empty() {
        return Err("插件 ID 不能为空".to_string());
    }

    let permit = proxyweave_plugin::activation::CompatibilityPermit::default_production();
    if !permit.is_compatible() {
        return Err(
            "CompatibilityUnknown: plugin update requires verified data compatibility".into(),
        );
    }
    // 0. Hold singleflight operation lock
    let _operation = state.plugin_operation(&clean_id).await;

    // 1. Initial validation under file lock
    let file_lock = state.registry.acquire_plugin_lock(&clean_id)?;
    let record = state.registry.load(&clean_id)?;

    // Reject external plugins with zero kill/modification
    if record.manifest.ownership() != Ownership::Managed {
        return Ok(UpdateResultDto::NotAllowedExternal);
    }

    let current_source = record
        .source
        .clone()
        .ok_or_else(|| "当前插件未绑定发布来源，无法检查或下载更新".to_string())?;

    // Pre-check active requests
    let initial_active = state.runtime.active_request_count(&clean_id).await;
    if initial_active > 0 {
        return Ok(UpdateResultDto::BlockedActiveRequests {
            active_requests: initial_active,
        });
    }

    let repo_url = format!(
        "https://github.com/{}/{}",
        current_source.owner, current_source.repo
    );
    let handle = app_handle.clone();
    let on_progress = move |progress: proxyweave_plugin::downloader::DownloadProgress| {
        let _ = handle.emit("plugin_download_progress", &progress);
    };

    // 2. Download and verify candidate package to temp directory
    let staged = proxyweave_plugin::downloader::download_and_stage_release_package(
        &repo_url,
        None,
        &clean_id,
        expected_sha256.as_deref(),
        None,
        on_progress,
    )
    .await?;

    let candidate_version = staged.source.tag_name.trim_start_matches('v').to_string();

    // 3. Store version package in versions/<candidate_version>
    let (_version_dir, new_record) = state.registry.store_version_package(
        &clean_id,
        &candidate_version,
        &staged.manifest_path,
        Some(staged.source.clone()),
    )?;

    // 4. Read or initialize activation envelope
    let mut envelope = match state.registry.read_activation(&clean_id)? {
        Some(env) => env,
        None => state
            .registry
            .initialize_legacy_activation(&clean_id, &record, &file_lock)?,
    };

    let candidate_ptr = proxyweave_plugin::activation::VersionPointer {
        location: proxyweave_plugin::activation::VersionLocation::VersionKey {
            version_key: candidate_version.clone(),
        },
        fingerprint: new_record.fingerprint.clone(),
        version: new_record.manifest.metadata.version.clone(),
        source: Some(staged.source),
    };

    let txn_id = proxyweave_plugin::activation::generate_txn_id("upd", &clean_id);

    // CAS advance: Committed -> Prepared
    let stamp = envelope.stamp.clone();
    envelope.prepare_candidate(candidate_ptr, txn_id)?;
    envelope = state
        .registry
        .cas_advance_activation(&clean_id, &stamp, envelope, &file_lock)?;

    // 5. Determine runtime state and enter maintenance barrier if running
    let runtime_status = state.runtime.get_status(&clean_id).await;
    let is_running = matches!(
        runtime_status,
        RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }
    );

    let maintenance_token = if is_running {
        match state
            .runtime
            .enter_maintenance_barrier(&clean_id, Duration::from_secs(10))
            .await
        {
            Ok(token) => Some(token),
            Err(proxyweave_plugin::RuntimeError::ActiveRequestsTimeout {
                active_requests, ..
            }) => {
                // Rollback uncommitted envelope
                let mut env = envelope;
                let _ = env.mark_rollback_pending();
                let _ = env.complete_rollback(&state.registry.package_dir(&clean_id)?);
                let _ = state.registry.write_activation(&clean_id, &env, &file_lock);
                return Ok(UpdateResultDto::BlockedActiveRequests { active_requests });
            }
            Err(e) => {
                let mut env = envelope;
                let _ = env.mark_rollback_pending();
                let _ = env.complete_rollback(&state.registry.package_dir(&clean_id)?);
                let _ = state.registry.write_activation(&clean_id, &env, &file_lock);
                return Err(format!("进入维护屏障失败: {e}"));
            }
        }
    } else {
        None
    };

    // Keep every fallible operation inside this scope so only the owner releases the fence.
    let result = async {
        if let Some(token) = maintenance_token.as_ref() {
            // Stop old instance
            if let Err(e) = state
                .runtime
                .stop_plugin_under_maintenance(&clean_id, token)
                .await
            {
                let mut env = envelope;
                let _ = env.mark_rollback_pending();
                let _ = env.complete_rollback(&state.registry.package_dir(&clean_id)?);
                let _ = state.registry.write_activation(&clean_id, &env, &file_lock);
                return Err(format!("停止旧版本插件失败: {e}"));
            }

            // CAS: Prepared -> Stopped
            let stamp = envelope.stamp.clone();
            envelope.mark_stopped()?;
            envelope = state
                .registry
                .cas_advance_activation(&clean_id, &stamp, envelope, &file_lock)?;
        }

        // 6. Switch activation pointer: Stopped/Prepared -> Switched
        let stamp = envelope.stamp.clone();
        envelope.switch_to_candidate()?;
        envelope = state
            .registry
            .cas_advance_activation(&clean_id, &stamp, envelope, &file_lock)?;

        // 7. Restart or keep stopped
        let container_dir = state.registry.package_dir(&clean_id)?;
        let old_version = record.manifest.metadata.version.clone();
        let new_version = new_record.manifest.metadata.version.clone();

        if !is_running {
            // Was stopped: stay stopped
            let stamp = envelope.stamp.clone();
            envelope.commit_active()?;
            let _ = state
                .registry
                .cas_advance_activation(&clean_id, &stamp, envelope, &file_lock)?;

            state
                .record_event(
                    "plugin",
                    "update",
                    &clean_id,
                    true,
                    &format!("Plugin updated (stopped): {old_version} -> {new_version}"),
                )
                .await;

            return Ok(UpdateResultDto::Success {
                old_version,
                new_version,
                restarted: false,
            });
        }

        // Was running: start new version
        let token = maintenance_token
            .as_ref()
            .ok_or("missing maintenance owner")?;
        let active_pkg_dir = envelope.current.resolve_package_dir(&container_dir)?;
        let data_dir = state.registry.data_dir(&clean_id)?;
        let api_key = load_or_create_plugin_api_key(&clean_id)?;

        let start_res = state
            .runtime
            .start_plugin_under_maintenance(
                &new_record,
                &active_pkg_dir,
                None,
                &data_dir,
                Some(&api_key),
                token,
            )
            .await;

        match start_res {
            Ok(RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }) => {
                // New version started successfully!
                // Update gateway if selected
                let selected = state.selected_plugin_id.read().await.clone();
                if selected.as_deref() == Some(&clean_id) {
                    state
                        .gateway
                        .set_backend(&new_record.manifest, &api_key)
                        .await?;
                }

                // Commit activation
                let stamp = envelope.stamp.clone();
                envelope.commit_active()?;
                let _ = state
                    .registry
                    .cas_advance_activation(&clean_id, &stamp, envelope, &file_lock)?;

                state
                    .record_event(
                        "plugin",
                        "update",
                        &clean_id,
                        true,
                        &format!("Plugin updated and hot reloaded: {old_version} -> {new_version}"),
                    )
                    .await;

                Ok(UpdateResultDto::Success {
                    old_version,
                    new_version,
                    restarted: true,
                })
            }
            failed => {
                // New version failed to start: initiate automatic rollback!
                let start_err = match failed {
                    Err(e) => e.to_string(),
                    Ok(s) => format!("Unexpected status: {s:?}"),
                };

                // Kill candidate if partially started
                let _ = state
                    .runtime
                    .stop_plugin_under_maintenance(&clean_id, token)
                    .await;

                // Execute rollback in activation envelope
                let rollback_res =
                    (|| -> Result<proxyweave_plugin::activation::ActivationEnvelope, String> {
                        let stamp = envelope.stamp.clone();
                        envelope.mark_rollback_pending()?;
                        let advanced = state
                            .registry
                            .cas_advance_activation(&clean_id, &stamp, envelope, &file_lock)?;

                        let mut env = advanced;
                        env.complete_rollback(&container_dir)?;
                        state
                            .registry
                            .write_activation(&clean_id, &env, &file_lock)?;
                        Ok(env)
                    })();

                let rolled_back_env = match rollback_res {
                    Ok(env) => env,
                    Err(rb_err) => {
                        return Ok(UpdateResultDto::RollbackFailed {
                            error: start_err,
                            rollback_error: rb_err,
                        });
                    }
                };

                // Restart old version
                let old_pkg_dir = match rolled_back_env.current.resolve_package_dir(&container_dir)
                {
                    Ok(p) => p,
                    Err(e) => {
                        return Ok(UpdateResultDto::RollbackFailed {
                            error: start_err,
                            rollback_error: format!("Cannot resolve old package dir: {e}"),
                        });
                    }
                };

                let restart_old = state
                    .runtime
                    .start_plugin_under_maintenance(
                        &record,
                        &old_pkg_dir,
                        None,
                        &data_dir,
                        Some(&api_key),
                        token,
                    )
                    .await;

                match restart_old {
                    Ok(RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }) => {
                        let selected = state.selected_plugin_id.read().await.clone();
                        if selected.as_deref() == Some(&clean_id) {
                            state
                                .gateway
                                .set_backend(&record.manifest, &api_key)
                                .await?;
                        }

                        state
                            .record_event(
                                "plugin",
                                "update_rollback",
                                &clean_id,
                                true,
                                &format!(
                                "New version failed ({start_err}), rolled back to {old_version}"
                            ),
                            )
                            .await;

                        Ok(UpdateResultDto::RollbackSuccess {
                            error: start_err,
                            rolled_back_to_version: old_version,
                        })
                    }
                    failed_old => {
                        let old_err = match failed_old {
                            Err(e) => e.to_string(),
                            Ok(s) => format!("Unexpected old status: {s:?}"),
                        };
                        Ok(UpdateResultDto::RollbackFailed {
                            error: start_err,
                            rollback_error: format!("Old version failed to restart: {old_err}"),
                        })
                    }
                }
            }
        }
    }
    .await;
    if let Some(token) = maintenance_token.as_ref() {
        if matches!(&result, Ok(UpdateResultDto::Success { .. })) {
            state
                .runtime
                .exit_maintenance(token)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            state
                .runtime
                .cancel_maintenance_barrier(token)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    result
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

    let source = state.registry.load(&id).ok().and_then(|r| r.source);

    Ok(PluginStatusInfo {
        status: status_str,
        port,
        pid,
        reason,
        error,
        active_requests,
        source,
    })
}

#[tauri::command]
pub async fn start_plugin(
    state: State<'_, DesktopState>,
    id: String,
) -> Result<PluginStatusInfo, String> {
    let _operation = state.plugin_operation(&id).await;
    let (record, package_dir, _) = state.registry.resolve_active_package(&id)?;
    let data_dir = state.registry.data_dir(&id)?;

    let api_key = load_or_create_plugin_api_key(&id)?;

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
                    state
                        .gateway
                        .set_backend(&record.manifest, &api_key)
                        .await
                        .map_err(|_| {
                            "插件已运行，但统一网关后端挂载失败；请检查插件访问密钥和协议"
                                .to_string()
                        })?;
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
    let _operation = state.plugin_operation(&id).await;
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
    let _operation = state.plugin_operation(&id).await;
    state.registry.load(&id)?;
    // 1. Stop
    if let Err(e) = state.runtime.stop_plugin(&id).await {
        if matches!(e, proxyweave_plugin::RuntimeError::ActiveRequests) {
            let active = state.runtime.active_request_count(&id).await;
            return Err(format!(
                "插件当前有正在进行的网络请求（活跃连接数：{}），无法重启。请等待请求完成后再试。",
                active
            ));
        }
        if !matches!(e, proxyweave_plugin::RuntimeError::PluginNotFound(_)) {
            return Err(e.to_string());
        }
    }
    if state.selected_plugin_id.read().await.as_deref() == Some(&id) {
        state.gateway.clear_backend().await;
    }

    // 2. Start
    let (record, package_dir, _) = state.registry.resolve_active_package(&id)?;
    let data_dir = state.registry.data_dir(&id)?;
    let api_key = load_or_create_plugin_api_key(&id)?;

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
                    state
                        .gateway
                        .set_backend(&record.manifest, &api_key)
                        .await
                        .map_err(|_| {
                            "插件已运行，但统一网关后端挂载失败；请检查插件访问密钥和协议"
                                .to_string()
                        })?;
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

fn valid_plugin_api_key(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && value.bytes().all(|b| (0x21..=0x7e).contains(&b))
}

fn read_plugin_api_key(id: &str) -> Result<String, String> {
    quotio_platform::get_plugin_credential(id, "api_key")
        .map_err(|_| "无法读取插件访问密钥".to_string())?
        .filter(|key| valid_plugin_api_key(key))
        .ok_or_else(|| "插件访问密钥尚未配置或格式无效".to_string())
}

fn load_or_create_plugin_api_key(id: &str) -> Result<String, String> {
    match quotio_platform::get_plugin_credential(id, "api_key")
        .map_err(|_| "无法读取插件访问密钥；请在插件页明确保存新值".to_string())?
    {
        Some(key) if valid_plugin_api_key(&key) => Ok(key),
        Some(_) => Err("已存储的插件访问密钥格式无效；请在插件页明确保存新值".to_string()),
        None => {
            let mut bytes = [0u8; 16];
            fill(&mut bytes).map_err(|_| "无法生成插件访问密钥".to_string())?;
            let generated = format!("pw_up_{}", hex_encode(&bytes));
            quotio_platform::set_plugin_credential(id, "api_key", &generated)
                .map_err(|_| "无法保存插件访问密钥；插件未启动".to_string())?;
            Ok(generated)
        }
    }
}

fn plugin_secret_names(manifest: &proxyweave_plugin::Manifest) -> Vec<String> {
    let mut names = manifest.configuration.required_secrets.clone();
    if manifest.ownership() == Ownership::Managed && !names.iter().any(|name| name == "api_key") {
        names.insert(0, "api_key".to_string());
    }
    names
}

// Only the plugin-local access key is retrievable on explicit user copy.
// OAuth/account credentials remain status-only and are never returned to the UI.
#[tauri::command]
pub async fn get_plugin_api_key(
    window: tauri::WebviewWindow,
    state: State<'_, DesktopState>,
    id: String,
) -> Result<String, String> {
    check_management_caller(&window)?;
    let record = state.registry.load(&id)?;
    if record.manifest.ownership() != Ownership::Managed {
        return Err("外部服务访问密钥不由宿主公开".to_string());
    }
    read_plugin_api_key(&id)
}

#[tauri::command]
pub async fn get_plugin_secrets_status(
    state: State<'_, DesktopState>,
    id: String,
) -> Result<Vec<SecretStatusInfo>, String> {
    let record = state.registry.load(&id)?;
    let mut list = Vec::new();

    for name in plugin_secret_names(&record.manifest) {
        let is_set = quotio_platform::get_plugin_credential(&id, &name)
            .map_err(|_| "无法读取插件凭据状态".to_string())?
            .is_some_and(|value| !value.is_empty());
        list.push(SecretStatusInfo { name, is_set });
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
    save_plugin_secret_value(&state, id, name, value).await
}

async fn save_plugin_secret_value(
    state: &DesktopState,
    id: String,
    name: String,
    value: String,
) -> Result<(), String> {
    let _operation = state.plugin_operation(&id).await;
    let record = state.registry.load(&id)?;
    if !plugin_secret_names(&record.manifest).contains(&name) {
        return Err(format!("密钥名称 '{name}' 未在插件 Manifest 中声明"));
    }

    if value.trim().is_empty() {
        return Err("密钥内容不可为空；如需移除请点击删除按钮".to_string());
    }

    // Require stopped status before mutating credentials to avoid tearing
    let status = state.runtime.get_status(&id).await;
    if !matches!(status, RuntimeStatus::Stopped) {
        return Err("修改敏感凭据前请先停止插件（包括 Busy 或 Failed 状态）。".to_string());
    }
    if name == "api_key" && !valid_plugin_api_key(value.trim()) {
        return Err(
            "插件访问密钥须为 1–512 个可打印 ASCII 字符，不能含空格或控制字符。".to_string(),
        );
    }

    quotio_platform::set_plugin_credential(&id, &name, value.trim())
        .map_err(|_| "保存凭据到系统 Keyring 失败".to_string())?;

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
    let _operation = state.plugin_operation(&id).await;
    let record = state.registry.load(&id)?;
    if !plugin_secret_names(&record.manifest).contains(&name) {
        return Err("插件未声明此凭据".to_string());
    }
    let status = state.runtime.get_status(&id).await;
    if !matches!(status, RuntimeStatus::Stopped) {
        return Err("删除敏感凭据前请先停止插件（包括 Busy 或 Failed 状态）。".to_string());
    }

    quotio_platform::delete_plugin_credential(&id, &name)
        .map_err(|_| "从系统 Keyring 删除凭据失败".to_string())?;

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

    let active_backend = state.gateway.backend_plugin_id().await;
    let master_key = {
        let r = state.gateway_master_key.read().await;
        r.clone()
    };
    let master_key_preview = if master_key.len() >= 8 {
        format!("{}••••••••", &master_key[..6])
    } else {
        "••••••••".to_string()
    };

    let route_strategy = state
        .gateway
        .get_route_strategy()
        .await
        .as_str()
        .to_string();

    Ok(GatewayStatusInfo {
        is_running,
        port,
        listen_address,
        active_backend,
        master_key_preview,
        protocol_capabilities: GatewayProtocolCapabilities::compiled(),
        route_strategy,
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
    rotate_gateway_master_key_with(&state, |new_key| {
        quotio_platform::set_credential("gateway:master_key", new_key)
    })
    .await
}

async fn rotate_gateway_master_key_with<F>(
    state: &DesktopState,
    persist: F,
) -> Result<String, String>
where
    F: FnOnce(&str) -> Result<(), String>,
{
    {
        let gh = state.gateway_handle.lock().await;
        if gh.is_some() || state.gateway_starting.load(Ordering::SeqCst) {
            return Err("网关正在运行中，无法重新生成 Master Key；请先停止网关。".to_string());
        }
    }

    let new_key = generate_master_key()?;
    persist(&new_key)?;
    state
        .gateway
        .set_local_key(&new_key)
        .map_err(|e| format!("更新网关鉴权失败: {e}"))?;
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

    let attach_result = async {
        let selected_id = state.selected_plugin_id.read().await.clone();
        if let Some(id) = selected_id {
            let record = state.registry.load(&id)?;
            let status = state.runtime.get_status(&id).await;
            if matches!(
                status,
                RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }
            ) {
                let api_key = read_plugin_api_key(&id)?;
                state
                    .gateway
                    .set_backend(&record.manifest, &api_key)
                    .await
                    .map_err(|_| "统一网关后端挂载失败；请检查插件访问密钥和协议".to_string())?;
            } else {
                state.gateway.clear_backend().await;
            }
        }
        Ok::<(), String>(())
    }
    .await;
    if let Err(error) = attach_result {
        state.gateway_starting.store(false, Ordering::SeqCst);
        return Err(error);
    }

    let listen_res = state.gateway.listen(port).await;

    match listen_res {
        Ok(handle) => {
            let actual_port = handle.address().port();
            {
                let mut gh = state.gateway_handle.lock().await;
                *gh = Some(handle);
            }
            state.gateway_starting.store(false, Ordering::SeqCst);
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
            state.gateway_starting.store(false, Ordering::SeqCst);
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
    set_gateway_port_value(&state, port).await
}

#[tauri::command]
pub async fn set_gateway_route_strategy(
    state: State<'_, DesktopState>,
    strategy: String,
) -> Result<GatewayStatusInfo, String> {
    let strat = RouteStrategy::from_str_name(&strategy)
        .ok_or_else(|| format!("未知路由策略: {strategy}，仅支持 smart, order, rotate"))?;
    state.gateway.set_route_strategy(strat).await;
    get_gateway_status(state).await
}

#[tauri::command]
pub async fn get_gateway_traces(
    state: State<'_, DesktopState>,
    limit: Option<usize>,
) -> Result<Vec<RequestTrace>, String> {
    Ok(state.gateway.get_traces(limit).await)
}

#[tauri::command]
pub async fn clear_gateway_traces(state: State<'_, DesktopState>) -> Result<(), String> {
    state.gateway.clear_traces().await;
    Ok(())
}

async fn set_gateway_port_value(state: &DesktopState, port: u16) -> Result<(), String> {
    {
        let gh = state.gateway_handle.lock().await;
        if gh.is_some() || state.gateway_starting.load(Ordering::SeqCst) {
            return Err("网关正在运行中，无法修改端口；请先停止网关。".to_string());
        }
    }

    let selected_id = {
        let r = state.selected_plugin_id.read().await;
        r.clone()
    };
    state
        .save_desktop_config(&DesktopConfig {
            gateway_port: port,
            selected_plugin_id: selected_id,
        })
        .map_err(|e| format!("保存网关端口配置失败: {e}"))?;

    {
        let mut w = state.gateway_port.write().await;
        *w = port;
    }

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
    select_plugin_backend(&state, id).await
}

async fn select_plugin_backend(state: &DesktopState, id: String) -> Result<(), String> {
    let _operation = state.plugin_operation(&id).await;
    let record = state.registry.load(&id)?;
    let status = state.runtime.get_status(&id).await;
    if !matches!(
        status,
        RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }
    ) {
        return Err("插件尚未运行，未挂载到统一网关".to_string());
    }
    let api_key = read_plugin_api_key(&id)?;
    state
        .gateway
        .set_backend(&record.manifest, &api_key)
        .await
        .map_err(|_| "统一网关后端挂载失败；请检查插件访问密钥和协议".to_string())?;
    let current_port = *state.gateway_port.read().await;
    if state
        .save_desktop_config(&DesktopConfig {
            gateway_port: current_port,
            selected_plugin_id: Some(id.clone()),
        })
        .is_err()
    {
        state.gateway.clear_backend().await;
        return Err("后端选择保存失败，已撤销挂载".to_string());
    }
    *state.selected_plugin_id.write().await = Some(id.clone());

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

// ---------------------------------------------------------------------------
// Flight IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn flight_list_models(
    window: tauri::WebviewWindow,
    state: State<'_, DesktopState>,
) -> Result<crate::flight::FlightModelListResult, String> {
    crate::flight::check_flight_caller(&window)?;
    let (port, master_key) = {
        let gh = state.gateway_handle.lock().await;
        if let Some(h) = gh.as_ref() {
            let port = h.address().port();
            let key = state.gateway_master_key.read().await.clone();
            (port, key)
        } else {
            return Err("本地统一网关尚未启动，请先开启网关服务后再试飞。".to_string());
        }
    };
    let active_backend = state.gateway.backend_plugin_id().await;
    if active_backend.is_none() {
        return Err(
            "统一网关尚未挂载任何插件后端，请先在插件列表中选择并挂载模型后端。".to_string(),
        );
    }
    crate::flight::list_models_core(port, &master_key, active_backend).await
}

#[tauri::command]
pub async fn flight_chat_stream(
    window: tauri::WebviewWindow,
    state: State<'_, DesktopState>,
    channel: tauri::ipc::Channel<crate::flight::FlightStreamEvent>,
    input: crate::flight::FlightChatRequestInput,
) -> Result<crate::flight::FlightChatStartResult, String> {
    crate::flight::check_flight_caller(&window)?;
    let (port, master_key) = {
        let gh = state.gateway_handle.lock().await;
        if let Some(h) = gh.as_ref() {
            let port = h.address().port();
            let key = state.gateway_master_key.read().await.clone();
            (port, key)
        } else {
            return Err("本地统一网关尚未启动，请先开启网关服务后再试飞。".to_string());
        }
    };
    if state.gateway.backend_plugin_id().await.is_none() {
        return Err(
            "统一网关尚未挂载任何插件后端，请先在插件列表中选择并挂载模型后端。".to_string(),
        );
    }

    let window_label = window.label().to_string();
    let flight_mgr = state.flight();
    let request_id = input
        .request_id
        .clone()
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| flight_mgr.next_request_id());
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();

    flight_mgr
        .register_session(&window_label, &request_id, cancel_tx)
        .await?;

    let win_lbl = window_label.clone();
    let req_id = request_id.clone();
    let model = input.model.clone();

    tauri::async_runtime::spawn(async move {
        let res = crate::flight::run_flight_chat_stream_core(
            port,
            &master_key,
            input,
            req_id.clone(),
            channel,
            cancel_rx,
        )
        .await;
        flight_mgr.unregister_session(&win_lbl, &req_id).await;
        res
    });

    Ok(crate::flight::FlightChatStartResult { request_id, model })
}

#[tauri::command]
pub async fn flight_chat_nonstream(
    window: tauri::WebviewWindow,
    state: State<'_, DesktopState>,
    input: crate::flight::FlightChatRequestInput,
) -> Result<crate::flight::FlightChatNonstreamResult, String> {
    crate::flight::check_flight_caller(&window)?;
    let (port, master_key) = {
        let gh = state.gateway_handle.lock().await;
        if let Some(h) = gh.as_ref() {
            let port = h.address().port();
            let key = state.gateway_master_key.read().await.clone();
            (port, key)
        } else {
            return Err("本地统一网关尚未启动，请先开启网关服务后再试飞。".to_string());
        }
    };
    if state.gateway.backend_plugin_id().await.is_none() {
        return Err(
            "统一网关尚未挂载任何插件后端，请先在插件列表中选择并挂载模型后端。".to_string(),
        );
    }

    let window_label = window.label().to_string();
    let flight_mgr = state.flight();
    let request_id = input
        .request_id
        .clone()
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| flight_mgr.next_request_id());
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();

    flight_mgr
        .register_session(&window_label, &request_id, cancel_tx)
        .await?;

    let res = crate::flight::run_flight_chat_nonstream_core(
        port,
        &master_key,
        input,
        request_id.clone(),
        cancel_rx,
    )
    .await;
    flight_mgr
        .unregister_session(&window_label, &request_id)
        .await;

    res
}

#[tauri::command]
pub async fn flight_cancel(
    window: tauri::WebviewWindow,
    state: State<'_, DesktopState>,
    request_id: String,
) -> Result<crate::flight::FlightCancelResult, String> {
    crate::flight::check_flight_caller(&window)?;
    state
        .flight()
        .cancel_session(window.label(), &request_id)
        .await
}

// ---------------------------------------------------------------------------
// Agent / IDE Integration IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn detect_agent_clients(
    state: State<'_, DesktopState>,
) -> Result<Vec<crate::agent_import::AgentTargetStatus>, String> {
    let port = {
        let gh = state.gateway_handle.lock().await;
        if let Some(h) = gh.as_ref() {
            h.address().port()
        } else {
            *state.gateway_port.read().await
        }
    };
    let master_key = state.gateway_master_key.read().await.clone();
    let gateway_base = format!("http://127.0.0.1:{port}/v1");
    Ok(crate::agent_import::detect_all(&gateway_base, &master_key))
}

#[tauri::command]
pub async fn preview_agent_client(
    state: State<'_, DesktopState>,
    target: String,
    models: Vec<String>,
) -> Result<crate::agent_import::AgentDiffPreview, String> {
    let port = {
        let gh = state.gateway_handle.lock().await;
        if let Some(h) = gh.as_ref() {
            h.address().port()
        } else {
            *state.gateway_port.read().await
        }
    };
    let master_key = state.gateway_master_key.read().await.clone();
    let gateway_base = format!("http://127.0.0.1:{port}/v1");
    crate::agent_import::preview_target(&target, &gateway_base, &master_key, &models)
}

#[tauri::command]
pub async fn preview_agent_diff(
    state: State<'_, DesktopState>,
    target: String,
    models: Vec<String>,
) -> Result<crate::agent_import::AgentDiffPreview, String> {
    preview_agent_client(state, target, models).await
}

#[tauri::command]
pub async fn import_agent_client(
    state: State<'_, DesktopState>,
    target: String,
    models: Vec<String>,
) -> Result<crate::agent_import::AgentImportOutcome, String> {
    let port = {
        let gh = state.gateway_handle.lock().await;
        if let Some(h) = gh.as_ref() {
            h.address().port()
        } else {
            *state.gateway_port.read().await
        }
    };
    let master_key = state.gateway_master_key.read().await.clone();
    let gateway_base = format!("http://127.0.0.1:{port}/v1");
    let backup_dir = state.profile_dir.join("agent-backups");
    let outcome = crate::agent_import::import_target_in(
        &backup_dir,
        &target,
        &gateway_base,
        &master_key,
        &models,
    )?;

    state
        .record_event(
            "agent",
            "import",
            &target,
            true,
            &format!("Imported configuration to {target}"),
        )
        .await;

    Ok(outcome)
}

#[tauri::command]
pub async fn restore_agent_client(
    state: State<'_, DesktopState>,
    target: String,
    backup_id: Option<String>,
) -> Result<crate::agent_import::AgentRestoreOutcome, String> {
    let backup_dir = state.profile_dir.join("agent-backups");
    let target_backup_id = match backup_id {
        Some(ref id) if id != "latest" && !id.is_empty() => id.clone(),
        _ => {
            let backups = crate::agent_import::list_backups_in(&backup_dir, &target);
            backups
                .first()
                .map(|b| b.id.clone())
                .ok_or_else(|| format!("客户端 {target} 暂无可用历史备份"))?
        }
    };

    let restored_files =
        crate::agent_import::restore_backup_in(&backup_dir, &target, &target_backup_id)?;

    state
        .record_event(
            "agent",
            "restore",
            &target,
            true,
            &format!("Restored configuration for {target} from backup {target_backup_id}"),
        )
        .await;

    Ok(crate::agent_import::AgentRestoreOutcome {
        target,
        backup_id: target_backup_id,
        restored_files,
    })
}

#[tauri::command]
pub async fn list_agent_backups(
    state: State<'_, DesktopState>,
    target: String,
) -> Result<Vec<crate::agent_import::AgentBackupItem>, String> {
    let backup_dir = state.profile_dir.join("agent-backups");
    Ok(crate::agent_import::list_backups_in(&backup_dir, &target))
}

#[tauri::command]
pub async fn get_marketplace_catalog(
    refresh_online: Option<bool>,
) -> Result<proxyweave_plugin::marketplace::MarketplaceCatalogResponse, String> {
    let refresh = refresh_online.unwrap_or(false);
    proxyweave_plugin::marketplace::get_marketplace_catalog(refresh).await
}

#[tauri::command]
pub async fn search_marketplace_plugins(
    query: Option<String>,
    category: Option<String>,
    refresh_online: Option<bool>,
) -> Result<proxyweave_plugin::marketplace::MarketplaceCatalogResponse, String> {
    let refresh = refresh_online.unwrap_or(false);
    proxyweave_plugin::marketplace::search_marketplace(query, category, refresh).await
}

#[cfg(test)]
#[path = "host_key_tests.rs"]
mod key_tests;

pub async fn graceful_exit(state: &DesktopState, app: &AppHandle) {
    // 0. Cancel active flight requests
    state.flight().cancel_all().await;
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


// ============================================================================
// Phase 5 Workbench IPC Aliases (spec v6)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginProbeResult {
    pub id: String,
    pub status: String,
    pub port: Option<u16>,
    pub latency_ms: Option<u64>,
    pub healthy: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStatusInfo {
    pub is_running: bool,
    pub port: u16,
    pub listen_address: Option<String>,
    pub active_backend: Option<String>,
    pub route_strategy: String,
    pub master_key_preview: String,
    pub active_requests: u64,
    pub total_routed: u64,
}

#[tauri::command]
pub async fn list_plugins(
    state: State<'_, DesktopState>,
) -> Result<Vec<PluginRecord>, String> {
    let mut plugins = list_local_plugins(state.clone()).await?;
    let discovered = state.watcher.list_discovered();
    for item in discovered {
        if !plugins.iter().any(|p| p.manifest.metadata.id == item.manifest.metadata.id) {
            plugins.push(item);
        }
    }
    plugins.sort_by(|a, b| a.manifest.metadata.id.cmp(&b.manifest.metadata.id));
    Ok(plugins)
}

#[tauri::command]
pub async fn refresh_plugins(
    state: State<'_, DesktopState>,
) -> Result<Vec<PluginRecord>, String> {
    let watcher = state.watcher.clone();
    tauri::async_runtime::spawn_blocking(move || {
        watcher.scan_all();
    })
    .await
    .map_err(|_| "watcher scan panicked".to_string())?;

    state
        .record_event(
            "watcher",
            "refresh",
            "plugins",
            true,
            "Plugin directory scanned and refreshed",
        )
        .await;

    list_plugins(state).await
}

#[tauri::command]
pub async fn probe_plugin(
    state: State<'_, DesktopState>,
    id: String,
) -> Result<PluginProbeResult, String> {
    let start = std::time::Instant::now();
    let status_res = get_plugin_runtime_status(state.clone(), id.clone()).await;
    let (status, port, error, healthy) = match status_res {
        Ok(status_info) => {
            let healthy = status_info.status == "ready" || status_info.status == "busy";
            (status_info.status, status_info.port, status_info.error, healthy)
        }
        Err(e) => {
            if let Some(record) = state.watcher.get_plugin(&id) {
                if let proxyweave_plugin::Runtime::External { endpoint } = &record.manifest.runtime {
                    let health_path = &record.manifest.network.health_check.path;
                    let full_url = format!("{}{}", endpoint.trim_end_matches('/'), health_path);
                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_millis(record.manifest.network.health_check.timeout_ms))
                        .build()
                        .map_err(|err| err.to_string())?;
                    let resp = client.get(&full_url).send().await;
                    match resp {
                        Ok(res) if res.status().as_u16() == record.manifest.network.health_check.expected_status => {
                            ("ready".to_string(), None, None, true)
                        }
                        Ok(res) => {
                            (format!("http_{}", res.status().as_u16()), None, None, false)
                        }
                        Err(err) => {
                            ("unreachable".to_string(), None, Some(err.to_string()), false)
                        }
                    }
                } else {
                    ("stopped".to_string(), None, Some(e), false)
                }
            } else {
                ("stopped".to_string(), None, Some(e), false)
            }
        }
    };
    let elapsed = start.elapsed().as_millis() as u64;
    Ok(PluginProbeResult {
        id,
        status,
        port,
        latency_ms: Some(elapsed),
        healthy,
        error,
    })
}

#[tauri::command]
pub async fn get_route_status(
    state: State<'_, DesktopState>,
) -> Result<RouteStatusInfo, String> {
    let gw = get_gateway_status(state.clone()).await?;
    let active_requests = state.gateway.active_requests();
    let total_routed = state.gateway.total_routed();
    Ok(RouteStatusInfo {
        is_running: gw.is_running,
        port: gw.port,
        listen_address: gw.listen_address,
        active_backend: gw.active_backend,
        route_strategy: gw.route_strategy,
        master_key_preview: gw.master_key_preview,
        active_requests,
        total_routed,
    })
}
