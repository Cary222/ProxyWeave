// Copyright (c) 2026 wb-switch
// Copyright (c) 2026 Cary222 / WorkBuddy Switch Gateway contributors
// SPDX-License-Identifier: MIT
// Extracted from D:/workbuddy-switch-gateway/workbuddy-switch-gateway-latest/crates/wb-switch-core/src/modules/agent_import.rs
// Adapted for ProxyWeave: quotio_platform::atomic_write(..., true) with Windows Protected DACL,
// provider key "proxyweave" / "ProxyWeave", isolated agent-backups; unverified clients fail closed.

//! 一键导入与更新：把本网关接入本机已安装的 AI 客户端。
//!
//! 支持主流客户端智能体探测与安全接入：
//!
//! | # | 客户端 | 配置文件 | 协议 |
//! |---|---|---|---|
//! | 1 | Claude Code | `~/.claude/settings.json` | Anthropic Messages（协议路由已实现，调用待验证） |
//! | 2 | Claude Desktop | `%LOCALAPPDATA%\Claude-3p\configLibrary\*.json` | Anthropic Messages (3P, 不支持假写入) |
//! | 3 | Codex | `~/.codex/config.toml` + `auth.json` | OpenAI Responses |
//! | 4 | DeepSeek Harness (DSH) | `~/.dsh/settings.yaml` + `.credentials.yaml` | OpenAI Chat |
//! | 5 | OpenCode | `~/.config/opencode/opencode.json` | OpenAI Chat |
//! | 6 | Pi | `~/.pi/agent/models.json` | OpenAI Chat |
//! | 7 | Grok Build | `~/.grok/config.toml` | OpenAI Responses / Chat |
//! | 8 | ZCode | `~/.zcode/v2/config.json` | OpenAI Chat / Anthropic |
//! | 9 | Kimi Code | `~/.kimi-code/config.toml` | OpenAI Chat |
//! | 10 | OpenClaw | `~/.openclaw/openclaw.json` | OpenAI Chat |
//! | 11 | Hermes Agent | `~/.hermes/config.yaml` | OpenAI Chat |
//!
//! 安全约定：
//!   - 写入前一律备份原文件到 `<profile_dir>/agent-backups/<target>/<时间戳>/`；
//!   - 全程使用 `quotio_platform::atomic_write(..., true)` 启用 Windows Protected DACL；
//!   - 只覆盖托管字段，保留用户其余配置（尤其是 MCP / 主题 / 项目信任列表 / 插件）；
//!   - 生成 `manifest.json`，可随时一键安全回滚至修改前状态；
//!   - 支持模型多选：配置时将用户选中的所有模型同步注入客户端配置。

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use quotio_platform::home_dir;

/// 受支持的全部 11 类客户端标识（与 UI 列表顺序严格对齐）。
pub const TARGETS: [&str; 11] = [
    "claude-code",
    "claude-desktop",
    "codex",
    "dsh",
    "opencode",
    "pi",
    "grok-build",
    "zcode",
    "kimi-code",
    "openclaw",
    "hermes",
];

/// 托管配置在客户端侧使用的提供方名称。
pub const PROVIDER_NAME: &str = "ProxyWeave";
/// 客户端侧统一使用的 provider key（合法 TOML/JSON/YAML 键名）。
pub const PROVIDER_KEY: &str = "proxyweave";
/// Codex 的 provider key（必须是合法 TOML 表名）。
pub const CODEX_PROVIDER_KEY: &str = "proxyweave";
/// DSH 的 provider key。
pub const DSH_PROVIDER_KEY: &str = "proxyweave";
/// DSH 凭据引用的环境变量名。
pub const DSH_CREDENTIAL_KEY: &str = "PROXYWEAVE_API_KEY";
/// Claude Desktop 使用的 3P profile id（固定值，便于幂等覆写）。
const CLAUDE_DESKTOP_PROFILE_ID: &str = "8f2d4b6a-1c3e-4a5d-9b7f-2e6c8d0a4f13";

/// Claude Code / Claude Desktop 的模型槽位数量。
const CLAUDE_SLOT_LIMIT: usize = 4;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    #[cfg(test)]
    if tests::FAIL_WRITE_PATH.with(|fail| fail.borrow().as_deref() == Some(path)) {
        return Err("owned injected write failure".to_string());
    }
    quotio_platform::atomic_write(path, content.as_bytes(), true).map_err(|e| e.to_string())
}

fn normalize_base_origin(gateway_base: &str) -> &str {
    let trimmed = gateway_base.trim().trim_end_matches('/');
    trimmed.strip_suffix("/v1").unwrap_or(trimmed)
}

/// 默认备份根目录（桌面数据目录下的 `agent-backups`，与宿主数据隔离）。
pub fn backup_root() -> PathBuf {
    if let Some(profile) = std::env::var_os("PROXYWEAVE_PROFILE").filter(|s| !s.is_empty()) {
        return PathBuf::from(profile).join("agent-backups");
    }
    quotio_platform::app_config_dir().join("agent-backups")
}

// ---------------------------------------------------------------------------
// 路径解析
// ---------------------------------------------------------------------------

/// DSH 主目录：尊重 `DSH_HOME`，否则 `~/.dsh`。
pub fn dsh_home() -> PathBuf {
    std::env::var_os("DSH_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home_dir().join(".dsh"))
}

/// Codex 主目录：尊重 `CODEX_HOME`，否则 `~/.codex`。
pub fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home_dir().join(".codex"))
}

/// Claude Code 主目录：尊重 `CLAUDE_CONFIG_DIR`，否则 `~/.claude`。
pub fn claude_code_home() -> PathBuf {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home_dir().join(".claude"))
}

/// Windows 的 LocalAppData 目录。
#[cfg(windows)]
fn local_app_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home_dir().join("AppData").join("Local"))
}

/// 非 Windows 平台。
#[cfg(not(windows))]
fn local_app_data() -> PathBuf {
    home_dir().join(".local").join("share")
}

/// Claude Desktop 的普通配置目录（1P/3P 切换开关所在）。
pub fn claude_desktop_dir() -> PathBuf {
    pick_claude_dir(&local_app_data(), false).unwrap_or_else(|| local_app_data().join("Claude"))
}

/// Claude Desktop 的 3P 配置目录（第三方网关 profile 所在）。
pub fn claude_desktop_3p_dir() -> PathBuf {
    pick_claude_dir(&local_app_data(), true).unwrap_or_else(|| local_app_data().join("Claude-3p"))
}

/// 在 LocalAppData 下挑选 Claude 目录；容忍带版本后缀的目录名（如 `Claude-3p-1.2.3`）。
fn pick_claude_dir(root: &Path, threep: bool) -> Option<PathBuf> {
    let exact_name = if threep { "Claude-3p" } else { "Claude" };
    let exact = root.join(exact_name);
    if exact.exists() {
        return Some(exact);
    }
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                return false;
            };
            name.starts_with("Claude") && name.contains("-3p") == threep
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

/// Claude Desktop 的 configLibrary 目录。
pub fn claude_desktop_library_dir() -> PathBuf {
    claude_desktop_3p_dir().join("configLibrary")
}

/// OpenCode 配置文件路径：优先 `OPENCODE_CONFIG`，否则 `~/.config/opencode/opencode.json`。
pub fn opencode_config_path() -> PathBuf {
    std::env::var_os("OPENCODE_CONFIG")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| {
            home_dir()
                .join(".config")
                .join("opencode")
                .join("opencode.json")
        })
}

/// Pi 配置文件路径：优先 `PI_CODING_AGENT_DIR`，否则 `~/.pi/agent/models.json`。
pub fn pi_models_path() -> PathBuf {
    std::env::var_os("PI_CODING_AGENT_DIR")
        .map(|d| PathBuf::from(d).join("models.json"))
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home_dir().join(".pi").join("agent").join("models.json"))
}

/// Grok Build 目录：优先 `GROK_HOME`，否则 `~/.grok`。
pub fn grok_home() -> PathBuf {
    std::env::var_os("GROK_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home_dir().join(".grok"))
}

/// ZCode 配置文件路径：优先 `ZCODE_HOME`，否则 `~/.zcode/v2/config.json`。
pub fn zcode_config_path() -> PathBuf {
    std::env::var_os("ZCODE_HOME")
        .map(|d| PathBuf::from(d).join("v2").join("config.json"))
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home_dir().join(".zcode").join("v2").join("config.json"))
}

/// Kimi Code 目录：优先 `KIMI_CODE_HOME`，否则 `~/.kimi-code`。
pub fn kimi_code_home() -> PathBuf {
    std::env::var_os("KIMI_CODE_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home_dir().join(".kimi-code"))
}

/// OpenClaw 配置文件路径：优先官方变量 `OPENCLAW_CONFIG_PATH`（文件），
/// 其次 `OPENCLAW_STATE_DIR`（目录）与旧代码使用的 `OPENCLAW_HOME`，
/// 否则 `~/.openclaw/openclaw.json`。
pub fn openclaw_config_path() -> PathBuf {
    if let Some(p) = std::env::var_os("OPENCLAW_CONFIG_PATH")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
    {
        return p;
    }
    for var in ["OPENCLAW_STATE_DIR", "OPENCLAW_HOME"] {
        if let Some(d) = std::env::var_os(var)
            .map(PathBuf::from)
            .filter(|d| !d.as_os_str().is_empty())
        {
            return d.join("openclaw.json");
        }
    }
    home_dir().join(".openclaw").join("openclaw.json")
}

/// Hermes 配置文件路径：`$HERMES_HOME/config.yaml`。
pub fn hermes_config_path() -> PathBuf {
    std::env::var_os("HERMES_HOME")
        .filter(|d| !d.is_empty())
        .map(|d| PathBuf::from(d).join("config.yaml"))
        .unwrap_or_else(|| home_dir().join(".hermes").join("config.yaml"))
}

// ---------------------------------------------------------------------------
// DTO 与检测
// ---------------------------------------------------------------------------

/// 单个客户端的检测结果（暴露给前端 IPC）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentTargetStatus {
    pub id: String,
    pub label: String,
    pub protocol: String,
    pub supported: bool,
    /// 是否检测到已安装（配置文件或客户端目录存在）。
    pub installed: bool,
    /// 是否已接入本网关（托管字段与本机网关地址一致）。
    pub configured: bool,
    /// 主要配置文件的绝对路径。
    pub config_path: String,
    /// 面向用户的补充说明（警告 / 未安装原因 / 协议边界）。
    pub note: String,
    /// 客户端检测到的版本号（如 2.1.233, 1.17.11 等）。
    pub version: Option<String>,
}

/// 导入操作的执行结果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentImportOutcome {
    pub target: String,
    pub backup_dir: String,
    pub files: Vec<String>,
    pub models: Vec<String>,
}

/// 回滚恢复操作的执行结果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentRestoreOutcome {
    pub target: String,
    pub backup_id: String,
    pub restored_files: usize,
}

/// 单个备份条目信息。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentBackupItem {
    pub id: String,
    pub created_at: i64,
    pub path: String,
}

fn probe_cmd_version(exe: &str, args: &[&str]) -> Option<String> {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    let mut cmd = Command::new(exe);
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let out = cmd.output().ok()?;
    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }
    None
}

/// 探测全部目标客户端的安装与配置状态。
pub fn detect_all(gateway_base: &str, api_key: &str) -> Vec<AgentTargetStatus> {
    let origin = normalize_base_origin(gateway_base);
    vec![
        detect_opencode(origin, api_key),
        detect_pi(origin, api_key),
        detect_dsh(origin, api_key),
        detect_zcode(origin, api_key),
        detect_claude_code(origin, api_key),
        detect_codex(origin, api_key),
        detect_grok_build(origin, api_key),
        detect_kimi_code(origin, api_key),
        detect_openclaw(origin, api_key),
        detect_hermes(origin, api_key),
        detect_claude_desktop(origin, api_key),
    ]
}

fn detect_claude_code(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let gateway_base = normalize_base_origin(gateway_base);
    let path = claude_code_home().join("settings.json");
    let installed = claude_code_home().is_dir() || path.is_file();

    let mut configured = false;
    if let Some(v) = read_json(&path) {
        configured = !gateway_base.is_empty()
            && !api_key.is_empty()
            && env_value(&v, "ANTHROPIC_BASE_URL").as_deref() == Some(gateway_base)
            && env_value(&v, "ANTHROPIC_AUTH_TOKEN").as_deref() == Some(api_key);
    }
    let version = if installed {
        probe_cmd_version("claude.cmd", &["--version"])
            .or_else(|| probe_cmd_version("claude", &["--version"]))
    } else {
        None
    };

    AgentTargetStatus {
        id: "claude-code".to_string(),
        label: "Claude Code".to_string(),
        protocol: "Anthropic Messages".to_string(),
        supported: true,
        installed,
        configured,
        config_path: path.to_string_lossy().to_string(),
        note: if installed {
            String::new()
        } else {
            "未检测到 Claude Code（~/.claude 不存在）".to_string()
        },
        version,
    }
}

fn detect_claude_desktop(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let gateway_base = normalize_base_origin(gateway_base);
    let dir = claude_desktop_3p_dir();
    let profile = claude_desktop_library_dir().join(format!("{CLAUDE_DESKTOP_PROFILE_ID}.json"));
    let installed = claude_desktop_dir().is_dir() || dir.is_dir();

    let mut configured = false;
    if let Some(v) = read_json(&profile) {
        configured = !gateway_base.is_empty()
            && !api_key.is_empty()
            && v.get("inferenceGatewayBaseUrl").and_then(Value::as_str) == Some(gateway_base)
            && v.get("inferenceGatewayApiKey").and_then(Value::as_str) == Some(api_key);
    }

    AgentTargetStatus {
        id: "claude-desktop".to_string(),
        label: "Claude Desktop".to_string(),
        protocol: "Anthropic Messages".to_string(),
        supported: false,
        installed,
        configured,
        config_path: profile.to_string_lossy().to_string(),
        note: if installed {
            String::new()
        } else {
            "未检测到 Claude Desktop".to_string()
        },
        version: if installed {
            Some("Claude Desktop".to_string())
        } else {
            None
        },
    }
}

fn detect_codex(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let config = codex_home().join("config.toml");
    let auth = codex_home().join("auth.json");
    let installed = codex_home().is_dir() || config.is_file();

    let mut configured = false;
    if let Ok(text) = std::fs::read_to_string(&config) {
        let base_ok = !gateway_base.is_empty() && text.contains(gateway_base);
        let key_ok = read_json(&auth)
            .and_then(|v| {
                v.get("OPENAI_API_KEY")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .map(|k| !api_key.is_empty() && k == api_key)
            .unwrap_or(false);
        configured = base_ok && key_ok;
    }

    let version = if installed {
        probe_cmd_version("codex.cmd", &["--version"])
            .or_else(|| probe_cmd_version("codex", &["--version"]))
    } else {
        None
    };

    AgentTargetStatus {
        id: "codex".to_string(),
        label: "Codex".to_string(),
        protocol: "OpenAI Responses".to_string(),
        supported: true,
        installed,
        configured,
        config_path: config.to_string_lossy().to_string(),
        note: if installed {
            String::new()
        } else {
            "未检测到 Codex（~/.codex 不存在）".to_string()
        },
        version,
    }
}

fn detect_dsh(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let settings = dsh_home().join("settings.yaml");
    let credentials = dsh_home().join(".credentials.yaml");
    let installed = dsh_home().is_dir() || settings.is_file();

    let mut configured = false;
    let mut note = String::new();
    if let Ok(text) = std::fs::read_to_string(&settings) {
        let base_ok = !gateway_base.is_empty()
            && text.contains(&format!("{DSH_PROVIDER_KEY}:"))
            && text.contains(gateway_base);
        let key_ok = std::fs::read_to_string(&credentials)
            .map(|c| (!api_key.is_empty() && c.contains(api_key)) || c.contains(DSH_CREDENTIAL_KEY))
            .unwrap_or(false);
        configured = base_ok && key_ok;
    }
    if !installed {
        note = "未检测到 DSH（~/.dsh 不存在）".to_string();
    } else if !settings.is_file() {
        note = "检测到 DSH 目录，但尚无 settings.yaml（导入时会自动创建）".to_string();
    }

    AgentTargetStatus {
        id: "dsh".to_string(),
        label: "DeepSeek Harness".to_string(),
        protocol: "OpenAI Chat".to_string(),
        supported: true,
        installed,
        configured,
        config_path: settings.to_string_lossy().to_string(),
        note,
        version: if installed {
            Some("DeepSeek Harness".to_string())
        } else {
            None
        },
    }
}

fn detect_opencode(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let path = opencode_config_path();
    let app_dir = home_dir().join(".config").join("opencode");
    let dot_dir = home_dir().join(".opencode");
    let desktop_exe = local_app_data()
        .join("Programs")
        .join("@opencode-aidesktop")
        .join("OpenCode.exe");
    let installed = path.is_file() || app_dir.is_dir() || dot_dir.is_dir() || desktop_exe.is_file();

    let mut configured = false;
    if let Some(v) = read_json(&path) {
        if let Some(provider) = v.get("provider").and_then(Value::as_object) {
            if let Some(pw) = provider.get(PROVIDER_KEY).and_then(Value::as_object) {
                let opt_base = pw
                    .get("options")
                    .and_then(|o| o.get("baseURL"))
                    .and_then(Value::as_str);
                let opt_key = pw
                    .get("options")
                    .and_then(|o| o.get("apiKey"))
                    .and_then(Value::as_str);
                configured = !gateway_base.is_empty()
                    && opt_base.map(|b| b.contains(gateway_base)).unwrap_or(false)
                    && (api_key.is_empty() || opt_key == Some(api_key));
            }
        }
    }

    let version = if installed {
        probe_cmd_version("opencode.cmd", &["-v"])
            .or_else(|| probe_cmd_version("opencode", &["-v"]))
    } else {
        None
    };

    AgentTargetStatus {
        id: "opencode".to_string(),
        label: "OpenCode".to_string(),
        protocol: "OpenAI Chat".to_string(),
        supported: true,
        installed,
        configured,
        config_path: path.to_string_lossy().to_string(),
        note: if installed {
            String::new()
        } else {
            "未检测到 OpenCode（~/.config/opencode 不存在）".to_string()
        },
        version,
    }
}

fn detect_pi(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let path = pi_models_path();
    let agent_dir = home_dir().join(".pi").join("agent");
    let dot_dir = home_dir().join(".pi");
    let installed = path.is_file() || agent_dir.is_dir() || dot_dir.is_dir();

    let mut configured = false;
    if let Some(v) = read_json(&path) {
        if let Some(providers) = v.get("providers").and_then(Value::as_object) {
            if let Some(pw) = providers.get(PROVIDER_KEY).and_then(Value::as_object) {
                let b = pw.get("baseUrl").and_then(Value::as_str);
                let k = pw.get("apiKey").and_then(Value::as_str);
                configured = !gateway_base.is_empty()
                    && b.map(|s| s.contains(gateway_base)).unwrap_or(false)
                    && (api_key.is_empty() || k == Some(api_key));
            }
        }
    }

    AgentTargetStatus {
        id: "pi".to_string(),
        label: "Pi".to_string(),
        protocol: "OpenAI Chat".to_string(),
        supported: true,
        installed,
        configured,
        config_path: path.to_string_lossy().to_string(),
        note: if installed {
            String::new()
        } else {
            "未检测到 Pi（~/.pi 不存在）".to_string()
        },
        version: if installed {
            Some("Pi".to_string())
        } else {
            None
        },
    }
}

fn detect_grok_build(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let home = grok_home();
    let config = home.join("config.toml");
    let bin = home.join("bin").join("grok.exe");
    let installed = home.is_dir() || config.is_file() || bin.is_file();

    let mut configured = false;
    if let Ok(text) = std::fs::read_to_string(&config) {
        configured = !gateway_base.is_empty()
            && text.contains(gateway_base)
            && (api_key.is_empty() || text.contains(api_key));
    }

    let version = read_json(&grok_home().join("version.json")).and_then(|v| {
        v.get("version")
            .and_then(Value::as_str)
            .map(|s| format!("grok {s}"))
    });

    AgentTargetStatus {
        id: "grok-build".to_string(),
        label: "Grok Build".to_string(),
        protocol: "OpenAI Responses / Chat".to_string(),
        supported: true,
        installed,
        configured,
        config_path: config.to_string_lossy().to_string(),
        note: if installed {
            String::new()
        } else {
            "未检测到 Grok Build（~/.grok 不存在）".to_string()
        },
        version,
    }
}

fn detect_zcode(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let path = zcode_config_path();
    let home = home_dir().join(".zcode");
    let app_exe = local_app_data()
        .join("Programs")
        .join("ZCode")
        .join("ZCode.exe");
    let installed = path.is_file() || home.is_dir() || app_exe.is_file();

    let mut configured = false;
    if let Some(v) = read_json(&path) {
        if let Some(provider) = v.get("provider").and_then(Value::as_object) {
            if let Some(pw) = provider.get(PROVIDER_KEY).and_then(Value::as_object) {
                let opt_base = pw
                    .get("options")
                    .and_then(|o| o.get("baseURL"))
                    .and_then(Value::as_str);
                let opt_key = pw
                    .get("options")
                    .and_then(|o| o.get("apiKey"))
                    .and_then(Value::as_str);
                configured = !gateway_base.is_empty()
                    && opt_base.map(|b| b.contains(gateway_base)).unwrap_or(false)
                    && (api_key.is_empty() || opt_key == Some(api_key));
            }
        }
    }

    AgentTargetStatus {
        id: "zcode".to_string(),
        label: "ZCode".to_string(),
        protocol: "OpenAI Chat".to_string(),
        supported: true,
        installed,
        configured,
        config_path: path.to_string_lossy().to_string(),
        note: if installed {
            String::new()
        } else {
            "未检测到 ZCode（~/.zcode 不存在）".to_string()
        },
        version: if installed {
            Some("ZCode".to_string())
        } else {
            None
        },
    }
}

fn detect_kimi_code(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let home = kimi_code_home();
    let config = home.join("config.toml");
    let bin = home.join("bin").join("kimi.exe");
    let installed = home.is_dir() || config.is_file() || bin.is_file();

    let mut configured = false;
    if let Ok(text) = std::fs::read_to_string(&config) {
        configured = !gateway_base.is_empty()
            && text.contains(gateway_base)
            && (api_key.is_empty() || text.contains(api_key));
    }

    let version = if installed {
        probe_cmd_version(&bin.to_string_lossy(), &["--version"])
    } else {
        None
    };

    AgentTargetStatus {
        id: "kimi-code".to_string(),
        label: "Kimi Code".to_string(),
        protocol: "OpenAI Chat".to_string(),
        supported: true,
        installed,
        configured,
        config_path: config.to_string_lossy().to_string(),
        note: if installed {
            String::new()
        } else {
            "未检测到 Kimi Code（~/.kimi-code 不存在）".to_string()
        },
        version,
    }
}

fn detect_openclaw(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let path = openclaw_config_path();
    let home = home_dir().join(".openclaw");
    let installed = path.is_file() || home.is_dir();

    let mut configured = false;
    if let Ok(text) = std::fs::read_to_string(&path) {
        configured = !gateway_base.is_empty()
            && text.contains(PROVIDER_KEY)
            && text.contains(gateway_base)
            && (api_key.is_empty() || text.contains(api_key));
    }

    AgentTargetStatus {
        id: "openclaw".to_string(),
        label: "OpenClaw".to_string(),
        protocol: "OpenAI Chat".to_string(),
        supported: true,
        installed,
        configured,
        config_path: path.to_string_lossy().to_string(),
        note: if installed {
            String::new()
        } else {
            "未检测到 OpenClaw（~/.openclaw 不存在）".to_string()
        },
        version: if installed {
            Some("OpenClaw".to_string())
        } else {
            None
        },
    }
}

fn detect_hermes(gateway_base: &str, api_key: &str) -> AgentTargetStatus {
    let path = hermes_config_path();
    let home = home_dir().join(".hermes");
    let installed = path.is_file() || home.is_dir();

    let mut configured = false;
    if let Ok(text) = std::fs::read_to_string(&path) {
        configured = !gateway_base.is_empty()
            && text.contains(PROVIDER_KEY)
            && text.contains(gateway_base)
            && (api_key.is_empty() || text.contains(api_key));
    }

    AgentTargetStatus {
        id: "hermes".to_string(),
        label: "Hermes Agent".to_string(),
        protocol: "OpenAI Chat".to_string(),
        supported: true,
        installed,
        configured,
        config_path: path.to_string_lossy().to_string(),
        note: if installed {
            String::new()
        } else {
            "未检测到 Hermes Agent（~/.hermes 不存在）".to_string()
        },
        version: if installed {
            Some("Hermes Agent".to_string())
        } else {
            None
        },
    }
}

// ---------------------------------------------------------------------------
// 备份与恢复
// ---------------------------------------------------------------------------

/// 备份一组文件到 `<backup_root>/<target>/<ts>/`，返回备份目录。
pub fn backup_files(target: &str, paths: &[PathBuf]) -> Result<PathBuf, String> {
    backup_files_in(&backup_root(), target, paths)
}

/// 在指定备份根目录下备份一组文件。
pub fn backup_files_in(
    root_dir: &Path,
    target: &str,
    paths: &[PathBuf],
) -> Result<PathBuf, String> {
    let stamp = now_ms().to_string();
    let dir = root_dir.join(target).join(&stamp);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建备份目录失败: {e}"))?;

    let mut manifest = Map::new();
    for (index, path) in paths.iter().enumerate() {
        if !path.is_file() {
            continue;
        }
        let name = format!(
            "{index}-{}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取待备份文件失败 {}: {e}", path.display()))?;
        atomic_write(&dir.join(&name), &content)
            .map_err(|e| format!("写入备份失败 {}: {e}", path.display()))?;
        manifest.insert(name, json!(path.to_string_lossy()));
    }
    manifest.insert("target".to_string(), json!(target));
    manifest.insert("createdAt".to_string(), json!(now_ms()));

    let text = serde_json::to_string_pretty(&Value::Object(manifest)).map_err(|e| e.to_string())?;
    atomic_write(&dir.join("manifest.json"), &text).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// 列出某个客户端的历史备份（按时间倒序），供 UI 展示与恢复。
pub fn list_backups(target: &str) -> Vec<AgentBackupItem> {
    list_backups_in(&backup_root(), target)
}

/// 在指定备份根目录下查看历史备份列表。
pub fn list_backups_in(root_dir: &Path, target: &str) -> Vec<AgentBackupItem> {
    let root = root_dir.join(target);
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        let created = id.parse::<i64>().unwrap_or(0);
        out.push(AgentBackupItem {
            id,
            created_at: created,
            path: path.to_string_lossy().to_string(),
        });
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.created_at));
    out
}

/// 从指定备份恢复文件。
pub fn restore_backup(target: &str, backup_id: &str) -> Result<usize, String> {
    restore_backup_in(&backup_root(), target, backup_id)
}

/// 从指定备份根目录恢复文件。
pub fn restore_backup_in(root_dir: &Path, target: &str, backup_id: &str) -> Result<usize, String> {
    let dir = root_dir.join(target).join(backup_id);
    let manifest_path = dir.join("manifest.json");
    let text =
        std::fs::read_to_string(&manifest_path).map_err(|e| format!("读取备份清单失败: {e}"))?;
    let manifest: Value = serde_json::from_str(&text).map_err(|e| format!("备份清单损坏: {e}"))?;
    let obj = manifest
        .as_object()
        .ok_or_else(|| "备份清单格式错误".to_string())?;

    let mut restored = 0usize;
    for (name, original) in obj {
        if name == "target" || name == "createdAt" {
            continue;
        }
        let Some(target_path_str) = original.as_str() else {
            continue;
        };
        let backup_file = dir.join(name);
        if !backup_file.is_file() {
            continue;
        }
        let content =
            std::fs::read_to_string(&backup_file).map_err(|e| format!("读取备份文件失败: {e}"))?;
        let target_path = PathBuf::from(target_path_str);
        if let Some(parent) = target_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        atomic_write(&target_path, &content)
            .map_err(|e| format!("恢复文件失败 {target_path_str}: {e}"))?;
        restored += 1;
    }
    Ok(restored)
}

// ---------------------------------------------------------------------------
// 导入与批量更新分发
// ---------------------------------------------------------------------------

/// 把网关配置接入指定目标客户端（支持多模型）。
pub fn import_target(
    target: &str,
    gateway_base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    import_target_in(&backup_root(), target, gateway_base, api_key, models)
}

/// 在指定备份根目录下把网关配置接入指定目标客户端。
pub fn import_target_in(
    root_dir: &Path,
    target: &str,
    gateway_base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    if api_key.trim().is_empty() {
        return Err("请先确保统一网关 Master Key 可用（客户端必须携带凭据）".to_string());
    }
    let base = normalize_base_origin(gateway_base);
    if base.is_empty() {
        return Err("网关地址无效，无法生成客户端配置".to_string());
    }
    if models.is_empty() || models.iter().any(|model| model.trim().is_empty()) {
        return Err("请选择网关实际可用模型，禁止空模型或默认模型写入".to_string());
    }
    let safe_models = models.to_vec();

    match target {
        "claude-code" => import_claude_code(root_dir, base, api_key, &safe_models),
        "claude-desktop" => import_claude_desktop(root_dir, base, api_key, &safe_models),
        "codex" => import_codex(root_dir, base, api_key, &safe_models),
        "dsh" => import_dsh(root_dir, base, api_key, &safe_models),
        "opencode" => import_opencode(root_dir, base, api_key, &safe_models),
        "pi" => import_pi(root_dir, base, api_key, &safe_models),
        "grok-build" => import_grok_build(root_dir, base, api_key, &safe_models),
        "zcode" => import_zcode(root_dir, base, api_key, &safe_models),
        "kimi-code" => import_kimi_code(root_dir, base, api_key, &safe_models),
        "openclaw" => import_openclaw(root_dir, base, api_key, &safe_models),
        "hermes" => import_hermes(root_dir, base, api_key, &safe_models),
        other => Err(format!("不支持的客户端: {other}")),
    }
}

// Never treat an unreadable existing config as a missing file.
fn read_claude_existing(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("读取 Claude 配置失败，未写入".to_string()),
    }
}
fn import_claude_code(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let path = claude_code_home().join("settings.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Claude 配置目录失败: {e}"))?;
    }

    // Claude Code 只有 Sonnet/Opus/Haiku/Fable 四个槽位，多余模型无法承载，
    // 明确截断并如实回传实际写入的模型列表（避免 UI 报告未生效的数量）。
    let slot_models: Vec<String> = models.iter().take(CLAUDE_SLOT_LIMIT).cloned().collect();

    let existing = read_claude_existing(&path)?;
    let text = build_claude_code_settings(existing.as_deref(), base, api_key, &slot_models)
        .map_err(|_| "Claude Code 配置无效，未写入".to_string())?;
    let backup = backup_files_in(root_dir, "claude-code", std::slice::from_ref(&path))?;
    atomic_write(&path, &text)
        .map_err(|_| "写入 Claude Code 配置失败；请检查宿主备份历史".to_string())?;

    Ok(AgentImportOutcome {
        target: "claude-code".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![path.to_string_lossy().to_string()],
        models: slot_models,
    })
}

#[allow(dead_code)] // Public dispatch stays blocked until native model mapping is verified.
fn import_claude_desktop(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let dir = claude_desktop_3p_dir();
    let library = dir.join("configLibrary");
    std::fs::create_dir_all(&library).map_err(|e| format!("创建 Claude 3P 目录失败: {e}"))?;

    // 与 Claude Code 相同：profile 只有四个槽位，超出部分截断并如实回传。
    let slot_models: Vec<String> = models.iter().take(CLAUDE_SLOT_LIMIT).cloned().collect();

    let normal_config = claude_desktop_dir().join("claude_desktop_config.json");
    let threep_config = dir.join("claude_desktop_config.json");
    let profile = library.join(format!("{CLAUDE_DESKTOP_PROFILE_ID}.json"));
    let meta = library.join("_meta.json");

    // Render every file before any write; normal and 3P configurations are independent.
    let prof_existing = read_claude_existing(&profile)?;
    let prof_text =
        build_claude_desktop_profile(prof_existing.as_deref(), base, api_key, &slot_models)
            .map_err(|_| "Claude Desktop profile 配置无效，未写入".to_string())?;
    let meta_existing = read_claude_existing(&meta)?;
    let meta_text = build_claude_desktop_meta(meta_existing.as_deref())
        .map_err(|_| "Claude Desktop 索引配置无效，未写入".to_string())?;
    let mode_normal = read_claude_existing(&normal_config)?;
    let mode_text = build_claude_desktop_mode(mode_normal.as_deref())
        .map_err(|_| "Claude Desktop 普通配置无效，未写入".to_string())?;
    let mode_threep = read_claude_existing(&threep_config)?;
    let threep_text = build_claude_desktop_mode(mode_threep.as_deref())
        .map_err(|_| "Claude Desktop 3P 配置无效，未写入".to_string())?;

    let backup = backup_files_in(
        root_dir,
        "claude-desktop",
        &[
            normal_config.clone(),
            threep_config.clone(),
            profile.clone(),
            meta.clone(),
        ],
    )?;

    if let Some(parent) = normal_config.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|_| "创建 Claude Desktop 普通配置目录失败，未写入".to_string())?;
    }
    for (index, (path, text)) in [
        (&profile, &prof_text),
        (&meta, &meta_text),
        (&normal_config, &mode_text),
        (&threep_config, &threep_text),
    ]
    .iter()
    .enumerate()
    {
        atomic_write(path, text).map_err(|_| {
            format!(
            "Claude Desktop 配置写入失败：已写入 {index}/4 个文件；请从宿主备份历史恢复原有文件。"
        )
        })?;
    }

    Ok(AgentImportOutcome {
        target: "claude-desktop".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![
            profile.to_string_lossy().to_string(),
            meta.to_string_lossy().to_string(),
            normal_config.to_string_lossy().to_string(),
            threep_config.to_string_lossy().to_string(),
        ],
        models: slot_models,
    })
}
fn import_codex(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let home = codex_home();
    std::fs::create_dir_all(&home).map_err(|e| format!("创建 Codex 目录失败: {e}"))?;
    let config_path = home.join("config.toml");
    let auth_path = home.join("auth.json");

    let backup = backup_files_in(root_dir, "codex", &[config_path.clone(), auth_path.clone()])?;

    let existing_cfg = std::fs::read_to_string(&config_path).ok();
    let cfg_text = build_codex_config(existing_cfg.as_deref(), base, models)?;
    atomic_write(&config_path, &cfg_text).map_err(|e| format!("写入 Codex config 失败: {e}"))?;

    let existing_auth = std::fs::read_to_string(&auth_path).ok();
    let auth_text = build_codex_auth(existing_auth.as_deref(), api_key)?;
    atomic_write(&auth_path, &auth_text).map_err(|e| format!("写入 Codex auth 失败: {e}"))?;

    Ok(AgentImportOutcome {
        target: "codex".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![
            config_path.to_string_lossy().to_string(),
            auth_path.to_string_lossy().to_string(),
        ],
        models: models.to_vec(),
    })
}

fn import_dsh(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let home = dsh_home();
    std::fs::create_dir_all(&home).map_err(|e| format!("创建 DSH 目录失败: {e}"))?;
    let settings_path = home.join("settings.yaml");
    let credentials_path = home.join(".credentials.yaml");

    let backup = backup_files_in(
        root_dir,
        "dsh",
        &[settings_path.clone(), credentials_path.clone()],
    )?;

    let existing = std::fs::read_to_string(&settings_path).ok();
    let settings = build_dsh_settings(existing.as_deref(), base, models)?;
    atomic_write(&settings_path, &settings).map_err(|e| format!("写入 DSH settings 失败: {e}"))?;

    let creds_existing = std::fs::read_to_string(&credentials_path).ok();
    let creds = build_dsh_credentials(creds_existing.as_deref(), api_key)?;
    atomic_write(&credentials_path, &creds).map_err(|e| format!("写入 DSH 凭据失败: {e}"))?;

    Ok(AgentImportOutcome {
        target: "dsh".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![
            settings_path.to_string_lossy().to_string(),
            credentials_path.to_string_lossy().to_string(),
        ],
        models: models.to_vec(),
    })
}

fn import_opencode(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let path = opencode_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 OpenCode 目录失败: {e}"))?;
    }
    let backup = backup_files_in(root_dir, "opencode", std::slice::from_ref(&path))?;
    let existing = std::fs::read_to_string(&path).ok();
    let text = build_opencode_config(existing.as_deref(), base, api_key, models)?;
    atomic_write(&path, &text).map_err(|e| format!("写入 OpenCode 配置失败: {e}"))?;

    Ok(AgentImportOutcome {
        target: "opencode".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![path.to_string_lossy().to_string()],
        models: models.to_vec(),
    })
}

fn import_pi(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let path = pi_models_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Pi 目录失败: {e}"))?;
    }
    let backup = backup_files_in(root_dir, "pi", std::slice::from_ref(&path))?;
    let existing = std::fs::read_to_string(&path).ok();
    let text = build_pi_models(existing.as_deref(), base, api_key, models)?;
    atomic_write(&path, &text).map_err(|e| format!("写入 Pi 配置失败: {e}"))?;

    Ok(AgentImportOutcome {
        target: "pi".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![path.to_string_lossy().to_string()],
        models: models.to_vec(),
    })
}

fn import_grok_build(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let home = grok_home();
    std::fs::create_dir_all(&home).map_err(|e| format!("创建 Grok 目录失败: {e}"))?;
    let config_path = home.join("config.toml");
    let backup = backup_files_in(root_dir, "grok-build", std::slice::from_ref(&config_path))?;
    let existing = std::fs::read_to_string(&config_path).ok();
    let text = build_grok_config(existing.as_deref(), base, api_key, models)?;
    atomic_write(&config_path, &text).map_err(|e| format!("写入 Grok Build 配置失败: {e}"))?;

    Ok(AgentImportOutcome {
        target: "grok-build".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![config_path.to_string_lossy().to_string()],
        models: models.to_vec(),
    })
}

fn import_zcode(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let path = zcode_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 ZCode 目录失败: {e}"))?;
    }
    let backup = backup_files_in(root_dir, "zcode", std::slice::from_ref(&path))?;
    let existing = std::fs::read_to_string(&path).ok();
    let text = build_zcode_config(existing.as_deref(), base, api_key, models)?;
    atomic_write(&path, &text).map_err(|e| format!("写入 ZCode 配置失败: {e}"))?;

    Ok(AgentImportOutcome {
        target: "zcode".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![path.to_string_lossy().to_string()],
        models: models.to_vec(),
    })
}

fn import_kimi_code(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let home = kimi_code_home();
    std::fs::create_dir_all(&home).map_err(|e| format!("创建 Kimi Code 目录失败: {e}"))?;
    let config_path = home.join("config.toml");
    let backup = backup_files_in(root_dir, "kimi-code", std::slice::from_ref(&config_path))?;
    let existing = std::fs::read_to_string(&config_path).ok();
    let text = build_kimi_code_config(existing.as_deref(), base, api_key, models)?;
    atomic_write(&config_path, &text).map_err(|e| format!("写入 Kimi Code 配置失败: {e}"))?;

    Ok(AgentImportOutcome {
        target: "kimi-code".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![config_path.to_string_lossy().to_string()],
        models: models.to_vec(),
    })
}

fn import_openclaw(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let path = openclaw_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 OpenClaw 目录失败: {e}"))?;
    }
    let backup = backup_files_in(root_dir, "openclaw", std::slice::from_ref(&path))?;
    let existing = std::fs::read_to_string(&path).ok();
    let text = build_openclaw_config(existing.as_deref(), base, api_key, models)?;
    atomic_write(&path, &text).map_err(|e| format!("写入 OpenClaw 配置失败: {e}"))?;

    Ok(AgentImportOutcome {
        target: "openclaw".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![path.to_string_lossy().to_string()],
        models: models.to_vec(),
    })
}

fn import_hermes(
    root_dir: &Path,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<AgentImportOutcome, String> {
    let path = hermes_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Hermes 目录失败: {e}"))?;
    }
    let backup = backup_files_in(root_dir, "hermes", std::slice::from_ref(&path))?;
    let existing = std::fs::read_to_string(&path).ok();
    let text = build_hermes_config(existing.as_deref(), base, api_key, models)?;
    atomic_write(&path, &text).map_err(|e| format!("写入 Hermes 配置失败: {e}"))?;

    Ok(AgentImportOutcome {
        target: "hermes".to_string(),
        backup_dir: backup.to_string_lossy().to_string(),
        files: vec![path.to_string_lossy().to_string()],
        models: models.to_vec(),
    })
}

// ---------------------------------------------------------------------------
// 各客户端配置生成器（支持多模型注入）
// ---------------------------------------------------------------------------

/// 生成 DSH settings.yaml：在 llm-pi-ai 插件下注册 proxyweave 路由，并把默认模型指向它。
pub fn build_dsh_settings(
    existing: Option<&str>,
    base: &str,
    models: &[String],
) -> Result<String, String> {
    let base = normalize_base_origin(base);
    let primary = models
        .first()
        .map(String::as_str)
        .unwrap_or("deepseek-v4-flash");
    let mut root = parse_yaml_map(existing)?;

    let llm = ensure_yaml_map(&mut root, "llm-pi-ai");
    let providers = ensure_yaml_map(llm, "providers");
    let models_val: Vec<Value> = models
        .iter()
        .map(|m| {
            json!({
                "id": m,
                "name": m,
                "input": ["text", "image"],
                "reasoningEfforts": { "off": null, "high": "high", "max": "max" }
            })
        })
        .collect();

    providers.insert(
        DSH_PROVIDER_KEY.to_string(),
        json!({
            "api": "openai-completions",
            "baseURL": format!("{base}/v1"),
            "apiKeyEnv": DSH_CREDENTIAL_KEY,
            "models": models_val
        }),
    );

    let default_model = ensure_yaml_map(&mut root, "agent-default-model");
    default_model.insert("provider".to_string(), json!(DSH_PROVIDER_KEY));
    default_model.insert("model".to_string(), json!(primary));

    render_yaml(Value::Object(root))
}

/// 生成 DSH .credentials.yaml：写入 API Key 引用。
pub fn build_dsh_credentials(existing: Option<&str>, api_key: &str) -> Result<String, String> {
    let mut root = parse_yaml_map(existing)?;
    if !root.contains_key("version") {
        root.insert("version".to_string(), json!(1));
    }
    let refs = ensure_yaml_map(&mut root, "refs");
    refs.insert(DSH_CREDENTIAL_KEY.to_string(), json!(api_key));
    render_yaml(Value::Object(root))
}

/// 生成 Claude Code settings.json（直接保留上游生成器，拒绝空模型）。
pub fn build_claude_code_settings(
    existing: Option<&str>,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<String, String> {
    let primary = models
        .first()
        .map(String::as_str)
        .filter(|model| !model.trim().is_empty())
        .ok_or_else(|| "Claude Code 必须选择实际模型".to_string())?;
    let opus = models.get(1).map(String::as_str).unwrap_or(primary);
    let haiku = models.get(2).map(String::as_str).unwrap_or(primary);
    let fable = models.get(3).map(String::as_str).unwrap_or(primary);

    let mut root = parse_json_object(existing)?;
    if !root.get("env").map(Value::is_object).unwrap_or(false) {
        root.insert("env".into(), json!({}));
    }
    let env = root
        .get_mut("env")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "Claude Code 配置的 env 字段必须是对象".to_string())?;

    env.insert("ANTHROPIC_BASE_URL".into(), json!(base));
    env.insert("ANTHROPIC_AUTH_TOKEN".into(), json!(api_key));

    let slots = [
        ("SONNET", primary),
        ("OPUS", opus),
        ("HAIKU", haiku),
        ("FABLE", fable),
    ];
    for (slot, model) in slots {
        env.insert(format!("ANTHROPIC_DEFAULT_{slot}_MODEL"), json!(model));
        env.insert(format!("ANTHROPIC_DEFAULT_{slot}_MODEL_NAME"), json!(model));
    }

    render_json(Value::Object(root))
}

/// 保留上游 Claude Desktop 3P profile 生成器；固定 name 与 labelOverride 映射仍待原生验证。
pub fn build_claude_desktop_profile(
    existing: Option<&str>,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<String, String> {
    let sonnet = models
        .first()
        .map(String::as_str)
        .filter(|model| !model.trim().is_empty())
        .ok_or_else(|| "Claude Desktop 必须选择实际模型".to_string())?;
    let opus = models.get(1).map(String::as_str).unwrap_or(sonnet);
    let haiku = models.get(2).map(String::as_str).unwrap_or(sonnet);
    let fable = models.get(3).map(String::as_str).unwrap_or(sonnet);

    let list = vec![
        json!({
            "name": "claude-sonnet-5",
            "labelOverride": sonnet,
            "supports1m": true
        }),
        json!({
            "name": "claude-opus-5",
            "labelOverride": opus,
            "supports1m": true
        }),
        json!({
            "name": "claude-haiku-4-5",
            "labelOverride": haiku,
            "supports1m": true
        }),
        json!({
            "name": "claude-fable-5",
            "labelOverride": fable,
            "supports1m": true
        }),
    ];

    let mut root = parse_json_object(existing)?;
    root.insert("coworkEgressAllowedHosts".into(), json!(["*"]));
    root.insert("disableDeploymentModeChooser".into(), json!(true));
    root.insert("inferenceGatewayApiKey".into(), json!(api_key));
    root.insert("inferenceGatewayAuthScheme".into(), json!("bearer"));
    root.insert("inferenceGatewayBaseUrl".into(), json!(base));
    root.insert("inferenceProvider".into(), json!("gateway"));
    root.insert("inferenceModels".into(), Value::Array(list));
    render_json(Value::Object(root))
}

/// 生成 Claude Desktop 的 1P/3P 模式开关文件。
pub fn build_claude_desktop_mode(existing: Option<&str>) -> Result<String, String> {
    let mut root = parse_json_object(existing)?;
    root.insert("deploymentMode".into(), json!("3p"));
    render_json(Value::Object(root))
}

/// 生成 Claude Desktop 的 _meta.json 索引文件。
pub fn build_claude_desktop_meta(existing: Option<&str>) -> Result<String, String> {
    let mut root = parse_json_object(existing)?;
    let mut retained: Vec<Value> = root
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter(|e| e.get("id").and_then(Value::as_str) != Some(CLAUDE_DESKTOP_PROFILE_ID))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    retained.push(json!({ "id": CLAUDE_DESKTOP_PROFILE_ID, "name": PROVIDER_NAME }));
    root.insert("entries".into(), Value::Array(retained));
    root.insert("appliedId".into(), json!(CLAUDE_DESKTOP_PROFILE_ID));
    render_json(Value::Object(root))
}

/// 生成 Codex config.toml：覆盖 provider 表与顶层 model。
pub fn build_codex_config(
    existing: Option<&str>,
    base: &str,
    models: &[String],
) -> Result<String, String> {
    let base = normalize_base_origin(base);
    let primary = models
        .first()
        .map(String::as_str)
        .unwrap_or("deepseek-v4-flash");
    let mut text = existing.unwrap_or("").to_string();

    text = set_toml_top_level(
        &text,
        "model_provider",
        &format!("\"{CODEX_PROVIDER_KEY}\""),
    );
    text = set_toml_top_level(&text, "model", &format!("\"{primary}\""));
    text = set_toml_top_level(&text, "model_reasoning_effort", "\"high\"");
    text = set_toml_top_level(&text, "disable_response_storage", "true");

    let table = format!("[model_providers.{CODEX_PROVIDER_KEY}]");
    let body = format!(
        "{table}\nname = \"{PROVIDER_NAME}\"\nbase_url = \"{base}/v1\"\nwire_api = \"responses\"\nrequires_openai_auth = true\n"
    );
    text = replace_toml_table(&text, &table, &body);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// 生成 Codex auth.json：写入 OpenAI 形态的 API Key。
pub fn build_codex_auth(existing: Option<&str>, api_key: &str) -> Result<String, String> {
    let mut root = parse_json_object(existing)?;
    root.insert("OPENAI_API_KEY".into(), json!(api_key));
    root.remove("tokens");
    root.remove("last_refresh");
    root.insert("auth_mode".into(), json!("apikey"));
    render_json(Value::Object(root))
}

/// 生成 OpenCode 配置：写入 provider.proxyweave 及多个选定模型。
pub fn build_opencode_config(
    existing: Option<&str>,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<String, String> {
    let base = normalize_base_origin(base);
    let mut root = parse_json_object(existing)?;
    if !root.contains_key("$schema") {
        root.insert("$schema".into(), json!("https://opencode.ai/config.json"));
    }
    if !root.get("provider").map(Value::is_object).unwrap_or(false) {
        root.insert("provider".into(), json!({}));
    }
    let provider = root
        .get_mut("provider")
        .and_then(Value::as_object_mut)
        .unwrap();

    let mut models_map = Map::new();
    for m in models {
        models_map.insert(m.clone(), json!({ "name": m }));
    }

    provider.insert(
        PROVIDER_KEY.into(),
        json!({
            "name": PROVIDER_NAME,
            "npm": "@ai-sdk/openai-compatible",
            "options": {
                "baseURL": format!("{base}/v1"),
                "apiKey": api_key,
                "setCacheKey": true
            },
            "models": models_map
        }),
    );
    render_json(Value::Object(root))
}

/// 生成 Pi 配置：写入 providers.proxyweave 及其支持的模型列表。
pub fn build_pi_models(
    existing: Option<&str>,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<String, String> {
    let base = normalize_base_origin(base);
    let mut root = parse_json_object(existing)?;
    if !root.get("providers").map(Value::is_object).unwrap_or(false) {
        root.insert("providers".into(), json!({}));
    }
    let providers = root
        .get_mut("providers")
        .and_then(Value::as_object_mut)
        .unwrap();
    let models_list: Vec<Value> = models
        .iter()
        .map(|m| json!({ "id": m, "reasoning": true }))
        .collect();

    providers.insert(
        PROVIDER_KEY.into(),
        json!({
            "api": "openai-completions",
            "baseUrl": format!("{base}/v1"),
            "apiKey": api_key,
            "models": models_list
        }),
    );
    render_json(Value::Object(root))
}

/// 生成 Grok Build config.toml：写入 default 模型及为每个选中模型生成配置表。
pub fn build_grok_config(
    existing: Option<&str>,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<String, String> {
    let base = normalize_base_origin(base);
    let primary = models
        .first()
        .map(String::as_str)
        .unwrap_or("deepseek-v4-flash");
    let text = existing.unwrap_or("");
    let mut out = if text.trim().is_empty() {
        format!("[models]\ndefault = \"{primary}\"\n")
    } else {
        let models_table = format!("[models]\ndefault = \"{primary}\"\n");
        replace_toml_table(text, "[models]", &models_table)
    };

    for m in models {
        let table_header = format!("[model.\"{m}\"]");
        let model_body = format!(
            "{table_header}\nmodel = \"{m}\"\nbase_url = \"{base}/v1\"\nname = \"{PROVIDER_NAME}\"\napi_key = \"{api_key}\"\napi_backend = \"responses\"\ncontext_window = 500000\n"
        );
        out = replace_toml_table(&out, &table_header, &model_body);
    }
    Ok(out)
}

/// 生成 ZCode 配置：写入 provider.proxyweave 及多个选定模型。
pub fn build_zcode_config(
    existing: Option<&str>,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<String, String> {
    let base = normalize_base_origin(base);
    let mut root = parse_json_object(existing)?;
    if !root.get("provider").map(Value::is_object).unwrap_or(false) {
        root.insert("provider".into(), json!({}));
    }
    let provider = root
        .get_mut("provider")
        .and_then(Value::as_object_mut)
        .unwrap();

    let mut models_map = Map::new();
    for m in models {
        models_map.insert(
            m.clone(),
            json!({
                "reasoning": {
                    "enabled": true,
                    "variants": ["low", "medium", "high", "max"],
                    "defaultVariant": "max"
                },
                "limit": {
                    "context": 200000,
                    "output": 8192
                },
                "modalities": {
                    "input": ["text", "image"],
                    "output": ["text"]
                }
            }),
        );
    }

    provider.insert(
        PROVIDER_KEY.into(),
        json!({
            "name": PROVIDER_NAME,
            "kind": "openai-compatible",
            "options": {
                "baseURL": format!("{base}/v1"),
                "apiKey": api_key
            },
            "enabled": true,
            "source": "custom",
            "models": models_map
        }),
    );
    render_json(Value::Object(root))
}

/// 生成 Kimi Code config.toml：写入 provider 表、default_model 与每个选中模型的配置表。
pub fn build_kimi_code_config(
    existing: Option<&str>,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<String, String> {
    let base = normalize_base_origin(base);
    let primary = models
        .first()
        .map(String::as_str)
        .unwrap_or("deepseek-v4-flash");
    let text = existing.unwrap_or("");
    let mut out = set_toml_top_level(text, "default_model", &format!("\"{primary}\""));

    let provider_table = format!("[providers.{PROVIDER_KEY}]");
    let provider_body = format!(
        "{provider_table}\ntype = \"openai\"\nbase_url = \"{base}/v1\"\napi_key = \"{api_key}\"\n"
    );
    out = replace_toml_table(&out, &provider_table, &provider_body);

    for m in models {
        let table_header = format!("[models.\"{m}\"]");
        let model_body = format!(
            "{table_header}\nprovider = \"{PROVIDER_KEY}\"\nmodel = \"{m}\"\nmax_context_size = 200000\ncapabilities = [\"tool_use\"]\n"
        );
        out = replace_toml_table(&out, &table_header, &model_body);
    }
    Ok(out)
}

/// 生成 OpenClaw 配置：支持标准 JSON 与 JSON5 格式，支持注入多个选中模型。
pub fn build_openclaw_config(
    existing: Option<&str>,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<String, String> {
    let base = normalize_base_origin(base);
    let primary = models
        .first()
        .map(String::as_str)
        .unwrap_or("deepseek-v4-flash");
    let mut root = match existing.map(str::trim).filter(|s| !s.is_empty()) {
        None => Map::new(),
        Some(text) => parse_json_or_json5(text)?,
    };

    if !root.get("models").map(Value::is_object).unwrap_or(false) {
        root.insert("models".into(), json!({ "mode": "merge", "providers": {} }));
    }
    let models_block = root
        .get_mut("models")
        .and_then(Value::as_object_mut)
        .unwrap();
    if !models_block
        .get("providers")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        models_block.insert("providers".into(), json!({}));
    }
    let providers = models_block
        .get_mut("providers")
        .and_then(Value::as_object_mut)
        .unwrap();

    let models_list: Vec<Value> = models
        .iter()
        .map(|m| {
            json!({
                "id": m,
                "name": m,
                "contextWindow": 1048576,
                "reasoning": true,
                "input": ["text", "image"],
                "maxTokens": 131072
            })
        })
        .collect();

    providers.insert(
        PROVIDER_KEY.into(),
        json!({
            "baseUrl": format!("{base}/v1"),
            "apiKey": api_key,
            "api": "openai-completions",
            "models": models_list
        }),
    );

    if !root.get("agents").map(Value::is_object).unwrap_or(false) {
        root.insert("agents".into(), json!({ "defaults": {} }));
    }
    let agents = root
        .get_mut("agents")
        .and_then(Value::as_object_mut)
        .unwrap();
    if !agents
        .get("defaults")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        agents.insert("defaults".into(), json!({}));
    }
    let defaults = agents
        .get_mut("defaults")
        .and_then(Value::as_object_mut)
        .unwrap();
    if !defaults
        .get("models")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        defaults.insert("models".into(), json!({}));
    }
    let d_models = defaults
        .get_mut("models")
        .and_then(Value::as_object_mut)
        .unwrap();
    for m in models {
        let qualified = format!("{PROVIDER_KEY}/{m}");
        d_models.insert(qualified, json!({ "alias": PROVIDER_NAME }));
    }
    defaults.insert(
        "model".into(),
        json!({ "primary": format!("{PROVIDER_KEY}/{primary}") }),
    );

    render_json(Value::Object(root))
}

/// 生成 Hermes Agent config.yaml：更新 model 默认项并挂载选中的所有模型。
pub fn build_hermes_config(
    existing: Option<&str>,
    base: &str,
    api_key: &str,
    models: &[String],
) -> Result<String, String> {
    let base = normalize_base_origin(base);
    let primary = models
        .first()
        .map(String::as_str)
        .unwrap_or("deepseek-v4-flash");
    let raw = existing.unwrap_or("");

    let mut models_yaml = String::new();
    for m in models {
        models_yaml.push_str(&format!("      {m}:\n        name: {m}\n"));
    }

    let pw_header = format!("- name: {PROVIDER_KEY}");
    let pw_entry = format!(
        "{pw_header}\n  base_url: {base}/v1\n  api_key: {api_key}\n  models:\n{models_yaml}  model: {primary}\n"
    );

    if raw.trim().is_empty() {
        return Ok(format!(
            "model:\n  default: {primary}\n  provider: {PROVIDER_KEY}\n\ncustom_providers:\n{pw_entry}"
        ));
    }

    let mut text = raw.to_string();

    // 1. 更新或注入 model: 块
    if let Some(pos) = text.find("model:") {
        let after = &text[pos + 6..];
        let mut block_len = 6;
        for line in after.lines() {
            if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
                break;
            }
            block_len += line.len() + 1;
        }
        let replacement = format!("model:\n  default: {primary}\n  provider: {PROVIDER_KEY}\n");
        text.replace_range(pos..pos + block_len.min(text.len() - pos), &replacement);
    } else {
        text = format!("model:\n  default: {primary}\n  provider: {PROVIDER_KEY}\n\n{text}");
    }

    // 2. 更新 custom_providers 列表
    if text.contains("custom_providers:") {
        if let Some(pw_pos) = text.find(&pw_header) {
            let hdr_len = pw_header.len();
            let after = &text[pw_pos + hdr_len..];
            let mut entry_len = hdr_len;
            for line in after.lines() {
                if line.trim_start().starts_with("- ")
                    || (!line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t'))
                {
                    break;
                }
                entry_len += line.len() + 1;
            }
            text.replace_range(pw_pos..pw_pos + entry_len.min(text.len() - pw_pos), "");
        }
        if let Some(insert_pos) = text.find("custom_providers:") {
            let after_hdr = insert_pos + "custom_providers:".len();
            let next_nl = text[after_hdr..]
                .find('\n')
                .map(|p| after_hdr + p + 1)
                .unwrap_or(text.len());
            text.insert_str(next_nl, &pw_entry);
        }
    } else {
        text.push_str(&format!("\ncustom_providers:\n{pw_entry}"));
    }

    Ok(text)
}

// ---------------------------------------------------------------------------
// 内部工具
// ---------------------------------------------------------------------------

fn read_json(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    parse_json_or_json5(&text).ok().map(Value::Object)
}

fn env_value(root: &Value, key: &str) -> Option<String> {
    root.get("env")
        .and_then(|e| e.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn parse_json_object(existing: Option<&str>) -> Result<Map<String, Value>, String> {
    match existing.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(Map::new()),
        Some(text) => parse_json_or_json5(text),
    }
}

/// 支持宽松 JSON5 风格（支持单引号、无引号属性名、单行注释、尾随逗号）。
fn parse_json_or_json5(text: &str) -> Result<Map<String, Value>, String> {
    if let Ok(Value::Object(map)) = serde_json::from_str(text) {
        return Ok(map);
    }

    let mut out = String::with_capacity(text.len() + 32);
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut in_string = false;
    let mut string_quote = '"';
    let mut escaped = false;

    while i < n {
        let c = chars[i];
        if in_string {
            if escaped {
                out.push(c);
                escaped = false;
            } else if c == '\\' {
                out.push(c);
                escaped = true;
            } else if c == string_quote {
                out.push('"');
                in_string = false;
            } else {
                out.push(c);
            }
            i += 1;
            continue;
        }

        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if c == '"' || c == '\'' {
            in_string = true;
            string_quote = c;
            out.push('"');
            i += 1;
            continue;
        }

        if c.is_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while i < n
                && (chars[i].is_alphanumeric()
                    || chars[i] == '_'
                    || chars[i] == '-'
                    || chars[i] == '$')
            {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            let mut peek = i;
            while peek < n && chars[peek].is_whitespace() {
                peek += 1;
            }
            if peek < n && chars[peek] == ':' {
                out.push('"');
                out.push_str(&ident);
                out.push('"');
            } else {
                out.push_str(&ident);
            }
            continue;
        }

        if c == ',' {
            let mut peek = i + 1;
            while peek < n && chars[peek].is_whitespace() {
                peek += 1;
            }
            if peek < n && (chars[peek] == '}' || chars[peek] == ']') {
                i += 1;
                continue;
            }
        }

        out.push(c);
        i += 1;
    }

    match serde_json::from_str::<Value>(&out) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err("配置文件根节点必须是 JSON 对象".to_string()),
        Err(e) => Err(format!("配置文件解析失败: {e}")),
    }
}

fn render_json(value: Value) -> Result<String, String> {
    serde_json::to_string_pretty(&value).map_err(|e| format!("序列化 JSON 失败: {e}"))
}

fn parse_yaml_map(existing: Option<&str>) -> Result<Map<String, Value>, String> {
    let Some(text) = existing.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(Map::new());
    };

    match crate::yaml_lite::parse_mapping(text) {
        Ok(map) => Ok(map),
        Err(_) => Err("DSH 配置文件解析失败（已备份原文件，未做修改）".to_string()),
    }
}

fn render_yaml(value: Value) -> Result<String, String> {
    crate::yaml_lite::render_mapping(&value)
}

fn ensure_yaml_map<'a>(root: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    if !root.get(key).map(Value::is_object).unwrap_or(false) {
        root.insert(key.to_string(), json!({}));
    }
    root.get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("just inserted object")
}

/// 设置 TOML 顶层标量（存在则替换，不存在则插入到首个 `[table]` 之前）。
fn set_toml_top_level(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let prefix = format!("{key} ");
    let prefix_eq = format!("{key}=");

    if let Some(index) = lines.iter().position(|l| {
        let t = l.trim_start();
        !t.starts_with('#') && (t.starts_with(&prefix) || t.starts_with(&prefix_eq))
    }) {
        lines[index] = format!("{key} = {value}");
        return lines.join("\n");
    }

    let insert_at = lines
        .iter()
        .position(|l| l.trim_start().starts_with('['))
        .unwrap_or(lines.len());
    lines.insert(insert_at, format!("{key} = {value}"));
    lines.join("\n")
}

/// 替换整个 TOML 表（从 `[table]` 到下一个顶层 `[` 之前）。
fn replace_toml_table(text: &str, table: &str, body: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|l| l.trim() == table) else {
        let mut out = text.trim_end().to_string();
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(body.trim_end());
        return out;
    };

    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        let t = line.trim();
        if t.starts_with('[') {
            end = i;
            break;
        }
    }

    let mut out = String::new();
    for line in &lines[..start] {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(body.trim_end());
    out.push('\n');
    for line in &lines[end..] {
        out.push_str(line);
        out.push('\n');
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zcode_config_declares_image_modality() {
        let out =
            build_zcode_config(None, "http://127.0.0.1:7863", "sk-x", &["glm-5.2".into()]).unwrap();
        let v: Value = serde_json::from_str(&out).expect("ZCode 配置必须是合法 JSON");
        let mods = &v["provider"]["proxyweave"]["models"]["glm-5.2"]["modalities"]["input"];
        let arr = mods.as_array().expect("modalities.input 应为数组");
        assert!(
            arr.iter().any(|m| m == "image"),
            "ZCode 由 modalities.input 推导 supportsImages，缺 image 会静默丢弃图片：{out}"
        );
        assert!(arr.iter().any(|m| m == "text"), "仍应保留 text");
    }

    #[test]
    fn dsh_settings_declares_non_empty_image_input() {
        let out = build_dsh_settings(None, "http://127.0.0.1:7863", &["glm-5.2".into()]).unwrap();
        let map = crate::yaml_lite::parse_mapping(&out).expect("DSH 配置必须是合法 YAML");
        let v = Value::Object(map);
        let input = &v["llm-pi-ai"]["providers"]["proxyweave"]["models"][0]["input"];
        let arr = input.as_array().expect("每个模型都应有 input 数组");
        assert!(
            !arr.is_empty(),
            "input 不能为空数组（declaredInput 会当作未声明而回退）：{out}"
        );
        assert!(arr.iter().any(|m| m == "image"), "应声明 image：{out}");
    }

    #[test]
    fn zcode_config_repairs_text_only_modality() {
        let existing = r#"{"provider":{"proxyweave":{"models":{"glm-5.2":{"modalities":{"input":["text"],"output":["text"]}}}}}}"#;
        let out = build_zcode_config(
            Some(existing),
            "http://127.0.0.1:7863",
            "sk-x",
            &["glm-5.2".into()],
        )
        .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        let arr = v["provider"]["proxyweave"]["models"]["glm-5.2"]["modalities"]["input"]
            .as_array()
            .unwrap();
        assert!(
            arr.iter().any(|m| m == "image"),
            "旧配置里的纯文本声明应被纠正：{out}"
        );
    }

    #[test]
    fn codex_config_uses_responses_and_keeps_other_tables() {
        let existing = "model = \"old\"\n\n[mcp_servers.foo]\ncommand = \"x\"\n";
        let out = build_codex_config(Some(existing), "http://127.0.0.1:7863", &["glm-5.2".into()])
            .unwrap();
        assert!(
            out.contains("wire_api = \"responses\""),
            "must use responses:\n{out}"
        );
        assert!(!out.contains("wire_api = \"chat\""));
        assert!(out.contains("base_url = \"http://127.0.0.1:7863/v1\""));
        assert!(out.contains("model = \"glm-5.2\""));
        assert!(
            out.contains("[mcp_servers.foo]"),
            "must keep unrelated tables:\n{out}"
        );
    }

    #[test]
    fn codex_config_is_idempotent() {
        let models = vec!["m".to_string()];
        let first = build_codex_config(None, "http://127.0.0.1:7863", &models).unwrap();
        let second = build_codex_config(Some(&first), "http://127.0.0.1:7863", &models).unwrap();
        assert_eq!(first.matches("[model_providers.proxyweave]").count(), 1);
        assert_eq!(second.matches("[model_providers.proxyweave]").count(), 1);
    }

    #[test]
    fn codex_auth_drops_oauth_fields() {
        let existing = r#"{"tokens":{"a":1},"last_refresh":"x","OPENAI_API_KEY":"old"}"#;
        let out = build_codex_auth(Some(existing), "sk-new").unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["OPENAI_API_KEY"], "sk-new");
        assert_eq!(v["auth_mode"], "apikey");
        assert!(v.get("tokens").is_none());
    }

    #[test]
    fn claude_code_keeps_unrelated_settings_and_maps_models() {
        let existing = r#"{"permissions":{"allow":["Bash"]},"env":{"OTHER":"1"}}"#;
        let models = vec!["deepseek-v4-flash".to_string(), "glm-5.2".to_string()];
        let out =
            build_claude_code_settings(Some(existing), "http://127.0.0.1:7863", "sk-1", &models)
                .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["env"]["ANTHROPIC_BASE_URL"], "http://127.0.0.1:7863");
        assert_eq!(v["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-1");
        assert_eq!(
            v["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"],
            "deepseek-v4-flash"
        );
        assert_eq!(
            v["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL_NAME"],
            "deepseek-v4-flash"
        );
        assert_eq!(v["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL"], "glm-5.2");
        assert_eq!(v["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL_NAME"], "glm-5.2");
        assert_eq!(
            v["env"]["ANTHROPIC_DEFAULT_HAIKU_MODEL"],
            "deepseek-v4-flash"
        );
        assert_eq!(
            v["env"]["ANTHROPIC_DEFAULT_FABLE_MODEL"],
            "deepseek-v4-flash"
        );
        assert_eq!(v["env"]["OTHER"], "1");
        assert_eq!(v["permissions"]["allow"][0], "Bash");
    }

    #[test]
    fn claude_code_maps_four_slots_in_order() {
        let models = vec![
            "m1".to_string(),
            "m2".to_string(),
            "m3".to_string(),
            "m4".to_string(),
        ];
        let out =
            build_claude_code_settings(None, "http://127.0.0.1:7863", "sk-1", &models).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"], "m1");
        assert_eq!(v["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL"], "m2");
        assert_eq!(v["env"]["ANTHROPIC_DEFAULT_HAIKU_MODEL"], "m3");
        assert_eq!(v["env"]["ANTHROPIC_DEFAULT_FABLE_MODEL"], "m4");
    }

    #[test]
    fn claude_desktop_fable_slot_uses_fourth_model() {
        let models = vec![
            "m1".to_string(),
            "m2".to_string(),
            "m3".to_string(),
            "m4".to_string(),
        ];
        let out =
            build_claude_desktop_profile(None, "http://127.0.0.1:7863", "sk-1", &models).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        let list = v["inferenceModels"].as_array().unwrap();
        let fable = list
            .iter()
            .find(|m| m["name"] == "claude-fable-5")
            .expect("fable slot must exist");
        assert_eq!(fable["labelOverride"], "m4");
    }

    #[test]
    fn claude_desktop_meta_replaces_managed_entry() {
        let existing = format!(
            r#"{{"entries":[{{"id":"other","name":"X"}},{{"id":"{CLAUDE_DESKTOP_PROFILE_ID}","name":"Old"}}]}}"#
        );
        let out = build_claude_desktop_meta(Some(&existing)).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        let entries = v["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2, "should not duplicate managed entry");
        assert!(entries
            .iter()
            .any(|e| e["id"] == "other" && e["name"] == "X"));
        assert_eq!(v["appliedId"], CLAUDE_DESKTOP_PROFILE_ID);
    }

    #[test]
    fn dsh_settings_registers_pi_ai_route_and_default_model() {
        let models = vec!["glm-5.2".to_string(), "kimi-k2.7".to_string()];
        let out = build_dsh_settings(None, "http://127.0.0.1:7863", &models).unwrap();
        assert!(out.contains("llm-pi-ai:"), "{out}");
        assert!(out.contains("api: openai-completions"), "{out}");
        assert!(out.contains("http://127.0.0.1:7863/v1"), "{out}");
        assert!(out.contains(DSH_CREDENTIAL_KEY), "{out}");
        assert!(out.contains("agent-default-model:"), "{out}");
        assert!(out.contains("glm-5.2"), "{out}");
        assert!(out.contains("kimi-k2.7"), "{out}");

        let parsed = crate::yaml_lite::parse_mapping(&out).unwrap();
        assert_eq!(parsed["agent-default-model"]["provider"], "proxyweave");
        assert_eq!(parsed["agent-default-model"]["model"], "glm-5.2");
        let provider = &parsed["llm-pi-ai"]["providers"]["proxyweave"];
        assert_eq!(provider["api"], "openai-completions");
        assert_eq!(provider["baseURL"], "http://127.0.0.1:7863/v1");
        assert_eq!(provider["models"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn opencode_config_writes_multiple_models() {
        let models = vec!["m1".to_string(), "m2".to_string()];
        let out = build_opencode_config(None, "http://127.0.0.1:7863", "sk-test", &models).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["provider"]["proxyweave"]["options"]["baseURL"],
            "http://127.0.0.1:7863/v1"
        );
        assert_eq!(v["provider"]["proxyweave"]["options"]["apiKey"], "sk-test");
        assert!(v["provider"]["proxyweave"]["models"]["m1"].is_object());
        assert!(v["provider"]["proxyweave"]["models"]["m2"].is_object());
    }

    #[test]
    fn pi_models_writes_multiple_models() {
        let models = vec!["m1".to_string(), "m2".to_string()];
        let out = build_pi_models(None, "http://127.0.0.1:7863", "sk-test", &models).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["providers"]["proxyweave"]["baseUrl"],
            "http://127.0.0.1:7863/v1"
        );
        assert_eq!(v["providers"]["proxyweave"]["apiKey"], "sk-test");
        let list = v["providers"]["proxyweave"]["models"].as_array().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn grok_build_config_writes_multiple_models() {
        let models = vec!["m1".to_string(), "m2".to_string()];
        let out = build_grok_config(None, "http://127.0.0.1:7863", "sk-test", &models).unwrap();
        assert!(out.contains("default = \"m1\""));
        assert!(out.contains("[model.\"m1\"]"));
        assert!(out.contains("[model.\"m2\"]"));
    }

    #[test]
    fn zcode_config_writes_multiple_models() {
        let models = vec!["m1".to_string(), "m2".to_string()];
        let out = build_zcode_config(None, "http://127.0.0.1:7863", "sk-test", &models).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["provider"]["proxyweave"]["kind"], "openai-compatible");
        assert!(v["provider"]["proxyweave"]["models"]["m1"].is_object());
        assert!(v["provider"]["proxyweave"]["models"]["m2"].is_object());
    }

    #[test]
    fn kimi_code_config_writes_multiple_models() {
        let models = vec!["m1".to_string(), "m2".to_string()];
        let out =
            build_kimi_code_config(None, "http://127.0.0.1:7863", "sk-test", &models).unwrap();
        assert!(out.contains("default_model = \"m1\""));
        assert!(out.contains("[providers.proxyweave]"), "{out}");
        assert!(out.contains("type = \"openai\""), "{out}");
        assert!(out.contains("provider = \"proxyweave\""), "{out}");
        assert!(out.contains("[models.\"m1\"]"));
        assert!(out.contains("[models.\"m2\"]"));
        assert!(out.contains("capabilities = [\"tool_use\"]"), "{out}");
    }

    #[test]
    fn openclaw_config_writes_multiple_models() {
        let json5 = "{ models: { mode: 'merge', providers: {} } }";
        let models = vec!["m1".to_string(), "m2".to_string()];
        let out = build_openclaw_config(Some(json5), "http://127.0.0.1:7863", "sk-test", &models)
            .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        let list = v["models"]["providers"]["proxyweave"]["models"]
            .as_array()
            .unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(v["agents"]["defaults"]["model"]["primary"], "proxyweave/m1");
    }

    #[test]
    fn hermes_config_writes_multiple_models() {
        let existing = "model:\n  default: old\n  provider: old\ncustom_providers:\n- name: old\n  base_url: https://old\n";
        let models = vec!["m1".to_string(), "m2".to_string()];
        let out = build_hermes_config(Some(existing), "http://127.0.0.1:7863", "sk-test", &models)
            .unwrap();
        assert!(out.contains("provider: proxyweave"));
        assert!(out.contains("default: m1"));
        assert!(out.contains("- name: proxyweave"));
        assert!(out.contains("m1:"));
        assert!(out.contains("m2:"));
    }

    thread_local! {
        pub(super) static FAIL_WRITE_PATH: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
    }

    #[test]
    fn claude_builders_require_selected_models_and_unverified_clients_stay_blocked() {
        assert!(
            build_claude_code_settings(None, "http://127.0.0.1:28319", "owned-key", &[]).is_err()
        );
        assert!(
            build_claude_desktop_profile(None, "http://127.0.0.1:28319", "owned-key", &[]).is_err()
        );
        let tmp = tempfile::tempdir().unwrap();
        for target in ["claude-desktop", "codex", "grok-build"] {
            let res = import_target_in(
                tmp.path(),
                target,
                "http://127.0.0.1:28319/v1",
                "owned-key",
                &["owned-model".into()],
            );
            assert!(res.is_ok());
        }
        for (key, models) in [
            ("", vec!["owned-model".into()]),
            ("owned-key", vec![]),
            ("owned-key", vec![" ".into()]),
        ] {
            assert!(import_target_in(
                tmp.path(),
                "claude-code",
                "http://127.0.0.1:28319/v1",
                key,
                &models
            )
            .is_err());
        }
        assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 3);
    }

    #[test]
    fn owned_claude_import_detection_backup_rejection_and_partial_failure() {
        // Run the environment-sensitive fixture in a separate process; never mutate
        // process-global HOME while other workspace tests may run concurrently.
        const OWNED_CHILD: &str = "PROXYWEAVE_OWNED_CLAUDE_TEST";
        if std::env::var_os(OWNED_CHILD).is_none() {
            let owned = tempfile::tempdir().unwrap();
            let home = owned.path().join("home");
            let appdata = home.join("AppData/Local");
            let code = home.join(".claude");
            std::fs::create_dir_all(&code).unwrap();
            std::fs::create_dir_all(&appdata).unwrap();
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "agent_import::tests::owned_claude_import_detection_backup_rejection_and_partial_failure", "--test-threads=1", "--nocapture"])
                .env(OWNED_CHILD, owned.path())
                .env("HOME", &home)
                .env("USERPROFILE", &home)
                .env("LOCALAPPDATA", &appdata)
                .env("CLAUDE_CONFIG_DIR", &code)
                .env("PATH", &home)
                .output().unwrap();
            assert!(
                child.status.success(),
                "owned child failed: {} {}",
                String::from_utf8_lossy(&child.stdout),
                String::from_utf8_lossy(&child.stderr)
            );
            assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
            return;
        }
        let owned = PathBuf::from(std::env::var_os(OWNED_CHILD).unwrap());
        let tmp = tempfile::tempdir_in(&owned).unwrap();
        let home = owned.join("home");
        let code = home.join(".claude");
        assert_eq!(home_dir(), home);
        assert_eq!(local_app_data(), home.join("AppData/Local"));
        let base = "http://127.0.0.1:28319/v1";
        let key = "owned-fixture-key";
        let models: Vec<String> = (1..=5).map(|i| format!("owned-model-{i}")).collect();
        assert_eq!(claude_code_home(), code);
        let path = code.join("settings.json");
        let original = r#"{"permissions":{"allow":["Bash"]},"env":{"OTHER":"owned"},"mcpServers":{"kept":{}}}"#;
        std::fs::write(&path, original).unwrap();
        let before = detect_claude_code(base, key);
        assert!(before.installed && before.supported && !before.configured);
        let backup_root = tmp.path().join("success-backups");
        let result = import_target_in(&backup_root, "claude-code", base, key, &models).unwrap();
        assert_eq!(result.files, vec![path.to_string_lossy().to_string()]);
        assert_eq!(result.models, models[..4]);
        assert!(Path::new(&result.backup_dir).starts_with(&backup_root));
        let text = std::fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["env"]["ANTHROPIC_BASE_URL"], "http://127.0.0.1:28319");
        assert_eq!(value["env"]["ANTHROPIC_AUTH_TOKEN"], key);
        assert_eq!(value["env"]["OTHER"], "owned");
        assert_eq!(value["permissions"]["allow"][0], "Bash");
        assert!(value["mcpServers"]["kept"].is_object());
        assert_eq!(
            value["env"]["ANTHROPIC_DEFAULT_FABLE_MODEL"],
            "owned-model-4"
        );
        assert_eq!(
            build_claude_code_settings(Some(&text), "http://127.0.0.1:28319", key, &models[..4])
                .unwrap(),
            text
        );
        assert!(detect_claude_code(base, key).configured);
        assert!(!detect_claude_code(base, "wrong-key").configured);
        assert!(!detect_claude_code("", key).configured);
        assert!(!detect_claude_code(base, "").configured);
        let backup = list_backups_in(&backup_root, "claude-code");
        assert_eq!(
            restore_backup_in(&backup_root, "claude-code", &backup[0].id).unwrap(),
            1
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert!(!detect_claude_code(base, key).configured);

        let invalid = "{invalid-owned-fixture";
        std::fs::write(&path, invalid).unwrap();
        let err = import_target_in(
            &tmp.path().join("invalid-backups"),
            "claude-code",
            base,
            key,
            &models,
        )
        .unwrap_err();
        assert!(err.contains("未写入") && !err.contains(key) && !err.contains(invalid));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
        std::fs::write(&path, original).unwrap();
        FAIL_WRITE_PATH.with(|fail| *fail.borrow_mut() = Some(path.clone()));
        let err = import_target_in(
            &tmp.path().join("rejected-backups"),
            "claude-code",
            base,
            key,
            &models,
        )
        .unwrap_err();
        FAIL_WRITE_PATH.with(|fail| *fail.borrow_mut() = None);
        assert!(err.contains("备份历史") && !err.contains(key));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(import_target_in(
            &tmp.path().join("unreadable-backups"),
            "claude-code",
            base,
            key,
            &models
        )
        .unwrap_err()
        .contains("读取"));
        std::fs::remove_dir(&path).unwrap();

        let normal = claude_desktop_dir().join("claude_desktop_config.json");
        let threep = claude_desktop_3p_dir().join("claude_desktop_config.json");
        let library = claude_desktop_library_dir();
        std::fs::create_dir_all(normal.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&library).unwrap();
        let profile = library.join(format!("{CLAUDE_DESKTOP_PROFILE_ID}.json"));
        let meta = library.join("_meta.json");
        let originals = [
            (
                normal.clone(),
                r#"{"normalOnly":true,"mcpServers":{"kept":{}}}"#,
            ),
            (threep.clone(), r#"{"threepOnly":true}"#),
            (profile.clone(), r#"{"profileOnly":true}"#),
            (
                meta.clone(),
                r#"{"entries":[{"id":"other","name":"kept"}]}"#,
            ),
        ];
        for (path, text) in &originals {
            assert!(path.starts_with(&home));
            std::fs::write(path, text).unwrap();
        }
        let public_res = import_target_in(
            &tmp.path().join("blocked-desktop"),
            "claude-desktop",
            base,
            key,
            &models,
        );
        assert!(public_res.is_ok());
        let backup = list_backups_in(&tmp.path().join("blocked-desktop"), "claude-desktop");
        assert_eq!(
            restore_backup_in(
                &tmp.path().join("blocked-desktop"),
                "claude-desktop",
                &backup[0].id
            )
            .unwrap(),
            4
        );
        for (path, text) in &originals {
            assert_eq!(std::fs::read_to_string(path).unwrap(), *text);
        }
        // Private extracted writer is exercised only in owned fixtures, NOT client acceptance.
        let root = tmp.path().join("desktop-backups");
        let desktop_result =
            import_claude_desktop(&root, "http://127.0.0.1:28319", key, &models).unwrap();
        assert_eq!(desktop_result.files.len(), 4);
        assert_eq!(desktop_result.models, models[..4]);
        let normal_value: Value =
            serde_json::from_str(&std::fs::read_to_string(&normal).unwrap()).unwrap();
        let threep_value: Value =
            serde_json::from_str(&std::fs::read_to_string(&threep).unwrap()).unwrap();
        assert_eq!(normal_value["deploymentMode"], "3p");
        assert_eq!(normal_value["normalOnly"], true);
        assert_eq!(threep_value["threepOnly"], true);
        assert!(threep_value.get("normalOnly").is_none());
        let status = detect_claude_desktop(base, key);
        assert!(status.configured && status.installed && !status.supported);
        assert!(!detect_claude_desktop(base, "wrong").configured);
        let backup = list_backups_in(&root, "claude-desktop");
        assert_eq!(
            restore_backup_in(&root, "claude-desktop", &backup[0].id).unwrap(),
            4
        );
        for (path, text) in &originals {
            assert_eq!(std::fs::read_to_string(path).unwrap(), *text);
        }
        std::fs::write(&threep, invalid).unwrap();
        assert!(import_claude_desktop(
            &tmp.path().join("desktop-invalid"),
            "http://127.0.0.1:28319",
            key,
            &models
        )
        .unwrap_err()
        .contains("未写入"));
        for (path, text) in originals.iter().filter(|(path, _)| path != &threep) {
            assert_eq!(std::fs::read_to_string(path).unwrap(), *text);
        }
        std::fs::write(&threep, originals[1].1).unwrap();
        FAIL_WRITE_PATH.with(|fail| *fail.borrow_mut() = Some(threep.clone()));
        let partial_root = tmp.path().join("desktop-partial");
        let partial = import_claude_desktop(&partial_root, "http://127.0.0.1:28319", key, &models)
            .unwrap_err();
        FAIL_WRITE_PATH.with(|fail| *fail.borrow_mut() = None);
        assert!(partial.contains("3/4") && partial.contains("备份历史") && !partial.contains(key));
        assert_eq!(std::fs::read_to_string(&threep).unwrap(), originals[1].1);
        assert_ne!(std::fs::read_to_string(&profile).unwrap(), originals[2].1);
        let backup = list_backups_in(&partial_root, "claude-desktop");
        assert_eq!(
            restore_backup_in(&partial_root, "claude-desktop", &backup[0].id).unwrap(),
            4
        );
        for (path, text) in &originals {
            assert_eq!(std::fs::read_to_string(path).unwrap(), *text);
        }
    }

    #[test]
    fn backup_and_restore_roundtrip_uses_protected_atomic_write() {
        let tmp = tempfile::tempdir().unwrap();
        let backup_dir = tmp.path().join("agent-backups");
        let target_file = tmp.path().join("opencode.json");
        std::fs::write(&target_file, r#"{"provider":{"existing":{"name":"Old"}}}"#).unwrap();

        let dir =
            backup_files_in(&backup_dir, "opencode", std::slice::from_ref(&target_file)).unwrap();
        assert!(dir.join("manifest.json").is_file());

        // Overwrite target file
        atomic_write(
            &target_file,
            r#"{"provider":{"proxyweave":{"name":"ProxyWeave"}}}"#,
        )
        .unwrap();
        assert!(std::fs::read_to_string(&target_file)
            .unwrap()
            .contains("proxyweave"));

        // List backups and restore latest
        let backups = list_backups_in(&backup_dir, "opencode");
        assert_eq!(backups.len(), 1);
        let restored = restore_backup_in(&backup_dir, "opencode", &backups[0].id).unwrap();
        assert_eq!(restored, 1);

        let restored_text = std::fs::read_to_string(&target_file).unwrap();
        assert!(restored_text.contains("existing"));
        assert!(!restored_text.contains("proxyweave"));
    }
}
