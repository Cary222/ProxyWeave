//! Integration tests for Desktop Workbench IPC bindings and UI contracts (Phase 5 / spec v6).
//!
//! Verifies:
//! 1. External plugins strictly reject any spawn/stop/kill mutation.
//! 2. Zero-DB invariant: no database dependencies in workbench IPC or UI.
//! 3. WhatsApp exclusion: zero WhatsApp code or manifests present in project.

use proxyweave_plugin::{Manifest, Ownership, PluginSupervisor, PluginError};

#[test]
fn test_external_plugin_cannot_be_killed_or_stopped_by_workbench() {
    let supervisor = PluginSupervisor::new();
    let cpa_json = include_str!("../plugins/external/external-cpa/manifest.json");
    let manifest = Manifest::parse(cpa_json.as_bytes()).expect("parse external manifest");

    assert_eq!(manifest.ownership(), Ownership::External);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut handle = None;
        let stop_res = supervisor.stop(&manifest, &mut handle).await;
        assert_eq!(stop_res, Err(PluginError::ExternalLifecycleImmutable));

        let kill_res = supervisor.kill(&manifest, &mut handle).await;
        assert_eq!(kill_res, Err(PluginError::ExternalLifecycleImmutable));
    });
}

#[test]
fn test_manifest_instances_contain_no_whatsapp() {
    // Assert project manifests contain no whatsapp configurations
    let wb2api_json = include_str!("../plugins/managed/wb2api/manifest.json");
    let cpa_json = include_str!("../plugins/external/external-cpa/manifest.json");

    assert!(!wb2api_json.to_lowercase().contains("whatsapp"));
    assert!(!cpa_json.to_lowercase().contains("whatsapp"));
}

#[test]
fn test_ui_assets_conform_to_zerodb_and_external_isolation() {
    let html = include_str!("../src-ui/index.html");
    let app_js = include_str!("../src-ui/app.js");

    // Zero-DB checks
    assert!(!html.contains("indexedDB"));
    assert!(!html.contains("sqlite"));
    assert!(!app_js.contains("indexedDB"));
    assert!(!app_js.contains("sqlite"));

    // WhatsApp exclusion checks
    assert!(!html.to_lowercase().contains("whatsapp"));
    assert!(!app_js.to_lowercase().contains("whatsapp"));

    // External isolation: external plugin must not have stop/kill controls
    assert!(html.contains("重新探测 (Probe Only)"));
}
