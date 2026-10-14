use proxyweave_plugin::{
    detect_project, stage_and_import_project, ManagementPresentation, Ownership, Registry,
};
use std::fs;
use std::path::Path;

#[test]
fn test_detect_nonexistent_path_returns_err() {
    let result = detect_project("nonexistent_directory_12345");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("不存在"));
}

#[test]
fn test_detect_dir_without_exe_returns_err() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("README.md"), b"# Test Project").unwrap();
    let result = detect_project(tmp.path());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("未在目录中检测到 Windows 可执行文件 (.exe)"));
}

#[test]
fn test_detect_real_wb2api_if_exists() {
    let path = Path::new("D:/workbuddy2api-panel");
    if !path.exists() {
        eprintln!("D:/workbuddy2api-panel does not exist; skipping test");
        return;
    }
    let report = detect_project(path).expect("Failed to detect wb2api project");
    assert_eq!(report.id, "wb2api");
    assert_eq!(report.name, "WorkBuddy 2API");
    assert_eq!(report.project_type, "wb2api");
    assert_eq!(report.executable_relative_path, "wb2api.exe");
    assert!(!report.executable_sha256.is_empty());
    assert_eq!(report.manifest.metadata.id, "wb2api");
    assert_eq!(report.manifest.ownership(), Ownership::Managed);
    assert!(report.manifest.validate().is_ok());

    let mgmt = report
        .manifest
        .management
        .as_ref()
        .expect("wb2api should declare management");
    assert_eq!(mgmt.path, "/panel/");
    assert_eq!(mgmt.presentation, ManagementPresentation::Browser);
}

#[test]
fn test_detect_real_cli_proxy_api_if_exists() {
    let path = Path::new("D:/CLIProxyAPI_7.2.115_windows_amd64");
    if !path.exists() {
        eprintln!("D:/CLIProxyAPI_7.2.115_windows_amd64 does not exist; skipping test");
        return;
    }
    let report = detect_project(path).expect("Failed to detect cli-proxy-api project");
    assert_eq!(report.id, "cli-proxy-api");
    assert_eq!(report.name, "CLI Proxy API");
    assert_eq!(report.project_type, "cli-proxy-api");
    assert_eq!(report.executable_relative_path, "cli-proxy-api.exe");
    assert_eq!(report.version, "1.0.0");
    assert!(!report.executable_sha256.is_empty());
    assert_eq!(report.manifest.metadata.id, "cli-proxy-api");
    assert_eq!(report.manifest.ownership(), Ownership::Managed);
    assert!(report.manifest.validate().is_ok());

    let mgmt = report
        .manifest
        .management
        .as_ref()
        .expect("cli-proxy-api should declare management");
    assert_eq!(mgmt.path, "/static/management.html");
    assert_eq!(mgmt.presentation, ManagementPresentation::Browser);
}

#[test]
fn test_stage_and_import_project_synthetic() {
    let src_dir = tempfile::tempdir().unwrap();
    let reg_dir = tempfile::tempdir().unwrap();

    let fake_exe_bytes = b"MZ\x90\x00synthetic executable binary for testing";
    fs::write(src_dir.path().join("my-custom-proxy.exe"), fake_exe_bytes).unwrap();
    fs::write(
        src_dir.path().join("LICENSE"),
        b"MIT License\n\nCopyright (c) 2026 Test",
    )
    .unwrap();
    fs::write(
        src_dir.path().join("README.md"),
        b"# Custom Proxy\n\nVersion: 2.1.0\n",
    )
    .unwrap();

    // 记录源目录初始内容列表
    let mut initial_entries: Vec<String> = fs::read_dir(src_dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    initial_entries.sort();

    let registry = Registry::open(reg_dir.path()).expect("Failed to open registry");

    let import_result = stage_and_import_project(&registry, src_dir.path(), None)
        .expect("stage_and_import_project failed");

    assert!(!import_result.already_registered);
    assert_eq!(import_result.record.manifest.metadata.id, "my-custom-proxy");
    assert_eq!(import_result.record.manifest.metadata.version, "2.1.0");
    assert_eq!(import_result.record.manifest.metadata.license, "MIT");

    // 验证源目录未被污染或修改（未多出 manifest.yaml 等文件）
    let mut current_entries: Vec<String> = fs::read_dir(src_dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    current_entries.sort();
    assert_eq!(initial_entries, current_entries);
    assert!(!src_dir.path().join("manifest.yaml").exists());

    // 验证 registry 中成功注册并可以加载
    let loaded = registry
        .load("my-custom-proxy")
        .expect("Failed to load registered plugin");
    assert_eq!(loaded.manifest.metadata.id, "my-custom-proxy");
    assert_eq!(loaded.manifest.ownership(), Ownership::Managed);
}
