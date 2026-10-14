use crate::manifest::{
    Capabilities, Configuration, HealthCheck, Lifecycle, Management, ManagementPresentation,
    Manifest, Metadata, Network, Ownership, Runtime,
};
use crate::registry::{ImportResult, Registry};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectionReport {
    pub id: String,
    pub name: String,
    pub version: String,
    pub project_type: String,
    pub executable_relative_path: String,
    pub executable_sha256: String,
    pub manifest: Manifest,
    pub notes: Vec<String>,
}

fn compute_sha256(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("无法打开文件进行哈希计算: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("读取文件失败: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn detect_license(dir: &Path) -> String {
    for name in [
        "LICENSE",
        "LICENSE.txt",
        "LICENSE.md",
        "COPYING",
        "LICENSE-MIT",
        "LICENSE-APACHE",
    ] {
        let p = dir.join(name);
        if p.is_file() {
            if let Ok(mut file) = fs::File::open(&p) {
                let mut buf = [0u8; 4096];
                if let Ok(n) = file.read(&mut buf) {
                    let text = String::from_utf8_lossy(&buf[..n]).to_ascii_uppercase();
                    if text.contains("MIT") {
                        return "MIT".to_string();
                    } else if text.contains("APACHE") {
                        return "Apache-2.0".to_string();
                    } else if text.contains("BSD") {
                        return "BSD-3-Clause".to_string();
                    } else if text.contains("GPL") {
                        return "GPL-3.0".to_string();
                    }
                }
            }
            return "MIT".to_string();
        }
    }
    "MIT".to_string()
}

fn extract_version(dir: &Path) -> Option<String> {
    let readme_re =
        regex::Regex::new(r"(?i)(?:version|release)\s*[:=]?\s*v?([0-9]+\.[0-9]+\.[0-9]+)").ok()?;
    let tag_re = regex::Regex::new(r"(?m)^#+\s*v?([0-9]+\.[0-9]+\.[0-9]+)").ok()?;
    for file_name in ["README.md", "README_CN.md", "readme.md"] {
        let p = dir.join(file_name);
        if let Ok(content) = fs::read_to_string(&p) {
            let found_version = readme_re
                .captures(&content)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str())
                .filter(|v| semver::Version::parse(v).is_ok())
                .map(|v| v.to_string())
                .or_else(|| {
                    tag_re
                        .captures(&content)
                        .and_then(|cap| cap.get(1))
                        .map(|m| m.as_str())
                        .filter(|v| semver::Version::parse(v).is_ok())
                        .map(|v| v.to_string())
                });
            if let Some(v) = found_version {
                return Some(v);
            }
        }
    }
    let gomod_p = dir.join("go.mod");
    if let Ok(content) = fs::read_to_string(&gomod_p) {
        let gomod_re =
            regex::Regex::new(r"(?m)^\s*//\s*(?:version|v)?\s*v?([0-9]+\.[0-9]+\.[0-9]+)").ok()?;
        if let Some(v) = gomod_re
            .captures(&content)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str())
            .filter(|v| semver::Version::parse(v).is_ok())
        {
            return Some(v.to_string());
        }
    }
    None
}

fn sanitize_id(stem: &str) -> String {
    let lower = stem.to_ascii_lowercase();
    let mut s: String = lower
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();

    while s.chars().next().is_some_and(|c| !c.is_ascii_alphanumeric()) {
        s.remove(0);
    }
    while s.ends_with('-') || s.ends_with('_') {
        s.pop();
    }
    if s.is_empty() {
        s = "app".to_string();
    }
    if s.len() > 64 {
        s.truncate(64);
        while s.ends_with('-') || s.ends_with('_') {
            s.pop();
        }
        if s.is_empty() {
            s = "app".to_string();
        }
    }
    let upper = s.to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || (upper.len() == 4
        && (upper.starts_with("COM") || upper.starts_with("LPT"))
        && matches!(upper.as_bytes()[3], b'1'..=b'9'))
    {
        s = format!("app-{s}");
        if s.len() > 64 {
            s.truncate(64);
        }
    }
    s
}

fn prettify_name(stem: &str) -> String {
    let trimmed = stem.trim();
    if trimmed.is_empty() {
        return "Application".to_string();
    }
    if trimmed.len() > 128 {
        trimmed[..128].to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn detect_project(dir: impl AsRef<Path>) -> Result<DetectionReport, String> {
    let dir = dir.as_ref();
    if !dir.exists() {
        return Err(format!("目标路径不存在: {}", dir.display()));
    }
    if !dir.is_dir() {
        return Err(format!("目标路径不是有效目录: {}", dir.display()));
    }

    // 优先读取已存在的 manifest.yaml
    let manifest_path = dir.join("manifest.yaml");
    if manifest_path.is_file() {
        let bytes =
            fs::read(&manifest_path).map_err(|e| format!("读取 manifest.yaml 失败: {e}"))?;
        let manifest = Manifest::parse(&bytes)?;
        let (executable_relative_path, executable_sha256) = match &manifest.runtime {
            Runtime::Managed {
                executable, sha256, ..
            } => (executable.clone(), sha256.clone()),
            Runtime::External { .. } => ("".to_string(), "".to_string()),
        };
        return Ok(DetectionReport {
            id: manifest.metadata.id.clone(),
            name: manifest.metadata.name.clone(),
            version: manifest.metadata.version.clone(),
            project_type: "manifest".to_string(),
            executable_relative_path,
            executable_sha256,
            manifest,
            notes: vec!["Loaded existing manifest.yaml".to_string()],
        });
    }

    // 识别已知 2API 项目：wb2api
    let wb2api_rel = if dir.join("wb2api.exe").is_file() {
        Some("wb2api.exe".to_string())
    } else if dir.join("bin").join("wb2api.exe").is_file() {
        Some("bin/wb2api.exe".to_string())
    } else {
        None
    };

    if let Some(rel_path) = wb2api_rel {
        let full_path = dir.join(&rel_path);
        let sha256 = compute_sha256(&full_path)?;
        let version = extract_version(dir).unwrap_or_else(|| "1.13.0".to_string());
        let license = detect_license(dir);
        let mut env = BTreeMap::new();
        env.insert("WB2A_LISTEN".into(), "127.0.0.1:{PORT}".into());

        let manifest = Manifest {
            schema_version: "1.0".to_string(),
            metadata: Metadata {
                id: "wb2api".to_string(),
                name: "WorkBuddy 2API".to_string(),
                version: version.clone(),
                license,
                source_commit: None,
            },
            runtime: Runtime::Managed {
                executable: rel_path.clone(),
                sha256: sha256.clone(),
                args: vec!["-config".into(), "{CONFIG_PATH}".into()],
                env,
                resources: BTreeMap::new(),
            },
            network: Network {
                health_check: HealthCheck {
                    path: "/healthz".into(),
                    expected_status: 200,
                    identity_header: "x-service".into(),
                    identity_value: "workbuddy2api".into(),
                    timeout_ms: 2000,
                    busy_statuses: vec![503],
                },
            },
            lifecycle: Lifecycle {
                startup_timeout_ms: 15000,
                idle_timeout_ms: None,
                max_port_retries: 3,
            },
            capabilities: Capabilities {
                protocol: "openai_chat".into(),
                models_path: "/v1/models".into(),
                inference_path: "/v1/chat/completions".into(),
                streaming: true,
            },
            configuration: Configuration {
                settings: BTreeMap::new(),
                config_template: serde_json::json!({
                    "listen": "127.0.0.1:{PORT}",
                    "api_key": "{PLUGIN_API_KEY}"
                }),
                credential_files: BTreeMap::new(),
                required_secrets: vec![],
            },
            management: Some(Management {
                path: "/panel/".into(),
                presentation: ManagementPresentation::Browser,
            }),
        };
        manifest.validate()?;
        return Ok(DetectionReport {
            id: "wb2api".to_string(),
            name: "WorkBuddy 2API".to_string(),
            version,
            project_type: "wb2api".to_string(),
            executable_relative_path: rel_path,
            executable_sha256: sha256,
            manifest,
            notes: vec!["Detected WorkBuddy 2API project".to_string()],
        });
    }

    // 识别已知 2API 项目：cli-proxy-api
    let cli_proxy_rel = if dir.join("cli-proxy-api.exe").is_file() {
        Some("cli-proxy-api.exe".to_string())
    } else if dir.join("bin").join("cli-proxy-api.exe").is_file() {
        Some("bin/cli-proxy-api.exe".to_string())
    } else {
        None
    };

    if let Some(rel_path) = cli_proxy_rel {
        let full_path = dir.join(&rel_path);
        let sha256 = compute_sha256(&full_path)?;
        let version = "1.0.0".to_string();
        let license = detect_license(dir);
        let mgmt_path = if dir.join("static/management.html").is_file()
            || dir.join("static").join("management.html").is_file()
        {
            "/static/management.html".to_string()
        } else {
            "/".to_string()
        };
        let mut env = BTreeMap::new();
        env.insert("PORT".into(), "{PORT}".into());

        let manifest = Manifest {
            schema_version: "1.0".to_string(),
            metadata: Metadata {
                id: "cli-proxy-api".to_string(),
                name: "CLI Proxy API".to_string(),
                version: version.clone(),
                license,
                source_commit: None,
            },
            runtime: Runtime::Managed {
                executable: rel_path.clone(),
                sha256: sha256.clone(),
                args: vec![],
                env,
                resources: BTreeMap::new(),
            },
            network: Network {
                health_check: HealthCheck {
                    path: "/".into(),
                    expected_status: 200,
                    identity_header: "server".into(),
                    identity_value: "cli-proxy-api".into(),
                    timeout_ms: 2000,
                    busy_statuses: vec![],
                },
            },
            lifecycle: Lifecycle {
                startup_timeout_ms: 15000,
                idle_timeout_ms: None,
                max_port_retries: 3,
            },
            capabilities: Capabilities {
                protocol: "openai_chat".into(),
                models_path: "/v1/models".into(),
                inference_path: "/v1/chat/completions".into(),
                streaming: true,
            },
            configuration: Configuration {
                settings: BTreeMap::new(),
                config_template: serde_json::json!({
                    "port": "{PORT}"
                }),
                credential_files: BTreeMap::new(),
                required_secrets: vec![],
            },
            management: Some(Management {
                path: mgmt_path,
                presentation: ManagementPresentation::Browser,
            }),
        };
        manifest.validate()?;
        return Ok(DetectionReport {
            id: "cli-proxy-api".to_string(),
            name: "CLI Proxy API".to_string(),
            version,
            project_type: "cli-proxy-api".to_string(),
            executable_relative_path: rel_path,
            executable_sha256: sha256,
            manifest,
            notes: vec!["Detected CLI Proxy API project".to_string()],
        });
    }

    // 通用推断：搜索目录根或 bin/ 下的第一个 .exe 文件
    let mut exe_files: Vec<(String, String, PathBuf)> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_exe = path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"));
            if is_exe {
                if let Some(name) = path.file_name().and_then(|f| f.to_str()) {
                    exe_files.push((name.to_string(), name.to_string(), path));
                }
            }
        }
    }
    exe_files.sort_by(|a, b| a.0.cmp(&b.0));

    if exe_files.is_empty() {
        let bin_dir = dir.join("bin");
        if bin_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&bin_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let is_exe = path.is_file()
                        && path
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"));
                    if is_exe {
                        if let Some(name) = path.file_name().and_then(|f| f.to_str()) {
                            exe_files.push((name.to_string(), format!("bin/{name}"), path));
                        }
                    }
                }
            }
            exe_files.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }

    if exe_files.is_empty() {
        return Err("未在目录中检测到 Windows 可执行文件 (.exe)".into());
    }

    let (file_name, rel_path, full_path) = &exe_files[0];
    let sha256 = compute_sha256(full_path)?;
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("app");
    let id = sanitize_id(stem);
    let name = prettify_name(stem);
    let version = extract_version(dir).unwrap_or_else(|| "1.0.0".to_string());
    let license = detect_license(dir);
    let mut env = BTreeMap::new();
    env.insert("PORT".into(), "{PORT}".into());

    let manifest = Manifest {
        schema_version: "1.0".to_string(),
        metadata: Metadata {
            id: id.clone(),
            name: name.clone(),
            version: version.clone(),
            license,
            source_commit: None,
        },
        runtime: Runtime::Managed {
            executable: rel_path.clone(),
            sha256: sha256.clone(),
            args: vec![],
            env,
            resources: BTreeMap::new(),
        },
        network: Network {
            health_check: HealthCheck {
                path: "/".into(),
                expected_status: 200,
                identity_header: "server".into(),
                identity_value: id.clone(),
                timeout_ms: 2000,
                busy_statuses: vec![],
            },
        },
        lifecycle: Lifecycle {
            startup_timeout_ms: 15000,
            idle_timeout_ms: None,
            max_port_retries: 3,
        },
        capabilities: Capabilities {
            protocol: "openai_chat".into(),
            models_path: "/v1/models".into(),
            inference_path: "/v1/chat/completions".into(),
            streaming: true,
        },
        configuration: Configuration {
            settings: BTreeMap::new(),
            config_template: serde_json::json!({
                "port": "{PORT}"
            }),
            credential_files: BTreeMap::new(),
            required_secrets: vec![],
        },
        management: None,
    };
    manifest.validate()?;

    Ok(DetectionReport {
        id,
        name,
        version,
        project_type: "generic".to_string(),
        executable_relative_path: rel_path.clone(),
        executable_sha256: sha256,
        manifest,
        notes: vec![format!(
            "Inferred generic plugin configuration from {rel_path}"
        )],
    })
}

pub fn stage_and_import_project(
    registry: &Registry,
    dir: impl AsRef<Path>,
    manifest_override: Option<Manifest>,
) -> Result<ImportResult, String> {
    let dir = dir.as_ref();
    if !dir.exists() {
        return Err(format!("目标路径不存在: {}", dir.display()));
    }
    if !dir.is_dir() {
        return Err(format!("目标路径不是有效目录: {}", dir.display()));
    }

    let (manifest, exe_rel_path) = match manifest_override {
        Some(m) => {
            m.validate()?;
            let exe = match &m.runtime {
                Runtime::Managed { executable, .. } => executable.clone(),
                Runtime::External { .. } => {
                    return Err("无法对外部服务(External)进行本地文件分发与打包".into());
                }
            };
            (m, exe)
        }
        None => {
            let report = detect_project(dir)?;
            (report.manifest, report.executable_relative_path)
        }
    };

    if manifest.ownership() != Ownership::Managed {
        return Err("无法对外部服务(External)进行本地文件分发与打包".into());
    }

    let staging_dir = tempfile::Builder::new()
        .prefix(".stage-pkg-")
        .tempdir()
        .map_err(|e| format!("无法创建临时工作包目录: {e}"))?;

    // 复制主可执行文件
    let src_exe = dir.join(&exe_rel_path);
    if !src_exe.exists() {
        return Err(format!("源可执行文件不存在: {}", src_exe.display()));
    }
    let dst_exe = staging_dir.path().join(&exe_rel_path);
    if let Some(parent) = dst_exe.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("无法创建目标子目录: {e}"))?;
    }
    fs::copy(&src_exe, &dst_exe).map_err(|e| format!("复制可执行文件失败: {e}"))?;

    // 复制声明的资源文件（若存在）
    if let Runtime::Managed { resources, .. } = &manifest.runtime {
        for res_path in resources.keys() {
            let src_res = dir.join(res_path);
            if src_res.exists() {
                let dst_res = staging_dir.path().join(res_path);
                if let Some(parent) = dst_res.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("无法创建目标资源子目录: {e}"))?;
                }
                fs::copy(&src_res, &dst_res).map_err(|e| format!("复制资源文件失败: {e}"))?;
            }
        }
    }

    // 若源目录存在 LICENSE，复制到临时目录中
    for license_name in ["LICENSE", "LICENSE.txt", "LICENSE.md"] {
        let src_license = dir.join(license_name);
        if src_license.is_file() {
            let _ = fs::copy(&src_license, staging_dir.path().join(license_name));
            break;
        }
    }

    // 将 manifest 序列化为 manifest.yaml 写入临时目录
    let manifest_yaml =
        serde_yaml_ng::to_string(&manifest).map_err(|e| format!("无法序列化 manifest: {e}"))?;
    fs::write(
        staging_dir.path().join("manifest.yaml"),
        manifest_yaml.as_bytes(),
    )
    .map_err(|e| format!("写入 manifest.yaml 失败: {e}"))?;

    // 原子注册进 Registry
    registry.import(staging_dir.path().join("manifest.yaml"))
}
