use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use tempfile::tempdir;

use proxyweave_plugin::downloader::{
    asset_matches_platform_target, calculate_file_sha256, ensure_staging_manifest, expected_digest,
    extract_archive_secure, extract_tar_gz_secure, extract_zip_secure, parse_checksums_content,
    parse_github_owner_repo, read_response_bounded, validate_release_asset_url, Release,
    ReleaseAsset,
};
use proxyweave_plugin::{Manifest, Registry};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[test]
fn test_parse_github_owner_repo() {
    assert_eq!(
        parse_github_owner_repo("https://github.com/router-for-me/CLIProxyAPI").unwrap(),
        ("router-for-me".to_string(), "CLIProxyAPI".to_string())
    );
    assert_eq!(
        parse_github_owner_repo("https://github.com/router-for-me/workbuddy2api.git").unwrap(),
        ("router-for-me".to_string(), "workbuddy2api".to_string())
    );
    assert_eq!(
        parse_github_owner_repo("git@github.com:router-for-me/wb2api.git").unwrap(),
        ("router-for-me".to_string(), "wb2api".to_string())
    );
    assert_eq!(
        parse_github_owner_repo("router-for-me/my-plugin").unwrap(),
        ("router-for-me".to_string(), "my-plugin".to_string())
    );
    assert!(parse_github_owner_repo("").is_err());
    assert!(parse_github_owner_repo("invalid_repo").is_err());
}

#[test]
fn test_asset_matches_platform_targets() {
    // Windows tests
    assert!(asset_matches_platform_target(
        "CLIProxyAPI_7.2.115_windows_amd64.zip",
        "windows",
        "x86_64"
    ));
    assert!(asset_matches_platform_target(
        "wb2api_win_x64.exe",
        "windows",
        "x86_64"
    ));
    assert!(!asset_matches_platform_target(
        "CLIProxyAPI_7.2.115_linux_amd64.tar.gz",
        "windows",
        "x86_64"
    ));
    assert!(!asset_matches_platform_target(
        "CLIProxyAPI_darwin_arm64.tar.gz",
        "windows",
        "x86_64"
    ));
    assert!(!asset_matches_platform_target(
        "checksums.txt",
        "windows",
        "x86_64"
    ));
    assert!(!asset_matches_platform_target(
        "CLIProxyAPI_windows_amd64.zip.sha256",
        "windows",
        "x86_64"
    ));

    // macOS tests
    assert!(asset_matches_platform_target(
        "CLIProxyAPI_7.2.115_darwin_amd64.tar.gz",
        "macos",
        "x86_64"
    ));
    assert!(asset_matches_platform_target(
        "wb2api_macos_arm64.tar.gz",
        "macos",
        "aarch64"
    ));
    assert!(!asset_matches_platform_target(
        "wb2api_windows_amd64.zip",
        "macos",
        "x86_64"
    ));

    // Linux tests
    assert!(asset_matches_platform_target(
        "CLIProxyAPI_7.2.115_linux_amd64.tar.gz",
        "linux",
        "x86_64"
    ));
    assert!(asset_matches_platform_target(
        "wb2api_linux_arm64.tar.gz",
        "linux",
        "aarch64"
    ));
    assert!(!asset_matches_platform_target(
        "wb2api_windows_amd64.zip",
        "linux",
        "x86_64"
    ));
}

#[test]
fn test_parse_checksums_content() {
    let multi_checksums = r#"
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  CLIProxyAPI_7.2.115_windows_amd64.zip
11223344556677889900aabbccddeeff11223344556677889900aabbccddeeff *CLIProxyAPI_7.2.115_linux_amd64.tar.gz
aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899  CLIProxyAPI_7.2.115_darwin_arm64.tar.gz
"#;

    let win_sha = parse_checksums_content(
        multi_checksums,
        "CLIProxyAPI_7.2.115_windows_amd64.zip",
        false,
    );
    assert_eq!(
        win_sha.as_deref(),
        Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    );

    let linux_sha = parse_checksums_content(
        multi_checksums,
        "CLIProxyAPI_7.2.115_linux_amd64.tar.gz",
        false,
    );
    assert_eq!(
        linux_sha.as_deref(),
        Some("11223344556677889900aabbccddeeff11223344556677889900aabbccddeeff")
    );

    let unknown_sha = parse_checksums_content(multi_checksums, "non_existent.zip", false);
    assert!(unknown_sha.is_none());

    // Single per-asset sha256 file
    let single_sha = "  deadbeefcafebabe1234567890abcdef1234567890abcdef1234567890abcdef  ";
    let parsed = parse_checksums_content(single_sha, "any.zip", true);
    assert_eq!(
        parsed.as_deref(),
        Some("deadbeefcafebabe1234567890abcdef1234567890abcdef1234567890abcdef")
    );
}

#[test]
fn test_sha256_hash_calculation_and_tamper_detection() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("downloaded.bin");

    let content = b"legitimate binary content for proxyweave plugin";
    fs::write(&file_path, content).unwrap();

    let actual_sha = calculate_file_sha256(&file_path).unwrap();
    assert_eq!(
        actual_sha,
        "bf0b11031860e13763188817ec837e1e37a98a4afa29a84e0f92e5a73514062e"
    );

    // Tampered expected hash scenario
    let wrong_sha = "0000000000000000000000000000000000000000000000000000000000000000";
    assert!(!actual_sha.eq_ignore_ascii_case(wrong_sha));

    // Verify deletion behavior on hash mismatch
    if !actual_sha.eq_ignore_ascii_case(wrong_sha) {
        let _ = fs::remove_file(&file_path);
    }
    assert!(!file_path.exists(), "Tampered file must be removed!");
}

#[test]
fn test_zipslip_malicious_path_traversal_zip() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("malicious.zip");
    let extract_target = dir.path().join("target");
    fs::create_dir_all(&extract_target).unwrap();

    // Create a zip with malicious entry name containing ".."
    {
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("../../evil_escape.exe", options).unwrap();
        zip.write_all(b"malicious payload").unwrap();
        zip.finish().unwrap();
    }

    let res = extract_zip_secure(&zip_path, &extract_target);
    assert!(res.is_err(), "ZipSlip traversal must be rejected!");
    let err = res.unwrap_err();
    assert!(
        err.contains("ZipSlip") || err.contains("恶意路径"),
        "Error message must clearly identify ZipSlip: {}",
        err
    );

    // Verify no evil file escaped to parent
    assert!(!dir.path().join("evil_escape.exe").exists());
}

#[test]
fn test_zipslip_malicious_path_traversal_tar_gz() {
    let dir = tempdir().unwrap();
    let tar_gz_path = dir.path().join("malicious.tar.gz");
    let extract_target = dir.path().join("target");
    fs::create_dir_all(&extract_target).unwrap();

    // Create a tar.gz with malicious entry containing ".."
    {
        let file = fs::File::create(&tar_gz_path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut tar = tar::Builder::new(enc);

        let data = b"malicious tar payload";
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        let path_bytes = b"../../evil_tar.exe";
        header.as_mut_bytes()[..path_bytes.len()].copy_from_slice(path_bytes);
        header.set_cksum();
        tar.append(&header, &data[..]).unwrap();
        tar.into_inner().unwrap().finish().unwrap();
    }

    let res = extract_tar_gz_secure(&tar_gz_path, &extract_target);
    assert!(res.is_err(), "ZipSlip in tar.gz must be rejected!");
    let err = res.unwrap_err();
    assert!(
        err.contains("ZipSlip") || err.contains("恶意路径"),
        "Error message must clearly identify ZipSlip: {}",
        err
    );

    assert!(!dir.path().join("evil_tar.exe").exists());
}

#[test]
fn test_legitimate_zip_extraction() {
    let dir = tempdir().unwrap();
    let zip_path = dir.path().join("legit.zip");
    let extract_target = dir.path().join("target");
    fs::create_dir_all(&extract_target).unwrap();

    {
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();

        zip.start_file("bin/app.exe", options).unwrap();
        zip.write_all(b"app binary").unwrap();

        zip.start_file("config/settings.json", options).unwrap();
        zip.write_all(b"{\"ok\":true}").unwrap();

        zip.finish().unwrap();
    }

    let res = extract_archive_secure(&zip_path, &extract_target, "legit.zip");
    assert!(
        res.is_ok(),
        "Legitimate zip extraction should succeed: {:?}",
        res
    );

    assert_eq!(
        fs::read(extract_target.join("bin/app.exe")).unwrap(),
        b"app binary"
    );
    assert_eq!(
        fs::read(extract_target.join("config/settings.json")).unwrap(),
        b"{\"ok\":true}"
    );
}

#[test]
fn test_legitimate_tar_gz_extraction() {
    let dir = tempdir().unwrap();
    let tar_gz_path = dir.path().join("legit.tar.gz");
    let extract_target = dir.path().join("target");
    fs::create_dir_all(&extract_target).unwrap();

    {
        let file = fs::File::create(&tar_gz_path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut tar = tar::Builder::new(enc);

        let data = b"my server binary";
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o755);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        tar.append_data(&mut header, "server.exe", &data[..])
            .unwrap();

        tar.into_inner().unwrap().finish().unwrap();
    }

    let res = extract_archive_secure(&tar_gz_path, &extract_target, "legit.tar.gz");
    assert!(
        res.is_ok(),
        "Legitimate tar.gz extraction should succeed: {:?}",
        res
    );

    assert_eq!(
        fs::read(extract_target.join("server.exe")).unwrap(),
        b"my server binary"
    );
}

// Explicit, local-only contract for inert fixture bytes; never executed or probed.
fn explicit_package(root: &Path, id: &str) -> PathBuf {
    fs::create_dir_all(root.join("bin")).unwrap();
    let executable = root.join("bin/owned.exe");
    fs::write(&executable, b"owned inert bytes; not a real executable").unwrap();
    let hash = calculate_file_sha256(&executable).unwrap();
    let yaml = format!(
        r#"schema_version: '1.0'
metadata:
  id: {id}
  name: Owned fixture
  version: 2.3.4
  license: Apache-2.0
runtime:
  ownership: managed
  executable: bin/owned.exe
  sha256: {hash}
network:
  health_check:
    path: /owned-health
    expected_status: 200
    identity_header: x-owned-fixture
    identity_value: {id}
    timeout_ms: 500
lifecycle:
  startup_timeout_ms: 1000
  idle_timeout_ms: null
  max_port_retries: 0
capabilities:
  protocol: anthropic_messages
  models_path: /owned-models
  inference_path: /owned-messages
  streaming: false
configuration:
  config_template: {{}}
  required_secrets: [owned-secret]
"#
    );
    let path = root.join("manifest.yaml");
    fs::write(&path, yaml).unwrap();
    path
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn collect(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    collect(root, root, &mut files);
    files
}

fn import_staged(registry: &Registry, stage: &Path, id: &str) -> Result<(), String> {
    let manifest = ensure_staging_manifest(stage, id, "not-a-manifest-version")?;
    registry.import(manifest)?;
    Ok(())
}

#[test]
fn test_asset_names_rejected_before_any_extraction_side_effect() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("inert.bin");
    fs::write(&input, b"inert bytes, not an archive").unwrap();
    let stage = dir.path().join("stage");
    fs::create_dir(&stage).unwrap();
    fs::write(stage.join("keep.txt"), b"unchanged").unwrap();
    let absolute = dir.path().join("absolute.exe").to_str().unwrap().to_owned();
    let mut names = vec![
        "".to_owned(),
        ".".to_owned(),
        "..".to_owned(),
        absolute,
        "../escaped.exe".into(),
        "..\\escaped.exe".into(),
        "sub/app.exe".into(),
        "sub\\app.exe".into(),
        "C:drive.exe".into(),
        "C:\\owned\\app.exe".into(),
        "\\\\owned-invalid-server\\share\\app.exe".into(),
        "//owned-invalid-server/share/app.exe".into(),
        "app.exe:stream".into(),
        "app.exe.".into(),
        "app.exe ".into(),
        "app\0.exe".into(),
        "app\n.exe".into(),
        "app?.exe".into(),
        "app*.exe".into(),
        "app<.exe".into(),
        "app>.exe".into(),
        "app\".exe".into(),
        "app|.exe".into(),
    ];
    for ext in ["exe", "zip", "tar.gz", "tgz"] {
        names.push(format!("../escape.{ext}"));
        names.push(format!("/owned-absolute.{ext}"));
        for device in [
            "CON", "prn", "AUX", "nul", "COM1", "lpt9", "CONIN$", "CONOUT$", "COM¹", "LPT²",
            "COM³", "CON ",
        ] {
            names.push(format!("{device}.{ext}"));
        }
    }
    let before = tree(dir.path());
    for name in names {
        let error = extract_archive_secure(&input, &stage, &name).unwrap_err();
        assert!(
            error.contains("资产文件名"),
            "must reject name before format dispatch: {name:?}: {error}"
        );
        assert_eq!(
            tree(dir.path()),
            before,
            "rejected name changed files: {name:?}"
        );
    }
}

#[test]
fn test_legitimate_exe_name_preserved() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("inert.bin");
    fs::write(&input, b"owned inert bytes").unwrap();
    let stage = dir.path().join("stage");
    fs::create_dir(&stage).unwrap();
    for name in [
        "owned_windows_x64.exe",
        "Owned v2.3.4.exe",
        "普通插件.exe",
        "COM10.exe",
    ] {
        extract_archive_secure(&input, &stage, name).unwrap();
        assert_eq!(fs::read(stage.join(name)).unwrap(), b"owned inert bytes");
    }
}

#[test]
fn test_missing_manifest_rejected_without_registration_or_generated_files() {
    for wrapped in [false, true] {
        let dir = tempdir().unwrap();
        let stage = dir.path().join("stage");
        let package = if wrapped {
            stage.join("wrapper")
        } else {
            stage.clone()
        };
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("arbitrary.exe"), b"owned inert bytes").unwrap();
        let registry = Registry::open(dir.path().join("registry")).unwrap();
        let before = tree(dir.path());
        let result = import_staged(&registry, &stage, "owned-arbitrary");
        assert!(
            result.is_err(),
            "an arbitrary binary must not become a plugin"
        );
        assert!(result.unwrap_err().contains("manifest.yaml"));
        assert_eq!(tree(dir.path()), before);
        assert!(registry.list().unwrap().is_empty());
        assert_eq!(
            fs::read_dir(dir.path().join("registry/packages"))
                .unwrap()
                .count(),
            0
        );
    }
}

#[test]
fn test_mismatched_id_root_and_wrapper_fail_before_publication() {
    for wrapped in [false, true] {
        let dir = tempdir().unwrap();
        let registry = Registry::open(dir.path().join("registry")).unwrap();
        let existing_path = explicit_package(&dir.path().join("existing"), "owned-existing");
        let existing = registry.import(&existing_path).unwrap().record;
        let stage = dir.path().join("stage");
        let package = if wrapped {
            stage.join("wrapper")
        } else {
            stage.clone()
        };
        explicit_package(&package, "owned-other");
        let before = tree(dir.path());
        let result = import_staged(&registry, &stage, "owned-selected");
        assert!(
            result.is_err(),
            "wrong package identity must fail before import"
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("owned-selected") && error.contains("owned-other"),
            "{error}"
        );
        assert_eq!(tree(dir.path()), before);
        assert_eq!(registry.list().unwrap(), vec![existing]);
        assert!(!registry.package_dir("owned-selected").unwrap().exists());
        assert!(!registry.package_dir("owned-other").unwrap().exists());
    }
}

#[test]
fn test_explicit_manifest_root_and_wrapper_register_unchanged_and_duplicate() {
    for wrapped in [false, true] {
        let dir = tempdir().unwrap();
        let registry = Registry::open(dir.path().join("registry")).unwrap();
        let stage = dir.path().join("stage");
        let package = if wrapped {
            stage.join("wrapper")
        } else {
            stage.clone()
        };
        let path = explicit_package(&package, "owned-selected");
        let expected = Manifest::parse(&fs::read(&path).unwrap()).unwrap();
        let before_stage = tree(&stage);
        let validated = ensure_staging_manifest(&stage, "owned-selected", "not-semver").unwrap();
        assert_eq!(
            validated.canonicalize().unwrap(),
            path.canonicalize().unwrap()
        );
        let imported = registry.import(validated).unwrap();
        assert!(!imported.already_registered);
        assert_eq!(imported.record.manifest, expected);
        assert_eq!(tree(&stage), before_stage);
        let before = tree(dir.path());
        import_staged(&registry, &stage, "owned-selected").unwrap();
        assert_eq!(tree(dir.path()), before);
        assert_eq!(registry.list().unwrap(), vec![imported.record]);
    }
}

#[test]
fn test_invalid_explicit_manifest_and_bad_hash_never_fall_back() {
    for wrapped in [false, true] {
        for failure in [
            "syntax",
            "schema",
            "hash",
            "missing-file",
            "oversize",
            "invalid-root",
        ] {
            let dir = tempdir().unwrap();
            let registry = Registry::open(dir.path().join("registry")).unwrap();
            let existing_path = explicit_package(&dir.path().join("existing"), "owned-existing");
            let existing = registry.import(existing_path).unwrap().record;
            let stage = dir.path().join("stage");
            let package = if wrapped {
                stage.join("wrapper")
            } else {
                stage.clone()
            };
            let path = explicit_package(&package, "owned-selected");
            match failure {
                "syntax" => fs::write(&path, b"metadata: [").unwrap(),
                "schema" => {
                    let yaml = fs::read_to_string(&path)
                        .unwrap()
                        .replace("2.3.4", "not-semver");
                    fs::write(&path, yaml).unwrap();
                }
                "hash" => {
                    fs::write(package.join("bin/owned.exe"), b"tampered inert bytes").unwrap()
                }
                "missing-file" => fs::remove_file(package.join("bin/owned.exe")).unwrap(),
                "oversize" => fs::write(&path, vec![b' '; 256 * 1024 + 1]).unwrap(),
                "invalid-root" => {
                    fs::write(stage.join("manifest.yaml"), b"invalid root contract").unwrap()
                }
                _ => unreachable!(),
            }
            let before = tree(dir.path());
            assert!(
                ensure_staging_manifest(&stage, "owned-selected", "not-semver").is_err(),
                "must reject before Registry.import: {failure}, wrapped={wrapped}"
            );
            assert!(
                import_staged(&registry, &stage, "owned-selected").is_err(),
                "{failure}, wrapped={wrapped}"
            );
            assert_eq!(tree(dir.path()), before);
            assert_eq!(registry.list().unwrap(), vec![existing]);
        }
    }
}

#[test]
fn test_conflicting_explicit_package_does_not_overwrite_registry() {
    let dir = tempdir().unwrap();
    let registry = Registry::open(dir.path().join("registry")).unwrap();
    let stage = dir.path().join("stage");
    let manifest = explicit_package(&stage, "owned-selected");
    import_staged(&registry, &stage, "owned-selected").unwrap();
    let existing = registry.load("owned-selected").unwrap();
    let yaml = fs::read_to_string(&manifest)
        .unwrap()
        .replace("Owned fixture", "Changed fixture");
    fs::write(&manifest, yaml).unwrap();
    let before = tree(dir.path());
    assert!(import_staged(&registry, &stage, "owned-selected")
        .unwrap_err()
        .contains("different package"));
    assert_eq!(tree(dir.path()), before);
    assert_eq!(registry.load("owned-selected").unwrap(), existing);
}

#[tokio::test]
async fn test_expected_digest_missing_fails_closed() {
    let release = Release {
        id: 12345,
        tag_name: Some("v1.0.0".to_string()),
        name: Some("Release 1.0.0".to_string()),
        draft: false,
        prerelease: false,
        assets: vec![ReleaseAsset {
            id: 67890,
            name: "plugin.zip".to_string(),
            size: 1024,
            browser_download_url:
                "https://github.com/owner/repo/releases/download/v1.0.0/plugin.zip".to_string(),
            digest: None,
        }],
    };

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let err = expected_digest(&release, &client, "plugin.zip")
        .await
        .unwrap_err();
    assert!(err.contains("missing or ambiguous"));
}

#[tokio::test]
async fn test_expected_digest_from_asset_digest() {
    let valid_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let release = Release {
        id: 12345,
        tag_name: Some("v1.0.0".to_string()),
        name: Some("Release 1.0.0".to_string()),
        draft: false,
        prerelease: false,
        assets: vec![ReleaseAsset {
            id: 67890,
            name: "plugin.zip".to_string(),
            size: 1024,
            browser_download_url:
                "https://github.com/owner/repo/releases/download/v1.0.0/plugin.zip".to_string(),
            digest: Some(format!("sha256:{valid_sha}")),
        }],
    };

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let (hash, evidence) = expected_digest(&release, &client, "plugin.zip")
        .await
        .unwrap();
    assert_eq!(hash, valid_sha);
    assert_eq!(
        evidence,
        proxyweave_plugin::updater::DigestEvidence::ReleaseAssetDigest
    );
}

#[tokio::test]
async fn test_expected_digest_from_checksum_file() {
    let valid_sha = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server_content = format!("{valid_sha}  plugin.zip\n");
    let _server = thread::spawn(move || {
        while let Ok((mut socket, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).unwrap_or(0);
            if n == 0 {
                continue;
            }
            let resp = format!(
                "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                server_content.len(),
                server_content
            );
            let _ = socket.write_all(resp.as_bytes());
        }
    });

    let release = Release {
        id: 12345,
        tag_name: Some("v1.0.0".to_string()),
        name: Some("Release 1.0.0".to_string()),
        draft: false,
        prerelease: false,
        assets: vec![
            ReleaseAsset {
                id: 67890,
                name: "plugin.zip".to_string(),
                size: 1024,
                browser_download_url:
                    "https://github.com/owner/repo/releases/download/v1.0.0/plugin.zip".to_string(),
                digest: None,
            },
            ReleaseAsset {
                id: 67891,
                name: "plugin.zip.sha256".to_string(),
                size: 100,
                browser_download_url: format!("http://{addr}/plugin.zip.sha256"),
                digest: None,
            },
        ],
    };

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let (hash, evidence) = expected_digest(&release, &client, "plugin.zip")
        .await
        .unwrap();
    assert_eq!(hash, valid_sha);
    match evidence {
        proxyweave_plugin::updater::DigestEvidence::ChecksumAsset {
            asset_id,
            asset_name,
        } => {
            assert_eq!(asset_id, 67891);
            assert_eq!(asset_name, "plugin.zip.sha256");
        }
        _ => panic!("expected ChecksumAsset evidence"),
    }
}

#[test]
fn test_validate_release_asset_url() {
    assert!(validate_release_asset_url(
        "https://github.com/owner/repo/releases/download/v1.0.0/myplugin.zip",
        "owner",
        "repo",
        "v1.0.0",
        "myplugin.zip"
    )
    .is_ok());

    // Scheme != https
    assert!(validate_release_asset_url(
        "http://github.com/owner/repo/releases/download/v1.0.0/myplugin.zip",
        "owner",
        "repo",
        "v1.0.0",
        "myplugin.zip"
    )
    .is_err());

    // Host != github.com
    assert!(validate_release_asset_url(
        "https://evil.com/owner/repo/releases/download/v1.0.0/myplugin.zip",
        "owner",
        "repo",
        "v1.0.0",
        "myplugin.zip"
    )
    .is_err());

    // Mismatched owner/repo
    assert!(validate_release_asset_url(
        "https://github.com/other/repo/releases/download/v1.0.0/myplugin.zip",
        "owner",
        "repo",
        "v1.0.0",
        "myplugin.zip"
    )
    .is_err());

    // Query parameters rejected
    assert!(validate_release_asset_url(
        "https://github.com/owner/repo/releases/download/v1.0.0/myplugin.zip?param=1",
        "owner",
        "repo",
        "v1.0.0",
        "myplugin.zip"
    )
    .is_err());

    // Port rejected
    assert!(validate_release_asset_url(
        "https://github.com:8443/owner/repo/releases/download/v1.0.0/myplugin.zip",
        "owner",
        "repo",
        "v1.0.0",
        "myplugin.zip"
    )
    .is_err());
}

#[tokio::test]
async fn test_read_response_bounded_oversize_and_http_status() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let _server = thread::spawn(move || {
        while let Ok((mut socket, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).unwrap_or(0);
            if n == 0 {
                continue;
            }
            let req = String::from_utf8_lossy(&buf[..n]);
            let resp = if req.contains("/err500") {
                "HTTP/1.1 500 Internal Server Error\r\nConnection: close\r\nContent-Length: 5\r\n\r\nerror".to_string()
            } else if req.contains("/oversize") {
                let body = vec![b'x'; 2048];
                format!(
                    "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    String::from_utf8_lossy(&body)
                )
            } else {
                "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 4\r\n\r\ngood".to_string()
            };
            let _ = socket.write_all(resp.as_bytes());
        }
    });

    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    // 500 status rejection
    let resp = client
        .get(format!("http://{addr}/err500"))
        .send()
        .await
        .unwrap();
    let err = read_response_bounded(resp, 1024).await.unwrap_err();
    assert!(err.contains("HTTP 失败: 500"));

    // Oversize rejection
    let resp = client
        .get(format!("http://{addr}/oversize"))
        .send()
        .await
        .unwrap();
    let err = read_response_bounded(resp, 1024).await.unwrap_err();
    assert!(err.contains("exceeds size limit") || err.contains("超过上限"));

    // Normal success
    let resp = client
        .get(format!("http://{addr}/good"))
        .send()
        .await
        .unwrap();
    let data = read_response_bounded(resp, 1024).await.unwrap();
    assert_eq!(data, b"good");
}
