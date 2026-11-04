use super::{reject_existing_links, InstalledSource};
use crate::manifest::validate_id;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    time::Duration,
};

pub const ACTIVATION_SCHEMA_VERSION: u32 = 1;
pub const MAX_ACTIVATION_BYTES: u64 = 512 * 1024;

/// Validates a version key for directory naming under `packages/<plugin_id>/versions/<version_key>/`.
/// Rejects path traversal, special characters, and non-conforming formats.
pub fn validate_version_key(key: &str) -> Result<(), String> {
    if key.is_empty() || key.len() > 64 {
        return Err("version key must be between 1 and 64 characters".into());
    }
    if key.starts_with('.') || key.ends_with('.') || key.starts_with('-') || key.ends_with('-') {
        return Err("version key cannot start or end with dot or dash".into());
    }
    if key.contains('/') || key.contains('\\') || key.contains("..") {
        return Err("version key cannot contain path separators or parent traversals".into());
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err(
            "version key contains invalid characters (only alphanumeric, '.', '-', '_' allowed)"
                .into(),
        );
    }
    Ok(())
}

/// Exclusive file lock protecting `activation.json` read/write and version transitions.
/// Retains the `.lock` file across drops and owner deaths.
pub struct PluginFileLock {
    plugin_id: String,
    path: PathBuf,
    _file: File,
}

impl PluginFileLock {
    /// Attempts to acquire an exclusive lock on `<container_dir>/.lock` for the given plugin without blocking.
    pub fn try_acquire(container_dir: &Path, plugin_id: &str) -> Result<Self, String> {
        validate_id(plugin_id)?;
        reject_existing_links(container_dir)?;
        let lock_path = container_dir.join(".lock");
        if lock_path.exists() {
            reject_existing_links(&lock_path)?;
        }

        #[cfg(windows)]
        let file_res = {
            use std::os::windows::fs::OpenOptionsExt;
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .share_mode(0)
                .open(&lock_path)
        };

        #[cfg(not(windows))]
        let file_res = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path);

        match file_res {
            Ok(file) => Ok(Self {
                plugin_id: plugin_id.to_string(),
                path: lock_path,
                _file: file,
            }),
            Err(e) => Err(format!(
                "plugin lock busy on {} for plugin '{}': {e}",
                lock_path.display(),
                plugin_id
            )),
        }
    }

    /// Acquires an exclusive lock, retrying until timeout is reached.
    pub fn acquire_with_timeout(
        container_dir: &Path,
        plugin_id: &str,
        timeout: Duration,
    ) -> Result<Self, String> {
        let start = std::time::Instant::now();
        loop {
            match Self::try_acquire(container_dir, plugin_id) {
                Ok(lock) => return Ok(lock),
                Err(e) => {
                    if start.elapsed() >= timeout {
                        return Err(format!("lock timeout after {:?}: {e}", timeout));
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
            }
        }
    }

    pub fn acquire(container_dir: &Path, plugin_id: &str) -> Result<Self, String> {
        Self::acquire_with_timeout(container_dir, plugin_id, Duration::from_millis(3000))
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn assert_plugin(&self, expected_plugin_id: &str) -> Result<(), String> {
        if self.plugin_id != expected_plugin_id {
            return Err(format!(
                "lock plugin_id mismatch: lock held for '{}', but operation targeted '{}'",
                self.plugin_id, expected_plugin_id
            ));
        }
        Ok(())
    }
}

/// Generation and revision stamp ensuring strict CAS checking and ABA prevention.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Stamp {
    pub generation: u64,
    pub revision: u64,
    pub fingerprint: String,
}

impl Stamp {
    pub fn new(generation: u64, revision: u64, fingerprint: impl Into<String>) -> Self {
        Self {
            generation,
            revision,
            fingerprint: fingerprint.into(),
        }
    }

    pub fn matches(&self, other: &Stamp) -> bool {
        self.generation == other.generation
            && self.revision == other.revision
            && self.fingerprint == other.fingerprint
    }

    pub fn assert_match(&self, expected: &Stamp) -> Result<(), String> {
        if self.generation != expected.generation
            || self.revision != expected.revision
            || self.fingerprint != expected.fingerprint
        {
            return Err(format!(
                "CAS stamp mismatch: current (gen: {}, rev: {}, fp: {}) does not match expected (gen: {}, rev: {}, fp: {})",
                self.generation, self.revision, self.fingerprint,
                expected.generation, expected.revision, expected.fingerprint
            ));
        }
        Ok(())
    }

    /// Increments generation, resetting revision to 0. Used for commits and rollbacks.
    /// Rollback with a new generation prevents ABA race conditions.
    pub fn next_generation(&self, new_fingerprint: impl Into<String>) -> Result<Self, String> {
        let generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| "generation counter overflow".to_string())?;
        Ok(Self {
            generation,
            revision: 0,
            fingerprint: new_fingerprint.into(),
        })
    }

    /// Increments revision within the current generation. Used for intermediate phase
    /// transitions and settings updates.
    pub fn next_revision(&self) -> Result<Self, String> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| "revision counter overflow".to_string())?;
        Ok(Self {
            generation: self.generation,
            revision,
            fingerprint: self.fingerprint.clone(),
        })
    }
}

/// Location of a physical package version on disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VersionLocation {
    /// In an immutable version sub-directory: `packages/<plugin_id>/versions/<version_key>`
    VersionKey { version_key: String },
    /// In the legacy package directory: `packages/<plugin_id>`
    Legacy,
}

/// Version pointer referencing a physical package and its manifest fingerprint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VersionPointer {
    pub location: VersionLocation,
    pub fingerprint: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<InstalledSource>,
}

impl VersionPointer {
    /// Resolves the absolute path to the package directory within the plugin container directory.
    pub fn resolve_package_dir(&self, container_dir: &Path) -> Result<PathBuf, String> {
        match &self.location {
            VersionLocation::VersionKey { version_key } => {
                validate_version_key(version_key)?;
                let path = container_dir.join("versions").join(version_key);
                Ok(path)
            }
            VersionLocation::Legacy => Ok(container_dir.to_path_buf()),
        }
    }
}

/// Lifecycle phase of the activation envelope.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivationPhase {
    /// Normal operational steady state.
    Committed,
    /// Candidate package published, waiting for stop/switch.
    Prepared,
    /// Old instance stopped, waiting for activation pointer switch.
    Stopped,
    /// Pointer switched to candidate, waiting for health check verification.
    Switched,
    /// Probing candidate instance health.
    HealthPending,
    /// Candidate failed start/health check, undergoing rollback.
    RollbackPending,
    /// Inconsistent or corrupted state requiring manual recovery.
    RecoveryRequired,
}

impl ActivationPhase {
    pub fn is_uncommitted(&self) -> bool {
        matches!(
            self,
            ActivationPhase::Prepared
                | ActivationPhase::Stopped
                | ActivationPhase::Switched
                | ActivationPhase::HealthPending
                | ActivationPhase::RollbackPending
        )
    }
}

/// The single authoritative activation pointer and journal on disk (`activation.json`).
/// Contains zero plaintext credentials, API keys, or sensitive configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActivationEnvelope {
    pub schema_version: u32,
    pub plugin_id: String,
    pub txn_id: String,
    pub phase: ActivationPhase,
    pub current: VersionPointer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmed_old: Option<VersionPointer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate: Option<VersionPointer>,
    pub stamp: Stamp,
    #[serde(default)]
    pub settings: BTreeMap<String, Value>,
}

impl ActivationEnvelope {
    pub fn new_legacy_initial(
        plugin_id: &str,
        fingerprint: &str,
        version: &str,
        source: Option<InstalledSource>,
        settings: BTreeMap<String, Value>,
    ) -> Result<Self, String> {
        validate_id(plugin_id)?;
        let current = VersionPointer {
            location: VersionLocation::Legacy,
            fingerprint: fingerprint.to_string(),
            version: version.to_string(),
            source,
        };
        let stamp = Stamp {
            generation: 1,
            revision: 0,
            fingerprint: fingerprint.to_string(),
        };
        let envelope = Self {
            schema_version: ACTIVATION_SCHEMA_VERSION,
            plugin_id: plugin_id.to_string(),
            txn_id: generate_txn_id("init", plugin_id),
            phase: ActivationPhase::Committed,
            current,
            confirmed_old: None,
            candidate: None,
            stamp,
            settings,
        };
        envelope.validate(plugin_id)?;
        Ok(envelope)
    }

    pub fn new_version_initial(
        plugin_id: &str,
        version_key: &str,
        fingerprint: &str,
        version: &str,
        source: Option<InstalledSource>,
        settings: BTreeMap<String, Value>,
    ) -> Result<Self, String> {
        validate_id(plugin_id)?;
        validate_version_key(version_key)?;
        let current = VersionPointer {
            location: VersionLocation::VersionKey {
                version_key: version_key.to_string(),
            },
            fingerprint: fingerprint.to_string(),
            version: version.to_string(),
            source,
        };
        let stamp = Stamp {
            generation: 1,
            revision: 0,
            fingerprint: fingerprint.to_string(),
        };
        let envelope = Self {
            schema_version: ACTIVATION_SCHEMA_VERSION,
            plugin_id: plugin_id.to_string(),
            txn_id: generate_txn_id("init", plugin_id),
            phase: ActivationPhase::Committed,
            current,
            confirmed_old: None,
            candidate: None,
            stamp,
            settings,
        };
        envelope.validate(plugin_id)?;
        Ok(envelope)
    }

    /// Validates structural integrity and bounds of the activation envelope.
    pub fn validate(&self, expected_plugin_id: &str) -> Result<(), String> {
        validate_id(expected_plugin_id)?;
        if self.schema_version != ACTIVATION_SCHEMA_VERSION {
            return Err(format!(
                "unsupported activation schema version: {}",
                self.schema_version
            ));
        }
        if self.plugin_id != expected_plugin_id {
            return Err(format!(
                "activation envelope plugin_id mismatch: expected {}, got {}",
                expected_plugin_id, self.plugin_id
            ));
        }
        if self.txn_id.is_empty() || self.txn_id.len() > 128 {
            return Err("activation envelope txn_id must be between 1 and 128 characters".into());
        }
        if self.current.fingerprint.is_empty() {
            return Err("current version fingerprint cannot be empty".into());
        }
        if self.current.fingerprint != self.stamp.fingerprint {
            return Err("current version fingerprint must match stamp fingerprint".into());
        }
        if let VersionLocation::VersionKey { version_key } = &self.current.location {
            validate_version_key(version_key)?;
        }
        if let Some(candidate) = &self.candidate {
            if let VersionLocation::VersionKey { version_key } = &candidate.location {
                validate_version_key(version_key)?;
            }
        }
        if let Some(old) = &self.confirmed_old {
            if let VersionLocation::VersionKey { version_key } = &old.location {
                validate_version_key(version_key)?;
            }
        }
        Ok(())
    }

    pub fn prepare_candidate(
        &mut self,
        candidate: VersionPointer,
        txn_id: impl Into<String>,
    ) -> Result<(), String> {
        if self.phase != ActivationPhase::Committed {
            return Err(format!(
                "cannot prepare candidate while in non-committed phase: {:?}",
                self.phase
            ));
        }
        self.candidate = Some(candidate);
        self.phase = ActivationPhase::Prepared;
        self.stamp = self.stamp.next_revision()?;
        self.txn_id = txn_id.into();
        self.validate(&self.plugin_id)?;
        Ok(())
    }

    pub fn mark_stopped(&mut self) -> Result<(), String> {
        if self.phase != ActivationPhase::Prepared {
            return Err(format!(
                "cannot transition to Stopped phase from {:?}",
                self.phase
            ));
        }
        self.phase = ActivationPhase::Stopped;
        self.stamp = self.stamp.next_revision()?;
        self.validate(&self.plugin_id)?;
        Ok(())
    }

    pub fn switch_to_candidate(&mut self) -> Result<(), String> {
        if self.phase != ActivationPhase::Stopped && self.phase != ActivationPhase::Prepared {
            return Err(format!(
                "cannot switch to candidate from phase {:?}",
                self.phase
            ));
        }
        let candidate = self
            .candidate
            .take()
            .ok_or_else(|| "cannot switch to candidate: no candidate staged".to_string())?;
        let old = std::mem::replace(&mut self.current, candidate);
        let next_stamp = self.stamp.next_generation(&self.current.fingerprint)?;
        self.confirmed_old = Some(old);
        self.phase = ActivationPhase::Switched;
        self.stamp = next_stamp;
        self.validate(&self.plugin_id)?;
        Ok(())
    }

    pub fn mark_health_pending(&mut self) -> Result<(), String> {
        if self.phase != ActivationPhase::Switched {
            return Err(format!(
                "cannot mark health pending from phase {:?}",
                self.phase
            ));
        }
        self.phase = ActivationPhase::HealthPending;
        self.stamp = self.stamp.next_revision()?;
        self.validate(&self.plugin_id)?;
        Ok(())
    }

    pub fn commit_active(&mut self) -> Result<(), String> {
        if self.phase != ActivationPhase::Switched && self.phase != ActivationPhase::HealthPending {
            return Err(format!(
                "cannot commit active version from phase {:?}",
                self.phase
            ));
        }
        self.confirmed_old = None;
        self.candidate = None;
        self.phase = ActivationPhase::Committed;
        self.stamp = self.stamp.next_revision()?;
        self.validate(&self.plugin_id)?;
        Ok(())
    }

    pub fn mark_rollback_pending(&mut self) -> Result<(), String> {
        if !self.phase.is_uncommitted() {
            return Err(format!(
                "cannot mark rollback pending from phase {:?}",
                self.phase
            ));
        }
        self.phase = ActivationPhase::RollbackPending;
        self.stamp = self.stamp.next_revision()?;
        self.validate(&self.plugin_id)?;
        Ok(())
    }

    pub fn complete_rollback(&mut self, container_dir: &Path) -> Result<(), String> {
        self.recover_if_uncommitted(container_dir)?;
        Ok(())
    }

    /// Recovers an uncommitted activation phase back to `confirmed_old` if available and valid.
    /// Generates a new generation to avoid ABA and sets phase to Committed.
    /// Returns Ok(true) if recovery occurred, Ok(false) if already Committed,
    /// or Err if manual recovery is required.
    pub fn recover_if_uncommitted(&mut self, container_dir: &Path) -> Result<bool, String> {
        if self.phase == ActivationPhase::Committed {
            return Ok(false);
        }
        if self.phase == ActivationPhase::RecoveryRequired {
            return Err(
                "activation envelope in recovery_required state; manual intervention required"
                    .into(),
            );
        }
        if !self.phase.is_uncommitted() {
            return Ok(false);
        }

        if self.phase == ActivationPhase::Prepared || self.phase == ActivationPhase::Stopped {
            let current_dir = self.current.resolve_package_dir(container_dir)?;
            if !current_dir.exists() {
                self.phase = ActivationPhase::RecoveryRequired;
                return Err(format!(
                    "current package directory missing at {}; manual recovery required",
                    current_dir.display()
                ));
            }
            self.candidate = None;
            self.confirmed_old = None;
            self.phase = ActivationPhase::Committed;
            self.stamp = self.stamp.next_generation(&self.current.fingerprint)?;
            self.txn_id = generate_txn_id("rec", &self.plugin_id);
            return Ok(true);
        }

        let old = match self.confirmed_old.take() {
            Some(old) => old,
            None => {
                self.phase = ActivationPhase::RecoveryRequired;
                return Err(
                    "uncommitted activation has no confirmed_old; manual recovery required".into(),
                );
            }
        };

        let old_dir = old.resolve_package_dir(container_dir)?;
        if !old_dir.exists() {
            self.phase = ActivationPhase::RecoveryRequired;
            return Err(format!(
                "confirmed_old package directory missing at {}; manual recovery required",
                old_dir.display()
            ));
        }

        let new_stamp = self.stamp.next_generation(&old.fingerprint)?;
        self.current = old;
        self.candidate = None;
        self.confirmed_old = None;
        self.phase = ActivationPhase::Committed;
        self.stamp = new_stamp;
        self.txn_id = generate_txn_id("rec", &self.plugin_id);

        Ok(true)
    }
}

/// Data migration compatibility decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityDecision {
    CompatibilityUnknown,
    Compatible,
    Incompatible,
}

/// Explicit compatibility permit governing data migration and activation.
/// In production, defaults to `CompatibilityUnknown` and cannot be constructed
/// as `Compatible`. Positive permits are restricted to unit test authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityPermit {
    decision: CompatibilityDecision,
    reason: String,
}

impl CompatibilityPermit {
    /// Production default: strictly CompatibilityUnknown.
    pub fn default_production() -> Self {
        Self {
            decision: CompatibilityDecision::CompatibilityUnknown,
            reason: "no verified data migration compatibility evidence".into(),
        }
    }

    pub fn decision(&self) -> CompatibilityDecision {
        self.decision
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn is_compatible(&self) -> bool {
        self.decision == CompatibilityDecision::Compatible
    }

    #[doc(hidden)]
    #[cfg(any(test, feature = "test-support"))]
    pub fn test_permit(reason: impl Into<String>) -> Self {
        Self {
            decision: CompatibilityDecision::Compatible,
            reason: reason.into(),
        }
    }

    #[doc(hidden)]
    #[cfg(any(test, feature = "test-support"))]
    pub fn test_incompatible(reason: impl Into<String>) -> Self {
        Self {
            decision: CompatibilityDecision::Incompatible,
            reason: reason.into(),
        }
    }
}

/// Generates a collision-resistant, bounded transaction ID for activation operations.
pub fn generate_txn_id(prefix: &str, plugin_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut hasher = Sha256::new();
    hasher.update(plugin_id.as_bytes());
    hasher.update(now.to_le_bytes());
    hasher.update(count.to_le_bytes());
    let hex = format!("{:x}", hasher.finalize());
    format!("{}-{}-{}", prefix, &hex[..8], &hex[8..16])
}
