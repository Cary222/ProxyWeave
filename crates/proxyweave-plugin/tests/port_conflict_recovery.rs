//! Spec v10 / Plan.md §6.3 B acceptance: a Managed plugin whose child dies from a proven
//! bind conflict is re-screened onto a new port, re-launched with the new `{{PORT}}`, and is
//! reported as `Failed` with a structured diagnostic once the retries are spent.
//!
//! These tests drive the real `RuntimeManager` against a real child process
//! (`pw-port-fixture`): no allocator mock, no injected evidence. The conflict message the
//! runtime classifies comes from the operating system, produced by the fixture holding the
//! port and asking the kernel for it again.

use std::path::{Path, PathBuf};

use proxyweave_plugin::{Manifest, PluginRecord, RuntimeError, RuntimeManager, RuntimeStatus};
use sha2::Digest;

fn fixture_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pw-port-fixture"))
}

/// Stages the fixture as a plugin package and builds a Manifest that renders `{{PORT}}` into
/// the fixture's environment, exactly as a real Managed plugin is started.
fn staged_plugin(package_dir: &Path, conflict_attempts: u32, state: &Path) -> PluginRecord {
    std::fs::create_dir_all(package_dir).expect("package dir");
    // Manifest Schema 1.0 requires the `.exe` suffix on every platform, so the staged fixture
    // is named the same way here as it is on a Windows runner.
    let exe_name = "port-fixture.exe".to_string();
    let dest = package_dir.join(&exe_name);
    std::fs::copy(fixture_binary(), &dest).expect("stage fixture binary");
    let bytes = std::fs::read(&dest).expect("read staged fixture");
    let sha256 = format!("{:x}", sha2::Sha256::digest(&bytes));
    let state_path = state.to_string_lossy().to_string();

    let yaml = format!(
        r#"schema_version: '1.0'
metadata:
  id: port-fixture
  name: Port Fixture
  version: 1.0.0
  license: MIT
runtime:
  ownership: managed
  executable: {exe_name}
  sha256: {sha256}
  args: []
  env:
    FIXTURE_PORT: '{{PORT}}'
    FIXTURE_CONFLICT_ATTEMPTS: '{conflict_attempts}'
    FIXTURE_STATE: '{state_path}'
network:
  health_check:
    path: /healthz
    expected_status: 200
    identity_header: X-Service
    identity_value: port-fixture
    timeout_ms: 2000
lifecycle:
  startup_timeout_ms: 20000
  idle_timeout_ms: null
  max_port_retries: 0
capabilities:
  protocol: openai_chat
  models_path: /v1/models
  inference_path: /v1/chat/completions
  streaming: true
configuration:
  required_secrets: []
  config_template:
    port: '{{PORT}}'
"#
    );

    let manifest = Manifest::parse(yaml.as_bytes()).expect("fixture manifest parses");
    PluginRecord {
        manifest,
        fingerprint: "port-fixture-fingerprint".to_string(),
        settings: Default::default(),
        source: None,
    }
}

fn ready_port(status: &RuntimeStatus) -> u16 {
    match status {
        RuntimeStatus::Ready { port, .. } => *port,
        other => panic!("expected a Ready instance, got {other:?}"),
    }
}

#[tokio::test]
async fn managed_start_retries_after_a_proven_bind_conflict_and_recovers() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = tmp.path().join("pkg");
    let data_dir = tmp.path().join("data");
    let state = tmp.path().join("attempts.txt");
    // The first spawn loses the port to the OS; the second must succeed on a new port.
    let record = staged_plugin(&package_dir, 1, &state);

    let runtime = RuntimeManager::new();
    let status = runtime
        .start_plugin(&record, &package_dir, None, &data_dir, None)
        .await
        .expect("start recovers from one proven conflict");

    let port = ready_port(&status);
    assert!(
        (20_000..=60_000).contains(&port),
        "port {port} must come from the Plan.md §6.3 range"
    );
    assert_ne!(port, 8080, "a Managed instance never falls back to 8080");
    assert_eq!(
        std::fs::read_to_string(&state).expect("attempt counter").trim(),
        "2",
        "exactly one conflict then one successful attempt"
    );

    let allocator = runtime.port_allocator();
    assert_eq!(allocator.owner_of(port).as_deref(), Some("port-fixture"));
    assert_eq!(
        allocator.reserved_count(),
        1,
        "the blacklisted port was released before the retry"
    );

    runtime
        .stop_plugin("port-fixture")
        .await
        .expect("stop after recovery");
}

#[tokio::test]
async fn managed_start_reports_a_structured_diagnostic_when_every_retry_conflicts() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = tmp.path().join("pkg");
    let data_dir = tmp.path().join("data");
    let state = tmp.path().join("attempts.txt");
    let record = staged_plugin(&package_dir, 99, &state);

    let runtime = RuntimeManager::new();
    let err = runtime
        .start_plugin(&record, &package_dir, None, &data_dir, None)
        .await
        .expect_err("every attempt conflicts");

    let diagnostic = match err {
        RuntimeError::PortConflictExhausted { diagnostic } => diagnostic,
        other => panic!("expected PortConflictExhausted, got {other:?}"),
    };
    assert_eq!(diagnostic.plugin_id, "port-fixture");
    assert_eq!(diagnostic.max_retries, 3, "Plan.md §6.3 B caps retries at 3");
    assert_eq!(
        diagnostic.attempts.len(),
        4,
        "one initial attempt plus three retries"
    );
    let mut ports: Vec<u16> = diagnostic.attempts.iter().map(|a| a.port).collect();
    let attempted = ports.len();
    ports.sort_unstable();
    ports.dedup();
    assert_eq!(ports.len(), attempted, "every retry screened a fresh port");
    for attempt in &diagnostic.attempts {
        let evidence = attempt.evidence.to_ascii_lowercase();
        assert!(
            evidence.contains("address already in use") || evidence.contains("addrinuse"),
            "evidence must be the OS bind error, got {:?}",
            attempt.evidence
        );
    }
    // The diagnostic is serialisable for the API Doctor / console.
    let json = diagnostic.to_json();
    assert!(json.contains("\"attempts\""));

    assert!(matches!(
        runtime.get_status("port-fixture").await,
        RuntimeStatus::Failed { .. }
    ));
    assert_eq!(
        runtime.port_allocator().reserved_count(),
        0,
        "a failed start leaks no port"
    );
    assert_eq!(
        std::fs::read_to_string(&state).expect("attempt counter").trim(),
        "4"
    );
}

#[tokio::test]
async fn managed_stop_returns_the_port_and_the_next_start_gets_one_from_the_range() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let package_dir = tmp.path().join("pkg");
    let data_dir = tmp.path().join("data");
    let state = tmp.path().join("attempts.txt");
    let record = staged_plugin(&package_dir, 0, &state);

    let runtime = RuntimeManager::new();
    let allocator = runtime.port_allocator();

    let first = ready_port(
        &runtime
            .start_plugin(&record, &package_dir, None, &data_dir, None)
            .await
            .expect("first start"),
    );
    assert_eq!(allocator.refs(first), 1, "the instance holds one lease");
    assert_eq!(allocator.reserved_count(), 1);

    runtime.stop_plugin("port-fixture").await.expect("stop");
    assert_eq!(
        allocator.owner_of(first),
        None,
        "stopping the instance returns its port"
    );
    assert_eq!(allocator.reserved_count(), 0);

    let second = ready_port(
        &runtime
            .start_plugin(&record, &package_dir, None, &data_dir, None)
            .await
            .expect("restart"),
    );
    assert!(
        (20_000..=60_000).contains(&second),
        "restart port {second} must come from the range"
    );
    assert_ne!(second, 8080);
    assert_eq!(
        allocator.reserved_count(),
        1,
        "exactly one live reservation after the restart"
    );
    assert!(
        matches!(
            runtime.get_status("port-fixture").await,
            RuntimeStatus::Ready { .. }
        ),
        "the restarted instance is ready, not stuck in Failed"
    );

    runtime.stop_plugin("port-fixture").await.expect("final stop");
    assert_eq!(allocator.reserved_count(), 0);
}
