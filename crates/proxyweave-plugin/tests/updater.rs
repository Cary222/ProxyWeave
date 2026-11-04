use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use proxyweave_plugin::downloader::Release;
use proxyweave_plugin::updater::{
    check_plugin_update_at, evaluate_release, valid_digest, valid_repository_part, valid_tag,
    DigestEvidence, Immutability, InstalledSource, PackageContract, UpdateStatus,
};
use proxyweave_plugin::{Manifest, PluginRecord};
use serde_json::json;

fn sample_manifest(version: &str, external: bool) -> Manifest {
    let yaml = format!(
        r#"schema_version: '1.0'
metadata:
  id: test-updater-plugin
  name: Test Updater Plugin
  version: '{version}'
  license: MIT
runtime:
  ownership: {}
  {}
network:
  health_check:
    path: /healthz
    expected_status: 200
    identity_header: X-Service
    identity_value: test-updater-plugin
    timeout_ms: 500
lifecycle:
  startup_timeout_ms: 2000
  idle_timeout_ms: null
  max_port_retries: {}
capabilities:
  protocol: openai_chat
  models_path: /v1/models
  inference_path: /v1/chat/completions
  streaming: true
configuration: {}
"#,
        if external { "external" } else { "managed" },
        if external {
            "endpoint: http://127.0.0.1:28317"
        } else {
            "executable: bin/app.exe\n  sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        },
        if external { 0 } else { 3 },
        if external {
            "{}"
        } else {
            "{config_template: {}}"
        },
    );
    Manifest::parse(yaml.as_bytes()).unwrap()
}

fn sample_source(fingerprint: &str, tag: &str) -> InstalledSource {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let ext = if os == "windows" { "zip" } else { "tar.gz" };
    let asset_name = format!("test-plugin-{}-{}.{}", os, arch, ext);
    let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    InstalledSource {
        owner: "test-owner".to_string(),
        repo: "test-repo".to_string(),
        release_id: 1001,
        tag_name: tag.to_string(),
        asset_id: 2001,
        asset_name,
        target_os: os.to_string(),
        target_arch: arch.to_string(),
        archive_sha256: sha.to_string(),
        expected_sha256: sha.to_string(),
        digest_evidence: DigestEvidence::UserProvidedSha256,
        package_contract: PackageContract::ManifestV1,
        manifest_fingerprint: fingerprint.to_string(),
        immutability: Immutability::Unknown,
    }
}

fn sample_record(version: &str, tag: &str, external: bool) -> PluginRecord {
    let manifest = sample_manifest(version, external);
    let fingerprint =
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();
    let source = sample_source(&fingerprint, tag);
    PluginRecord {
        manifest,
        fingerprint,
        settings: Default::default(),
        source: Some(source),
    }
}

#[test]
fn test_repository_part_and_tag_and_digest_validation() {
    assert!(valid_repository_part("valid-owner", true));
    assert!(valid_repository_part("owner123", true));
    assert!(!valid_repository_part("-invalid-start", true));
    assert!(!valid_repository_part("invalid-end-", true));
    assert!(!valid_repository_part("double--hyphen", true));
    assert!(!valid_repository_part("has.dot", true)); // owner cannot contain dot
    assert!(!valid_repository_part("", true));
    assert!(!valid_repository_part(".", true));
    assert!(!valid_repository_part("..", true));

    assert!(valid_repository_part("valid_repo.js", false));
    assert!(valid_repository_part("repo-name_1", false));
    assert!(!valid_repository_part("", false));
    assert!(!valid_repository_part(".", false));
    assert!(!valid_repository_part("..", false));

    assert!(valid_tag("v1.0.0"));
    assert!(valid_tag("release-2026.03.1"));
    assert!(!valid_tag(""));
    assert!(!valid_tag("/leading-slash"));
    assert!(!valid_tag("trailing-slash/"));
    assert!(!valid_tag("path/../traversal"));

    assert!(valid_digest(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    ));
    assert!(!valid_digest("short"));
    assert!(!valid_digest(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg"
    )); // 'g' is not hex
}

#[test]
fn test_installed_source_validation_consistency() {
    let mut source = sample_source(
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "v1.0.0",
    );
    assert!(source
        .validate("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        .is_ok());

    // Wrong fingerprint rejected
    assert!(source
        .validate("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
        .is_err());

    // Archive and expected SHA mismatch rejected
    source.expected_sha256 =
        "1111111111111111111111111111111111111111111111111111111111111111".to_string();
    assert!(source
        .validate("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        .is_err());
    source.expected_sha256 = source.archive_sha256.clone();

    // Zero release_id / asset_id rejected
    source.release_id = 0;
    assert!(source
        .validate("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        .is_err());
    source.release_id = 1001;

    source.asset_id = 0;
    assert!(source
        .validate("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        .is_err());
    source.asset_id = 2001;

    // Asset traversal rejected
    source.asset_name = "../traversal.zip".to_string();
    assert!(source
        .validate("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        .is_err());

    // ChecksumAsset with same asset_id or invalid name rejected
    source.asset_name = format!(
        "test-plugin-{}-{}.zip",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    source.digest_evidence = DigestEvidence::ChecksumAsset {
        asset_id: 2001,
        asset_name: "checksums.txt".into(),
    };
    assert!(source
        .validate("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        .is_err()); // same asset id
}

#[test]
fn test_evaluate_release_status_matrix() {
    let record = sample_record("1.0.0", "v1.0.0", false);
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let ext = if os == "windows" { "zip" } else { "tar.gz" };

    // 1. Update available
    let release_newer: Release = serde_json::from_value(json!({
        "id": 1002,
        "tag_name": "v1.1.0",
        "draft": false,
        "prerelease": false,
        "assets": [{
            "id": 2002,
            "name": format!("plugin-{}-{}.{}", os, arch, ext),
            "size": 1024,
            "browser_download_url": "https://example.com/asset"
        }]
    }))
    .unwrap();
    let check = evaluate_release(&record, &release_newer);
    assert_eq!(check.status, UpdateStatus::UpdateAvailable);
    assert_eq!(check.latest_tag, Some("v1.1.0".to_string()));
    assert!(check.owned_update_allowed);

    // 2. Up to date (same version, same asset id, same name)
    let release_same: Release = serde_json::from_value(json!({
        "id": 1001,
        "tag_name": "v1.0.0",
        "draft": false,
        "prerelease": false,
        "assets": [{
            "id": 2001,
            "name": format!("test-plugin-{}-{}.{}", os, arch, ext),
            "size": 1024,
            "browser_download_url": "https://example.com/asset"
        }]
    }))
    .unwrap();
    let check = evaluate_release(&record, &release_same);
    assert_eq!(check.status, UpdateStatus::UpToDate);
    assert_eq!(check.latest_tag, Some("v1.0.0".to_string()));

    // 3. Release changed (same version, but asset ID changed)
    let release_changed_asset: Release = serde_json::from_value(json!({
        "id": 1001,
        "tag_name": "v1.0.0",
        "draft": false,
        "prerelease": false,
        "assets": [{
            "id": 9999, // Changed asset id!
            "name": format!("test-plugin-{}-{}.{}", os, arch, ext),
            "size": 1024,
            "browser_download_url": "https://example.com/asset"
        }]
    }))
    .unwrap();
    let check = evaluate_release(&record, &release_changed_asset);
    assert_eq!(check.status, UpdateStatus::ReleaseChanged);

    // 4. Installed newer
    let release_older: Release = serde_json::from_value(json!({
        "id": 900,
        "tag_name": "v0.9.0",
        "draft": false,
        "prerelease": false,
        "assets": [{
            "id": 1900,
            "name": format!("plugin-{}-{}.{}", os, arch, ext),
            "size": 1024,
            "browser_download_url": "https://example.com/asset"
        }]
    }))
    .unwrap();
    let check = evaluate_release(&record, &release_older);
    assert_eq!(check.status, UpdateStatus::InstalledNewer);

    // 5. Invalid version in release tag (e.g. branch or non-semver)
    let release_invalid_tag: Release = serde_json::from_value(json!({
        "id": 1003,
        "tag_name": "nightly-main",
        "draft": false,
        "prerelease": false,
        "assets": []
    }))
    .unwrap();
    let check = evaluate_release(&record, &release_invalid_tag);
    assert_eq!(check.status, UpdateStatus::InvalidVersion);

    // 6. Draft or prerelease returns Error (not a stable release)
    let release_draft: Release = serde_json::from_value(json!({
        "id": 1004,
        "tag_name": "v1.2.0",
        "draft": true,
        "prerelease": false,
        "assets": []
    }))
    .unwrap();
    let check = evaluate_release(&record, &release_draft);
    assert_eq!(check.status, UpdateStatus::Error);
    assert_eq!(check.detail, "not_stable_release");

    // 7. External service: owned_update_allowed is false
    let ext_record = sample_record("1.0.0", "v1.0.0", true);
    let check = evaluate_release(&ext_record, &release_newer);
    assert_eq!(check.status, UpdateStatus::UpdateAvailable);
    assert!(!check.owned_update_allowed);
}

#[test]
fn test_eligibility_checks() {
    // Record without source
    let mut record = sample_record("1.0.0", "v1.0.0", false);
    record.source = None;
    let release_newer: Release = serde_json::from_value(json!({
        "id": 1002,
        "tag_name": "v1.1.0",
        "draft": false,
        "prerelease": false,
        "assets": []
    }))
    .unwrap();
    let check = evaluate_release(&record, &release_newer);
    assert_eq!(check.status, UpdateStatus::NotCheckable);
    assert_eq!(check.detail, "source_missing");

    // Version mismatch between manifest and source tag
    let mut record = sample_record("1.0.0", "v1.0.0", false);
    record.source.as_mut().unwrap().tag_name = "v2.0.0".to_string();
    let check = evaluate_release(&record, &release_newer);
    assert_eq!(check.status, UpdateStatus::InvalidVersion);
    assert_eq!(check.detail, "installed_tag_mismatch");
}

#[tokio::test]
async fn test_check_plugin_update_at_http_matrix() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let _server_thread = thread::spawn(move || {
        while let Ok((mut socket, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let n = socket.read(&mut buf).unwrap_or(0);
            if n == 0 {
                continue;
            }
            let req = String::from_utf8_lossy(&buf[..n]);

            if req.contains("/success-update") {
                let body = json!({
                    "id": 2000,
                    "tag_name": "v2.0.0",
                    "draft": false,
                    "prerelease": false,
                    "assets": []
                })
                .to_string();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(), body
                );
                let _ = socket.write_all(resp.as_bytes());
            } else if req.contains("/forbidden") {
                let resp = "HTTP/1.1 403 Forbidden\r\nConnection: close\r\nx-ratelimit-remaining: 0\r\nx-ratelimit-reset: 1700000000\r\nContent-Length: 0\r\n\r\n";
                let _ = socket.write_all(resp.as_bytes());
            } else if req.contains("/not-found") {
                let resp =
                    "HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
                let _ = socket.write_all(resp.as_bytes());
            } else if req.contains("/server-error") {
                let resp = "HTTP/1.1 500 Internal Server Error\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
                let _ = socket.write_all(resp.as_bytes());
            } else if req.contains("/bad-json") {
                let resp = "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: 12\r\n\r\n{invalid json";
                let _ = socket.write_all(resp.as_bytes());
            }
        }
    });

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let record = sample_record("1.0.0", "v1.0.0", false);

    // 1. Success 200 OK -> UpdateAvailable
    let check =
        check_plugin_update_at(&record, &client, &format!("http://{addr}/success-update")).await;
    assert_eq!(check.status, UpdateStatus::UpdateAvailable);
    assert_eq!(check.latest_tag, Some("v2.0.0".to_string()));
    assert_eq!(check.http_status, Some(200));

    // 2. 403 Forbidden -> Forbidden, keeps rate limit headers, does not leak URL or secret
    let check = check_plugin_update_at(&record, &client, &format!("http://{addr}/forbidden")).await;
    assert_eq!(check.status, UpdateStatus::Forbidden);
    assert_eq!(check.http_status, Some(403));
    assert_eq!(check.rate_limit_remaining, Some(0));
    assert_eq!(check.rate_limit_reset, Some(1700000000));
    assert_eq!(check.detail, "http_forbidden");

    // 3. 404 Not Found -> NotFound
    let check = check_plugin_update_at(&record, &client, &format!("http://{addr}/not-found")).await;
    assert_eq!(check.status, UpdateStatus::NotFound);
    assert_eq!(check.http_status, Some(404));
    assert_eq!(check.detail, "release_not_found_or_inaccessible");

    // 4. 500 Server Error -> Error
    let check =
        check_plugin_update_at(&record, &client, &format!("http://{addr}/server-error")).await;
    assert_eq!(check.status, UpdateStatus::Error);
    assert_eq!(check.http_status, Some(500));
    assert_eq!(check.detail, "http_error");

    // 5. Bad JSON -> Error
    let check = check_plugin_update_at(&record, &client, &format!("http://{addr}/bad-json")).await;
    assert_eq!(check.status, UpdateStatus::Error);
    assert_eq!(check.http_status, Some(200));
    assert_eq!(check.detail, "invalid_release_json");
}

#[cfg(windows)]
mod update_containment {
    use super::*;
    use proxyweave_plugin::updater::{apply_plugin_update, apply_plugin_update_with_backend};
    use proxyweave_plugin::{Registry, Runtime, RuntimeManager, RuntimeStatus};
    use sha2::{Digest, Sha256};
    use std::{fs, path::Path, time::Duration};

    const ID: &str = "test-updater-plugin";

    #[test]
    #[ignore = "isolated managed test server"]
    fn update_fixture_child() {
        if std::env::var("UPDATE_FAIL").as_deref() == Ok("true") {
            std::process::exit(1);
        }
        let port = std::env::var("UPDATE_PORT").unwrap();
        let listener = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();
        for mut socket in listener.incoming().flatten() {
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 4096];
            let _ = socket.read(&mut request);
            let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nX-Service: test-updater-plugin\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK");
        }
    }

    fn package(root: &Path, version: &str, fail: bool, external: bool) -> std::path::PathBuf {
        fs::create_dir_all(root.join("bin")).unwrap();
        let bytes = fs::read(std::env::current_exe().unwrap()).unwrap();
        fs::write(root.join("bin/app.exe"), &bytes).unwrap();
        let mut manifest = sample_manifest(version, external);
        if let Runtime::Managed {
            sha256, args, env, ..
        } = &mut manifest.runtime
        {
            *sha256 = format!("{:x}", Sha256::digest(bytes));
            *args = vec![
                "--exact".into(),
                "update_containment::update_fixture_child".into(),
                "--ignored".into(),
                "--nocapture".into(),
            ];
            env.insert("UPDATE_PORT".into(), "{PORT}".into());
            env.insert("UPDATE_FAIL".into(), fail.to_string());
        }
        if !external {
            manifest
                .configuration
                .settings
                .insert("fixture_setting".into(), json!("initial"));
        }
        manifest.lifecycle.startup_timeout_ms = 3000;
        let path = root.join("manifest.yaml");
        fs::write(&path, serde_yaml_ng::to_string(&manifest).unwrap()).unwrap();
        path
    }

    fn sandbox(external: bool) -> (tempfile::TempDir, Registry) {
        let root = tempfile::tempdir().unwrap();
        let registry = Registry::open(root.path().join("registry")).unwrap();
        registry
            .import(package(&root.path().join("old"), "1.0.0", false, external))
            .unwrap();
        (root, registry)
    }

    async fn start(registry: &Registry, runtime: &RuntimeManager) -> (u16, u32) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        match runtime
            .start_plugin(
                &registry.load(ID).unwrap(),
                &registry.package_dir(ID).unwrap(),
                Some(port),
                &registry.data_dir(ID).unwrap(),
                None,
            )
            .await
            .unwrap()
        {
            RuntimeStatus::Ready {
                port,
                pid: Some(pid),
            } => (port, pid),
            other => panic!("expected ready: {other:?}"),
        }
    }

    fn alive(pid: u32) -> bool {
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
        };
        unsafe {
            let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if process.is_null() {
                return false;
            }
            let running = WaitForSingleObject(process, 0) == 258;
            CloseHandle(process);
            running
        }
    }

    const UNAVAILABLE: &str = "update_unavailable_pending_safety_validation";

    // Only authoritative package/data bytes; consumed staging TempDirs may clean themselves up.
    fn installed_bytes(
        registry: &Registry,
    ) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
        fn collect(
            root: &Path,
            path: &Path,
            out: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
        ) {
            for entry in fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    collect(root, &path, out);
                } else {
                    out.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }
        let package = registry.package_dir(ID).unwrap();
        let mut out = std::collections::BTreeMap::new();
        collect(package.parent().unwrap(), &package, &mut out);
        let data = registry.data_dir(ID).unwrap();
        if data.exists() {
            collect(data.parent().unwrap(), &data, &mut out);
        }
        out
    }

    fn assert_no_swap_artifacts(registry: &Registry) {
        let package = registry.package_dir(ID).unwrap();
        let names: Vec<_> = fs::read_dir(package.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from(ID)]);
    }
    #[tokio::test]
    async fn test_containment_with_active_requests_does_not_touch_running_instance() {
        let (root, registry) = sandbox(false);
        let runtime = RuntimeManager::new();
        let (_, pid) = start(&registry, &runtime).await;
        let lease = runtime.acquire_request(ID).await.unwrap();
        let before = installed_bytes(&registry);
        let status = runtime.get_status(ID).await;
        let staged = registry
            .prepare_staged_package(
                &package(&root.path().join("new"), "2.0.0", false, false),
                None,
            )
            .unwrap();
        assert_eq!(
            apply_plugin_update(&registry, &runtime, ID, staged, None)
                .await
                .unwrap_err(),
            UNAVAILABLE
        );
        assert!(alive(pid));
        assert_eq!(runtime.get_status(ID).await, status);
        assert_eq!(runtime.active_request_count(ID).await, 1);
        assert_eq!(installed_bytes(&registry), before);
        assert_no_swap_artifacts(&registry);
        assert_eq!(
            registry.load(ID).unwrap().manifest.metadata.version,
            "1.0.0"
        );
        drop(lease);
        runtime.stop_plugin(ID).await.unwrap();
    }

    #[tokio::test]
    async fn test_containment_external_instance_and_record_untouched() {
        // An independently owned server must stay alive and attached throughout rejection.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        struct ExternalChild(std::process::Child);
        impl Drop for ExternalChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut child = ExternalChild(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "update_containment::update_fixture_child",
                    "--ignored",
                ])
                .env("UPDATE_PORT", port.to_string())
                .env("UPDATE_FAIL", "false")
                .spawn()
                .unwrap(),
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "external fixture did not start"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let root = tempfile::tempdir().unwrap();
        let registry = Registry::open(root.path().join("registry")).unwrap();
        let manifest_path = package(&root.path().join("old"), "1.0.0", false, true);
        let mut manifest = sample_manifest("1.0.0", true);
        if let Runtime::External { endpoint } = &mut manifest.runtime {
            *endpoint = format!("http://127.0.0.1:{port}");
        }
        fs::write(&manifest_path, serde_yaml_ng::to_string(&manifest).unwrap()).unwrap();
        registry.import(manifest_path).unwrap();
        let runtime = RuntimeManager::new();
        let status = runtime
            .start_plugin(
                &registry.load(ID).unwrap(),
                &registry.package_dir(ID).unwrap(),
                None,
                &registry.data_dir(ID).unwrap(),
                None,
            )
            .await
            .unwrap();
        assert!(matches!(status, RuntimeStatus::Ready { pid: None, .. }));
        let before = fs::read(registry.package_dir(ID).unwrap().join("record.json")).unwrap();
        let staged = registry
            .prepare_staged_package(
                &package(&root.path().join("new"), "2.0.0", false, false),
                None,
            )
            .unwrap();
        assert_eq!(
            apply_plugin_update(&registry, &runtime, ID, staged, None)
                .await
                .unwrap_err(),
            UNAVAILABLE
        );
        assert_eq!(
            before,
            fs::read(registry.package_dir(ID).unwrap().join("record.json")).unwrap()
        );
        assert_eq!(runtime.get_status(ID).await, status);
        assert!(child.0.try_wait().unwrap().is_none());
        assert_eq!(
            reqwest::get(format!("http://127.0.0.1:{port}/healthz"))
                .await
                .unwrap()
                .status(),
            200
        );
    }

    #[tokio::test]
    async fn test_containment_stopped_apply_never_starts_or_changes_package() {
        let (root, registry) = sandbox(false);
        let runtime = RuntimeManager::new();
        let before = installed_bytes(&registry);
        let staged = registry
            .prepare_staged_package(
                &package(&root.path().join("new"), "2.0.0", false, false),
                None,
            )
            .unwrap();
        assert_eq!(
            apply_plugin_update(&registry, &runtime, ID, staged, None)
                .await
                .unwrap_err(),
            UNAVAILABLE
        );
        assert_eq!(runtime.get_status(ID).await, RuntimeStatus::Stopped);
        assert_eq!(installed_bytes(&registry), before);
        assert_no_swap_artifacts(&registry);
    }

    #[tokio::test]
    async fn test_containment_ready_backend_callback_never_called() {
        let (root, registry) = sandbox(false);
        let runtime = RuntimeManager::new();
        let (_, pid) = start(&registry, &runtime).await;
        let status = runtime.get_status(ID).await;
        let before = installed_bytes(&registry);
        let staged = registry
            .prepare_staged_package(
                &package(&root.path().join("new"), "2.0.0", false, false),
                None,
            )
            .unwrap();
        let mut called = false;
        let result = apply_plugin_update_with_backend(
            &registry,
            &runtime,
            ID,
            staged,
            Some("fixture-secret"),
            |_| {
                called = true;
                async { Ok(()) }
            },
        )
        .await;
        assert_eq!(result.unwrap_err(), UNAVAILABLE);
        assert!(!called);
        assert_eq!(runtime.get_status(ID).await, status);
        assert!(alive(pid));
        assert_eq!(installed_bytes(&registry), before);
        assert_no_swap_artifacts(&registry);
        runtime.stop_plugin(ID).await.unwrap();
    }

    #[tokio::test]
    async fn test_containment_bad_candidate_does_not_start_or_rollback_any_instance() {
        let (root, registry) = sandbox(false);
        let runtime = RuntimeManager::new();
        let (_, pid) = start(&registry, &runtime).await;
        let status = runtime.get_status(ID).await;
        let before = installed_bytes(&registry);
        let staged = registry
            .prepare_staged_package(
                &package(&root.path().join("new"), "2.0.0", true, false),
                None,
            )
            .unwrap();
        assert_eq!(
            apply_plugin_update(&registry, &runtime, ID, staged, None)
                .await
                .unwrap_err(),
            UNAVAILABLE
        );
        assert_eq!(runtime.get_status(ID).await, status);
        assert!(alive(pid));
        assert_eq!(installed_bytes(&registry), before);
        assert_no_swap_artifacts(&registry);
        runtime.stop_plugin(ID).await.unwrap();
    }

    #[test]
    fn test_containment_direct_registry_swap_never_replaces_installed_bytes() {
        let (root, registry) = sandbox(false);
        let mut settings = std::collections::BTreeMap::new();
        settings.insert("fixture_setting".to_string(), json!("preserve"));
        registry.save_settings(ID, settings).unwrap();
        fs::create_dir_all(registry.data_dir(ID).unwrap()).unwrap();
        fs::write(
            registry.data_dir(ID).unwrap().join("sentinel.json"),
            b"{\"preserve\":true}",
        )
        .unwrap();
        let before = installed_bytes(&registry);
        let staged = registry
            .prepare_staged_package(
                &package(&root.path().join("new"), "2.0.0", false, false),
                None,
            )
            .unwrap();
        assert_eq!(
            registry.atomic_swap_package(ID, staged).err().unwrap(),
            UNAVAILABLE
        );
        assert_eq!(installed_bytes(&registry), before);
        assert_no_swap_artifacts(&registry);
    }
}
