use proxyweave_plugin::{
    activation::{
        validate_version_key, ActivationPhase, CompatibilityDecision, CompatibilityPermit,
        PluginFileLock, Stamp, VersionLocation, VersionPointer, ACTIVATION_SCHEMA_VERSION,
    },
    Registry,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

fn create_test_package(root: &Path, id: &str, version: &str) -> std::path::PathBuf {
    fs::create_dir_all(root.join("bin")).unwrap();
    let exe_content = format!("test executable for {id} version {version}");
    fs::write(root.join("bin/test.exe"), exe_content.as_bytes()).unwrap();
    let hash = format!("{:x}", Sha256::digest(exe_content.as_bytes()));
    let yaml = format!(
        r#"schema_version: '1.0'
metadata:
  id: {id}
  name: Test Plugin {id}
  version: {version}
  license: MIT
runtime:
  ownership: managed
  executable: bin/test.exe
  sha256: {hash}
  args: ['-config', '{{CONFIG_PATH}}']
  env: {{LISTEN: '127.0.0.1:{{PORT}}'}}
network:
  health_check:
    path: /healthz
    expected_status: 200
    identity_header: X-Service
    identity_value: {id}
    timeout_ms: 500
lifecycle:
  startup_timeout_ms: 2000
  idle_timeout_ms: null
  max_port_retries: 3
capabilities:
  protocol: openai_chat
  models_path: /v1/models
  inference_path: /v1/chat/completions
  streaming: true
configuration:
  settings: {{mode: standard, enabled: true}}
  required_secrets: [token]
  config_template: {{listen: '127.0.0.1:{{PORT}}', api_key: '{{PLUGIN_API_KEY}}'}}
"#
    );
    let path = root.join("manifest.yaml");
    fs::write(&path, yaml).unwrap();
    path
}

#[test]
fn test_version_key_validation() {
    assert!(validate_version_key("1.0.0").is_ok());
    assert!(validate_version_key("v2.1.0-alpha.1").is_ok());
    assert!(validate_version_key("rel_2026.10_patch-3").is_ok());

    assert!(validate_version_key("").is_err());
    assert!(validate_version_key(".hidden").is_err());
    assert!(validate_version_key("v1.0.").is_err());
    assert!(validate_version_key("-start-dash").is_err());
    assert!(validate_version_key("end-dash-").is_err());
    assert!(validate_version_key("path/traversal").is_err());
    assert!(validate_version_key("path\\traversal").is_err());
    assert!(validate_version_key("../parent").is_err());
    assert!(validate_version_key("has space").is_err());
    assert!(validate_version_key("bad@char!").is_err());
    assert!(validate_version_key(&"a".repeat(65)).is_err());
}

#[test]
fn test_windows_file_lock_mutual_exclusion() {
    let tmp = tempfile::tempdir().unwrap();
    let plugin_dir = tmp.path().join("packages/demo-plugin");
    fs::create_dir_all(&plugin_dir).unwrap();

    let lock1 = PluginFileLock::acquire(&plugin_dir, "demo-plugin").unwrap();
    assert_eq!(lock1.plugin_id(), "demo-plugin");
    assert!(plugin_dir.join(".lock").exists());

    assert!(lock1.assert_plugin("demo-plugin").is_ok());
    assert!(lock1.assert_plugin("other-plugin").is_err());

    let try2 = PluginFileLock::try_acquire(&plugin_dir, "demo-plugin");
    assert!(
        try2.is_err(),
        "second lock attempt must be rejected while lock1 is held"
    );

    drop(lock1);

    let lock3 = PluginFileLock::try_acquire(&plugin_dir, "demo-plugin");
    assert!(
        lock3.is_ok(),
        "lock acquisition must succeed after first lock is dropped"
    );

    drop(lock3);
    assert!(
        plugin_dir.join(".lock").exists(),
        ".lock file must persist and never be deleted"
    );
}

#[test]
fn test_activation_envelope_cas_and_stamp() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();

    let manifest_source = create_test_package(&tmp.path().join("src_v1"), "cas-plugin", "1.0.0");
    let import_res = registry.import(&manifest_source).unwrap();
    let record_v1 = import_res.record;

    let lock = registry.acquire_plugin_lock("cas-plugin").unwrap();
    let initial_envelope = registry
        .initialize_legacy_activation("cas-plugin", &record_v1, &lock)
        .unwrap();

    assert_eq!(initial_envelope.schema_version, ACTIVATION_SCHEMA_VERSION);
    assert_eq!(initial_envelope.phase, ActivationPhase::Committed);
    assert_eq!(initial_envelope.stamp.generation, 1);
    assert_eq!(initial_envelope.stamp.revision, 0);

    let matching_stamp = initial_envelope.stamp.clone();
    let mut updated_envelope = initial_envelope.clone();
    updated_envelope.stamp = updated_envelope.stamp.next_revision().unwrap();

    let advanced = registry
        .cas_advance_activation("cas-plugin", &matching_stamp, updated_envelope, &lock)
        .unwrap();
    assert_eq!(advanced.stamp.revision, 1);

    let stale_stamp = Stamp::new(1, 0, &record_v1.fingerprint);
    let mut next_try = advanced.clone();
    next_try.stamp = next_try.stamp.next_revision().unwrap();

    let cas_err = registry.cas_advance_activation("cas-plugin", &stale_stamp, next_try, &lock);
    assert!(cas_err.is_err());
    let err_msg = cas_err.unwrap_err();
    assert!(
        err_msg.contains("CAS stamp mismatch"),
        "error must explicitly report CAS mismatch: {err_msg}"
    );

    let wrong_fp_stamp = Stamp::new(1, 1, "00000000000000000000000000000000");
    let mut next_try2 = advanced.clone();
    next_try2.stamp = next_try2.stamp.next_revision().unwrap();
    assert!(registry
        .cas_advance_activation("cas-plugin", &wrong_fp_stamp, next_try2, &lock)
        .is_err());

    let mut regressed = advanced.clone();
    regressed.stamp = Stamp::new(0, 0, &record_v1.fingerprint);
    let reg_err = registry.cas_advance_activation("cas-plugin", &advanced.stamp, regressed, &lock);
    assert!(reg_err.is_err());
    assert!(reg_err.unwrap_err().contains("cannot regress generation"));
}

#[test]
fn test_immutable_version_storage_and_activation() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();

    let manifest_v1 = create_test_package(&tmp.path().join("src_v1"), "immut-plugin", "1.0.0");
    let import_res = registry.import(&manifest_v1).unwrap();
    let record_v1 = import_res.record;

    let manifest_v1_copy =
        create_test_package(&tmp.path().join("src_v1_store"), "immut-plugin", "1.0.0");
    let (v1_dir, v1_record) = registry
        .store_version_package("immut-plugin", "1.0.0", &manifest_v1_copy, None)
        .unwrap();
    assert!(v1_dir.exists());
    assert_eq!(v1_record.fingerprint, record_v1.fingerprint);

    let lock = registry.acquire_plugin_lock("immut-plugin").unwrap();
    let env_v1 = registry
        .initialize_version_activation("immut-plugin", "1.0.0", &v1_record, &lock)
        .unwrap();
    assert_eq!(env_v1.phase, ActivationPhase::Committed);

    let loaded_v1 = registry.load("immut-plugin").unwrap();
    assert_eq!(loaded_v1.manifest.metadata.version, "1.0.0");

    let manifest_v2 = create_test_package(&tmp.path().join("src_v2"), "immut-plugin", "2.0.0");
    let (v2_dir, v2_record) = registry
        .store_version_package("immut-plugin", "2.0.0", &manifest_v2, None)
        .unwrap();
    assert!(v2_dir.exists());

    let (v1_repeat_dir, v1_repeat_record) = registry
        .store_version_package("immut-plugin", "1.0.0", &manifest_v1_copy, None)
        .unwrap();
    assert_eq!(v1_repeat_dir, v1_dir);
    assert_eq!(v1_repeat_record.fingerprint, v1_record.fingerprint);

    let mut env = registry.read_activation("immut-plugin").unwrap().unwrap();
    let expected = env.stamp.clone();
    let candidate_ptr = VersionPointer {
        location: VersionLocation::VersionKey {
            version_key: "2.0.0".to_string(),
        },
        fingerprint: v2_record.fingerprint.clone(),
        version: "2.0.0".to_string(),
        source: None,
    };
    env.prepare_candidate(candidate_ptr, "txn-update-1")
        .unwrap();
    let env = registry
        .cas_advance_activation("immut-plugin", &expected, env, &lock)
        .unwrap();
    assert_eq!(env.phase, ActivationPhase::Prepared);

    let expected = env.stamp.clone();
    let mut env = env;
    env.mark_stopped().unwrap();
    let env = registry
        .cas_advance_activation("immut-plugin", &expected, env, &lock)
        .unwrap();
    assert_eq!(env.phase, ActivationPhase::Stopped);

    let expected = env.stamp.clone();
    let mut env = env;
    env.switch_to_candidate().unwrap();
    let env = registry
        .cas_advance_activation("immut-plugin", &expected, env, &lock)
        .unwrap();
    assert_eq!(env.phase, ActivationPhase::Switched);
    assert_eq!(env.stamp.generation, 2);

    let expected = env.stamp.clone();
    let mut env = env;
    env.commit_active().unwrap();
    let env = registry
        .cas_advance_activation("immut-plugin", &expected, env, &lock)
        .unwrap();
    assert_eq!(env.phase, ActivationPhase::Committed);

    let loaded_v2 = registry.load("immut-plugin").unwrap();
    assert_eq!(loaded_v2.manifest.metadata.version, "2.0.0");
    assert_eq!(loaded_v2.fingerprint, v2_record.fingerprint);

    assert!(v1_dir.join("record.json").exists());
    assert!(v1_dir.join("bin/test.exe").exists());
}

#[test]
fn test_legacy_single_package_backward_compatibility() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();

    let manifest_source =
        create_test_package(&tmp.path().join("src_legacy"), "legacy-plg", "1.0.0");
    registry.import(&manifest_source).unwrap();

    let act_path = registry.activation_path("legacy-plg").unwrap();
    assert!(!act_path.exists());

    let record = registry.load("legacy-plg").unwrap();
    assert_eq!(record.manifest.metadata.id, "legacy-plg");
    assert_eq!(record.manifest.metadata.version, "1.0.0");

    let lock = registry.acquire_plugin_lock("legacy-plg").unwrap();
    let env = registry
        .initialize_legacy_activation("legacy-plg", &record, &lock)
        .unwrap();
    assert_eq!(env.current.location, VersionLocation::Legacy);
    assert!(act_path.exists());

    let record2 = registry.load("legacy-plg").unwrap();
    assert_eq!(record2.fingerprint, record.fingerprint);
}

#[test]
fn test_uncommitted_phase_crash_recovery_on_load() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();

    let manifest_v1 = create_test_package(&tmp.path().join("src_v1"), "crash-plugin", "1.0.0");
    registry.import(&manifest_v1).unwrap();
    let (_v1_dir, record_v1) = registry
        .store_version_package("crash-plugin", "1.0.0", &manifest_v1, None)
        .unwrap();

    let lock = registry.acquire_plugin_lock("crash-plugin").unwrap();
    let env_v1 = registry
        .initialize_version_activation("crash-plugin", "1.0.0", &record_v1, &lock)
        .unwrap();
    assert_eq!(env_v1.phase, ActivationPhase::Committed);

    let manifest_v2 = create_test_package(&tmp.path().join("src_v2"), "crash-plugin", "2.0.0");
    let (_v2_dir, record_v2) = registry
        .store_version_package("crash-plugin", "2.0.0", &manifest_v2, None)
        .unwrap();

    let mut env = registry.read_activation("crash-plugin").unwrap().unwrap();
    let candidate_ptr = VersionPointer {
        location: VersionLocation::VersionKey {
            version_key: "2.0.0".to_string(),
        },
        fingerprint: record_v2.fingerprint.clone(),
        version: "2.0.0".to_string(),
        source: None,
    };
    env.prepare_candidate(candidate_ptr, "txn-crash-prep")
        .unwrap();
    registry
        .write_activation("crash-plugin", &env, &lock)
        .unwrap();
    assert_eq!(env.phase, ActivationPhase::Prepared);
    drop(lock);

    let loaded = registry.load("crash-plugin").unwrap();
    assert_eq!(
        loaded.manifest.metadata.version, "1.0.0",
        "load must automatically recover uncommitted Prepared phase to current version 1.0.0"
    );

    let env_after_prep_crash = registry.read_activation("crash-plugin").unwrap().unwrap();
    assert_eq!(env_after_prep_crash.phase, ActivationPhase::Committed);
    assert_eq!(env_after_prep_crash.current.version, "1.0.0");
    assert!(env_after_prep_crash.candidate.is_none());
    assert_eq!(env_after_prep_crash.stamp.generation, 2);

    let lock = registry.acquire_plugin_lock("crash-plugin").unwrap();
    let mut env = env_after_prep_crash;
    let candidate_ptr = VersionPointer {
        location: VersionLocation::VersionKey {
            version_key: "2.0.0".to_string(),
        },
        fingerprint: record_v2.fingerprint.clone(),
        version: "2.0.0".to_string(),
        source: None,
    };
    env.prepare_candidate(candidate_ptr, "txn-crash-post")
        .unwrap();
    env.mark_stopped().unwrap();
    env.switch_to_candidate().unwrap();
    env.mark_health_pending().unwrap();
    assert_eq!(env.phase, ActivationPhase::HealthPending);
    assert_eq!(env.current.version, "2.0.0");
    assert!(env.confirmed_old.is_some());

    registry
        .write_activation("crash-plugin", &env, &lock)
        .unwrap();
    drop(lock);

    let loaded = registry.load("crash-plugin").unwrap();
    assert_eq!(
        loaded.manifest.metadata.version, "1.0.0",
        "load must roll back HealthPending candidate to confirmed_old 1.0.0"
    );

    let env_after_switch_crash = registry.read_activation("crash-plugin").unwrap().unwrap();
    assert_eq!(env_after_switch_crash.phase, ActivationPhase::Committed);
    assert_eq!(env_after_switch_crash.current.version, "1.0.0");
    assert!(env_after_switch_crash.confirmed_old.is_none());
    assert!(env_after_switch_crash.candidate.is_none());
}

#[test]
fn test_corrupt_or_recovery_required_blocks_load() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();

    let manifest = create_test_package(&tmp.path().join("src"), "bad-plugin", "1.0.0");
    registry.import(&manifest).unwrap();
    let lock = registry.acquire_plugin_lock("bad-plugin").unwrap();
    let record = registry.load("bad-plugin").unwrap();
    let mut env = registry
        .initialize_legacy_activation("bad-plugin", &record, &lock)
        .unwrap();

    env.phase = ActivationPhase::RecoveryRequired;
    registry
        .write_activation("bad-plugin", &env, &lock)
        .unwrap();
    drop(lock);

    let load_res = registry.load("bad-plugin");
    assert!(load_res.is_err());
    let err_msg = load_res.unwrap_err();
    assert!(
        err_msg.contains("recovery_required"),
        "error must explicitly report recovery_required state: {err_msg}"
    );

    let act_path = registry.activation_path("bad-plugin").unwrap();
    fs::write(&act_path, b"not valid json").unwrap();
    let load_corrupt = registry.load("bad-plugin");
    assert!(load_corrupt.is_err());
    assert!(load_corrupt
        .unwrap_err()
        .contains("activation envelope is corrupt"));
}

#[test]
fn test_settings_update_under_lock_advances_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();

    let manifest = create_test_package(&tmp.path().join("src"), "settings-plugin", "1.0.0");
    registry.import(&manifest).unwrap();
    let lock = registry.acquire_plugin_lock("settings-plugin").unwrap();
    let record = registry.load("settings-plugin").unwrap();
    let env_init = registry
        .initialize_legacy_activation("settings-plugin", &record, &lock)
        .unwrap();
    assert_eq!(env_init.stamp.revision, 0);
    drop(lock);

    let mut new_settings = BTreeMap::new();
    new_settings.insert("mode".to_string(), serde_json::json!("advanced"));
    new_settings.insert("enabled".to_string(), serde_json::json!(false));

    let saved = registry
        .save_settings("settings-plugin", new_settings.clone())
        .unwrap();
    assert_eq!(
        saved.settings.get("mode"),
        Some(&serde_json::json!("advanced"))
    );

    let env_after = registry
        .read_activation("settings-plugin")
        .unwrap()
        .unwrap();
    assert_eq!(env_after.stamp.revision, 1);
    assert_eq!(
        env_after.settings.get("mode"),
        Some(&serde_json::json!("advanced"))
    );

    let reloaded = registry.load("settings-plugin").unwrap();
    assert_eq!(
        reloaded.settings.get("mode"),
        Some(&serde_json::json!("advanced"))
    );
}

#[test]
fn test_production_compatibility_permit_baseline() {
    let permit = CompatibilityPermit::default_production();
    assert_eq!(
        permit.decision(),
        CompatibilityDecision::CompatibilityUnknown
    );
    assert!(!permit.is_compatible());
    assert!(permit
        .reason()
        .contains("no verified data migration compatibility evidence"));

    let test_permit = CompatibilityPermit::test_permit("verified in stage1 test");
    assert_eq!(test_permit.decision(), CompatibilityDecision::Compatible);
    assert!(test_permit.is_compatible());

    let test_incomp = CompatibilityPermit::test_incompatible("schema breaking change");
    assert_eq!(test_incomp.decision(), CompatibilityDecision::Incompatible);
    assert!(!test_incomp.is_compatible());
}

#[test]
fn resolve_active_package_returns_record_and_directory_from_one_generation() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    let id = "coherent-plugin";
    let v1 = registry
        .import(create_test_package(&tmp.path().join("v1"), id, "1.0.0"))
        .unwrap()
        .record;
    let (legacy, container, stamp) = registry.resolve_active_package(id).unwrap();
    assert_eq!(legacy, v1);
    assert_eq!(container, registry.package_dir(id).unwrap());
    assert!(stamp.is_none());
    let (v2_dir, v2) = registry
        .store_version_package(
            id,
            "release-2",
            &create_test_package(&tmp.path().join("v2"), id, "2.0.0"),
            None,
        )
        .unwrap();
    let lock = registry.acquire_plugin_lock(id).unwrap();
    let mut env = registry
        .initialize_legacy_activation(id, &v1, &lock)
        .unwrap();
    env.prepare_candidate(
        VersionPointer {
            location: VersionLocation::VersionKey {
                version_key: "release-2".into(),
            },
            fingerprint: v2.fingerprint.clone(),
            version: "2.0.0".into(),
            source: None,
        },
        "coherent-transaction",
    )
    .unwrap();
    env.mark_stopped().unwrap();
    env.switch_to_candidate().unwrap();
    env.commit_active().unwrap();
    registry.write_activation(id, &env, &lock).unwrap();
    drop(lock);
    let (active, dir, stamp) = registry.resolve_active_package(id).unwrap();
    assert_eq!(active, v2);
    assert_eq!(dir, v2_dir);
    assert_eq!(stamp, Some(env.stamp));
    assert_eq!(
        fs::read(dir.join("bin/test.exe")).unwrap(),
        b"test executable for coherent-plugin version 2.0.0"
    );
    assert_ne!(
        fs::read(container.join("bin/test.exe")).unwrap(),
        fs::read(dir.join("bin/test.exe")).unwrap()
    );
    fs::write(dir.join("bin/test.exe"), b"tampered").unwrap();
    assert!(registry.resolve_active_package(id).is_err());
}

#[test]
fn immutable_version_key_requires_identical_valid_candidate_and_existing_package() {
    use proxyweave_plugin::registry::{
        DigestEvidence, Immutability, InstalledSource, PackageContract,
    };
    let tmp = tempfile::tempdir().unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    let id = "immutable-plugin";
    let manifest = create_test_package(&tmp.path().join("v1"), id, "1.0.0");
    let record = registry.import(&manifest).unwrap().record;
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let ext = if os == "windows" { "zip" } else { "tar.gz" };
    let source = InstalledSource {
        owner: "test-owner".into(),
        repo: "test-repo".into(),
        release_id: 1,
        tag_name: "v1.0.0".into(),
        asset_id: 2,
        asset_name: format!("test-{os}-{arch}.{ext}"),
        target_os: os.into(),
        target_arch: arch.into(),
        archive_sha256: "a".repeat(64),
        expected_sha256: "a".repeat(64),
        digest_evidence: DigestEvidence::UserProvidedSha256,
        package_contract: PackageContract::ManifestV1,
        manifest_fingerprint: record.fingerprint.clone(),
        immutability: Immutability::Unknown,
    };
    let (dir, stored) = registry
        .store_version_package(id, "fixed-key", &manifest, Some(source.clone()))
        .unwrap();
    let original = fs::read(dir.join("record.json")).unwrap();
    let again = registry
        .store_version_package(id, "fixed-key", &manifest, Some(source.clone()))
        .unwrap();
    assert_eq!(again, (dir.clone(), stored));
    let mut other_source = source.clone();
    other_source.release_id += 1;
    assert!(registry
        .store_version_package(id, "fixed-key", &manifest, Some(other_source))
        .unwrap_err()
        .contains("conflict"));
    assert!(registry
        .store_version_package(id, "fixed-key", &manifest, None)
        .unwrap_err()
        .contains("conflict"));
    let v2 = create_test_package(&tmp.path().join("v2"), id, "2.0.0");
    assert!(registry
        .store_version_package(id, "fixed-key", &v2, None)
        .unwrap_err()
        .contains("conflict"));
    // Same version but different declared executable hash must not alias the stored version.
    let changed = create_test_package(&tmp.path().join("changed"), id, "1.0.0");
    let old_hash = format!(
        "{:x}",
        Sha256::digest(fs::read(changed.parent().unwrap().join("bin/test.exe")).unwrap())
    );
    fs::write(
        changed.parent().unwrap().join("bin/test.exe"),
        b"different executable",
    )
    .unwrap();
    let new_hash = format!("{:x}", Sha256::digest(b"different executable"));
    fs::write(
        &changed,
        fs::read_to_string(&changed)
            .unwrap()
            .replace(&old_hash, &new_hash),
    )
    .unwrap();
    assert!(registry
        .store_version_package(id, "fixed-key", &changed, None)
        .unwrap_err()
        .contains("conflict"));
    // Existing key never bypasses validation of incoming executable bytes.
    fs::write(
        manifest.parent().unwrap().join("bin/test.exe"),
        b"invalid candidate",
    )
    .unwrap();
    assert!(registry
        .store_version_package(id, "fixed-key", &manifest, Some(source.clone()))
        .is_err());
    create_test_package(manifest.parent().unwrap(), id, "1.0.0");
    assert_eq!(fs::read(dir.join("record.json")).unwrap(), original);
    fs::write(dir.join("bin/test.exe"), b"corrupt stored executable").unwrap();
    assert!(registry
        .store_version_package(id, "fixed-key", &manifest, Some(source))
        .is_err());
    assert_eq!(fs::read(dir.join("record.json")).unwrap(), original);
    assert!(!fs::read_dir(dir.parent().unwrap()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".update-")));
}
