//! Integration tests for Credential Vault Contract & Security Modal (spec v9 / Phase 3).
//!
//! Verifies:
//! 1. In-memory vault isolation: credentials stored for plugin A are isolated from plugin B (Zero-DB).
//! 2. Sensitive credential masking: tokens and API keys are masked before display, never leaking plaintext.
//! 3. Missing required credentials block startup: plugins declaring `required_secrets` fail fast if missing.
//! 4. Standard environment variable sanitization: credentials map to `PROXYWEAVE_CREDENTIAL_<NAME>`.
//! 5. External plugin isolation: external plugins reject credential projections and remain lifecycle immutable.
//! 6. Zero-DB invariant: verifies no database crates (sqlite, sqlx, diesel, etc.) exist in the workspace.

use proxyweave_plugin::{
    credential::{
        mask_credential_value, resolve_and_verify_credentials, sanitize_credential_env_name,
        CredentialError, CredentialVault, InMemoryVault,
    },
    Manifest, Ownership, PluginError, PluginSupervisor,
};
use std::collections::HashMap;

const TEST_MANAGED_WITH_SECRETS: &str = r#"{
  "schema_version": "1.0",
  "metadata": {
    "id": "managed-with-secrets",
    "name": "Managed Plugin With Secrets",
    "version": "1.0.0",
    "license": "MIT"
  },
  "runtime": {
    "ownership": "managed",
    "executable": "bin/test.exe",
    "sha256": "38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485",
    "args": ["-listen", "127.0.0.1:{PORT}"]
  },
  "network": {
    "health_check": {
      "path": "/healthz",
      "expected_status": 200,
      "identity_header": "X-Service",
      "identity_value": "test-service",
      "timeout_ms": 2000,
      "busy_statuses": [503]
    }
  },
  "lifecycle": {
    "startup_timeout_ms": 5000,
    "idle_timeout_ms": null,
    "max_port_retries": 3
  },
  "capabilities": {
    "protocol": "openai_chat",
    "models_path": "/v1/models",
    "inference_path": "/v1/chat/completions",
    "streaming": true
  },
  "configuration": {
    "settings": {},
    "config_template": null,
    "credential_files": {},
    "required_secrets": ["OPENAI_API_KEY", "ANTHROPIC_KEY"]
  }
}"#;

const TEST_EXTERNAL_MANIFEST: &str = r#"{
  "schema_version": "1.0",
  "metadata": {
    "id": "external-service",
    "name": "External Service",
    "version": "1.0.0",
    "license": "MIT"
  },
  "runtime": {
    "ownership": "external",
    "endpoint": "http://127.0.0.1:28317"
  },
  "network": {
    "health_check": {
      "path": "/health",
      "expected_status": 200,
      "identity_header": "X-Service",
      "identity_value": "external-service",
      "timeout_ms": 2000,
      "busy_statuses": [503]
    }
  },
  "lifecycle": {
    "startup_timeout_ms": 5000,
    "idle_timeout_ms": null,
    "max_port_retries": 0
  },
  "capabilities": {
    "protocol": "openai_chat",
    "models_path": "/v1/models",
    "inference_path": "/v1/chat/completions",
    "streaming": true
  },
  "configuration": {
    "settings": {},
    "config_template": null,
    "credential_files": {},
    "required_secrets": []
  }
}"#;

#[test]
fn test_in_memory_vault_isolation() {
    let vault = InMemoryVault::new();
    let plugin_a = "plugin-a";
    let plugin_b = "plugin-b";

    // Set credentials for Plugin A
    vault
        .set_credential(plugin_a, "API_KEY", "secret-val-a")
        .unwrap();
    assert_eq!(
        vault.get_credential(plugin_a, "API_KEY").unwrap(),
        Some("secret-val-a".to_string())
    );

    // Verify Plugin B cannot read Plugin A's credentials
    assert_eq!(vault.get_credential(plugin_b, "API_KEY").unwrap(), None);

    // Verify deletion
    vault.delete_credential(plugin_a, "API_KEY").unwrap();
    assert_eq!(vault.get_credential(plugin_a, "API_KEY").unwrap(), None);
}

#[test]
fn test_credential_masking_security() {
    // Normal long sk key
    let masked_sk = mask_credential_value("sk-proj-1234567890abcdef1234");
    assert!(masked_sk.starts_with("sk-pr"));
    assert!(masked_sk.ends_with("1234"));
    assert!(masked_sk.contains("••••"));
    assert!(!masked_sk.contains("1234567890abcdef"));

    // Short secret
    let masked_short = mask_credential_value("secret");
    assert_eq!(masked_short, "••••••");

    // Empty secret
    let masked_empty = mask_credential_value("");
    assert_eq!(masked_empty, "");
}

#[test]
fn test_missing_required_credential_blocks_startup() {
    let manifest: Manifest = serde_json::from_str(TEST_MANAGED_WITH_SECRETS).unwrap();
    let vault = InMemoryVault::new();

    // 1. None provided -> blocked
    let res = resolve_and_verify_credentials(&vault, &manifest);
    match res {
        Err(CredentialError::MissingRequiredCredential {
            plugin_id,
            missing_keys,
        }) => {
            assert_eq!(plugin_id, "managed-with-secrets");
            assert!(missing_keys.contains(&"OPENAI_API_KEY".to_string()));
            assert!(missing_keys.contains(&"ANTHROPIC_KEY".to_string()));
        }
        other => panic!("Expected MissingRequiredCredential error, got: {:?}", other),
    }

    // 2. Partial provided -> still blocked
    vault
        .set_credential("managed-with-secrets", "OPENAI_API_KEY", "sk-test-key")
        .unwrap();
    let res2 = resolve_and_verify_credentials(&vault, &manifest);
    match res2 {
        Err(CredentialError::MissingRequiredCredential { missing_keys, .. }) => {
            assert_eq!(missing_keys, vec!["ANTHROPIC_KEY".to_string()]);
        }
        other => panic!("Expected missing ANTHROPIC_KEY, got: {:?}", other),
    }

    // 3. All provided -> succeeds
    vault
        .set_credential("managed-with-secrets", "ANTHROPIC_KEY", "sk-ant-test")
        .unwrap();
    let resolved = resolve_and_verify_credentials(&vault, &manifest).unwrap();
    assert_eq!(resolved.get("OPENAI_API_KEY").unwrap(), "sk-test-key");
    assert_eq!(resolved.get("ANTHROPIC_KEY").unwrap(), "sk-ant-test");
}

#[test]
fn test_env_var_sanitization_and_injection() {
    assert_eq!(
        sanitize_credential_env_name("OPENAI_API_KEY"),
        "PROXYWEAVE_CREDENTIAL_OPENAI_API_KEY"
    );
    assert_eq!(
        sanitize_credential_env_name("my-custom.token_v1"),
        "PROXYWEAVE_CREDENTIAL_MY_CUSTOM_TOKEN_V1"
    );

    let supervisor = PluginSupervisor::new();
    let mut creds = HashMap::new();
    creds.insert("api-key".to_string(), "val-123".to_string());
    let envs = supervisor.prepare_credentials(&creds);
    assert_eq!(
        envs.get("PROXYWEAVE_CREDENTIAL_API_KEY").unwrap(),
        "val-123"
    );
}

#[test]
fn test_external_plugin_credential_isolation() {
    let manifest: Manifest = serde_json::from_str(TEST_EXTERNAL_MANIFEST).unwrap();
    assert_eq!(manifest.ownership(), Ownership::External);

    let vault = InMemoryVault::new();
    let res = resolve_and_verify_credentials(&vault, &manifest);
    assert!(
        res.is_ok(),
        "External with no required secrets resolves cleanly"
    );

    // Supervisor rejects spawn/stop/kill on external plugins
    let supervisor = PluginSupervisor::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let creds = HashMap::new();
        let spawn_res = supervisor.spawn(&manifest, None, &creds).await;
        assert_eq!(
            spawn_res.err(),
            Some(PluginError::ExternalLifecycleImmutable)
        );
    });
}

#[test]
fn test_zero_db_credential_invariant() {
    // Assert no persistent database dependencies exist in workspace manifests or credential module
    let credential_src = include_str!("../crates/proxyweave-plugin/src/credential.rs");
    let plugin_toml = include_str!("../crates/proxyweave-plugin/Cargo.toml");
    let root_toml = include_str!("../Cargo.toml");
    let forbidden_db = [
        "sqlx", "rusqlite", "sqlite3", "diesel", "sea-orm", "mongodb",
    ];
    for db in forbidden_db {
        assert!(
            !plugin_toml.contains(db) && !root_toml.contains(db) && !credential_src.contains(db),
            "RED LINE VIOLATION: Zero-DB required, found database dependency: {}",
            db
        );
    }
}
