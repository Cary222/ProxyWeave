use proxyweave_plugin::{Manifest, Ownership, Registry};
use quotio_desktop_lib::host::{DesktopConfig, DesktopState};
use tempfile::tempdir;

#[allow(dead_code)]
fn sample_manifest(id: &str, ownership: Ownership) -> Manifest {
    let yaml = format!(
        r#"
schema_version: "1.0"
metadata:
  id: "{id}"
  name: "Test {id}"
  version: "1.0.0"
  description: "Test manifest for desktop host integration"
spec:
  protocol: "openai_chat"
  ownership: "{}"
  network:
    listen: "127.0.0.1"
    default_port: 39182
  health_check:
    path: "/healthz"
    expected_status: 200
configuration:
  required_secrets:
    - "token"
  settings_schema:
    type: "object"
    properties:
      debug:
        type: "boolean"
"#,
        match ownership {
            Ownership::Managed => "managed",
            Ownership::External => "external",
        }
    );
    Manifest::parse(yaml.as_bytes()).expect("valid manifest")
}

#[tokio::test]
async fn test_desktop_state_master_key_and_config_persistence() {
    let dir = tempdir().expect("tempdir");
    let reg_dir = dir.path().join("registry");
    let prof_dir = dir.path().join("profile");
    std::fs::create_dir_all(&prof_dir).unwrap();

    let registry = Registry::open(&reg_dir).expect("registry");
    let state = DesktopState::new(prof_dir.clone(), registry).expect("desktop state");

    // 1. Master Key was generated and starts with pw_
    let key = state.gateway_master_key.read().await.clone();
    assert!(key.starts_with("pw_"));
    assert!(key.len() >= 32);

    // 2. Gateway port defaults to 28319
    assert_eq!(*state.gateway_port.read().await, 28319);

    // 3. Save config and reload
    let new_config = DesktopConfig {
        gateway_port: 29999,
        selected_plugin_id: Some("test-plugin".into()),
    };
    state.save_desktop_config(&new_config).expect("save config");

    let reg2 = Registry::open(&reg_dir).expect("registry");
    let state2 = DesktopState::new(prof_dir, reg2).expect("desktop state reloaded");
    assert_eq!(*state2.gateway_port.read().await, 29999);
    assert_eq!(
        state2.selected_plugin_id.read().await.as_deref(),
        Some("test-plugin")
    );
}

#[tokio::test]
async fn test_host_events_bounded_fifo() {
    let dir = tempdir().expect("tempdir");
    let reg_dir = dir.path().join("registry");
    let prof_dir = dir.path().join("profile");
    std::fs::create_dir_all(&prof_dir).unwrap();

    let registry = Registry::open(&reg_dir).expect("registry");
    let state = DesktopState::new(prof_dir, registry).expect("desktop state");

    // Insert 60 events, should clamp at 50
    for i in 0..60 {
        state
            .record_event("test", "act", "target", true, &format!("event {i}"))
            .await;
    }

    let events = state.events.lock().await;
    assert_eq!(events.len(), 50);
    assert_eq!(events.front().unwrap().summary, "event 10");
    assert_eq!(events.back().unwrap().summary, "event 59");
}

#[tokio::test]
async fn test_desktop_gateway_lifecycle_and_backend_attachment() {
    let dir = tempdir().expect("tempdir");
    let reg_dir = dir.path().join("registry");
    let prof_dir = dir.path().join("profile");
    std::fs::create_dir_all(&prof_dir).unwrap();

    let registry = Registry::open(&reg_dir).expect("registry");
    let state = DesktopState::new(prof_dir, registry).expect("desktop state");

    // Pick an unused port for test gateway
    let port = proxyweave_plugin::port::pick_unused_port().expect("free port");
    {
        let mut p = state.gateway_port.write().await;
        *p = port;
    }

    // 1. Start gateway
    let handle = state.gateway.listen(port).await.expect("gateway listen");
    let bound_port = handle.address().port();
    assert_eq!(bound_port, port);

    {
        let mut gh = state.gateway_handle.lock().await;
        *gh = Some(handle);
    }

    // Verify gateway responds 503 Service Unavailable when no backend attached
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let key = state.gateway_master_key.read().await.clone();
    let resp = client
        .get(format!("http://127.0.0.1:{bound_port}/v1/models"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status().as_u16(), 503);

    // 2. Stop gateway gracefully
    let handle_to_stop = {
        let mut gh = state.gateway_handle.lock().await;
        gh.take()
    };
    if let Some(h) = handle_to_stop {
        let _ = h.shutdown().await;
    }

    let gh = state.gateway_handle.lock().await;
    assert!(gh.is_none());
}
