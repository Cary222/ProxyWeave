// Compile the exact source modules here to inject fixture assets without a public
// override API, feature flag, or production hash relaxation.
#[allow(dead_code)]
#[path = "../src/detector.rs"]
mod detector;
#[allow(dead_code)]
#[path = "../src/manifest.rs"]
mod manifest;
#[allow(dead_code)]
#[path = "../src/registry.rs"]
mod registry;

use manifest::{Manifest, Runtime};
pub use proxyweave_plugin::management;
use registry::Registry;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

const CATALOG: &[u8] = include_bytes!("../../../manifests/local-project-adapters.yaml");
const APPROVED: &str = include_str!("../../../manifests/wb2api.manifest.yaml");
const EXE: &[u8] = b"offline fixture, not an executable; never run";
const LICENSE: &[u8] = b"synthetic license text; not upstream copyright evidence";

fn yaml() -> String {
    APPROVED
        .replace(
            "38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485",
            &format!("{:x}", Sha256::digest(EXE)),
        )
        .replace(
            "49b9e27ddfc7cceab55e19eb7d405bda30a8229de3cc094e6b4d0397ab76a34b",
            &format!("{:x}", Sha256::digest(LICENSE)),
        )
}

fn fixture(root: &Path, exe: &str) {
    let path = root.join(exe);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, EXE).unwrap();
    fs::write(root.join("LICENSE"), LICENSE).unwrap();
}

fn detect(root: &Path) -> Result<detector::DetectionReport, String> {
    detector::detect_with_assets(root, CATALOG, &[("wb2api", yaml().as_bytes())])
}

fn stage(registry: &Registry, root: &Path) -> Result<registry::ImportResult, String> {
    detector::stage_with_assets(
        registry,
        root,
        None,
        CATALOG,
        &[("wb2api", yaml().as_bytes())],
    )
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(base, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

#[test]
fn root_and_bin_map_to_identical_canonical_fingerprint_and_reload() {
    let root = tempfile::tempdir().unwrap();
    let bin = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    fixture(root.path(), "wb2api.exe");
    fixture(bin.path(), "bin/wb2api.exe");
    let registry = Registry::open(profile.path()).unwrap();
    let first = stage(&registry, root.path()).unwrap();
    let second = stage(&registry, bin.path()).unwrap();
    assert!(!first.already_registered);
    assert!(second.already_registered);
    assert_eq!(first.record, second.record);
    let Runtime::Managed { executable, .. } = &first.record.manifest.runtime else {
        panic!()
    };
    assert_eq!(executable, "bin/wb2api.exe");
    assert_eq!(
        fs::read(registry.package_dir("wb2api").unwrap().join(executable)).unwrap(),
        EXE
    );
    let mut settings = first.record.settings.clone();
    settings.insert("label".into(), serde_json::json!("saved"));
    registry.save_settings("wb2api", settings.clone()).unwrap();
    assert_eq!(
        stage(&registry, root.path()).unwrap().record.settings,
        settings
    );
    drop(registry);
    let reopened = Registry::open(profile.path()).unwrap();
    assert_eq!(reopened.load("wb2api").unwrap().settings, settings);
    assert_eq!(reopened.list().unwrap().len(), 1);
    assert_eq!(
        reopened.load("wb2api").unwrap().fingerprint,
        first.record.fingerprint
    );
}

#[test]
fn complete_manifest_preserves_secrets_auth_state_and_management() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), "wb2api.exe");
    let report = detect(root.path()).unwrap();
    let expected = Manifest::parse(yaml().as_bytes()).unwrap();
    assert_eq!(report.manifest, expected);
    assert_eq!(report.version, "1.13.0-panel");
    assert_eq!(report.executable_relative_path, "wb2api.exe");
    assert_eq!(report.project_type, "local_adapter");
    let config = &report.manifest.configuration;
    assert_eq!(
        config.required_secrets,
        ["access_token", "refresh_token", "uid"]
    );
    assert_eq!(config.config_template["auth_dir"], "{AUTH_DIR}");
    assert_eq!(
        config.config_template["state_file"],
        "{DATA_DIR}/state.json"
    );
    assert_eq!(config.config_template["api_key"], "{PLUGIN_API_KEY}");
    assert_eq!(
        config.credential_files["auths/workbuddy-test.json"]["auth"]["accessToken"],
        "{SECRET:access_token}"
    );
    assert_eq!(
        config.credential_files["auths/workbuddy-test.json"]["auth"]["refreshToken"],
        "{SECRET:refresh_token}"
    );
    assert_eq!(
        config.credential_files["auths/workbuddy-test.json"]["account"]["uid"],
        "{SECRET:uid}"
    );
    let mgmt = report.manifest.management.unwrap();
    assert_eq!(mgmt.path, "/panel/");
    assert_eq!(mgmt.presentation, manifest::ManagementPresentation::Browser);
}

#[test]
fn production_asset_hashes_remain_pinned_and_do_not_accept_fixture() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), "wb2api.exe");
    let approved = Manifest::parse(APPROVED.as_bytes()).unwrap();
    let Runtime::Managed {
        sha256, resources, ..
    } = approved.runtime
    else {
        panic!()
    };
    assert_eq!(
        sha256,
        "38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485"
    );
    assert_eq!(
        resources["LICENSE"],
        "49b9e27ddfc7cceab55e19eb7d405bda30a8229de3cc094e6b4d0397ab76a34b"
    );
    assert!(proxyweave_plugin::detect_project(root.path())
        .unwrap_err()
        .contains("SHA256"));
    let profile = tempfile::tempdir().unwrap();
    let registry = proxyweave_plugin::Registry::open(profile.path()).unwrap();
    assert!(
        proxyweave_plugin::stage_and_import_project(&registry, root.path(), None)
            .unwrap_err()
            .contains("SHA256")
    );
    assert!(registry.list().unwrap().is_empty());
}

#[test]
fn unsupported_projects_are_not_guessed_from_names_or_readme() {
    let root = tempfile::tempdir().unwrap();
    for name in [
        "anything.exe",
        "cli-proxy-api.exe",
        "README.md",
        "LICENSE.txt",
    ] {
        fs::write(root.path().join(name), b"MIT Version 99.0.0").unwrap();
    }
    assert!(detect(root.path()).unwrap_err().contains("unsupported"));
    let profile = tempfile::tempdir().unwrap();
    let registry = Registry::open(profile.path()).unwrap();
    assert!(stage(&registry, root.path())
        .unwrap_err()
        .contains("unsupported"));
    assert!(registry.list().unwrap().is_empty());
    assert!(detect(&root.path().join("absent"))
        .unwrap_err()
        .contains("missing"));
}

#[test]
fn hash_license_missing_resource_and_ambiguous_sources_reject_without_publish() {
    for (relative, bytes, expected) in [
        ("wb2api.exe", Some(b"changed".as_slice()), "SHA256"),
        ("LICENSE", Some(b"changed".as_slice()), "SHA256"),
        ("LICENSE", None, "missing"),
        ("wb2api.exe", None, "unsupported"),
        ("bin/wb2api.exe", Some(EXE), "ambiguous"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let profile = tempfile::tempdir().unwrap();
        fixture(root.path(), "wb2api.exe");
        let path = root.path().join(relative);
        if let Some(bytes) = bytes {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        } else {
            fs::remove_file(path).unwrap();
        }
        assert!(detect(root.path()).unwrap_err().contains(expected));
        let registry = Registry::open(profile.path()).unwrap();
        assert!(stage(&registry, root.path())
            .unwrap_err()
            .contains(expected));
        assert!(registry.list().unwrap().is_empty());
    }
}

#[test]
fn invalid_explicit_manifest_never_falls_back_even_with_override() {
    let root = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    fixture(root.path(), "wb2api.exe");
    let registry = Registry::open(profile.path()).unwrap();
    for invalid in [
        "not: a manifest",
        "[broken",
        "schema_version: '1.0'\nschema_version: '1.0'",
        "---\n---",
    ] {
        fs::write(root.path().join("manifest.yaml"), invalid).unwrap();
        assert!(detect(root.path()).is_err());
        assert!(stage(&registry, root.path()).is_err());
        assert!(detector::stage_with_assets(
            &registry,
            root.path(),
            Some(Manifest::parse(yaml().as_bytes()).unwrap()),
            CATALOG,
            &[("wb2api", yaml().as_bytes())]
        )
        .is_err());
        assert!(registry.list().unwrap().is_empty());
    }
}
#[test]
fn explicit_manifest_missing_resource_or_wrong_hash_never_uses_valid_adapter() {
    let root = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    fixture(root.path(), "wb2api.exe");
    fixture(root.path(), "bin/wb2api.exe");
    let registry = Registry::open(profile.path()).unwrap();
    for invalid in [
        yaml().replace("executable: bin/wb2api.exe", "executable: missing.exe"),
        yaml().replace(
            &format!("{:x}", Sha256::digest(EXE)),
            &format!("{:x}", Sha256::digest(b"wrong artifact")),
        ),
    ] {
        fs::write(root.path().join("manifest.yaml"), invalid).unwrap();
        let detection_error = detect(root.path()).unwrap_err();
        assert!(detection_error.contains("missing") || detection_error.contains("SHA256"));
        let import_error = stage(&registry, root.path()).unwrap_err();
        assert!(import_error.contains("missing") || import_error.contains("SHA256"));
        assert!(registry.list().unwrap().is_empty());
    }
}

#[test]
fn explicit_package_priority_and_override_remain_strict() {
    let root = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    fixture(root.path(), "bin/wb2api.exe");
    // Both adapter names exist, but explicit manifest owns source selection.
    fs::write(root.path().join("wb2api.exe"), b"not the explicit artifact").unwrap();
    let explicit = yaml().replace("name: WorkBuddy 2API Panel", "name: Explicit Package");
    fs::write(root.path().join("manifest.yaml"), &explicit).unwrap();
    let registry = Registry::open(profile.path()).unwrap();
    assert_eq!(detect(root.path()).unwrap().project_type, "manifest");
    assert_eq!(
        stage(&registry, root.path())
            .unwrap()
            .record
            .manifest
            .metadata
            .name,
        "Explicit Package"
    );
    fs::remove_file(root.path().join("manifest.yaml")).unwrap();
    let profile2 = tempfile::tempdir().unwrap();
    let registry2 = Registry::open(profile2.path()).unwrap();
    let manifest = Manifest::parse(explicit.as_bytes()).unwrap();
    assert!(detector::stage_with_assets(
        &registry2,
        root.path(),
        Some(manifest.clone()),
        CATALOG,
        &[]
    )
    .is_ok());
    let mut invalid = manifest;
    invalid.metadata.version = "invalid".into();
    assert!(
        detector::stage_with_assets(&registry2, root.path(), Some(invalid), CATALOG, &[]).is_err()
    );
    fs::write(
        root.path().join("bin/wb2api.exe"),
        b"changed after detection",
    )
    .unwrap();
    assert!(stage(&registry, root.path()).is_err());
    assert_eq!(
        registry.load("wb2api").unwrap().manifest.metadata.name,
        "Explicit Package"
    );
}

#[test]
fn public_explicit_package_roundtrip_without_catalog_injection() {
    let root = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    fixture(root.path(), "bin/wb2api.exe");
    fs::write(root.path().join("manifest.yaml"), yaml()).unwrap();
    let report = proxyweave_plugin::detect_project(root.path()).unwrap();
    assert_eq!(report.project_type, "manifest");
    let registry = proxyweave_plugin::Registry::open(profile.path()).unwrap();
    let result = proxyweave_plugin::stage_and_import_project(&registry, root.path(), None).unwrap();
    assert_eq!(result.record.manifest, report.manifest);
    assert!(
        proxyweave_plugin::stage_and_import_project(&registry, root.path(), None)
            .unwrap()
            .already_registered
    );
}

#[test]
fn conflict_keeps_registered_record_and_settings_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    fixture(root.path(), "wb2api.exe");
    let registry = Registry::open(profile.path()).unwrap();
    stage(&registry, root.path()).unwrap();
    let mut settings = registry.load("wb2api").unwrap().settings;
    settings.insert("label".into(), serde_json::json!("keep"));
    let prior = registry.save_settings("wb2api", settings).unwrap();
    let changed = yaml().replace("name: WorkBuddy 2API Panel", "name: Conflict");
    assert!(detector::stage_with_assets(
        &registry,
        root.path(),
        None,
        CATALOG,
        &[("wb2api", changed.as_bytes())]
    )
    .unwrap_err()
    .contains("different package"));
    assert_eq!(registry.load("wb2api").unwrap(), prior);
}

#[test]
fn only_declared_assets_copied_and_source_byte_tree_is_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    fixture(root.path(), "wb2api.exe");
    for name in [
        "README.md",
        ".git/config",
        "config.json",
        "auths/private.json",
        "data/state.json",
        "LICENSE.txt",
    ] {
        let path = root.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"synthetic sentinel, not a real credential").unwrap();
    }
    let before = tree(root.path());
    let registry = Registry::open(profile.path()).unwrap();
    stage(&registry, root.path()).unwrap();
    assert_eq!(tree(root.path()), before);
    let package = tree(&registry.package_dir("wb2api").unwrap());
    assert_eq!(
        package.keys().map(String::as_str).collect::<Vec<_>>(),
        ["LICENSE", "bin/wb2api.exe", "record.json"]
    );
    assert_eq!(package["LICENSE"], LICENSE);
    assert!(!root.path().join("manifest.yaml").exists());
    assert!(tree(&profile.path().join("data")).is_empty());
}

#[test]
fn extra_declared_resource_must_exist_and_match_hash() {
    let root = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    fixture(root.path(), "wb2api.exe");
    let extra = yaml().replace(
        "  resources:\n",
        &format!(
            "  resources:\n    assets/notice: {:x}\n",
            Sha256::digest(b"notice")
        ),
    );
    let registry = Registry::open(profile.path()).unwrap();
    assert!(detector::stage_with_assets(
        &registry,
        root.path(),
        None,
        CATALOG,
        &[("wb2api", extra.as_bytes())]
    )
    .unwrap_err()
    .contains("missing"));
    fs::create_dir_all(root.path().join("assets")).unwrap();
    fs::write(root.path().join("assets/notice"), b"wrong").unwrap();
    assert!(detector::stage_with_assets(
        &registry,
        root.path(),
        None,
        CATALOG,
        &[("wb2api", extra.as_bytes())]
    )
    .unwrap_err()
    .contains("SHA256"));
    fs::write(root.path().join("assets/notice"), b"notice").unwrap();
    detector::stage_with_assets(
        &registry,
        root.path(),
        None,
        CATALOG,
        &[("wb2api", extra.as_bytes())],
    )
    .unwrap();
    assert_eq!(
        fs::read(
            registry
                .package_dir("wb2api")
                .unwrap()
                .join("assets/notice")
        )
        .unwrap(),
        b"notice"
    );
}
#[test]
fn source_changed_after_detection_is_rechecked_without_overwriting_registration() {
    let root = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    fixture(root.path(), "wb2api.exe");
    let registry = Registry::open(profile.path()).unwrap();
    let prior = stage(&registry, root.path()).unwrap().record;
    let detected = detect(root.path()).unwrap();
    assert_eq!(detected.manifest, prior.manifest);
    fs::write(root.path().join("wb2api.exe"), b"changed after detection").unwrap();
    assert!(stage(&registry, root.path())
        .unwrap_err()
        .contains("SHA256"));
    assert_eq!(registry.load("wb2api").unwrap(), prior);
}

#[test]
fn external_manifest_detection_and_registry_mount_do_not_stage_or_own_process() {
    let root = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    let external = r#"schema_version: '1.0'
metadata: {id: wb2api, name: External wb2api, version: 1.0.0, license: MIT}
runtime: {ownership: external, endpoint: 'http://127.0.0.1:12345/'}
network:
  health_check: {path: /healthz, expected_status: 200, identity_header: X-Service, identity_value: workbuddy2api, timeout_ms: 2000}
lifecycle: {startup_timeout_ms: 1000, idle_timeout_ms: null, max_port_retries: 0}
capabilities: {protocol: openai_chat, models_path: /v1/models, inference_path: /v1/chat/completions, streaming: true}
"#;
    fs::write(root.path().join("manifest.yaml"), external).unwrap();
    assert_eq!(
        detect(root.path()).unwrap().manifest.ownership(),
        manifest::Ownership::External
    );
    let registry = Registry::open(profile.path()).unwrap();
    assert!(stage(&registry, root.path())
        .unwrap_err()
        .contains("External"));
    registry.import(root.path().join("manifest.yaml")).unwrap();
    assert_eq!(
        registry.load("wb2api").unwrap().manifest.ownership(),
        manifest::Ownership::External
    );
}

#[test]
fn catalog_and_source_paths_use_existing_validation() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), "wb2api.exe");
    assert!(detector::detect_with_assets(
        root.path(),
        b"- manifest_asset: unknown\n  executable_sources: [wb2api.exe]",
        &[]
    )
    .unwrap_err()
    .contains("unknown embedded"));
    for source in ["../wb2api.exe", "C:/wb2api.exe", "bin/x:stream", "CON.exe"] {
        let catalog = format!("- manifest_asset: wb2api\n  executable_sources: ['{source}']");
        assert!(detector::detect_with_assets(
            root.path(),
            catalog.as_bytes(),
            &[("wb2api", yaml().as_bytes())]
        )
        .is_err());
    }
    assert!(detector::detect_with_assets(
        root.path(),
        b"- manifest_asset: wb2api\n  executable_sources: [wb2api.exe]\n  script: forbidden",
        &[("wb2api", yaml().as_bytes())]
    )
    .is_err());
}

#[cfg(any(unix, windows))]
fn file_link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let result = std::os::windows::fs::symlink_file(target, link);
    match result {
        Ok(()) => true,
        #[cfg(windows)]
        Err(e) if e.raw_os_error() == Some(1314) => {
            eprintln!("SKIP file symlink assertion: privilege not held (OS error 1314)");
            false
        }
        Err(e) => panic!("cannot create fixture symlink: {e}"),
    }
}

#[cfg(any(unix, windows))]
#[test]
fn source_file_and_manifest_links_rejected_before_open() {
    for name in ["wb2api.exe", "LICENSE", "manifest.yaml"] {
        let root = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let profile = tempfile::tempdir().unwrap();
        fixture(root.path(), "wb2api.exe");
        let target = elsewhere.path().join("payload");
        fs::write(
            &target,
            if name == "manifest.yaml" {
                yaml().into_bytes()
            } else {
                EXE.to_vec()
            },
        )
        .unwrap();
        let link = root.path().join(name);
        if link.exists() {
            fs::remove_file(&link).unwrap();
        }
        if !file_link(&target, &link) {
            continue;
        }
        assert!(detect(root.path()).unwrap_err().contains("links"));
        let registry = Registry::open(profile.path()).unwrap();
        assert!(stage(&registry, root.path()).unwrap_err().contains("links"));
        assert!(registry.list().unwrap().is_empty());
    }
}

#[cfg(any(unix, windows))]
#[test]
fn directory_links_or_windows_junction_ancestors_rejected() {
    let parent = tempfile::tempdir().unwrap();
    let target = parent.path().join("target");
    fixture(&target, "bin/wb2api.exe");
    let linked = parent.path().join("linked");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &linked).unwrap();
    #[cfg(windows)]
    {
        let output = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&linked)
            .arg(&target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "fixture junction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(detect(&linked).is_err());
    assert!(detect(&linked.join("bin")).is_err());
    let source = parent.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("LICENSE"), LICENSE).unwrap();
    let bin = source.join("bin");
    #[cfg(unix)]
    std::os::unix::fs::symlink(target.join("bin"), &bin).unwrap();
    #[cfg(windows)]
    {
        let output = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&bin)
            .arg(target.join("bin"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "fixture junction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let profile = tempfile::tempdir().unwrap();
    let registry = Registry::open(profile.path()).unwrap();
    assert!(stage(&registry, &source).is_err());
    assert!(registry.list().unwrap().is_empty());
    // Unix directory symlinks are removed with `remove_file`; Windows junctions
    // are real directory entries and need `remove_dir`.
    #[cfg(unix)]
    {
        fs::remove_file(linked).unwrap();
        fs::remove_file(bin).unwrap();
    }
    #[cfg(windows)]
    {
        fs::remove_dir(linked).unwrap();
        fs::remove_dir(bin).unwrap();
    }
}
