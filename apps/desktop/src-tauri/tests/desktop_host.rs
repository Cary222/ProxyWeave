use proxyweave_plugin::{Manifest, Ownership, Registry};
use quotio_desktop_lib::host::{DesktopConfig, DesktopState};
use tempfile::tempdir;

type MemoryKeyStore = std::sync::Arc<std::sync::Mutex<Option<String>>>;

/// In-memory master-key store. Integration tests must not touch the OS keychain:
/// a headless Linux runner has no Secret Service, and a developer machine must not
/// be mutated. Same wiring as `DesktopState::new`, credential store injected.
fn open_state_with_memory_keyring(
    profile_dir: std::path::PathBuf,
    registry: Registry,
    store: MemoryKeyStore,
) -> Result<DesktopState, String> {
    let get_store = store.clone();
    let set_store = store;
    DesktopState::new_with_credential_store(
        profile_dir,
        registry,
        move || Ok(get_store.lock().unwrap().clone()),
        move |key| {
            *set_store.lock().unwrap() = Some(key.to_string());
            Ok(())
        },
    )
}

fn memory_key_store() -> MemoryKeyStore {
    MemoryKeyStore::default()
}

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
    let keys: MemoryKeyStore = memory_key_store();
    let state = open_state_with_memory_keyring(prof_dir.clone(), registry, keys.clone())
        .expect("desktop state");

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
    let state2 =
        open_state_with_memory_keyring(prof_dir, reg2, keys).expect("desktop state reloaded");
    assert_eq!(
        *state2.gateway_master_key.read().await,
        key,
        "reloaded state must reuse the persisted master key"
    );
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
    let state = open_state_with_memory_keyring(prof_dir, registry, memory_key_store())
        .expect("desktop state");

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
    let state = open_state_with_memory_keyring(prof_dir, registry, memory_key_store())
        .expect("desktop state");

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

    // Verify gateway responds 502 Bad Gateway when no backend attached (spec v7 shape)
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let key = state.gateway_master_key.read().await.clone();
    let resp = client
        .get(format!("http://127.0.0.1:{bound_port}/v1/models"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status().as_u16(), 502);

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

#[tokio::test]
async fn test_inspect_project_from_github_url_validation_and_author_extraction() {
    use quotio_desktop_lib::host::{
        extract_github_author, inspect_project_from_github, validate_github_repo_url,
        InspectProjectResult,
    };

    // 1. Empty URL
    let err = validate_github_repo_url("   ").unwrap_err();
    assert!(err.contains("不能为空"));
    let err = inspect_project_from_github("".into()).await.unwrap_err();
    assert!(err.contains("不能为空"));

    // 2. Illegal chars & dashes
    let err = validate_github_repo_url("--upload-pack=evil").unwrap_err();
    assert!(err.contains("格式非法"));
    let err = inspect_project_from_github("https://github.com/a/b\0c".into())
        .await
        .unwrap_err();
    assert!(err.contains("格式非法"));
    let err = inspect_project_from_github("https://github.com/a/b\nc".into())
        .await
        .unwrap_err();
    assert!(err.contains("格式非法"));

    // 3. Non-GitHub URL
    let err = validate_github_repo_url("https://gitlab.com/owner/repo").unwrap_err();
    assert!(err.contains("当前仅支持标准 GitHub 仓库 URL"));

    // 4. Valid GitHub URLs
    let ok =
        validate_github_repo_url("  https://github.com/router-for-me/workbuddy2api  ").unwrap();
    assert_eq!(ok, "https://github.com/router-for-me/workbuddy2api");
    let ok_ssh =
        validate_github_repo_url("git@github.com:router-for-me/workbuddy2api.git").unwrap();
    assert_eq!(ok_ssh, "git@github.com:router-for-me/workbuddy2api.git");

    // 5. Author extraction
    assert_eq!(
        extract_github_author("https://github.com/linguo2625469/workbuddy2api-panel"),
        "linguo2625469"
    );
    assert_eq!(
        extract_github_author("git@github.com:proxyweave/test-plugin.git"),
        "proxyweave"
    );

    // 6. InspectProjectResult serialization round-trip
    let inspect_sample = InspectProjectResult {
        id: "wb2api".into(),
        name: "WorkBuddy 2API".into(),
        version: "1.13.0".into(),
        description: "Test description".into(),
        author: "linguo2625469".into(),
        homepage: "https://github.com/linguo2625469/workbuddy2api-panel".into(),
        license: "MIT".into(),
        project_type: "manifest".into(),
        runtime_kind: "managed".into(),
        entrypoint: "bin/wb2api.exe".into(),
        sha256: "38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485".into(),
        protocols: vec!["openai_chat".into(), "streaming".into()],
        exported_models: vec![],
        requires_secrets: vec!["access_token".into()],
        permissions: vec![
            "独立子进程执行 (managed_process)".into(),
            "本地网络端口监听与通信 (local_network)".into(),
        ],
        notes: vec!["Only declared files are read".into()],
        raw_manifest: serde_json::json!({
            "schema_version": "1.0",
            "metadata": { "id": "wb2api" }
        }),
    };
    let json = serde_json::to_string(&inspect_sample).expect("serialize");
    let deserialized: InspectProjectResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(deserialized.id, "wb2api");
    assert_eq!(deserialized.runtime_kind, "managed");
    assert_eq!(deserialized.protocols.len(), 2);
    assert_eq!(deserialized.permissions.len(), 2);
}
