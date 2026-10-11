use crate::manifest::Manifest;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Context containing variables for rendering plugin templates.
#[derive(Debug, Clone)]
pub struct ProjectionContext {
    pub port: u16,
    pub plugin_id: String,
    pub plugin_data_dir: PathBuf,
    pub auth_dir: PathBuf,
    pub config_path: PathBuf,
    pub plugin_api_key: String,
    pub secrets: BTreeMap<String, String>,
}

impl ProjectionContext {
    pub fn new(
        plugin_id: impl Into<String>,
        port: u16,
        plugin_data_dir: PathBuf,
        api_key: Option<&str>,
        secrets: BTreeMap<String, String>,
    ) -> Self {
        let plugin_id = plugin_id.into();
        let auth_dir = plugin_data_dir.join("auths");
        let config_path = plugin_data_dir.join("config.json");
        let plugin_api_key = api_key
            .unwrap_or("proxyweave-internal-managed-key")
            .to_string();
        Self {
            port,
            plugin_id,
            plugin_data_dir,
            auth_dir,
            config_path,
            plugin_api_key,
            secrets,
        }
    }

    /// Renders placeholders within a template string.
    pub fn render_str(&self, template: &str) -> Result<String, String> {
        let mut rendered = template.to_string();
        rendered = rendered.replace("{PORT}", &self.port.to_string());
        rendered = rendered.replace("{CONFIG_PATH}", &self.config_path.to_string_lossy());
        rendered = rendered.replace("{DATA_DIR}", &self.plugin_data_dir.to_string_lossy());
        rendered = rendered.replace("{AUTH_DIR}", &self.auth_dir.to_string_lossy());
        rendered = rendered.replace("{PLUGIN_API_KEY}", &self.plugin_api_key);

        for (k, v) in &self.secrets {
            rendered = rendered.replace(&format!("{{SECRET:{}}}", k), v);
        }

        if let Some(pos) = rendered.find("{SECRET:") {
            let end = rendered[pos..]
                .find('}')
                .map(|e| pos + e)
                .unwrap_or(rendered.len());
            return Err(format!(
                "Unresolved secret placeholder in template: {}",
                &rendered[pos..=end.min(rendered.len() - 1)]
            ));
        }

        Ok(rendered)
    }

    /// Recursively renders placeholders within a JSON Value.
    pub fn render_json_value(&self, value: &Value) -> Result<Value, String> {
        match value {
            Value::String(s) => Ok(Value::String(self.render_str(s)?)),
            Value::Array(arr) => {
                let mut out = Vec::with_capacity(arr.len());
                for item in arr {
                    out.push(self.render_json_value(item)?);
                }
                Ok(Value::Array(out))
            }
            Value::Object(map) => {
                let mut out = serde_json::Map::with_capacity(map.len());
                for (k, v) in map {
                    out.insert(k.clone(), self.render_json_value(v)?);
                }
                Ok(Value::Object(out))
            }
            other => Ok(other.clone()),
        }
    }
}

/// Result of projecting configuration and credential files for a managed plugin.
#[derive(Debug, Clone)]
pub struct ProjectedFiles {
    pub config_path: Option<PathBuf>,
    pub credential_paths: Vec<PathBuf>,
}

/// Generic projection engine: renders `config_template` and `credential_files`
/// strictly from Manifest declarations, writing every sensitive file using
/// atomic writes with Windows Protected DACL (or Unix 0o600).
///
/// Contains ZERO hardcoded logic or special branches for any plugin ID.
pub fn project_managed_configuration(
    manifest: &Manifest,
    context: &ProjectionContext,
) -> Result<ProjectedFiles, String> {
    let mut credential_paths = Vec::new();

    // 1. Project config_template if provided as an object
    let config_path = if manifest.configuration.config_template.is_object() {
        let rendered_config = context.render_json_value(&manifest.configuration.config_template)?;
        let json_bytes = serde_json::to_vec_pretty(&rendered_config)
            .map_err(|e| format!("Failed to serialize rendered config.json: {}", e))?;

        quotio_platform::atomic_write(&context.config_path, &json_bytes, true).map_err(|e| {
            format!(
                "Failed to write sensitive config.json at {}: {}",
                context.config_path.display(),
                e
            )
        })?;

        Some(context.config_path.clone())
    } else {
        None
    };

    // 2. Project all declared credential_files
    for (rel_path, template) in &manifest.configuration.credential_files {
        let target_path = context.plugin_data_dir.join(rel_path);

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create credential directory {}: {}",
                    parent.display(),
                    e
                )
            })?;
        }

        let rendered_cred = context.render_json_value(template)?;
        let json_bytes = serde_json::to_vec_pretty(&rendered_cred)
            .map_err(|e| format!("Failed to serialize credential file {}: {}", rel_path, e))?;

        quotio_platform::atomic_write(&target_path, &json_bytes, true).map_err(|e| {
            format!(
                "Failed to write sensitive credential file at {}: {}",
                target_path.display(),
                e
            )
        })?;

        credential_paths.push(target_path);
    }

    Ok(ProjectedFiles {
        config_path,
        credential_paths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_replaces_all_standard_placeholders() {
        let mut secrets = BTreeMap::new();
        secrets.insert("api_token".to_string(), "secret-val-123".to_string());

        let ctx = ProjectionContext::new(
            "custom-plugin",
            9090,
            PathBuf::from(r"C:\data\plugins\custom-plugin"),
            Some("my-key"),
            secrets,
        );

        let tpl = "http://127.0.0.1:{PORT}/dir={DATA_DIR}&cfg={CONFIG_PATH}&auth={AUTH_DIR}&key={PLUGIN_API_KEY}&sec={SECRET:api_token}";
        let rendered = ctx.render_str(tpl).unwrap();

        assert!(rendered.contains("9090"));
        assert!(rendered.contains(r"C:\data\plugins\custom-plugin"));
        assert!(rendered.contains("config.json"));
        assert!(rendered.contains("auths"));
        assert!(rendered.contains("my-key"));
        assert!(rendered.contains("secret-val-123"));
    }

    #[test]
    fn test_projection_fails_on_unresolved_secret() {
        let ctx = ProjectionContext::new(
            "custom-plugin",
            9090,
            PathBuf::from(r"C:\data\plugins\custom-plugin"),
            None,
            BTreeMap::new(),
        );

        let tpl = "token={SECRET:missing_token}";
        let res = ctx.render_str(tpl);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Unresolved secret"));
    }

    #[test]
    fn test_generic_json_rendering_and_atomic_write() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_data = dir.path().join("plugins").join("test-plug");
        std::fs::create_dir_all(&plugin_data).unwrap();

        let mut secrets = BTreeMap::new();
        secrets.insert("token".into(), "tok_xyz".into());

        let ctx = ProjectionContext::new("test-plug", 8081, plugin_data.clone(), None, secrets);

        let manifest_yaml = r#"
schema_version: '1.0'
metadata:
  id: test-plug
  name: Test Plugin
  version: 1.0.0
  license: MIT
runtime:
  ownership: managed
  executable: test.exe
  sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
network:
  health_check:
    path: /health
    expected_status: 200
    identity_header: X-Test
    identity_value: test
    timeout_ms: 1000
lifecycle:
  startup_timeout_ms: 5000
  idle_timeout_ms: null
  max_port_retries: 0
capabilities:
  protocol: openai_chat
  models_path: /v1/models
  inference_path: /v1/chat/completions
  streaming: true
configuration:
  required_secrets: [token]
  config_template:
    port: '{PORT}'
    key: '{PLUGIN_API_KEY}'
  credential_files:
    auth/token.json:
      secret_value: '{SECRET:token}'
"#;
        let manifest = Manifest::parse(manifest_yaml.as_bytes()).unwrap();
        let projected = project_managed_configuration(&manifest, &ctx).unwrap();

        assert!(projected.config_path.is_some());
        let cfg_path = projected.config_path.unwrap();
        assert!(cfg_path.exists());
        let cfg_content: Value =
            serde_json::from_str(&std::fs::read_to_string(&cfg_path).unwrap()).unwrap();
        assert_eq!(cfg_content["port"], "8081");
        assert_eq!(cfg_content["key"], "proxyweave-internal-managed-key");

        assert_eq!(projected.credential_paths.len(), 1);
        let cred_path = &projected.credential_paths[0];
        assert!(cred_path.exists());
        let cred_content: Value =
            serde_json::from_str(&std::fs::read_to_string(cred_path).unwrap()).unwrap();
        assert_eq!(cred_content["secret_value"], "tok_xyz");
    }
}
