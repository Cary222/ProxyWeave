//! Narrow management-link policy. No fetching, credentials, navigation or embedding.
use crate::{Manifest, Runtime, RuntimeStatus};
use serde::Serialize;
use url::Url;

const INVALID: &str = "invalid management entry";

pub fn validate_management_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.len() > 256
        || !path.starts_with('/')
        || path.starts_with("//")
        || !path
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/-_.".contains(&c))
        || path[1..]
            .trim_end_matches('/')
            .split('/')
            .any(|p| p == "." || p == ".." || (p.is_empty() && path != "/"))
        || path.contains("//")
    {
        return Err(INVALID.into());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ManagementLink {
    pub state: &'static str,
    pub url: Option<String>,
}

/// The caller supplies a freshly reconciled runtime status, never a cached UI port.
/// External origins come only from the validated registration (no lifecycle operation).
pub fn resolve_management(
    manifest: &Manifest,
    status: &RuntimeStatus,
) -> Result<ManagementLink, String> {
    manifest.validate().map_err(|_| INVALID.to_string())?;
    let Some(entry) = &manifest.management else {
        return Ok(ManagementLink {
            state: "undeclared",
            url: None,
        });
    };
    validate_management_path(&entry.path)?;
    let origin = match &manifest.runtime {
        Runtime::Managed { .. } => match status {
            RuntimeStatus::Ready { port, .. } | RuntimeStatus::Busy { port, .. } if *port != 0 => {
                format!("http://127.0.0.1:{port}")
            }
            _ => {
                return Ok(ManagementLink {
                    state: "unavailable",
                    url: None,
                })
            }
        },
        Runtime::External { endpoint } => endpoint.clone(),
    };
    let base = Url::parse(&origin).map_err(|_| INVALID.to_string())?;
    // Reserved app/IPC origins must never be treated as a plugin management origin.
    let host = base.host_str().ok_or(INVALID)?;
    if !matches!(base.scheme(), "http" | "https")
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
        || base.path() != "/"
        || host.ends_with(".localhost")
        || (matches!(host, "localhost" | "127.0.0.1" | "[::1]")
            && base.port_or_known_default() == Some(1420))
    {
        return Err(INVALID.into());
    }
    let url = base.join(&entry.path).map_err(|_| INVALID.to_string())?;
    if url.origin() != base.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(INVALID.into());
    }
    Ok(ManagementLink {
        state: "available",
        url: Some(url.into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_paths_only_and_errors_never_echo_input() {
        for p in ["/", "/admin", "/admin/", "/ui/index.html", "/a-b_c"] {
            assert!(validate_management_path(p).is_ok(), "{p}");
        }
        for p in [
            "",
            "admin",
            "//evil.invalid",
            "https://evil.invalid",
            "javascript:alert(1)",
            "file:///a",
            "data:text/html,x",
            "/a//",
            "/./a",
            "/a/../b",
            "/a%2fb",
            "/%0a",
            "/a?token=secret",
            "/a#secret",
            "/{SECRET:access}",
            "/a\\b",
            "/@user",
            "/a b",
            "/a\n",
            "/<script>",
            "/a\"b",
            "/a:b",
        ] {
            assert_eq!(validate_management_path(p).unwrap_err(), INVALID, "{p}");
        }
        assert!(validate_management_path(&format!("/{}", "a".repeat(256))).is_err());
    }
}
