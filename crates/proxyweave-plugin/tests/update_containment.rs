//! Stage0 is denial containment, not activation, rollback or hot-reload acceptance.
use proxyweave_plugin::updater::{apply_plugin_update, apply_plugin_update_with_backend};
use proxyweave_plugin::{Manifest, Registry, Runtime, RuntimeManager, RuntimeStatus};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

const ID: &str = "containment-fixture";
const ERROR: &str = "update_unavailable_pending_safety_validation";

fn package(root: &Path, version: &str, external: bool) -> std::path::PathBuf {
    fs::create_dir_all(root.join("bin")).unwrap();
    let bytes = b"inert owned fixture; must never execute";
    fs::write(root.join("bin/app.exe"), bytes).unwrap();
    let mut manifest = Manifest::parse(
        format!(
            r#"schema_version: '1.0'
metadata:
  id: {ID}
  name: Containment Fixture
  version: '{version}'
  license: MIT
runtime:
  ownership: managed
  executable: bin/app.exe
  sha256: {:x}
network:
  health_check:
    path: /healthz
    expected_status: 200
    identity_header: X-Fixture
    identity_value: containment
    timeout_ms: 100
lifecycle:
  startup_timeout_ms: 200
  idle_timeout_ms: null
  max_port_retries: 0
capabilities:
  protocol: openai_chat
  models_path: /v1/models
  inference_path: /v1/chat/completions
  streaming: true
configuration:
  config_template: {{}}
  settings:
    fixture_setting: initial
"#,
            Sha256::digest(bytes)
        )
        .as_bytes(),
    )
    .unwrap();
    if external {
        manifest.runtime = Runtime::External {
            endpoint: "http://127.0.0.1:1".into(),
        };
        manifest.configuration = Default::default();
    }
    let path = root.join("manifest.yaml");
    fs::write(&path, serde_yaml_ng::to_string(&manifest).unwrap()).unwrap();
    path
}

fn tree(root: &Path) -> BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn collect(root: &Path, dir: &Path, out: &mut BTreeMap<std::path::PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    collect(root, root, &mut out);
    out
}

#[tokio::test]
async fn public_entries_reject_before_registry_load_and_callback_for_managed_and_external() {
    for external in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let registry_root = root.path().join("registry");
        let registry = Registry::open(&registry_root).unwrap();
        registry
            .import(package(&root.path().join("old"), "1.0.0", external))
            .unwrap();
        let runtime = RuntimeManager::new();
        let mut settings = BTreeMap::new();
        if !external {
            settings.insert("fixture_setting".into(), serde_json::json!("preserve"));
        }
        registry.save_settings(ID, settings).unwrap();
        let data = registry.data_dir(ID).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("sentinel"), b"owned-data").unwrap();
        let candidate = package(&root.path().join("new"), "2.0.0", false);
        let before = tree(&registry_root);
        let staged = registry.prepare_staged_package(&candidate, None).unwrap();
        assert_eq!(
            apply_plugin_update(&registry, &runtime, ID, staged, Some("fixture-secret"))
                .await
                .unwrap_err(),
            ERROR
        );
        assert_eq!(tree(&registry_root), before);
        assert_eq!(runtime.get_status(ID).await, RuntimeStatus::Stopped);

        let staged = registry.prepare_staged_package(&candidate, None).unwrap();
        let mut called = false;
        let error = apply_plugin_update_with_backend(&registry, &runtime, ID, staged, None, |_| {
            called = true;
            async { Ok(()) }
        })
        .await
        .unwrap_err();
        assert_eq!(error, ERROR);
        assert!(!called);
        assert_eq!(tree(&registry_root), before);
        assert_eq!(runtime.get_status(ID).await, RuntimeStatus::Stopped);

        // Corrupt registration after staging: a load/validation must not precede fixed denial.
        let staged = registry.prepare_staged_package(&candidate, None).unwrap();
        fs::write(
            registry.package_dir(ID).unwrap().join("record.json"),
            b"invalid fixture record",
        )
        .unwrap();
        let corrupted = tree(&registry.package_dir(ID).unwrap());
        assert_eq!(
            registry
                .atomic_swap_package("../untrusted-fixture-id", staged)
                .err()
                .unwrap(),
            ERROR
        );
        assert_eq!(tree(&registry.package_dir(ID).unwrap()), corrupted);
        assert_eq!(fs::read(data.join("sentinel")).unwrap(), b"owned-data");
    }
}

#[test]
fn disabled_library_bodies_cannot_load_stop_publish_or_restore() {
    let updater = include_str!("../src/updater.rs");
    let apply = updater
        .split_once("pub async fn apply_plugin_update(")
        .unwrap()
        .1
        .split_once(") -> Result<UpdateResult, String> {")
        .unwrap()
        .1
        .split_once("\n}")
        .unwrap()
        .0;
    assert_eq!(apply.trim(), "let _ = (registry, runtime, id, staged, api_key);\n    Err(\"update_unavailable_pending_safety_validation\".into())");
    let backend = updater
        .split_once("pub async fn apply_plugin_update_with_backend<F, Fut>(")
        .unwrap()
        .1
        .split_once("\n{\n")
        .unwrap()
        .1
        .split_once("\n}")
        .unwrap()
        .0;
    assert_eq!(backend.trim(), "let _ = (registry, runtime, id, staged, api_key, set_backend);\n    Err(\"update_unavailable_pending_safety_validation\".into())");
    assert!(!updater.contains("async fn restore_update"));
    let registry = include_str!("../src/registry.rs");
    let swap = registry
        .split_once("pub fn atomic_swap_package(")
        .unwrap()
        .1
        .split_once(") -> Result<PackageBackup, String> {")
        .unwrap()
        .1
        .split_once("\n    }")
        .unwrap()
        .0;
    assert_eq!(swap.trim(), "let _ = (id, &staged.directory);\n        Err(\"update_unavailable_pending_safety_validation\".into())");
    for name in ["commit", "rollback"] {
        let signature = format!("pub fn {name}(self) -> Result<(), String> {{");
        let body = registry
            .split_once(&signature)
            .unwrap()
            .1
            .split_once("\n    }")
            .unwrap()
            .0;
        assert_eq!(
            body.trim(),
            "Err(\"update_unavailable_pending_safety_validation\".into())"
        );
    }
}
