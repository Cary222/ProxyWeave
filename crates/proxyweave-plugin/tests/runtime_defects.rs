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
        let _ = write!(connection, "HTTP/1.1 {code} Fixture\r\nX-Fixture: {identity}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
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
