//! Integration tests for plugin lifecycle supervisor and immutable External guard.
//!
//! Verifies:
//! 1. External plugins are immutable (spawn, stop, kill rejected with `ExternalLifecycleImmutable`).
//! 2. Dynamic loopback port probe returns valid available ports.
//! 3. Credential preparation correctly formats `PROXYWEAVE_CREDENTIAL_*`.
//! 4. Manifest 1.0 JSON instances parse and validate cleanly.

use std::collections::HashMap;
use proxyweave_plugin::{Manifest, Ownership, PluginError, PluginSupervisor};

#[test]
fn test_external_plugin_is_immutable() {
    let supervisor = PluginSupervisor::new();
    let cpa_json = include_str!("../plugins/external/external-cpa/manifest.json");
    let manifest = Manifest::parse(cpa_json.as_bytes()).expect("parse external manifest");

    assert_eq!(manifest.ownership(), Ownership::External);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let creds = HashMap::new();
        let spawn_res = supervisor.spawn(&manifest, None, &creds).await;
        match spawn_res {
            Err(PluginError::ExternalLifecycleImmutable) => {}
            other => panic!("Expected ExternalLifecycleImmutable, got: {:?}", other.is_err()),
        }

        let mut handle = None;
        let stop_res = supervisor.stop(&manifest, &mut handle).await;
        assert_eq!(stop_res, Err(PluginError::ExternalLifecycleImmutable));

        let kill_res = supervisor.kill(&manifest, &mut handle).await;
        assert_eq!(kill_res, Err(PluginError::ExternalLifecycleImmutable));
    });
}

#[test]
fn test_managed_manifest_parsing_and_port_probing() {
    let supervisor = PluginSupervisor::new();
    let wb2api_json = include_str!("../plugins/managed/wb2api/manifest.json");
    let manifest = Manifest::parse(wb2api_json.as_bytes()).expect("parse managed manifest");

    assert_eq!(manifest.ownership(), Ownership::Managed);
    assert_eq!(manifest.metadata.id, "wb2api");

    let port = supervisor.probe_dynamic_port().expect("probe dynamic port");
    assert!(port > 0);
}

#[test]
fn test_credential_sanitization_and_injection() {
    let supervisor = PluginSupervisor::new();
    let mut creds = HashMap::new();
    creds.insert("api_key".to_string(), "secret-123".to_string());
    creds.insert("access-token".to_string(), "tok-456".to_string());

    let prepared = supervisor.prepare_credentials(&creds);
    assert_eq!(
        prepared.get("PROXYWEAVE_CREDENTIAL_API_KEY"),
        Some(&"secret-123".to_string())
    );
    assert_eq!(
        prepared.get("PROXYWEAVE_CREDENTIAL_ACCESS_TOKEN"),
        Some(&"tok-456".to_string())
    );
}
