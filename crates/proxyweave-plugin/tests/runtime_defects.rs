#![cfg(windows)]

use proxyweave_plugin::{Manifest, PluginRecord, RuntimeError, RuntimeManager, RuntimeStatus};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::Path,
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
        .open(log)
        .unwrap()
        .write_all(format!("{}\n", std::process::id()).as_bytes())
        .unwrap();
    let delay: u64 = std::env::var("FIXTURE_DELAY").unwrap().parse().unwrap();
    std::thread::sleep(Duration::from_millis(delay));
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();
    let identity = std::env::var("FIXTURE_IDENTITY").unwrap();
    let code = std::env::var("FIXTURE_STATUS").unwrap();
    let mode = std::env::var("FIXTURE_MODE").unwrap_or_default();
    let mut requests = 0;
    let private_marker = "private-peer-marker-never-publish";
    for connection in listener.incoming() {
        let mut connection = connection.unwrap();
        connection
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = [0u8; 4096];
        let n = connection.read(&mut request).unwrap_or(0);
        if String::from_utf8_lossy(&request[..n]).starts_with("GET /exit ") {
            std::process::exit(0);
        }
        requests += 1;
        if mode == "timeout" {
            // Accept HTTP but never deliver headers within the Manifest probe deadline.
            std::thread::sleep(Duration::from_millis(250));
            continue;
        }
        if mode == "malformed" {
            let _ = write!(connection, "{private_marker}\r\n\r\n");
            continue;
        }
        let identity_header = if mode == "missing" {
            String::new()
        } else if mode == "recover" && requests > 1 {
            "X-Fixture: review-fixture\r\n".into()
        } else {
            format!("X-Fixture: {identity}\r\n")
        };
        let _ = write!(connection, "HTTP/1.1 {code} Fixture\r\n{identity_header}X-Private: {private_marker}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{private_marker}", private_marker.len());
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
    let deadline = Instant::now() + Duration::from_secs(5);
    while alive(pid) {
        assert!(Instant::now() < deadline, "owned PID {pid} still alive");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
async fn wait_for_spawn(root: &Path) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(5);
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
async fn def004_conflict_reselects_port_and_templates_reach_real_child() {
    let temp = sandbox();
    let manager = RuntimeManager::new();
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    let original = occupied.local_addr().unwrap().port();
    let (port, pid) = ready(
        manager
            .start_plugin(
                &record(0, "review-fixture", 200, 3),
                temp.path(),
                Some(original),
                temp.path(),
                None,
            )
            .await
            .unwrap(),
    );
    assert_ne!(port, original);
    assert!(alive(pid));
    assert_eq!(pids(temp.path()), vec![pid]);
    manager.stop_plugin("review-fixture").await.unwrap();
    wait_for_exit(pid).await;
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
