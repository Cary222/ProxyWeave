#![cfg(windows)]

use proxyweave_plugin::{Manifest, PluginRecord, RuntimeError, RuntimeManager, RuntimeStatus};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

// A real, owned subprocess, not an External no-op. Reuses Rust's test harness as the
// executable so no downloaded binary, compiler invocation, or runtime dependency is needed.
#[test]
#[ignore = "invoked only as an isolated Managed subprocess"]
fn managed_fixture_child() {
    let port = std::env::var("FIXTURE_PORT").unwrap();
    let log = std::env::var("FIXTURE_LOG").unwrap();
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .unwrap()
        .write_all(format!("{}\n", std::process::id()).as_bytes())
        .unwrap();
    let delay: u64 = std::env::var("FIXTURE_DELAY").unwrap().parse().unwrap();
    std::thread::sleep(Duration::from_millis(delay));
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();
    let identity = std::env::var("FIXTURE_IDENTITY").unwrap();
    let code = std::env::var("FIXTURE_STATUS").unwrap();
    let mode = std::env::var("FIXTURE_MODE").unwrap_or_default();
    if mode == "capture-projection" {
        let capture = serde_json::json!({
            "args": std::env::args().collect::<Vec<_>>(),
            "secret": std::env::var("FIXTURE_SECRET").unwrap(),
            "key": std::env::var("FIXTURE_KEY").unwrap(),
            "path": std::env::var("FIXTURE_PATH").unwrap(),
        });
        std::fs::write(
            std::env::var("FIXTURE_CAPTURE").unwrap(),
            serde_json::to_vec(&capture).unwrap(),
        )
        .unwrap();
    }
    let requests = Arc::new(AtomicUsize::new(0));
    let private_marker = "private-peer-marker-never-publish";
    let identity = Arc::<str>::from(identity);
    let code = Arc::<str>::from(code);
    let mode = Arc::<str>::from(mode);
    for connection in listener.incoming() {
        let Ok(mut connection) = connection else {
            break;
        };
        let identity = identity.clone();
        let code = code.clone();
        let mode = mode.clone();
        let requests = requests.clone();
        std::thread::spawn(move || {
            connection
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0u8; 4096];
            let n = connection.read(&mut request).unwrap_or(0);
            let req_str = String::from_utf8_lossy(&request[..n]);
            if req_str.starts_with("GET /exit ") {
                std::process::exit(0);
            }
            let req_count = requests.fetch_add(1, Ordering::SeqCst) + 1;
            if &*mode == "long-stream" && req_str.starts_with("POST /v1/chat/completions ") {
                // Hold an open SSE stream so the request stays active until the client disconnects.
                let _ = write!(
                    connection,
                    "HTTP/1.1 200 OK\r\nX-Fixture: {identity}\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\ndata: {{\"chunk\":0}}\n\n"
                );
                let _ = connection.flush();
                // Set short read timeout to poll for client cancellation/disconnect while keeping the stream alive.
                // Crucial requirement: a read timeout must NOT be interpreted as client cancellation.
                connection
                    .set_read_timeout(Some(Duration::from_millis(50)))
                    .unwrap();
                let mut sink = [0u8; 256];
                let mut chunk_index: u64 = 1;
                loop {
                    match connection.read(&mut sink) {
                        Ok(0) => {
                            // Client closed TCP connection (FIN/EOF)
                            break;
                        }
                        Ok(_) => {
                            // Data received on socket (e.g. downstream bytes)
                        }
                        Err(ref e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            // Read timeout while downstream is waiting/listening.
                            // Crucial: read timeout is NOT client cancellation. Write periodic SSE chunk
                            // to confirm the stream is alive and verify client socket is still open.
                            let chunk = format!("data: {{\"chunk\":{chunk_index}}}\n\n");
                            chunk_index += 1;
                            if connection.write_all(chunk.as_bytes()).is_err() {
                                // Write error: downstream client has disconnected / reset
                                break;
                            }
                            let _ = connection.flush();
                            std::thread::sleep(Duration::from_millis(50));
                        }
                        Err(_) => {
                            // Client disconnect / reset / abort
                            break;
                        }
                    }
                }
                return;
            }
            if &*mode == "timeout" {
                // Accept HTTP but never deliver headers within the Manifest probe deadline.
                std::thread::sleep(Duration::from_millis(250));
                return;
            }
            if &*mode == "malformed" {
                let _ = write!(connection, "{private_marker}\r\n\r\n");
                return;
            }
            let identity_header = if &*mode == "missing" {
                String::new()
            } else if &*mode == "recover" && req_count > 1 {
                "X-Fixture: review-fixture\r\n".into()
            } else {
                format!("X-Fixture: {identity}\r\n")
            };
            let _ = write!(
                connection,
                "HTTP/1.1 {code} Fixture\r\n{identity_header}X-Private: {private_marker}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{private_marker}",
                private_marker.len()
            );
        });
    }
}

fn record(delay: u64, identity: &str, status: u16, retries: u8) -> PluginRecord {
    use sha2::Digest;
    let exe = std::env::current_exe().unwrap();
    let hash = format!("{:x}", sha2::Sha256::digest(std::fs::read(exe).unwrap()));
    let yaml = format!(
        r#"schema_version: '1.0'
metadata: {{id: review-fixture, name: Runtime regression fixture, version: 1.0.0, license: MIT}}
runtime:
  ownership: managed
  executable: fixture.exe
  sha256: {hash}
  args: ['--exact', managed_fixture_child, '--ignored', '--nocapture']
  env:
    FIXTURE_PORT: '{{PORT}}'
    FIXTURE_LOG: '{{DATA_DIR}}/spawn.log'
    FIXTURE_DELAY: '{delay}'
    FIXTURE_IDENTITY: '{identity}'
    FIXTURE_STATUS: '{status}'
network:
  health_check:
    path: /healthz
    expected_status: 200
    identity_header: X-Fixture
    identity_value: review-fixture
    timeout_ms: 100
    busy_statuses: [503]
lifecycle: {{startup_timeout_ms: 1800, idle_timeout_ms: null, max_port_retries: {retries}}}
capabilities: {{protocol: openai_chat, models_path: /v1/models, inference_path: /v1/chat/completions, streaming: true}}
configuration: {{config_template: {{}}}}
"#
    );
    PluginRecord {
        manifest: Manifest::parse(yaml.as_bytes()).unwrap(),
        fingerprint: "fixture".into(),
        settings: Default::default(),
        source: None,
    }
}

fn sandbox() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    std::fs::copy(
        std::env::current_exe().unwrap(),
        temp.path().join("fixture.exe"),
    )
    .unwrap();
    temp
}
fn pids(root: &Path) -> Vec<u32> {
    std::fs::read_to_string(root.join("plugins/review-fixture/spawn.log"))
        .unwrap_or_default()
        .lines()
        .map(|s| s.parse().unwrap())
        .collect()
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
async fn wait_for_exit(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while alive(pid) {
        assert!(Instant::now() < deadline, "owned PID {pid} still alive");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
async fn wait_for_spawn(root: &Path) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(pid) = pids(root).last() {
            return *pid;
        }
        assert!(Instant::now() < deadline, "fixture did not spawn");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
fn ready(status: RuntimeStatus) -> (u16, u32) {
    match status {
        RuntimeStatus::Ready {
            port,
            pid: Some(pid),
        } => (port, pid),
        other => panic!("expected Ready: {other:?}"),
    }
}

#[tokio::test]
async fn def001_exited_child_reconciles_and_restarts_with_live_new_pid() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let record = record(0, "review-fixture", 200, 0);
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let (_, pid) = ready(
        manager
            .start_plugin(&record, temp.path(), Some(port), temp.path(), None)
            .await
            .unwrap(),
    );
    assert!(alive(pid));
    let _ = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .get(format!("http://127.0.0.1:{port}/exit"))
        .send()
        .await;
    wait_for_exit(pid).await;
    assert!(matches!(
        manager.get_status("review-fixture").await,
        RuntimeStatus::Failed { .. }
    ));
    let (_, new_pid) = ready(
        manager
            .start_plugin(&record, temp.path(), Some(port), temp.path(), None)
            .await
            .unwrap(),
    );
    assert_ne!(pid, new_pid);
    assert!(alive(new_pid));
    assert_eq!(pids(temp.path()).len(), 2);
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(new_pid).await;
}

#[tokio::test]
async fn def002_http200_wrong_identity_is_not_ready() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let status = manager
        .start_plugin(
            &record(0, "different-service", 200, 0),
            temp.path(),
            Some(port),
            temp.path(),
            None,
        )
        .await
        .unwrap();
    assert!(matches!(status, RuntimeStatus::Failed { .. }));
    let pid = wait_for_spawn(temp.path()).await;
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid).await;
}

#[tokio::test]
async fn def003_starting_visible_and_queued_stop_cleans_delayed_child() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let start = {
        let manager = manager.clone();
        let root = temp.path().to_owned();
        tokio::spawn(async move {
            manager
                .start_plugin(
                    &record(500, "review-fixture", 200, 0),
                    &root,
                    Some(port),
                    &root,
                    None,
                )
                .await
        })
    };
    let pid = wait_for_spawn(temp.path()).await;
    assert_eq!(
        manager.get_status("review-fixture").await,
        RuntimeStatus::Starting
    );
    manager.stop_plugin("review-fixture").await.unwrap();
    ready(start.await.unwrap().unwrap());
    assert_eq!(
        manager.get_status("review-fixture").await,
        RuntimeStatus::Stopped
    );
    wait_for_exit(pid).await;
}

#[tokio::test]
async fn def004_pinned_busy_port_is_refused_even_with_retries_left() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    let original = occupied.local_addr().unwrap().port();
    // A pinned port is never silently re-routed: the allocator proves it is held and refuses
    // it (tests\test_protocol_adapter_port.rs:101, `a_port_held_by_another_process_is_never_handed_out`).
    // The Plan.md §6.3 B reselect loop is driven by conflict evidence from the started child,
    // which port_conflict_recovery.rs covers; a retry budget must not move a pinned port.
    let error = manager
        .start_plugin(
            &record(0, "review-fixture", 200, 3),
            temp.path(),
            Some(original),
            temp.path(),
            None,
        )
        .await
        .expect_err("a pinned port held by another process must be refused");
    assert!(
        matches!(&error, RuntimeError::PortConflict(_)),
        "expected the proven conflict, got {error:?}"
    );
    assert!(pids(temp.path()).is_empty());
    assert!(!temp.path().join("plugins").exists());
    assert!(occupied.local_addr().is_ok());
}

#[tokio::test]
async fn zero_retry_conflict_has_no_spawn_and_no_projection() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    let error = manager
        .start_plugin(
            &record(0, "review-fixture", 200, 0),
            temp.path(),
            Some(occupied.local_addr().unwrap().port()),
            temp.path(),
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, RuntimeError::PortConflict(_)));
    assert!(pids(temp.path()).is_empty());
    assert!(!temp.path().join("plugins").exists());
}

#[tokio::test]
async fn real_managed_singleflight_spawns_once_and_busy_is_preserved() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let record = record(300, "review-fixture", 503, 0);
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let (a, b) = tokio::join!(
        manager.start_plugin(&record, temp.path(), Some(port), temp.path(), None),
        manager.start_plugin(&record, temp.path(), Some(port), temp.path(), None)
    );
    let a = a.unwrap();
    assert_eq!(a, b.unwrap());
    let pid = match a {
        RuntimeStatus::Busy { pid: Some(pid), .. } => pid,
        other => panic!("expected Busy: {other:?}"),
    };
    assert_eq!(pids(temp.path()), vec![pid]);
    assert!(alive(pid));
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid).await;
}

#[tokio::test]
async fn abort_during_start_reclaims_job_and_marks_failed() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let start = {
        let manager = manager.clone();
        let root = temp.path().to_owned();
        tokio::spawn(async move {
            manager
                .start_plugin(
                    &record(1000, "review-fixture", 200, 0),
                    &root,
                    Some(port),
                    &root,
                    None,
                )
                .await
        })
    };
    let pid = wait_for_spawn(temp.path()).await;
    start.abort();
    assert!(start.await.unwrap_err().is_cancelled());
    wait_for_exit(pid).await;
    assert!(matches!(
        manager.get_status("review-fixture").await,
        RuntimeStatus::Failed { .. }
    ));
    manager.stop_plugin("review-fixture").await.unwrap();
    assert_eq!(
        manager.get_status("review-fixture").await,
        RuntimeStatus::Stopped
    );
}

fn diagnostic_record(mode: &str, identity: &str, status: u16) -> PluginRecord {
    let mut record = record(0, identity, status, 0);
    if let proxyweave_plugin::manifest::Runtime::Managed { env, .. } = &mut record.manifest.runtime
    {
        env.insert("FIXTURE_MODE".into(), mode.into());
    }
    // Include a credential-like URL marker to detect accidental reqwest error echo.
    record.manifest.network.health_check.path = "/healthz?token=private-url-token".into();
    record.manifest.lifecycle.startup_timeout_ms = 1000;
    record
}

fn assert_sanitized(value: &impl std::fmt::Debug) {
    let text = format!("{value:?}");
    for private in [
        "private-peer-marker-never-publish",
        "private-url-token",
        "private-wrong-identity",
        "X-Fixture",
        "review-fixture",
    ] {
        assert!(
            !text.contains(private),
            "probe diagnostic exposed private data"
        );
    }
}

#[tokio::test]
async fn def005_missing_and_wrong_identity_preserve_safe_failure() {
    for (mode, identity, code) in [
        ("missing", "private-wrong-identity", 200),
        ("", "private-wrong-identity", 200),
        ("", "private-wrong-identity", 503),
    ] {
        let temp = sandbox();
        let manager = RuntimeManager::new();
        let status = manager
            .start_plugin(
                &diagnostic_record(mode, identity, code),
                temp.path(),
                Some(proxyweave_plugin::port::pick_unused_port().unwrap()),
                temp.path(),
                None,
            )
            .await
            .unwrap();
        let RuntimeStatus::Failed { ref error } = status else {
            panic!("expected identity failure");
        };
        assert!(
            error.contains("last probe failure: Health check identity mismatch or missing header")
        );
        assert_sanitized(&status);
        assert_eq!(manager.get_status("review-fixture").await, status);
        let pid = wait_for_spawn(temp.path()).await;
        assert!(alive(pid));
        manager.stop_plugin("review-fixture").await.unwrap();
        wait_for_exit(pid).await;
    }
}

#[tokio::test]
async fn def005_later_ready_supersedes_previous_identity_failure() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let (_, pid) = ready(
        manager
            .start_plugin(
                &diagnostic_record("recover", "private-wrong-identity", 200),
                temp.path(),
                Some(proxyweave_plugin::port::pick_unused_port().unwrap()),
                temp.path(),
                None,
            )
            .await
            .unwrap(),
    );
    assert!(alive(pid));
    assert_eq!(pids(temp.path()), vec![pid]);
    assert!(matches!(
        manager.get_status("review-fixture").await,
        RuntimeStatus::Ready { .. }
    ));
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid).await;
}

#[tokio::test]
async fn def005_pure_http_timeout_remains_timeout_without_stale_failure() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let status = manager
        .start_plugin(
            &diagnostic_record("timeout", "review-fixture", 200),
            temp.path(),
            Some(proxyweave_plugin::port::pick_unused_port().unwrap()),
            temp.path(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        status,
        RuntimeStatus::Failed {
            error: "Readiness probe timed out after 1s".into()
        }
    );
    assert_sanitized(&status);
    let pid = wait_for_spawn(temp.path()).await;
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid).await;
}

#[tokio::test]
async fn def005_correct_busy_is_not_replaced_by_failure_or_timeout() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let status = manager
        .start_plugin(
            &diagnostic_record("", "review-fixture", 503),
            temp.path(),
            Some(proxyweave_plugin::port::pick_unused_port().unwrap()),
            temp.path(),
            None,
        )
        .await
        .unwrap();
    let RuntimeStatus::Busy {
        pid: Some(pid),
        ref reason,
        ..
    } = status
    else {
        panic!("expected declared Busy");
    };
    assert_eq!(reason, "Manifest-declared unavailable HTTP status 503");
    assert_sanitized(&status);
    assert!(alive(pid));
    assert_eq!(manager.get_status("review-fixture").await, status);
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid).await;
}

#[tokio::test]
async fn def005_malformed_http_preserves_safe_error_without_url_or_body() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let status = manager
        .start_plugin(
            &diagnostic_record("malformed", "review-fixture", 200),
            temp.path(),
            Some(proxyweave_plugin::port::pick_unused_port().unwrap()),
            temp.path(),
            None,
        )
        .await
        .unwrap();
    let RuntimeStatus::Failed { ref error } = status else {
        panic!("expected HTTP failure");
    };
    assert!(error.contains("last probe failure: HTTP request failed"));
    assert_sanitized(&status);
    assert_eq!(manager.get_status("review-fixture").await, status);
    let pid = wait_for_spawn(temp.path()).await;
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid).await;
}

#[tokio::test]
async fn def006_actual_managed_args_env_and_keyring_projection_preserve_data() {
    use proxyweave_plugin::manifest::Runtime;
    let temp = sandbox();
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let plugin_id = format!("def006-{}-{port}", std::process::id());
    struct Credentials(String);
    impl Drop for Credentials {
        fn drop(&mut self) {
            for key in ["a", "z"] {
                let _ = quotio_platform::delete_plugin_credential(&self.0, key);
            }
        }
    }
    let _credentials = Credentials(plugin_id.clone());
    let opaque = "dummy-雪-\"quoted\"-\\-$1-{SECRET:z}-{SECRET:missing}-{PORT}-{AUTH_DIR}-{";
    let api_key = "dummy-key-{SECRET:a}-{PLUGIN_API_KEY}-{PORT}";
    quotio_platform::set_plugin_credential(&plugin_id, "a", opaque).unwrap();
    quotio_platform::set_plugin_credential(&plugin_id, "z", "must-not-replace").unwrap();
    let mut record = record(0, "review-fixture", 200, 0);
    record.manifest.metadata.id = plugin_id.clone();
    record.manifest.configuration.required_secrets = vec!["a".into(), "z".into()];
    record.manifest.configuration.config_template =
        serde_json::json!({"secret": "{SECRET:a}", "key": "{PLUGIN_API_KEY}"});
    record.manifest.configuration.credential_files.insert(
        "auths/dummy.json".into(),
        serde_json::json!({"value": "{SECRET:a}", "nested": ["{PLUGIN_API_KEY}", 17, true]}),
    );
    let Runtime::Managed { args, env, .. } = &mut record.manifest.runtime else {
        unreachable!()
    };
    args.extend(["--skip".into(), "{DATA_DIR}".into()]);
    env.insert("FIXTURE_MODE".into(), "capture-projection".into());
    env.insert("FIXTURE_SECRET".into(), "{SECRET:a}".into());
    env.insert("FIXTURE_KEY".into(), "{PLUGIN_API_KEY}".into());
    env.insert("FIXTURE_PATH".into(), "{DATA_DIR}".into());
    env.insert("FIXTURE_CAPTURE".into(), "{DATA_DIR}/capture.json".into());
    record.manifest.validate().unwrap();
    let data_root = temp.path().join("opaque-{AUTH_DIR}");
    let manager = RuntimeManager::new();
    let (_, pid) = ready(
        manager
            .start_plugin(&record, temp.path(), Some(port), &data_root, Some(api_key))
            .await
            .unwrap(),
    );
    let plugin_data = data_root.join("plugins").join(&plugin_id);
    let read_json = |name: &str| -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(plugin_data.join(name)).unwrap()).unwrap()
    };
    let capture = read_json("capture.json");
    assert_eq!(
        capture["secret"].as_str().unwrap().as_bytes(),
        opaque.as_bytes()
    );
    assert_eq!(capture["key"], api_key);
    assert_eq!(capture["path"], plugin_data.to_string_lossy().as_ref());
    assert!(capture["args"]
        .as_array()
        .unwrap()
        .windows(2)
        .any(|pair| pair[0] == "--skip" && pair[1] == plugin_data.to_string_lossy().as_ref()));
    let config = read_json("config.json");
    assert_eq!(
        config["secret"].as_str().unwrap().as_bytes(),
        opaque.as_bytes()
    );
    assert_eq!(config["key"], api_key);
    let credential = read_json("auths/dummy.json");
    assert_eq!(
        credential["value"].as_str().unwrap().as_bytes(),
        opaque.as_bytes()
    );
    assert_eq!(credential["nested"], serde_json::json!([api_key, 17, true]));
    manager.stop_plugin(&plugin_id).await.unwrap();
    wait_for_exit(pid).await;
    assert_eq!(manager.get_status(&plugin_id).await, RuntimeStatus::Stopped);
    record
        .manifest
        .configuration
        .required_secrets
        .push("absent".into());
    record.manifest.configuration.config_template =
        serde_json::json!({"value": "{SECRET:a}:{SECRET:absent}"});
    record.manifest.validate().unwrap();
    let fresh_port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let error = manager
        .start_plugin(
            &record,
            temp.path(),
            Some(fresh_port),
            &data_root,
            Some(api_key),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(&error, RuntimeError::ConfigProjectionFailed(_)),
        "expected a config projection failure, got {error:?}"
    );
    let public_error = format!("{error:?} {error}");
    let status = manager.get_status(&plugin_id).await;
    assert!(matches!(status, RuntimeStatus::Failed { .. }));
    let public_status = format!("{status:?}");
    for public in [&public_error, &public_status] {
        assert!(!public.contains(opaque));
        assert!(!public.contains(api_key));
        assert!(!public.contains("dummy-"));
        assert!(!public.contains("absent"));
    }
    assert_eq!(
        std::fs::read_to_string(plugin_data.join("spawn.log"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    for key in ["a", "z"] {
        quotio_platform::delete_plugin_credential(&plugin_id, key).unwrap();
        assert_eq!(
            quotio_platform::get_plugin_credential(&plugin_id, key).unwrap(),
            None
        );
    }
}

fn stream_record() -> PluginRecord {
    let mut record = record(0, "review-fixture", 200, 0);
    if let proxyweave_plugin::manifest::Runtime::Managed { env, .. } = &mut record.manifest.runtime
    {
        env.insert("FIXTURE_MODE".into(), "long-stream".into());
    }
    record
}

#[tokio::test]
async fn active_stream_blocks_stop_and_cancellation_releases_lease() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let record = stream_record();
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let (port, pid) = ready(
        manager
            .start_plugin(&record, temp.path(), Some(port), temp.path(), None)
            .await
            .unwrap(),
    );
    assert!(alive(pid));
    assert_eq!(manager.active_request_count("review-fixture").await, 0);

    // Architectural Boundary Note:
    // In this plugin-isolated test boundary (crates/proxyweave-plugin), proxyweave-gateway
    // cannot be imported due to crate dependency hierarchy (proxyweave-gateway depends on
    // proxyweave-plugin; adding gateway here would cause a circular dependency).
    // Automatic request-lease acquisition during gateway routing and automatic lease release
    // upon downstream client disconnect are proven in `crates/proxyweave-gateway/tests/passthrough.rs`
    // (`native_disconnect_during_stream_cancels_upstream_and_releases_lease`).
    // Here we explicitly test the direct upstream Managed fixture streaming protocol:
    // 1. Upstream fixture serves genuine text/event-stream SSE bytes.
    // 2. Stream cannot complete prematurely on its own before explicit client cancellation.
    // 3. The RuntimeManager request lease actively blocks stop_plugin and protects the owned PID.
    // 4. Client cancellation terminates the upstream stream, and lease release allows clean stop.
    let lease = manager.acquire_request("review-fixture").await.unwrap();
    assert_eq!(manager.active_request_count("review-fixture").await, 1);

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut response = client
        .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Verify Content-Type is genuine text/event-stream
    let content_type = response
        .headers()
        .get("content-type")
        .expect("missing content-type header")
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("text/event-stream"),
        "expected text/event-stream, got: {content_type}"
    );

    // Read the first SSE chunk to confirm stream is active and emitting bytes
    let first_chunk = response
        .chunk()
        .await
        .unwrap()
        .expect("expected initial SSE chunk");
    assert!(
        String::from_utf8_lossy(&first_chunk).contains("data: {\"chunk\":0}"),
        "unexpected first chunk: {:?}",
        first_chunk
    );

    // Verify the stream cannot complete before explicit cancellation:
    // read a subsequent chunk emitted by the active upstream loop.
    let second_chunk = response
        .chunk()
        .await
        .unwrap()
        .expect("stream terminated prematurely without client cancellation");
    assert!(
        String::from_utf8_lossy(&second_chunk).contains("data: {\"chunk\":"),
        "unexpected second chunk: {:?}",
        second_chunk
    );

    // While the stream is active, stop must be refused and the owned process must stay alive.
    let refused = manager.stop_plugin("review-fixture").await.unwrap_err();
    assert!(matches!(refused, RuntimeError::ActiveRequests));
    assert!(alive(pid));
    assert_eq!(manager.active_request_count("review-fixture").await, 1);

    // Explicit client cancellation: dropping the in-flight response terminates downstream TCP connection.
    drop(response);

    // Releasing the request lease clears active request count.
    drop(lease);
    assert_eq!(manager.active_request_count("review-fixture").await, 0);

    // After cancellation and lease release, stop succeeds and terminates the owned process.
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid).await;
    assert_eq!(
        manager.get_status("review-fixture").await,
        RuntimeStatus::Stopped
    );
}

#[tokio::test]
async fn dropped_lease_during_active_stream_releases_counter() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let record = stream_record();
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let (port, pid) = ready(
        manager
            .start_plugin(&record, temp.path(), Some(port), temp.path(), None)
            .await
            .unwrap(),
    );
    assert!(alive(pid));

    // Architectural Boundary Note on RAII Scope & Lifecycle:
    // ProxyWeave has NO background idle reaper thread (as designed in ADR-003 and AGENTS.md).
    // Lifecycle safety is governed exclusively by the RequestLease RAII guard and the
    // active_requests atomic counter.
    // This test verifies that:
    // 1. A genuine long-stream request is initiated and held open against the owned Managed child.
    // 2. An acquired RequestLease holds active_request_count at 1, which blocks stop_plugin.
    // 3. Dropping the RequestLease guard immediately releases the active counter to 0 (RAII drop).
    // 4. Downstream cancellation (dropping response) tears down the active stream.
    // 5. With active_requests at 0, stop_plugin cleanly reclaims the process.
    let lease = manager.acquire_request("review-fixture").await.unwrap();
    assert_eq!(manager.active_request_count("review-fixture").await, 1);

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut response = client
        .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let first_chunk = response
        .chunk()
        .await
        .unwrap()
        .expect("expected initial SSE chunk");
    assert!(String::from_utf8_lossy(&first_chunk).contains("data: {\"chunk\":0}"));

    // Stop is refused while lease is held
    assert!(matches!(
        manager.stop_plugin("review-fixture").await,
        Err(RuntimeError::ActiveRequests)
    ));
    assert!(alive(pid));

    // Dropping the RAII lease guard alone must settle the counter back to zero
    drop(lease);
    assert_eq!(manager.active_request_count("review-fixture").await, 0);

    // Downstream stream is cancelled
    drop(response);

    // Stop now succeeds because active_requests is 0
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid).await;
    assert_eq!(
        manager.get_status("review-fixture").await,
        RuntimeStatus::Stopped
    );
}

#[tokio::test]
async fn test_runtime_generation_and_maintenance_barrier() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let record = record(0, "review-fixture", 200, 0);
    let port = proxyweave_plugin::port::pick_unused_port().unwrap();
    let (_port1, pid1) = ready(
        manager
            .start_plugin(
                &record,
                temp.path(),
                Some(port1_binding(&record, port)),
                temp.path(),
                None,
            )
            .await
            .unwrap(),
    );
    assert!(alive(pid1));

    // Generation starts at 1
    assert_eq!(manager.instance_generation("review-fixture").await, Some(1));

    // Acquire lease for generation 1
    let lease1 = manager
        .acquire_request_for("review-fixture", Some(1))
        .await
        .unwrap();
    assert_eq!(lease1.instance_generation(), 1);

    // Stale generation lease acquisition fails
    assert!(matches!(
        manager.acquire_request_for("review-fixture", Some(0)).await,
        Err(RuntimeError::StaleGeneration {
            expected: 0,
            actual: 1
        })
    ));
    assert!(matches!(
        manager.acquire_request_for("review-fixture", Some(2)).await,
        Err(RuntimeError::StaleGeneration {
            expected: 2,
            actual: 1
        })
    ));

    // Enter maintenance barrier with active lease
    let barrier_manager = manager.clone();
    let barrier_task = tokio::spawn(async move {
        barrier_manager
            .enter_maintenance_barrier("review-fixture", Duration::from_millis(1500))
            .await
    });

    // While waiting for drain, new lease acquisition is rejected with ServiceUnderMaintenance
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(matches!(
        manager.acquire_request("review-fixture").await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));

    assert!(matches!(
        manager.stop_plugin("review-fixture").await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    assert!(matches!(
        manager
            .start_plugin(&record, temp.path(), None, temp.path(), None)
            .await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    assert!(matches!(
        manager
            .restart_plugin(&record, temp.path(), None, temp.path(), None)
            .await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    // Release active lease
    drop(lease1);

    // Barrier finishes cleanly
    let maintenance_token = barrier_task.await.unwrap().unwrap();

    // Cancel maintenance restores normal status
    manager
        .cancel_maintenance_barrier(&maintenance_token)
        .await
        .unwrap();
    let lease_after_cancel = manager.acquire_request("review-fixture").await.unwrap();
    assert_eq!(lease_after_cancel.instance_generation(), 1);
    drop(lease_after_cancel);

    // Stop and restart advances generation
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid1).await;
    assert_eq!(manager.instance_generation("review-fixture").await, None);

    let (_port2, pid2) = ready(
        manager
            .start_plugin(&record, temp.path(), None, temp.path(), None)
            .await
            .unwrap(),
    );
    assert_ne!(pid1, pid2);
    assert_eq!(manager.instance_generation("review-fixture").await, Some(3));

    let lease2 = manager
        .acquire_request_for("review-fixture", Some(3))
        .await
        .unwrap();
    assert_eq!(lease2.instance_generation(), 3);
    drop(lease2);

    // Expired owners cannot release a later transaction or authorize another plugin.
    let next_token = manager
        .enter_maintenance_barrier("review-fixture", Duration::from_secs(1))
        .await
        .unwrap();
    assert!(matches!(
        manager.exit_maintenance(&maintenance_token).await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    assert!(matches!(
        manager.cancel_maintenance_barrier(&maintenance_token).await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    assert!(matches!(
        manager
            .stop_plugin_under_maintenance("review-fixture", &maintenance_token)
            .await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    assert!(matches!(
        manager
            .start_plugin_under_maintenance(
                &record,
                temp.path(),
                None,
                temp.path(),
                None,
                &maintenance_token
            )
            .await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    assert!(matches!(
        manager
            .stop_plugin_under_maintenance("another-plugin", &next_token)
            .await,
        Err(RuntimeError::PluginNotFound(_))
    ));
    manager
        .stop_plugin_under_maintenance("review-fixture", &next_token)
        .await
        .unwrap();
    wait_for_exit(pid2).await;
    assert!(manager.is_maintenance("review-fixture").await);
    assert!(matches!(
        manager
            .start_plugin(&record, temp.path(), None, temp.path(), None)
            .await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    let (_, pid3) = ready(
        manager
            .start_plugin_under_maintenance(
                &record,
                temp.path(),
                None,
                temp.path(),
                None,
                &next_token,
            )
            .await
            .unwrap(),
    );
    assert!(manager.is_maintenance("review-fixture").await);
    assert!(matches!(
        manager.acquire_request("review-fixture").await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    manager.exit_maintenance(&next_token).await.unwrap();
    drop(manager.acquire_request("review-fixture").await.unwrap());
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid3).await;
}

fn port1_binding(_record: &PluginRecord, port: u16) -> u16 {
    port
}

#[tokio::test]
async fn maintenance_drain_timeout_restores_admission_without_stopping_owned_process() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let record = record(0, "review-fixture", 200, 0);
    let (_, pid) = ready(
        manager
            .start_plugin(&record, temp.path(), None, temp.path(), None)
            .await
            .unwrap(),
    );
    let lease = manager.acquire_request("review-fixture").await.unwrap();
    let other = manager.clone();
    let barrier = tokio::spawn(async move {
        other
            .enter_maintenance_barrier("review-fixture", Duration::from_millis(300))
            .await
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        while !manager.is_maintenance("review-fixture").await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        manager
            .enter_maintenance_barrier("review-fixture", Duration::ZERO)
            .await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    assert!(matches!(
        manager.stop_plugin("review-fixture").await,
        Err(RuntimeError::ServiceUnderMaintenance)
    ));
    assert!(matches!(
        barrier.await.unwrap(),
        Err(RuntimeError::ActiveRequestsTimeout {
            active_requests: 1,
            ..
        })
    ));
    assert!(!manager.is_maintenance("review-fixture").await);
    assert!(alive(pid));
    drop(manager.acquire_request("review-fixture").await.unwrap());
    drop(lease);
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid).await;
}
