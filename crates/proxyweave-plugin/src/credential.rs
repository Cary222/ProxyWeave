//! Credential Vault Contract and Environment Variable Injection (spec v9 / Phase 3).
//!
//! Architecture invariants:
//! 1. Zero-DB: Credentials are never stored in databases (SQLite/SQLx/etc.).
//!    Storage is backed purely by OS-native Keyring (`quotio_platform`) or in-memory vaults.
//! 2. Secure Injection: Credentials are dynamically injected into managed child processes
//!    via standard `PROXYWEAVE_CREDENTIAL_*` environment variables at spawn time.
//! 3. Startup Gate: If a plugin manifest declares `required_secrets`, startup is strictly
//!    blocked if any required credential is missing.
//! 4. External Isolation: External plugins are strictly isolated; credential injection
//!    and lifecycle mutations are forbidden.
//! 5. Masked Projection: Sensitive credential values are masked (e.g. `sk-••••••`) before
//!    being displayed in UI/logs, preventing credential leakage.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

use crate::manifest::{Manifest, Ownership};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialError {
    MissingRequiredCredential {
        plugin_id: String,
        missing_keys: Vec<String>,
    },
    ExternalCredentialsForbidden,
    VaultError(String),
}

impl std::fmt::Display for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequiredCredential {
                plugin_id,
                missing_keys,
            } => {
                write!(
                    f,
                    "Plugin '{}' cannot start: missing required credential(s): {:?}",
                    plugin_id, missing_keys
                )
            }
            Self::ExternalCredentialsForbidden => {
                write!(
                    f,
                    "External plugins cannot have credential projections or lifecycle mutations"
                )
            }
            Self::VaultError(msg) => write!(f, "Credential vault error: {}", msg),
        }
    }
}

impl std::error::Error for CredentialError {}

/// Pluggable credential vault contract.
pub trait CredentialVault: Send + Sync {
    fn get_credential(&self, plugin_id: &str, key: &str) -> Result<Option<String>, String>;
    fn set_credential(&self, plugin_id: &str, key: &str, value: &str) -> Result<(), String>;
    fn delete_credential(&self, plugin_id: &str, key: &str) -> Result<(), String>;
    fn list_credentials(&self, plugin_id: &str) -> Result<Vec<String>, String>;
}

/// Pure in-memory credential vault for testing and ephemeral secure sessions (Zero-DB).
#[derive(Debug, Default, Clone)]
pub struct InMemoryVault {
    storage: Arc<RwLock<HashMap<(String, String), String>>>,
}

impl InMemoryVault {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl CredentialVault for InMemoryVault {
    fn get_credential(&self, plugin_id: &str, key: &str) -> Result<Option<String>, String> {
        let lock = self
            .storage
            .read()
            .map_err(|e| format!("Lock poisoned: {}", e))?;
        Ok(lock.get(&(plugin_id.to_string(), key.to_string())).cloned())
    }

    fn set_credential(&self, plugin_id: &str, key: &str, value: &str) -> Result<(), String> {
        let mut lock = self
            .storage
            .write()
            .map_err(|e| format!("Lock poisoned: {}", e))?;
        lock.insert((plugin_id.to_string(), key.to_string()), value.to_string());
        Ok(())
    }

    fn delete_credential(&self, plugin_id: &str, key: &str) -> Result<(), String> {
        let mut lock = self
            .storage
            .write()
            .map_err(|e| format!("Lock poisoned: {}", e))?;
        lock.remove(&(plugin_id.to_string(), key.to_string()));
        Ok(())
    }

    fn list_credentials(&self, plugin_id: &str) -> Result<Vec<String>, String> {
        let lock = self
            .storage
            .read()
            .map_err(|e| format!("Lock poisoned: {}", e))?;
        let keys = lock
            .keys()
            .filter(|(p, _)| p == plugin_id)
            .map(|(_, k)| k.clone())
            .collect();
        Ok(keys)
    }
}

/// OS-native secure keyring vault backed by `quotio_platform` (Windows Credential Manager / macOS Keychain / Secret Service).
#[derive(Debug, Default, Clone)]
pub struct OsKeyringVault;

impl OsKeyringVault {
    pub fn new() -> Self {
        Self
    }
}

impl CredentialVault for OsKeyringVault {
    fn get_credential(&self, plugin_id: &str, key: &str) -> Result<Option<String>, String> {
        quotio_platform::get_plugin_credential(plugin_id, key)
    }

    fn set_credential(&self, plugin_id: &str, key: &str, value: &str) -> Result<(), String> {
        quotio_platform::set_plugin_credential(plugin_id, key, value)
    }

    fn delete_credential(&self, plugin_id: &str, key: &str) -> Result<(), String> {
        quotio_platform::delete_plugin_credential(plugin_id, key)
    }

    fn list_credentials(&self, _plugin_id: &str) -> Result<Vec<String>, String> {
        // OS Keyring does not support enumeration without manifest declaration
        Ok(Vec::new())
    }
}

/// Masks sensitive credential strings to prevent accidental exposure in UI/logs.
pub fn mask_credential_value(val: &str) -> String {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.len() <= 6 {
        return "••••••".to_string();
    }
    let prefix = if trimmed.starts_with("sk-") && trimmed.len() > 10 {
        &trimmed[..5]
    } else {
        &trimmed[..3]
    };
    let suffix = &trimmed[trimmed.len().saturating_sub(4)..];
    format!("{}••••{}", prefix, suffix)
}

/// Sanitizes credential keys into standard environment variable names: `PROXYWEAVE_CREDENTIAL_<NAME>`.
pub fn sanitize_credential_env_name(key: &str) -> String {
    let sanitized: String = key
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
        .collect();
    format!("PROXYWEAVE_CREDENTIAL_{}", sanitized)
}

/// Resolves required credentials for a plugin against a vault, verifying all requirements.
pub fn resolve_and_verify_credentials<V: CredentialVault>(
    vault: &V,
    manifest: &Manifest,
) -> Result<HashMap<String, String>, CredentialError> {
    if manifest.ownership() == Ownership::External {
        if !manifest.configuration.required_secrets.is_empty() {
            return Err(CredentialError::ExternalCredentialsForbidden);
        }
        return Ok(HashMap::new());
    }

    let mut resolved = HashMap::new();
    let mut missing = Vec::new();

    for key in &manifest.configuration.required_secrets {
        match vault.get_credential(&manifest.metadata.id, key) {
            Ok(Some(val)) if !val.trim().is_empty() => {
                resolved.insert(key.clone(), val.trim().to_string());
            }
            _ => {
                missing.push(key.clone());
            }
        }
    }

    if !missing.is_empty() {
        return Err(CredentialError::MissingRequiredCredential {
            plugin_id: manifest.metadata.id.clone(),
            missing_keys: missing,
        });
    }

    Ok(resolved)
}
