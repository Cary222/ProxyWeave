//! Owned dummy credentials only; no DesktopState::new/shared master-key access.
use super::*;

fn state_and_record(protocol: &str) -> (tempfile::TempDir, DesktopState, PluginRecord) {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let id = format!(
        "key-test-{}-{}",
        std::process::id(),
        dir.path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace('.', "-")
            .to_ascii_lowercase()
    );
    let source = dir.path().join("source");
    std::fs::create_dir_all(&source).unwrap();
    let exe = std::env::current_exe().unwrap();
    let bytes = std::fs::read(&exe).unwrap();
    std::fs::copy(&exe, source.join("fixture.exe")).unwrap();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let yaml = format!(
        r#"schema_version: '1.0'
metadata: {{id: '{id}', name: Owned, version: 1.0.0, license: MIT}}
runtime:
  ownership: managed
  executable: fixture.exe
  sha256: '{hash}'
  args: ['--exact', 'host::key_tests::owned_managed_key_child', '--ignored']
  env: {{OWNED_KEY_PORT: '{{PORT}}', OWNED_KEY_MODE: 'ready'}}
network:
  health_check: {{path: /health, expected_status: 200, busy_statuses: [503], identity_header: X-Owned, identity_value: fixture, timeout_ms: 100}}
lifecycle: {{startup_timeout_ms: 500, idle_timeout_ms: null, max_port_retries: 0}}
capabilities: {{protocol: '{protocol}', models_path: /v1/models, inference_path: /v1/chat/completions, streaming: true}}
configuration: {{required_secrets: [access_token], config_template: {{}}}}
"#
    );
    std::fs::write(source.join("manifest.yaml"), &yaml).unwrap();
    let registry = Registry::open(dir.path().join("registry")).unwrap();
    let record = registry
        .import(source.join("manifest.yaml"))
        .unwrap()
        .record;
    let runtime = RuntimeManager::new();
    let key = "owned-test-master-key-32-characters-only";
    let state = DesktopState {
        profile_dir: dir.path().join("profile"),
        registry,
        gateway: Gateway::new(runtime.clone(), key, Duration::from_secs(1)).unwrap(),
        runtime,
        gateway_handle: Arc::new(Mutex::new(None)),
        gateway_starting: Arc::new(AtomicBool::new(false)),
        gateway_port: Arc::new(RwLock::new(28319)),
        gateway_master_key: Arc::new(RwLock::new(key.into())),
        selected_plugin_id: Arc::new(RwLock::new(None)),
        events: Arc::new(Mutex::new(VecDeque::new())),
        plugin_operations: Mutex::new(BTreeMap::new()),
    };
    (dir, state, record)
}

fn managed_record(state: &DesktopState, record: &PluginRecord, mode: &str) -> PluginRecord {
    use sha2::{Digest, Sha256};
    let source = state.profile_dir.with_file_name(format!("managed-{mode}"));
    std::fs::create_dir_all(&source).unwrap();
    let bytes = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    std::fs::write(source.join("fixture.exe"), &bytes).unwrap();
    let id = &record.manifest.metadata.id;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let yaml = format!(
        r#"schema_version: '1.0'
metadata: {{id: '{id}-owned', name: Owned, version: 1.0.0, license: MIT}}
runtime:
  ownership: managed
  executable: fixture.exe
  sha256: '{hash}'
  args: ['--exact', 'host::key_tests::owned_managed_key_child', '--ignored']
  env: {{OWNED_KEY_PORT: '{{PORT}}', OWNED_KEY_MODE: '{mode}'}}
network:
  health_check: {{path: /health, expected_status: 200, busy_statuses: [503], identity_header: X-Owned, identity_value: fixture, timeout_ms: 100}}
lifecycle: {{startup_timeout_ms: 500, idle_timeout_ms: null, max_port_retries: 0}}
capabilities: {{protocol: openai_chat, models_path: /v1/models, inference_path: /v1/chat/completions, streaming: true}}
configuration: {{required_secrets: [api_key], config_template: {{}}}}
"#
    );
    std::fs::write(source.join("manifest.yaml"), yaml).unwrap();
    state
        .registry
        .import(source.join("manifest.yaml"))
        .unwrap()
        .record
}

#[test]
#[ignore = "Owned subprocess entry, explicitly executed by Managed regression"]
fn owned_managed_key_child() {
    use std::io::{Read, Write};
    let port = std::env::var("OWNED_KEY_PORT").unwrap();
    let mode = std::env::var("OWNED_KEY_MODE").unwrap();
    let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let mut bytes = [0; 2048];
        let _ = stream.read(&mut bytes);
        let (status, identity) = if mode == "busy" {
            ("503 Service Unavailable", "fixture")
        } else {
            ("200 OK", "wrong-identity")
        };
        let _ = stream.write_all(format!("HTTP/1.1 {status}\r\nX-Owned: {identity}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes());
    }
}

#[tokio::test]
async fn real_managed_busy_failed_child_cannot_edit_credentials_then_stop_can() {
    for mode in ["busy", "failed"] {
        let (_dir, state, base) = state_and_record("openai_chat");
        let record = managed_record(&state, &base, mode);
        let id = record.manifest.metadata.id.clone();
        let _cleanup = DummyCredential(id.clone());
        quotio_platform::set_plugin_credential(&id, "api_key", "owned-before").unwrap();
        let package = state.registry.package_dir(&id).unwrap();
        let data = state.registry.data_dir(&id).unwrap();
        let port = proxyweave_plugin::port::pick_unused_port().unwrap();
        let status = state
            .runtime
            .start_plugin(&record, &package, Some(port), &data, Some("owned-before"))
            .await
            .unwrap();
        if mode == "busy" {
            assert!(matches!(status, RuntimeStatus::Busy { pid: Some(_), .. }));
        } else {
            assert!(matches!(status, RuntimeStatus::Failed { .. }));
        }
        assert!(
            std::net::TcpStream::connect(("127.0.0.1", port)).is_ok(),
            "Owned child remains alive"
        );
        assert!(save_plugin_secret_value(
            &state,
            id.clone(),
            "api_key".into(),
            "owned-after".into()
        )
        .await
        .is_err());
        assert!(quotio_platform::get_plugin_credential(&id, "api_key")
            .unwrap()
            .is_some_and(|v| v == "owned-before"));
        assert!(state.events.lock().await.is_empty());
        state.runtime.stop_plugin(&id).await.unwrap();
        assert!(matches!(
            state.runtime.get_status(&id).await,
            RuntimeStatus::Stopped
        ));
        save_plugin_secret_value(&state, id.clone(), "api_key".into(), "owned-after".into())
            .await
            .unwrap();
        assert!(quotio_platform::get_plugin_credential(&id, "api_key")
            .unwrap()
            .is_some_and(|v| v == "owned-after"));
    }
}

struct DummyCredential(String);
impl Drop for DummyCredential {
    fn drop(&mut self) {
        for name in ["api_key", "access_token"] {
            let _ = quotio_platform::delete_plugin_credential(&self.0, name);
        }
    }
}

async fn ready(
    state: &DesktopState,
    record: &mut PluginRecord,
    busy: bool,
) -> tokio::task::JoinHandle<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap());
    record.manifest.runtime = proxyweave_plugin::manifest::Runtime::External { endpoint };
    if busy {
        record.manifest.network.health_check.busy_statuses = vec![503];
    }
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = [0; 2048];
        let _ = stream.read(&mut bytes).await;
        let status = if busy {
            "503 Service Unavailable"
        } else {
            "200 OK"
        };
        stream.write_all(format!("HTTP/1.1 {status}\r\nX-Owned: fixture\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    });
    let result = state
        .runtime
        .start_plugin(
            record,
            std::path::Path::new(""),
            None,
            std::path::Path::new(""),
            None,
        )
        .await
        .unwrap();
    assert!(matches!(
        result,
        RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }
    ));
    server
}

#[tokio::test]
async fn credential_save_uses_same_fifo_lock_until_keyring_and_event_complete() {
    let (_dir, state, record) = state_and_record("openai_chat");
    let id = record.manifest.metadata.id;
    let _cleanup = DummyCredential(id.clone());
    quotio_platform::set_plugin_credential(&id, "api_key", "owned-before").unwrap();
    let state = Arc::new(state);
    let guard = state.plugin_operation(&id).await;
    let queued = {
        let state = state.clone();
        let id = id.clone();
        tokio::spawn(async move {
            save_plugin_secret_value(&state, id, "api_key".into(), "owned-after".into()).await
        })
    };
    tokio::task::yield_now().await;
    assert!(!queued.is_finished());
    assert!(state.events.lock().await.is_empty());
    assert!(quotio_platform::get_plugin_credential(&id, "api_key")
        .unwrap()
        .is_some_and(|v| v == "owned-before"));
    drop(guard);
    assert!(queued.await.unwrap().is_ok());
    assert!(quotio_platform::get_plugin_credential(&id, "api_key")
        .unwrap()
        .is_some_and(|v| v == "owned-after"));
    let events = state.events.lock().await;
    assert_eq!(events.len(), 1);
    assert!(events[0].success);
    assert!(!events[0].summary.contains("owned-after"));
}

#[tokio::test]
async fn credential_save_refuses_ready_busy_failed_and_failed_write_has_no_success() {
    for busy in [false, true] {
        let (_dir, state, mut record) = state_and_record("openai_chat");
        let id = record.manifest.metadata.id.clone();
        let _cleanup = DummyCredential(id.clone());
        quotio_platform::set_plugin_credential(&id, "api_key", "owned-before").unwrap();
        ready(&state, &mut record, busy).await.await.unwrap();
        assert!(save_plugin_secret_value(
            &state,
            id.clone(),
            "api_key".into(),
            "owned-after".into()
        )
        .await
        .is_err());
        assert!(quotio_platform::get_plugin_credential(&id, "api_key")
            .unwrap()
            .is_some_and(|v| v == "owned-before"));
        assert!(state.events.lock().await.is_empty());
    }
    let (_dir, state, record) = state_and_record("openai_chat");
    let id = record.manifest.metadata.id.clone();
    let _cleanup = DummyCredential(id.clone());
    // Windows Credential Manager refuses blobs larger than its documented limit.
    #[cfg(windows)]
    {
        assert!(save_plugin_secret_value(
            &state,
            id.clone(),
            "access_token".into(),
            "x".repeat(100_000)
        )
        .await
        .is_err());
        assert!(state.events.lock().await.is_empty());
        assert!(quotio_platform::get_plugin_credential(&id, "access_token")
            .unwrap()
            .is_none());
    }
    let _ = state
        .runtime
        .start_plugin(
            &record,
            std::path::Path::new(""),
            None,
            std::path::Path::new(""),
            None,
        )
        .await;
    assert!(matches!(
        state.runtime.get_status(&id).await,
        RuntimeStatus::Failed { .. }
    ));
    assert!(
        save_plugin_secret_value(&state, id, "api_key".into(), "owned-after".into())
            .await
            .is_err()
    );
    assert!(state.events.lock().await.is_empty());
}

#[tokio::test]
async fn failed_backend_selection_does_not_publish_new_choice_or_success() {
    let (_dir, state, mut record) = state_and_record("openai_chat");
    let id = record.manifest.metadata.id.clone();
    let _cleanup = DummyCredential(id.clone());
    let mut previous = record.manifest.clone();
    previous.metadata.id = "owned-previous-backend".into();
    state
        .gateway
        .set_backend(&previous, "owned-previous-key")
        .await
        .unwrap();
    *state.selected_plugin_id.write().await = Some(previous.metadata.id.clone());
    ready(&state, &mut record, false).await.await.unwrap();
    quotio_platform::set_plugin_credential(&id, "api_key", "owned-invalid\0blob").unwrap();
    assert!(select_plugin_backend(&state, id.clone()).await.is_err());
    assert_eq!(
        state.gateway.backend_plugin_id().await.as_deref(),
        Some("owned-previous-backend")
    );
    assert_eq!(
        state.selected_plugin_id.read().await.as_deref(),
        Some("owned-previous-backend")
    );
    assert!(state.events.lock().await.is_empty());
    quotio_platform::set_plugin_credential(&id, "api_key", "owned-valid-key").unwrap();
    // Make config persistence fail after set_backend: bind is explicitly cleared,
    // but selection remains the last confirmed preference and no success is recorded.
    std::fs::write(&state.profile_dir, b"owned-file-not-a-directory").unwrap();
    assert!(select_plugin_backend(&state, id).await.is_err());
    assert!(state.gateway.backend_plugin_id().await.is_none());
    assert_eq!(
        state.selected_plugin_id.read().await.as_deref(),
        Some("owned-previous-backend")
    );
    assert!(state.events.lock().await.is_empty());
}

#[tokio::test]
async fn rejected_gateway_binding_preserves_actual_confirmed_backend() {
    let (_dir, state, record) = state_and_record("openai_chat");
    state
        .gateway
        .set_backend(&record.manifest, "owned-valid-key")
        .await
        .unwrap();
    assert!(state
        .gateway
        .set_backend(&record.manifest, "owned-invalid\0key")
        .await
        .is_err());
    assert_eq!(
        state.gateway.backend_plugin_id().await.as_deref(),
        Some(record.manifest.metadata.id.as_str())
    );
    let mut invalid = record.manifest.clone();
    invalid.capabilities.protocol = "unsupported-protocol".into();
    assert!(state
        .gateway
        .set_backend(&invalid, "owned-valid-key")
        .await
        .is_err());
    assert_eq!(
        state.gateway.backend_plugin_id().await.as_deref(),
        Some(record.manifest.metadata.id.as_str())
    );
    assert!(state.events.lock().await.is_empty());
}

#[test]
fn plugin_api_key_validation_rejects_controls_and_unicode_without_echo() {
    assert!(valid_plugin_api_key("owned-plugin-key"));
    for value in ["", "space here", "owned\0key", "中文"] {
        assert!(!valid_plugin_api_key(value));
    }
    assert!(!valid_plugin_api_key(&"x".repeat(513)));
}

#[tokio::test]
async fn master_key_rotation_updates_gateway_auth_and_persists_keyring() {
    let (_dir, state, _record) = state_and_record("openai_chat");
    let old_key = state.gateway_master_key.read().await.clone();
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let handle = state.gateway.listen(port).await.unwrap();

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = format!("http://127.0.0.1:{port}/v1/models");

    // Old key authorizes (503 Service Unavailable because no backend attached yet)
    let res_old = client.get(&url).bearer_auth(&old_key).send().await.unwrap();
    assert_eq!(res_old.status().as_u16(), 503);

    // Stop gateway before rotation
    handle.shutdown().await.unwrap();

    // Rotate with successful persistence
    let mut stored_key = String::new();
    let rotated = rotate_gateway_master_key_with(&state, |k| {
        stored_key = k.to_string();
        Ok(())
    })
    .await
    .unwrap();

    assert_eq!(stored_key, rotated);
    assert_ne!(old_key, rotated);
    assert_eq!(*state.gateway_master_key.read().await, rotated);

    // Re-listen gateway and verify authorization is updated
    let handle = state.gateway.listen(port).await.unwrap();

    // Old key is now rejected (401 Unauthorized)
    let res_old_after = client.get(&url).bearer_auth(&old_key).send().await.unwrap();
    assert_eq!(res_old_after.status().as_u16(), 401);

    // New key is now authorized (503 Service Unavailable because no backend)
    let res_new = client.get(&url).bearer_auth(&rotated).send().await.unwrap();
    assert_eq!(res_new.status().as_u16(), 503);

    handle.shutdown().await.unwrap();

    let events = state.events.lock().await;
    assert_eq!(events.back().unwrap().action, "regenerate_key");
    assert!(events.back().unwrap().success);
}

#[tokio::test]
async fn master_key_rotation_fails_atomically_on_keyring_persistence_error() {
    let (_dir, state, _record) = state_and_record("openai_chat");
    let old_key = state.gateway_master_key.read().await.clone();
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let handle = state.gateway.listen(port).await.unwrap();

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = format!("http://127.0.0.1:{port}/v1/models");

    handle.shutdown().await.unwrap();

    let res =
        rotate_gateway_master_key_with(&state, |_| Err("injected keyring failure".to_string()))
            .await;

    assert!(res.is_err());
    // Key in state must not change
    assert_eq!(*state.gateway_master_key.read().await, old_key);

    // Restart gateway, verify old key still works and was not changed
    let handle = state.gateway.listen(port).await.unwrap();
    let res_old = client.get(&url).bearer_auth(&old_key).send().await.unwrap();
    assert_eq!(res_old.status().as_u16(), 503);
    handle.shutdown().await.unwrap();

    let events = state.events.lock().await;
    assert!(
        events.is_empty(),
        "failed rotation must not record success event"
    );
}

#[test]
fn desktop_state_init_fails_if_master_key_cannot_be_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let reg_dir = dir.path().join("registry");
    let prof_dir = dir.path().join("profile");
    std::fs::create_dir_all(&prof_dir).unwrap();
    let registry = Registry::open(&reg_dir).unwrap();

    // 1. Injected get error
    let res1 = DesktopState::new_with_credential_store(
        prof_dir.clone(),
        registry.clone(),
        || Err("injected keyring read failure".to_string()),
        |_| Ok(()),
    );
    assert!(res1.is_err());

    // 2. Injected set error when generating new key
    let res2 = DesktopState::new_with_credential_store(
        prof_dir,
        registry,
        || Ok(None),
        |_| Err("injected keyring write failure".to_string()),
    );
    assert!(res2.is_err());
}
