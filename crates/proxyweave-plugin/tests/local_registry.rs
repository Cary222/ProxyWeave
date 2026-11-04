use proxyweave_plugin::{
    DigestEvidence, Immutability, InstalledSource, Manifest, Ownership, PackageContract, Registry,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

fn package(root: &Path) -> std::path::PathBuf {
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::write(
        root.join("bin/test.exe"),
        b"test executable bytes; never executed",
    )
    .unwrap();
    let hash = format!(
        "{:x}",
        Sha256::digest(b"test executable bytes; never executed")
    );
    let yaml = format!(
        r#"schema_version: '1.0'
metadata:
  id: test-plugin
  name: Test Plugin
  version: 1.0.0
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
    identity_value: test-plugin
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
  settings: {{label: local, enabled: true}}
  required_secrets: [access]
  config_template: {{listen: '127.0.0.1:{{PORT}}', api_key: '{{PLUGIN_API_KEY}}'}}
  credential_files:
    auths/workbuddy-test.json: {{accessToken: '{{SECRET:access}}'}}
"#
    );
    let path = root.join("manifest.yaml");
    fs::write(&path, yaml).unwrap();
    path
}

#[test]
fn import_duplicate_settings_and_restart_reload() {
    let tmp = tempfile::tempdir().unwrap();
    let source = package(&tmp.path().join("source"));
    fs::write(
        source.parent().unwrap().join("private.json"),
        b"not declared; must not copy",
    )
    .unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    let result = registry.import(&source).unwrap();
    assert!(!result.already_registered);
    assert_eq!(result.record.manifest.ownership(), Ownership::Managed);
    assert!(!registry
        .package_dir("test-plugin")
        .unwrap()
        .join("private.json")
        .exists());
    assert_eq!(
        registry.data_dir("test-plugin").unwrap(),
        tmp.path()
            .join("profile/data")
            .canonicalize()
            .unwrap()
            .join("test-plugin")
    );
    assert!(registry.import(&source).unwrap().already_registered);
    let settings = BTreeMap::from([
        ("label".into(), serde_json::json!("updated")),
        ("enabled".into(), serde_json::json!(false)),
    ]);
    registry
        .save_settings("test-plugin", settings.clone())
        .unwrap();
    drop(registry);
    let restarted = Registry::open(tmp.path().join("profile")).unwrap();
    assert_eq!(restarted.list().unwrap()[0].settings, settings);
}

#[test]
fn mismatched_hash_or_missing_file_does_not_register() {
    let tmp = tempfile::tempdir().unwrap();
    let source = package(&tmp.path().join("source"));
    fs::write(source.parent().unwrap().join("bin/test.exe"), b"tampered").unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    assert!(registry.import(&source).unwrap_err().contains("SHA256"));
    assert!(registry.list().unwrap().is_empty());
    fs::remove_file(source.parent().unwrap().join("bin/test.exe")).unwrap();
    assert!(registry.import(&source).is_err());
    assert!(registry.list().unwrap().is_empty());
}

#[test]
fn conflicting_duplicate_does_not_overwrite_existing_record() {
    let tmp = tempfile::tempdir().unwrap();
    let source = package(&tmp.path().join("source"));
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    let old = registry.import(&source).unwrap().record;
    let text = fs::read_to_string(&source)
        .unwrap()
        .replace("name: Test Plugin", "name: Other Plugin");
    fs::write(&source, text).unwrap();
    assert!(registry
        .import(&source)
        .unwrap_err()
        .contains("different package"));
    assert_eq!(registry.load("test-plugin").unwrap(), old);
}

#[test]
fn invalid_configuration_does_not_pollute_persisted_settings() {
    let tmp = tempfile::tempdir().unwrap();
    let source = package(&tmp.path().join("source"));
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    let old = registry.import(&source).unwrap().record;
    for settings in [
        BTreeMap::from([("api_key".into(), serde_json::json!("not accepted"))]),
        BTreeMap::from([
            ("label".into(), serde_json::json!({"nested": true})),
            ("enabled".into(), serde_json::json!(true)),
        ]),
    ] {
        assert!(registry.save_settings("test-plugin", settings).is_err());
        assert_eq!(registry.load("test-plugin").unwrap(), old);
    }
}

#[test]
fn tampered_registration_or_installed_executable_fails_closed_on_reload() {
    let tmp = tempfile::tempdir().unwrap();
    let source = package(&tmp.path().join("source"));
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    registry.import(&source).unwrap();
    let record_path = registry
        .package_dir("test-plugin")
        .unwrap()
        .join("record.json");
    let before = fs::read(&record_path).unwrap();
    fs::write(
        &record_path,
        before.iter().copied().take(20).collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(registry.list().is_err());
    fs::write(&record_path, before).unwrap();
    fs::write(
        registry
            .package_dir("test-plugin")
            .unwrap()
            .join("bin/test.exe"),
        b"modified",
    )
    .unwrap();
    assert!(registry.list().is_err());
}

#[test]
fn strict_parser_rejects_unknown_duplicate_and_multiple_documents() {
    let tmp = tempfile::tempdir().unwrap();
    let source = package(tmp.path());
    let valid = fs::read_to_string(source).unwrap();
    assert!(Manifest::parse(valid.as_bytes()).is_ok());
    for text in [
        format!("{valid}\nunknown: true\n"),
        format!("schema_version: '1.0'\n{valid}"),
        format!("{valid}\n---\n{valid}"),
        valid.replace("version: 1.0.0", "version: invalid"),
        valid.replace("{PORT}", "{UNDECLARED}"),
    ] {
        assert!(Manifest::parse(text.as_bytes()).is_err());
    }
}

#[test]
fn portable_package_path_boundaries() {
    let tmp = tempfile::tempdir().unwrap();
    let source = package(tmp.path());
    let valid = fs::read_to_string(source).unwrap();
    for p in [
        "../test.exe",
        "C:/test.exe",
        "/tmp/test.exe",
        "bin/test.exe:stream",
        "bin/CON.exe",
        "bin/test..exe/../test.exe",
        "bin//test.exe",
        "bin/test.exe ",
        "bin\\test.exe",
    ] {
        let yaml = valid.replace("executable: bin/test.exe", &format!("executable: '{p}'"));
        assert!(
            Manifest::parse(yaml.as_bytes()).is_err(),
            "accepted path {p}"
        );
    }
    for id in ["../test", "CON", "con", "lpt1", "", "test:ads"] {
        assert!(Manifest::parse(
            valid
                .replace("id: test-plugin", &format!("id: '{id}'"))
                .as_bytes()
        )
        .is_err());
    }
}

#[test]
fn external_has_no_owned_executable_or_lifecycle_side_effects() {
    let yaml = r#"schema_version: '1.0'
metadata: {id: external-test, name: External, version: 1.0.0, license: unverified}
runtime: {ownership: external, endpoint: 'http://127.0.0.1:28317/'}
network:
  health_check: {path: /health, expected_status: 200, identity_header: X-Service, identity_value: external-test, timeout_ms: 500}
lifecycle: {startup_timeout_ms: 2000, idle_timeout_ms: null, max_port_retries: 0}
capabilities: {protocol: openai_chat, models_path: /v1/models, inference_path: /v1/chat/completions, streaming: true}
"#;
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("manifest.yaml");
    fs::write(&source, yaml).unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    assert_eq!(
        registry
            .import(&source)
            .unwrap()
            .record
            .manifest
            .ownership(),
        Ownership::External
    );
    assert_eq!(registry.list().unwrap().len(), 1);
    for text in [
        yaml.replace("max_port_retries: 0", "max_port_retries: 3"),
        yaml.replace("idle_timeout_ms: null", "idle_timeout_ms: 1000"),
        yaml.replace(
            "http://127.0.0.1:28317/",
            "http://user:password@127.0.0.1:28317/",
        ),
        yaml.replace("endpoint:", "executable: bin/test.exe, endpoint:"),
    ] {
        assert!(Manifest::parse(text.as_bytes()).is_err());
    }
}

#[test]
fn magpie_manifest_external_template_validates_and_imports() {
    let manifest_bytes = include_bytes!("../../../manifests/magpie.manifest.yaml");
    let manifest =
        Manifest::parse(manifest_bytes).expect("magpie.manifest.yaml must parse and validate");
    assert_eq!(manifest.metadata.id, "magpie");
    assert_eq!(manifest.metadata.name, "Magpie AI Gateway");
    assert_eq!(manifest.ownership(), Ownership::External);
    assert_eq!(manifest.capabilities.protocol, "openai_chat");
    assert_eq!(manifest.capabilities.models_path, "/v1/models");
    assert_eq!(manifest.capabilities.inference_path, "/v1/chat/completions");
    assert!(manifest.capabilities.streaming);

    let management_link = proxyweave_plugin::management::resolve_management(
        &manifest,
        &proxyweave_plugin::RuntimeStatus::Stopped,
    )
    .expect("magpie management entry must resolve");
    assert_eq!(management_link.state, "available");
    assert_eq!(
        management_link.url.as_deref(),
        Some("http://127.0.0.1:3425/")
    );

    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("magpie.manifest.yaml");
    fs::write(&source, manifest_bytes).unwrap();
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    let import_res = registry
        .import(&source)
        .expect("import external magpie manifest");
    assert!(!import_res.already_registered);
    assert_eq!(import_res.record.manifest.metadata.id, "magpie");
    assert_eq!(import_res.record.manifest.ownership(), Ownership::External);

    let reimport = registry.import(&source).unwrap();
    assert!(reimport.already_registered);

    let loaded = registry.load("magpie").expect("load magpie from registry");
    assert_eq!(loaded.manifest.metadata.id, "magpie");
    assert_eq!(
        loaded.manifest.network.health_check.identity_header,
        "Server"
    );
    assert_eq!(
        loaded.manifest.network.health_check.identity_value,
        "magpie"
    );
}

#[test]
fn concurrent_import_on_clones_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let source = package(&tmp.path().join("source"));
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    let results = std::thread::scope(|s| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let registry = registry.clone();
                let source = source.clone();
                s.spawn(move || registry.import(source).unwrap())
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|r| !r.already_registered).count(), 1);
    assert_eq!(registry.list().unwrap().len(), 1);
}

#[cfg(windows)]
#[test]
fn junction_source_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let real = tmp.path().join("real");
    package(&real);
    let link = tmp.path().join("junction");
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&real)
        .status()
        .unwrap();
    assert!(status.success());
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    let error = registry.import(link.join("manifest.yaml")).unwrap_err();
    assert!(
        error.contains("reparse") || error.contains("links are not allowed"),
        "{error}"
    );
    assert!(registry.list().unwrap().is_empty());
    fs::remove_dir(link).unwrap();
}

#[cfg(windows)]
#[test]
fn read_only_record_write_failure_keeps_prior_configuration() {
    let tmp = tempfile::tempdir().unwrap();
    let source = package(&tmp.path().join("source"));
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    let old = registry.import(&source).unwrap().record;
    let record = registry
        .package_dir("test-plugin")
        .unwrap()
        .join("record.json");
    let mut permissions = fs::metadata(&record).unwrap().permissions();
    let original_permissions = permissions.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&record, permissions).unwrap();
    let changed = BTreeMap::from([
        ("label".into(), serde_json::json!("new")),
        ("enabled".into(), serde_json::json!(false)),
    ]);
    assert!(registry.save_settings("test-plugin", changed).is_err());
    assert_eq!(registry.load("test-plugin").unwrap(), old);
    fs::set_permissions(&record, original_permissions).unwrap();
    assert_eq!(fs::read_dir(record.parent().unwrap()).unwrap().count(), 2); // bin + record, no leaked temp.
}

#[test]
fn manifest_size_and_mapping_duplicates_are_rejected() {
    assert!(Manifest::parse(&vec![b' '; 256 * 1024 + 1]).is_err());
    let tmp = tempfile::tempdir().unwrap();
    let source = package(tmp.path());
    let valid = fs::read_to_string(source).unwrap();
    let duplicate = valid.replace(
        "settings: {label: local, enabled: true}",
        "settings: {label: local, label: other, enabled: true}",
    );
    assert!(Manifest::parse(duplicate.as_bytes()).is_err());
    assert!(Manifest::parse(
        valid
            .replace("protocol: openai_chat", "protocol: anthropic")
            .as_bytes()
    )
    .is_err());
    assert!(Manifest::parse(
        valid
            .replace("max_port_retries: 3", "max_port_retries: 4")
            .as_bytes()
    )
    .is_err());
}

#[test]
fn secret_command_lines_and_projection_aliases_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let source = package(tmp.path());
    let valid = fs::read_to_string(source).unwrap();
    for text in [
        valid.replace(
            "args: ['-config', '{CONFIG_PATH}']",
            "args: ['{SECRET:access}']",
        ),
        valid.replace(
            "args: ['-config', '{CONFIG_PATH}']",
            "args: ['{PLUGIN_API_KEY}']",
        ),
        valid.replace("auths/workbuddy-test.json:", "config.json:"),
        valid.replace("auths/workbuddy-test.json:", "bin/test.exe:"),
    ] {
        assert!(Manifest::parse(text.as_bytes()).is_err());
    }
}

#[cfg(windows)]
#[test]
fn real_executable_fresh_imports_and_conflicts_preserve_bytes() {
    // Real PE image; never spawned or read from a user plugin profile.
    let executable = fs::read(std::env::current_exe().unwrap()).unwrap();
    let hash = format!("{:x}", Sha256::digest(&executable));
    let tmp = tempfile::tempdir().unwrap();
    let source = package(&tmp.path().join("source"));
    fs::write(source.parent().unwrap().join("bin/test.exe"), &executable).unwrap();
    let yaml = fs::read_to_string(&source).unwrap();
    let placeholder = format!(
        "{:x}",
        Sha256::digest(b"test executable bytes; never executed")
    );
    fs::write(&source, yaml.replace(&placeholder, &hash)).unwrap();
    for profile in [
        tmp.path().join("short"),
        tmp.path()
            .join("owned-long-profile-for-real-executable-import")
            .join("nested-configuration")
            .join("fresh-one"),
        tmp.path().join("fresh-two"),
    ] {
        let registry = Registry::open(&profile).unwrap();
        let first = registry.import(&source).unwrap();
        assert!(!first.already_registered);
        let installed = registry
            .package_dir("test-plugin")
            .unwrap()
            .join("bin/test.exe");
        assert_eq!(fs::read(&installed).unwrap(), executable);
        let record_bytes = fs::read(
            installed
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("record.json"),
        )
        .unwrap();
        assert!(registry.import(&source).unwrap().already_registered);
        drop(registry);
        let reopened = Registry::open(&profile).unwrap();
        assert_eq!(reopened.load("test-plugin").unwrap(), first.record);
        assert_eq!(
            fs::read(
                installed
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("record.json")
            )
            .unwrap(),
            record_bytes
        );
        assert_eq!(fs::read_dir(profile.join("packages")).unwrap().count(), 1);
    }
}

#[test]
#[ignore = "requires an explicitly provided owned three-file real wb2api package"]
fn owned_wb2api_package_import_regression() {
    let source = std::path::PathBuf::from(
        std::env::var_os("PROXYWEAVE_TEST_PACKAGE").expect("owned package required"),
    );
    let tmp = tempfile::tempdir().unwrap();
    for name in [
        "short",
        "owned-long-real-package-profile-for-publication-regression/nested/fresh",
        "fresh-repeat",
    ] {
        let profile = tmp.path().join(name);
        let registry = Registry::open(&profile).unwrap();
        let first = registry.import(&source).unwrap();
        assert!(!first.already_registered);
        assert!(registry.import(&source).unwrap().already_registered);
        let id = &first.record.manifest.metadata.id;
        let installed = registry.package_dir(id).unwrap();
        let before = fs::read(installed.join("record.json")).unwrap();
        drop(registry);
        let reopened = Registry::open(&profile).unwrap();
        assert_eq!(reopened.load(id).unwrap(), first.record);
        assert_eq!(fs::read(installed.join("record.json")).unwrap(), before);
        assert_eq!(fs::read_dir(profile.join("packages")).unwrap().count(), 1);
        println!("real package import/duplicate/reopen succeeded: {name}");
    }
}

#[test]
fn old_record_omitted_source_loads_and_serializes_with_none() {
    let tmp = tempfile::tempdir().unwrap();
    let src = package(&tmp.path().join("source"));
    let registry = Registry::open(tmp.path().join("profile")).unwrap();
    let res = registry.import(&src).unwrap();
    assert_eq!(res.record.source, None);

    // Overwrite record.json with legacy format omitting "source" key entirely
    let installed_dir = registry.package_dir("test-plugin").unwrap();
    let record_path = installed_dir.join("record.json");
    let legacy_json = serde_json::json!({
        "manifest": res.record.manifest,
        "fingerprint": res.record.fingerprint,
        "settings": res.record.settings
    });
    fs::write(
        &record_path,
        serde_json::to_vec_pretty(&legacy_json).unwrap(),
    )
    .unwrap();

    // Reload from registry
    let reopened = Registry::open(tmp.path().join("profile")).unwrap();
    let loaded = reopened.load("test-plugin").unwrap();
    assert_eq!(loaded.source, None);
    assert_eq!(loaded.fingerprint, res.record.fingerprint);
    assert_eq!(reopened.list().unwrap(), vec![loaded]);
}

#[test]
fn import_with_source_persists_provenance_and_validates_on_reload() {
    let tmp = tempfile::tempdir().unwrap();
    let src = package(&tmp.path().join("source"));
    let registry = Registry::open(tmp.path().join("profile")).unwrap();

    let manifest_bytes = fs::read(&src).unwrap();
    let manifest = Manifest::parse(&manifest_bytes).unwrap();
    let fingerprint = proxyweave_plugin::manifest_fingerprint(&manifest).unwrap();

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let ext = if os == "windows" { "zip" } else { "tar.gz" };
    let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    let source = InstalledSource {
        owner: "test-owner".to_string(),
        repo: "test-repo".to_string(),
        release_id: 1001,
        tag_name: "v1.0.0".to_string(),
        asset_id: 2001,
        asset_name: format!("test-plugin-{}-{}.{}", os, arch, ext),
        target_os: os.to_string(),
        target_arch: arch.to_string(),
        archive_sha256: sha.to_string(),
        expected_sha256: sha.to_string(),
        digest_evidence: DigestEvidence::UserProvidedSha256,
        package_contract: PackageContract::ManifestV1,
        manifest_fingerprint: fingerprint.clone(),
        immutability: Immutability::Unknown,
    };

    let result = registry
        .import_with_source(&src, Some(source.clone()))
        .unwrap();
    assert!(!result.already_registered);
    assert_eq!(result.record.source, Some(source.clone()));

    // Verify on-disk record.json
    let installed_dir = registry.package_dir("test-plugin").unwrap();
    let record_bytes = fs::read(installed_dir.join("record.json")).unwrap();
    let record_val: serde_json::Value = serde_json::from_slice(&record_bytes).unwrap();
    assert!(record_val.get("source").is_some());
    assert_eq!(record_val["source"]["owner"], "test-owner");

    // Verify reload from fresh registry handle
    let reopened = Registry::open(tmp.path().join("profile")).unwrap();
    let loaded = reopened.load("test-plugin").unwrap();
    assert_eq!(loaded.source, Some(source.clone()));

    // Idempotent duplicate preserves existing source
    let dup_result = reopened.import(&src).unwrap();
    assert!(dup_result.already_registered);
    assert_eq!(dup_result.record.source, Some(source));
}

#[test]
fn tampered_source_rejected_on_reload() {
    let tmp = tempfile::tempdir().unwrap();
    let src = package(&tmp.path().join("source"));
    let registry = Registry::open(tmp.path().join("profile")).unwrap();

    let manifest_bytes = fs::read(&src).unwrap();
    let manifest = Manifest::parse(&manifest_bytes).unwrap();
    let fingerprint = proxyweave_plugin::manifest_fingerprint(&manifest).unwrap();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let ext = if os == "windows" { "zip" } else { "tar.gz" };
    let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    let source = InstalledSource {
        owner: "test-owner".to_string(),
        repo: "test-repo".to_string(),
        release_id: 1001,
        tag_name: "v1.0.0".to_string(),
        asset_id: 2001,
        asset_name: format!("test-plugin-{}-{}.{}", os, arch, ext),
        target_os: os.to_string(),
        target_arch: arch.to_string(),
        archive_sha256: sha.to_string(),
        expected_sha256: sha.to_string(),
        digest_evidence: DigestEvidence::UserProvidedSha256,
        package_contract: PackageContract::ManifestV1,
        manifest_fingerprint: fingerprint.clone(),
        immutability: Immutability::Unknown,
    };

    registry.import_with_source(&src, Some(source)).unwrap();

    // Tamper with record.json source fingerprint
    let installed_dir = registry.package_dir("test-plugin").unwrap();
    let record_path = installed_dir.join("record.json");
    let mut record_val: serde_json::Value =
        serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
    record_val["source"]["manifest_fingerprint"] =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    fs::write(
        &record_path,
        serde_json::to_vec_pretty(&record_val).unwrap(),
    )
    .unwrap();

    // Reload fails closed!
    let reopened = Registry::open(tmp.path().join("profile")).unwrap();
    assert!(reopened.load("test-plugin").is_err());
    assert!(reopened.list().is_err());
}

#[test]
fn import_with_mismatched_source_fails_atomically() {
    let tmp = tempfile::tempdir().unwrap();
    let src = package(&tmp.path().join("source"));
    let registry = Registry::open(tmp.path().join("profile")).unwrap();

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let ext = if os == "windows" { "zip" } else { "tar.gz" };
    let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    // Invalid source: wrong fingerprint
    let invalid_source = InstalledSource {
        owner: "test-owner".to_string(),
        repo: "test-repo".to_string(),
        release_id: 1001,
        tag_name: "v1.0.0".to_string(),
        asset_id: 2001,
        asset_name: format!("test-plugin-{}-{}.{}", os, arch, ext),
        target_os: os.to_string(),
        target_arch: arch.to_string(),
        archive_sha256: sha.to_string(),
        expected_sha256: sha.to_string(),
        digest_evidence: DigestEvidence::UserProvidedSha256,
        package_contract: PackageContract::ManifestV1,
        manifest_fingerprint: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        immutability: Immutability::Unknown,
    };

    let err = registry
        .import_with_source(&src, Some(invalid_source))
        .unwrap_err();
    assert!(err.contains("invalid installed source binding"));

    // Verify registry is empty: nothing installed
    assert!(registry.list().unwrap().is_empty());
}
