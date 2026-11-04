//! Installed acquisition facts and read-only stable-latest GitHub Release checks.
//! SHA256 evidence is integrity evidence, never a publisher signature or trust rating.
use crate::downloader::{read_response_bounded, Release, MAX_RELEASE_JSON_BYTES};
use crate::registry::{Registry, StagedPackage};
use crate::runtime::RuntimeManager;
use crate::{Ownership, PluginRecord};
use semver::Version;
use serde::{Deserialize, Serialize};

pub use crate::registry::{
    asset_matches_platform_target, valid_digest, valid_repository_part, valid_tag,
    validate_asset_filename, DigestEvidence, Immutability, InstalledSource, PackageContract,
    PluginSource, SourceProvenance,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatus {
    NotCheckable,
    UpdateAvailable,
    UpToDate,
    InstalledNewer,
    InvalidVersion,
    ReleaseChanged,
    Forbidden,
    NotFound,
    Error,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UpdateResult {
    Success {
        old_version: String,
        new_version: String,
        restarted: bool,
    },
    BlockedActiveRequests {
        active_requests: usize,
    },
    NotAllowedExternal,
    RollbackSuccess {
        error: String,
        rolled_back_to_version: String,
    },
    RollbackFailed {
        error: String,
        rollback_error: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateCheck {
    pub status: UpdateStatus,
    pub owned_update_allowed: bool,
    pub latest_tag: Option<String>,
    pub http_status: Option<u16>,
    pub rate_limit_remaining: Option<u64>,
    pub rate_limit_reset: Option<u64>,
    /// Fixed code, never remote body/URL/error text.
    pub detail: &'static str,
}
fn result(record: &PluginRecord, status: UpdateStatus, detail: &'static str) -> UpdateCheck {
    UpdateCheck {
        status,
        owned_update_allowed: record.manifest.ownership() == Ownership::Managed,
        latest_tag: None,
        http_status: None,
        rate_limit_remaining: None,
        rate_limit_reset: None,
        detail,
    }
}
fn version(raw: &str) -> Result<Version, semver::Error> {
    Version::parse(raw.strip_prefix(['v', 'V']).unwrap_or(raw))
}
fn eligibility(record: &PluginRecord) -> Option<UpdateCheck> {
    let Some(source) = &record.source else {
        return Some(result(record, UpdateStatus::NotCheckable, "source_missing"));
    };
    if source.validate(&record.fingerprint).is_err() {
        return Some(result(record, UpdateStatus::NotCheckable, "source_invalid"));
    }
    if version(&record.manifest.metadata.version).is_err() || version(&source.tag_name).is_err() {
        return Some(result(
            record,
            UpdateStatus::InvalidVersion,
            "installed_version_invalid",
        ));
    }
    // A package/tag disagreement must not silently select either version as installed truth.
    if version(&record.manifest.metadata.version)
        .ok()
        .map(|v| v.cmp_precedence(&version(&source.tag_name).unwrap()))
        != Some(std::cmp::Ordering::Equal)
    {
        return Some(result(
            record,
            UpdateStatus::InvalidVersion,
            "installed_tag_mismatch",
        ));
    }
    None
}
/// Pure evaluation seam. No source mutation, asset download, installation or process operation.
pub fn evaluate_release(record: &PluginRecord, release: &Release) -> UpdateCheck {
    if let Some(out) = eligibility(record) {
        return out;
    }
    let source = record.source.as_ref().unwrap();
    let Some(tag) = release.tag_name.as_deref().filter(|t| valid_tag(t)) else {
        return result(
            record,
            UpdateStatus::InvalidVersion,
            "latest_version_invalid",
        );
    };
    let Ok(latest) = version(tag) else {
        return result(
            record,
            UpdateStatus::InvalidVersion,
            "latest_version_invalid",
        );
    };
    if release.draft || release.prerelease || !latest.pre.is_empty() || release.id == 0 {
        return result(record, UpdateStatus::Error, "not_stable_release");
    }
    let mut out = result(record, UpdateStatus::UpToDate, "stable_latest");
    out.latest_tag = Some(tag.to_string());
    out.status = match version(&record.manifest.metadata.version)
        .unwrap()
        .cmp_precedence(&latest)
    {
        std::cmp::Ordering::Less => UpdateStatus::UpdateAvailable,
        std::cmp::Ordering::Greater => UpdateStatus::InstalledNewer,
        std::cmp::Ordering::Equal => {
            let assets: Vec<_> = release
                .assets
                .iter()
                .filter(|a| {
                    asset_matches_platform_target(&a.name, &source.target_os, &source.target_arch)
                })
                .collect();
            if assets.len() != 1 {
                return result(record, UpdateStatus::Error, "asset_missing_or_ambiguous");
            }
            let asset = assets[0];
            if crate::downloader::validate_asset_filename(&asset.name).is_err() || asset.id == 0 {
                return result(record, UpdateStatus::Error, "asset_invalid");
            }
            let changed_digest = match asset.digest.as_deref() {
                None => false, // unknown is not a verified equality claim
                Some(raw) => match raw.strip_prefix("sha256:").filter(|s| valid_digest(s)) {
                    Some(hash) => !hash.eq_ignore_ascii_case(&source.archive_sha256),
                    None => {
                        return result(record, UpdateStatus::Error, "digest_unsupported_or_invalid")
                    }
                },
            };
            if release.id != source.release_id
                || tag != source.tag_name
                || asset.id != source.asset_id
                || asset.name != source.asset_name
                || changed_digest
            {
                UpdateStatus::ReleaseChanged
            } else {
                out.detail = "identity_unchanged_digest_not_revalidated";
                UpdateStatus::UpToDate
            }
        }
    };
    out
}

/// Check only GitHub's stable `releases/latest` for the persisted repository.
pub async fn check_plugin_update(record: &PluginRecord) -> UpdateCheck {
    if let Some(out) = eligibility(record) {
        return out;
    }
    let source = record.source.as_ref().unwrap();
    let client = match reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(_) => return result(record, UpdateStatus::Error, "client_error"),
    };
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        source.owner, source.repo
    );
    check_plugin_update_at(record, &client, &url).await
}
pub async fn check_plugin_update_at(
    record: &PluginRecord,
    client: &reqwest::Client,
    endpoint: &str,
) -> UpdateCheck {
    if let Some(out) = eligibility(record) {
        return out;
    }
    let response = match client
        .get(endpoint)
        .header("User-Agent", "proxyweave-desktop")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return result(
                record,
                UpdateStatus::Error,
                if e.is_timeout() {
                    "timeout"
                } else {
                    "network_error"
                },
            )
        }
    };
    let status = response.status().as_u16();
    let remaining = response
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|s| s.to_str().ok())
        .and_then(|s| s.parse().ok());
    let reset = response
        .headers()
        .get("x-ratelimit-reset")
        .and_then(|s| s.to_str().ok())
        .and_then(|s| s.parse().ok());
    let mut out = if status == 403 {
        result(record, UpdateStatus::Forbidden, "http_forbidden")
    } else if status == 404 {
        result(
            record,
            UpdateStatus::NotFound,
            "release_not_found_or_inaccessible",
        )
    } else if !(200..300).contains(&status) {
        result(record, UpdateStatus::Error, "http_error")
    } else {
        match read_response_bounded(response, MAX_RELEASE_JSON_BYTES).await {
            Err(_) => result(record, UpdateStatus::Error, "release_body_error"),
            Ok(bytes) => match serde_json::from_slice::<Release>(&bytes) {
                Ok(release) => evaluate_release(record, &release),
                Err(_) => result(record, UpdateStatus::Error, "invalid_release_json"),
            },
        }
    };
    out.http_status = Some(status);
    out.rate_limit_remaining = remaining;
    out.rate_limit_reset = reset;
    out
}

/// Disabled until the maintenance and durable activation safety contract is validated.
pub async fn apply_plugin_update(
    registry: &Registry,
    runtime: &RuntimeManager,
    id: &str,
    staged: StagedPackage,
    api_key: Option<&str>,
) -> Result<UpdateResult, String> {
    let _ = (registry, runtime, id, staged, api_key);
    Err("update_unavailable_pending_safety_validation".into())
}

pub async fn apply_plugin_update_with_backend<F, Fut>(
    registry: &Registry,
    runtime: &RuntimeManager,
    id: &str,
    staged: StagedPackage,
    api_key: Option<&str>,
    set_backend: F,
) -> Result<UpdateResult, String>
where
    F: FnMut(PluginRecord) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let _ = (registry, runtime, id, staged, api_key, set_backend);
    Err("update_unavailable_pending_safety_validation".into())
}
