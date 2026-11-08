//! Integration tests for Dynamic Plugin Directory Watcher & Hot Discovery (spec v8 / Round 8).
//!
//! Verifies:
//! 1. Dynamic discovery of valid manifests (Managed and External).
//! 2. External physical isolation: dynamically discovered External plugins strictly reject start/stop/kill mutation.
//! 3. Graceful degradation: corrupted, malformed, or out-of-spec manifest files are safely skipped without panics.
//! 4. Dynamic updates: modifying an existing manifest in the directory hot-reloads the updated in-memory record.
//! 5. Zero-DB invariant: all watcher states and plugin registries operate 100% in-memory without persistent databases.

use proxyweave_plugin::{watcher::DirectoryWatcher, Ownership, PluginError, PluginSupervisor};
use std::fs;
use tempfile::tempdir;

const VALID_MANAGED_MANIFEST: &str = r#"{
  "schema_version": "1.0",
  "metadata": {
    "id": "dynamic-managed-1",
    "name": "Dynamic Managed Plugin",
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
    "config_template": {},
    "credential_files": {},
    "required_secrets": []
  }
}"#;

const VALID_EXTERNAL_MANIFEST: &str = r#"{
  "schema_version": "1.0",
  "metadata": {
    "id": "dynamic-ext-1",
    "name": "Dynamic External Daemon",
    "version": "2.1.0",
    "license": "Apache-2.0"
  },
  "runtime": {
    "ownership": "external",
    "endpoint": "http://127.0.0.1:28318"
  },
  "network": {
    "health_check": {
      "path": "/health",
      "expected_status": 200,
      "identity_header": "X-Service",
      "identity_value": "external-daemon",
      "timeout_ms": 2000,
      "busy_statuses": []
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
fn test_dynamic_watcher_load_valid_manifests() {
    let dir = tempdir().expect("create temp dir");
    let managed_path = dir.path().join("managed.manifest.json");
    let external_path = dir.path().join("external.manifest.json");

    fs::write(&managed_path, VALID_MANAGED_MANIFEST).expect("write managed manifest");
    fs::write(&external_path, VALID_EXTERNAL_MANIFEST).expect("write external manifest");

    let watcher = DirectoryWatcher::new();
    watcher.add_watch_dir(dir.path().to_path_buf());

    let report = watcher.scan_all();
    assert_eq!(report.total_discovered, 2);
    assert_eq!(report.managed_count, 1);
    assert_eq!(report.external_count, 1);
    assert_eq!(report.rejected_count, 0);

    let plugins = watcher.list_discovered();
    assert_eq!(plugins.len(), 2);

    let ext = watcher
        .get_plugin("dynamic-ext-1")
        .expect("find dynamic-ext-1");
    assert_eq!(ext.manifest.metadata.name, "Dynamic External Daemon");
    assert_eq!(ext.manifest.ownership(), Ownership::External);

    let managed = watcher
        .get_plugin("dynamic-managed-1")
        .expect("find dynamic-managed-1");
    assert_eq!(managed.manifest.metadata.name, "Dynamic Managed Plugin");
    assert_eq!(managed.manifest.ownership(), Ownership::Managed);
}

#[test]
fn test_dynamic_watcher_external_physical_isolation() {
    let dir = tempdir().expect("create temp dir");
    let external_path = dir.path().join("external.manifest.json");
    fs::write(&external_path, VALID_EXTERNAL_MANIFEST).expect("write external manifest");

    let watcher = DirectoryWatcher::new();
    watcher.add_watch_dir(dir.path().to_path_buf());
    let _ = watcher.scan_all();

    let ext_rec = watcher
        .get_plugin("dynamic-ext-1")
        .expect("ext plugin exists");
    assert_eq!(ext_rec.manifest.ownership(), Ownership::External);
    assert!(watcher.is_external("dynamic-ext-1"));

    // Verify PluginSupervisor strictly blocks stop and kill for discovered external plugin
    let supervisor = PluginSupervisor::new();
    let rt = tokio::runtime::Runtime::new().expect("tokio rt");
    rt.block_on(async {
        let mut handle = None;
        let stop_err = supervisor.stop(&ext_rec.manifest, &mut handle).await;
        assert_eq!(stop_err, Err(PluginError::ExternalLifecycleImmutable));

        let kill_err = supervisor.kill(&ext_rec.manifest, &mut handle).await;
        assert_eq!(kill_err, Err(PluginError::ExternalLifecycleImmutable));
    });
}

#[test]
fn test_dynamic_watcher_corrupted_manifest_graceful_degradation() {
    let dir = tempdir().expect("create temp dir");

    // 1. Valid manifest
    let valid_path = dir.path().join("valid.manifest.json");
    fs::write(&valid_path, VALID_EXTERNAL_MANIFEST).expect("write valid");

    // 2. Corrupted JSON file
    let corrupt_path = dir.path().join("corrupted.manifest.json");
    fs::write(&corrupt_path, "{ broken json syntax ...").expect("write corrupt");

    // 3. Schema invalid file (missing required metadata)
    let invalid_schema_path = dir.path().join("invalid_schema.manifest.json");
    fs::write(
        &invalid_schema_path,
        r#"{"schema_version": "1.0", "metadata": {}}"#,
    )
    .expect("write invalid");

    let watcher = DirectoryWatcher::new();
    watcher.add_watch_dir(dir.path().to_path_buf());

    // scan_all must gracefully degrade: NO panic, record errors, keep valid plugin
    let report = watcher.scan_all();
    assert_eq!(report.total_discovered, 1);
    assert_eq!(report.rejected_count, 2);
    assert_eq!(report.errors.len(), 2);

    let discovered = watcher.list_discovered();
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].manifest.metadata.id, "dynamic-ext-1");
}

#[test]
fn test_dynamic_watcher_hot_update_manifest() {
    let dir = tempdir().expect("create temp dir");
    let manifest_path = dir.path().join("dynamic-ext.manifest.json");
    fs::write(&manifest_path, VALID_EXTERNAL_MANIFEST).expect("write initial manifest");

    let watcher = DirectoryWatcher::new();
    watcher.add_watch_dir(dir.path().to_path_buf());
    let _ = watcher.scan_all();

    let initial_rec = watcher.get_plugin("dynamic-ext-1").expect("found initial");
    assert_eq!(initial_rec.manifest.metadata.version, "2.1.0");

    // Modify manifest file with updated version
    let updated_manifest = VALID_EXTERNAL_MANIFEST.replace("2.1.0", "2.2.0");
    fs::write(&manifest_path, updated_manifest).expect("write updated manifest");

    // Rescan directory
    let report = watcher.scan_all();
    assert_eq!(report.total_discovered, 1);
    assert_eq!(report.rejected_count, 0);

    let updated_rec = watcher.get_plugin("dynamic-ext-1").expect("found updated");
    assert_eq!(updated_rec.manifest.metadata.version, "2.2.0");
    assert_ne!(initial_rec.fingerprint, updated_rec.fingerprint);
}

#[test]
fn test_dynamic_watcher_zero_db_and_architectural_invariants() {
    let watcher_src = include_str!("../crates/proxyweave-plugin/src/watcher.rs");
    let plugin_toml = include_str!("../crates/proxyweave-plugin/Cargo.toml");

    // Zero-DB assertions
    assert!(!watcher_src.contains("sqlite"));
    assert!(!watcher_src.contains("sqlx"));
    assert!(!watcher_src.contains("diesel"));
    assert!(!plugin_toml.contains("sqlite"));
    assert!(!plugin_toml.contains("sqlx"));
    assert!(!plugin_toml.contains("diesel"));

    // External isolation: watcher must not have spawn/stop/kill controls
    assert!(!watcher_src.contains("kill("));
    assert!(!watcher_src.contains("stop("));
    assert!(!watcher_src.contains("Command::new"));
}
