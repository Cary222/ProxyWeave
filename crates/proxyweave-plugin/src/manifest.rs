use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, net::IpAddr, path::Path};
use url::Url;

pub const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
pub(crate) const MAX_PACKAGE_FILE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String,
    pub metadata: Metadata,
    pub runtime: Runtime,
    pub network: Network,
    pub lifecycle: Lifecycle,
    pub capabilities: Capabilities,
    #[serde(default)]
    pub configuration: Configuration,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: String,
    #[serde(default)]
    pub source_commit: Option<String>,
}

/// Ownership is explicit: an external endpoint has no executable or lifecycle controls.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "ownership", rename_all = "snake_case", deny_unknown_fields)]
pub enum Runtime {
    Managed {
        executable: String,
        sha256: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
        #[serde(default)]
        resources: BTreeMap<String, String>,
    },
    External {
        endpoint: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    Managed,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub health_check: HealthCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HealthCheck {
    pub path: String,
    pub expected_status: u16,
    pub identity_header: String,
    pub identity_value: String,
    pub timeout_ms: u64,
    /// Responsive but unavailable statuses; identity must still match exactly.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub busy_statuses: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Lifecycle {
    pub startup_timeout_ms: u64,
    pub idle_timeout_ms: Option<u64>,
    pub max_port_retries: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub protocol: String,
    pub models_path: String,
    pub inference_path: String,
    pub streaming: bool,
}

/// Non-sensitive defaults, plus separately declared projections. Secret values are never
/// accepted by the registry API; Feature 2 supplies them through its secure credential path.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    #[serde(default)]
    pub settings: BTreeMap<String, Value>,
    #[serde(default)]
    pub config_template: Value,
    #[serde(default)]
    pub credential_files: BTreeMap<String, Value>,
    #[serde(default)]
    pub required_secrets: Vec<String>,
}

impl Manifest {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() as u64 > MAX_MANIFEST_BYTES {
            return Err("manifest size outside 1..256 KiB".into());
        }
        // Direct typed parse preserves duplicate-field errors; no lossy intermediate mapping.
        // Validate mappings with the library's duplicate-entry checks (including maps
        // inside JSON templates), then deserialize the original bytes into strict types.
        serde_yaml_ng::from_slice::<serde_yaml_ng::Value>(bytes).map_err(|_| {
            "manifest YAML contains invalid or duplicate mapping entries".to_string()
        })?;
        let manifest: Self = serde_yaml_ng::from_slice(bytes).map_err(|_| {
            "manifest YAML does not match the strict Schema 1.0 contract".to_string()
        })?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn ownership(&self) -> Ownership {
        match &self.runtime {
            Runtime::Managed { .. } => Ownership::Managed,
            Runtime::External { .. } => Ownership::External,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != "1.0" {
            return Err("unsupported schema_version".into());
        }
        validate_id(&self.metadata.id)?;
        if self.metadata.name.trim().is_empty()
            || self.metadata.name.len() > 128
            || self.metadata.license.trim().is_empty()
        {
            return Err("name and license are required".into());
        }
        semver::Version::parse(&self.metadata.version)
            .map_err(|_| "version must be semantic version".to_string())?;
        if let Some(commit) = &self.metadata.source_commit {
            if commit.len() != 40 || !commit.bytes().all(|c| c.is_ascii_hexdigit()) {
                return Err("source_commit must be a full Git hash".into());
            }
        }
        let hc = &self.network.health_check;
        validate_http_path(&hc.path)?;
        if hc.expected_status != 200
            || !(1..=30_000).contains(&hc.timeout_ms)
            || hc.identity_value.is_empty()
            || !header_token(&hc.identity_header)
            || hc.identity_value.bytes().any(|c| c < 32 || c == 127)
            || hc.busy_statuses.len() > 8
            || hc
                .busy_statuses
                .iter()
                .any(|code| !(100..=599).contains(code) || *code == hc.expected_status)
            || hc
                .busy_statuses
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != hc.busy_statuses.len()
        {
            return Err(
                "health check requires HTTP 200, bounded timeout and a valid identity header"
                    .into(),
            );
        }
        if !(100..=120_000).contains(&self.lifecycle.startup_timeout_ms)
            || self.lifecycle.max_port_retries > 3
            || self.lifecycle.idle_timeout_ms.is_some_and(|n| n < 1_000)
        {
            return Err("lifecycle bounds are invalid".into());
        }
        if self.capabilities.protocol != "openai_chat" {
            return Err("only same-protocol openai_chat passthrough is supported".into());
        }
        validate_http_path(&self.capabilities.models_path)?;
        validate_http_path(&self.capabilities.inference_path)?;
        if self.configuration.required_secrets.len() > 16 {
            return Err("too many secret declarations".into());
        }
        let mut names = std::collections::BTreeSet::new();
        for name in &self.configuration.required_secrets {
            validate_id(name)?;
            if !names.insert(name) {
                return Err("duplicate secret declaration".into());
            }
        }
        validate_settings(&self.configuration.settings, &self.configuration.settings)?;
        match &self.runtime {
            Runtime::Managed {
                executable,
                sha256,
                args,
                env,
                resources,
            } => {
                validate_relative_path(executable)?;
                if !executable.ends_with(".exe") {
                    return Err("Windows executable must end with .exe".into());
                }
                validate_hash(sha256)?;
                if args.len() > 64 || env.len() > 32 || resources.len() > 32 {
                    return Err("too many package fields".into());
                }
                for arg in args {
                    validate_template(arg, &names)?;
                    if arg.contains("{SECRET:") || arg.contains("{PLUGIN_API_KEY}") {
                        return Err("secrets cannot be placed on the process command line".into());
                    }
                }
                for (key, value) in env {
                    if key.is_empty()
                        || key
                            .bytes()
                            .any(|c| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_'))
                    {
                        return Err("environment key must be uppercase ASCII".into());
                    }
                    validate_template(value, &names)?;
                }
                for (path, hash) in resources {
                    validate_relative_path(path)?;
                    validate_hash(hash)?;
                    if path.eq_ignore_ascii_case(executable)
                        || path.eq_ignore_ascii_case("manifest.yaml")
                        || path.eq_ignore_ascii_case("record.json")
                    {
                        return Err("resource collides with executable or registry metadata".into());
                    }
                }
                let mut normalized = std::collections::BTreeSet::new();
                for path in std::iter::once(executable).chain(resources.keys()) {
                    if !normalized.insert(path.to_ascii_lowercase()) {
                        return Err("case-insensitive package path collision".into());
                    }
                }
                if !self.configuration.config_template.is_object()
                    || self.configuration.credential_files.len() > 16
                {
                    return Err(
                        "managed configuration must be a JSON object with bounded projections"
                            .into(),
                    );
                }
                validate_json_template(&self.configuration.config_template, &names)?;
                for (path, value) in &self.configuration.credential_files {
                    validate_relative_path(path)?;
                    if path.eq_ignore_ascii_case("config.json")
                        || !normalized.insert(path.to_ascii_lowercase())
                    {
                        return Err(
                            "credential projection collides with config or another file".into()
                        );
                    }
                    validate_json_template(value, &names)?;
                }
            }
            Runtime::External { endpoint } => {
                let u =
                    Url::parse(endpoint).map_err(|_| "invalid external endpoint".to_string())?;
                if !matches!(u.scheme(), "http" | "https")
                    || u.host_str().is_none()
                    || !u.username().is_empty()
                    || u.password().is_some()
                    || u.query().is_some()
                    || u.fragment().is_some()
                    || u.path() != "/"
                {
                    return Err(
                        "external endpoint must be an HTTP origin without credentials/query/path"
                            .into(),
                    );
                }
                if let Some(url::Host::Ipv4(ip)) = u.host() {
                    if IpAddr::V4(ip).is_unspecified() {
                        return Err("unspecified external host is forbidden".into());
                    }
                }
                if let Some(url::Host::Ipv6(ip)) = u.host() {
                    if ip.is_unspecified() {
                        return Err("unspecified external host is forbidden".into());
                    }
                }
                if self.lifecycle.idle_timeout_ms.is_some()
                    || self.lifecycle.max_port_retries != 0
                    || self.configuration != Configuration::default()
                {
                    return Err(
                        "external endpoint cannot have managed lifecycle or credential projections"
                            .into(),
                    );
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
        || !id.as_bytes()[0].is_ascii_alphanumeric()
        || windows_reserved(id)
    {
        return Err("id must be a non-reserved lowercase ASCII identifier".into());
    }
    Ok(())
}

pub(crate) fn validate_hash(s: &str) -> Result<(), String> {
    if s.len() != 64
        || !s
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err("sha256 must be 64 lowercase hex characters".into());
    }
    Ok(())
}

pub(crate) fn validate_relative_path(s: &str) -> Result<(), String> {
    if s.is_empty()
        || s.len() > 240
        || Path::new(s).is_absolute()
        || s.contains('\\')
        || s.contains(':')
        || s.bytes()
            .any(|c| c < 32 || matches!(c, b'<' | b'>' | b'"' | b'|' | b'?' | b'*'))
    {
        return Err("package path must be a portable relative path".into());
    }
    for part in s.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with('.')
            || part.ends_with(' ')
            || windows_reserved(part.split('.').next().unwrap_or_default())
        {
            return Err("unsafe package path component".into());
        }
    }
    Ok(())
}

fn windows_reserved(s: &str) -> bool {
    let s = s.to_ascii_uppercase();
    matches!(
        s.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || (s.len() == 4
        && (s.starts_with("COM") || s.starts_with("LPT"))
        && matches!(s.as_bytes()[3], b'1'..=b'9'))
}

fn header_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&c))
}

fn validate_http_path(s: &str) -> Result<(), String> {
    if !s.starts_with('/')
        || s.starts_with("//")
        || s.contains(['?', '#', '\\'])
        || s.contains("..")
        || s.len() > 256
        || s.bytes().any(|c| c <= 32 || c >= 127)
    {
        return Err("HTTP path must be an unambiguous ASCII absolute path".into());
    }
    Ok(())
}

fn validate_template(s: &str, names: &std::collections::BTreeSet<&String>) -> Result<(), String> {
    if s.len() > 8192 || s.contains('\0') {
        return Err("template too large or contains NUL".into());
    }
    let mut rest = s;
    while let Some(start) = rest.find('{') {
        if rest[..start].contains('}') {
            return Err("unmatched template brace".into());
        }
        let end = rest[start..].find('}').ok_or("unmatched template brace")? + start;
        let token = &rest[start + 1..end];
        if !matches!(
            token,
            "PORT" | "DATA_DIR" | "CONFIG_PATH" | "AUTH_DIR" | "PLUGIN_API_KEY"
        ) && !token
            .strip_prefix("SECRET:")
            .is_some_and(|name| names.iter().any(|s| s.as_str() == name))
        {
            return Err("unknown template placeholder".into());
        }
        rest = &rest[end + 1..];
    }
    if rest.contains('}') {
        return Err("unmatched template brace".into());
    }
    Ok(())
}

fn validate_json_template(
    v: &Value,
    names: &std::collections::BTreeSet<&String>,
) -> Result<(), String> {
    match v {
        Value::String(s) => validate_template(s, names),
        Value::Array(a) => a.iter().try_for_each(|v| validate_json_template(v, names)),
        Value::Object(o) => o
            .values()
            .try_for_each(|v| validate_json_template(v, names)),
        _ => Ok(()),
    }
}

pub(crate) fn validate_settings(
    values: &BTreeMap<String, Value>,
    defaults: &BTreeMap<String, Value>,
) -> Result<(), String> {
    if values.len() != defaults.len() {
        return Err("settings must contain exactly the declared non-sensitive keys".into());
    }
    for (key, value) in values {
        let lower = key.to_ascii_lowercase();
        if [
            "secret",
            "token",
            "password",
            "api_key",
            "apikey",
            "credential",
            "authorization",
        ]
        .iter()
        .any(|s| lower.contains(s))
        {
            return Err("sensitive settings must use secure credential declarations".into());
        }
        let expected = defaults.get(key).ok_or("undeclared setting")?;
        if !(value.is_boolean() && expected.is_boolean()
            || value.is_string() && expected.is_string()
            || value.is_number() && expected.is_number())
        {
            return Err("settings must be scalar values matching their declared types".into());
        }
        if value.is_string()
            && value
                .as_str()
                .is_some_and(|s| s.len() > 4096 || s.contains('\0'))
        {
            return Err("setting string too large or contains NUL".into());
        }
    }
    Ok(())
}
