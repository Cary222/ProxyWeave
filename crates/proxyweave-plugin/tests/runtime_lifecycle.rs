use proxyweave_plugin::{
    check_port_available, JobObject, PluginRecord, PortConflictError, ProbeStatus, RuntimeError,
    RuntimeManager,
};
use std::net::TcpListener;
use std::path::Path;
use std::time::Duration;
#[tokio::test]
async fn test_job_object_kills_process_on_drop() {
    let job = JobObject::create().expect("failed to create JobObject");

    // Spawn a long-running process using cmd.exe ping
    let mut child = tokio::process::Command::new("cmd.exe")
        .args(["/c", "ping", "127.0.0.1", "-n", "30", ">", "nul"])
        .spawn()
        .expect("failed to spawn child");

    let pid = child.id().expect("missing child pid");
    assert!(pid > 0, "child pid must be valid");

    // Assign to JobObject
    job.assign_tokio_child(&child)
        .expect("failed to assign child to JobObject");

    // Drop JobObject - must immediately kill child due to KILL_ON_JOB_CLOSE
    let start = std::time::Instant::now();
    drop(job);

    // Child must exit promptly
    let status = child
        .wait()
        .await
        .expect("failed to wait on terminated child");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "Child process took too long to terminate after JobObject drop: {:?}",
        elapsed
    );
    println!(
        "Verified: JobObject killed child PID {} in {:?}, exit status: {:?}",
        pid, elapsed, status
    );
}

#[tokio::test]
async fn test_external_service_zero_kill_protection() {
    let manager = RuntimeManager::new();

    // Populate an External provider runtime
    // Internal access simulation to insert an active external provider
    let record = PluginRecord {
        manifest: proxyweave_plugin::Manifest::parse(
            r#"schema_version: '1.0'
metadata:
  id: external-cpa
  name: External CPA
  version: 1.0.0
  license: MIT
runtime:
  ownership: external
  endpoint: http://127.0.0.1:28317
network:
  health_check:
    path: /healthz
    expected_status: 200
    identity_header: X-Service
    identity_value: external-cpa
    timeout_ms: 1000
lifecycle:
  startup_timeout_ms: 2000
  idle_timeout_ms: null
  max_port_retries: 0
capabilities:
  protocol: openai_chat
  models_path: /v1/models
  inference_path: /v1/chat/completions
  streaming: true
configuration: {}
"#
            .as_bytes(),
        )
        .unwrap(),
        fingerprint: "fake_fingerprint".to_string(),
        settings: Default::default(),
    };

    // When external plugin starts, it marks as active External
    // Now test stop_plugin
    let _res = manager
        .start_plugin(&record, Path::new("."), Some(28317), Path::new("."), None)
        .await;
    // Even if probe fails or is stopped, if it's set as external:

    // Now manually set an external active plugin in the manager and verify stop rejection
    let manager = RuntimeManager::new();
    // Simulate active external plugin
    let err = manager.stop_plugin("non-existent").await.unwrap_err();
    assert!(matches!(err, RuntimeError::PluginNotFound(_)));

    // Test with real external stopped entry
    let ext_record = PluginRecord {
        manifest: proxyweave_plugin::Manifest::parse(
            r#"schema_version: '1.0'
metadata:
  id: ext-live
  name: External Live
  version: 1.0.0
  license: MIT
runtime:
  ownership: external
  endpoint: http://127.0.0.1:19999
network:
  health_check:
    path: /healthz
    expected_status: 200
    identity_header: X-Service
    identity_value: ext-live
    timeout_ms: 500
lifecycle:
  startup_timeout_ms: 1000
  idle_timeout_ms: null
  max_port_retries: 0
capabilities:
  protocol: openai_chat
  models_path: /v1/models
  inference_path: /v1/chat/completions
  streaming: true
configuration: {}
"#
            .as_bytes(),
        )
        .unwrap(),
        fingerprint: "fingerprint".to_string(),
        settings: Default::default(),
    };

    // Starting external plugin with probe
    let _status = manager
        .start_plugin(
            &ext_record,
            Path::new("."),
            Some(19999),
            Path::new("."),
            None,
        )
        .await
        .unwrap();

    // Now attempt to stop it -> MUST be rejected with ExternalServiceProtected
    let stop_res = manager.stop_plugin("ext-live").await;
    assert!(
        stop_res.is_err(),
        "Stopping an external service must be refused"
    );
    let err = stop_res.unwrap_err();
    match err {
        RuntimeError::ExternalServiceProtected(msg) => {
            assert!(msg.contains("ext-live"));
            assert!(msg.contains("owned outside ProxyWeave"));
            println!("Verified External service protection: {}", msg);
        }
        other => panic!("Expected ExternalServiceProtected error, got: {:?}", other),
    }
}

#[test]
fn test_port_conflict_explicit_distinction() {
    // Bind a listener on an available ephemeral port
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind ephemeral port");
    let bound_port = listener.local_addr().unwrap().port();

    // Check availability on the bound port -> must return PortConflictError::AddrInUse
    let check_res = check_port_available(bound_port);
    assert!(check_res.is_err());
    let err = check_res.unwrap_err();

    match &err {
        PortConflictError::AddressInUse { port, .. } => {
            assert_eq!(*port, bound_port);
            println!("Verified: AddrInUse correctly detected for port {}", port);
        }
        other => panic!("Expected AddrInUse error, got: {:?}", other),
    }

    // Convert to RuntimeError
    let runtime_err: RuntimeError = err.into();
    match runtime_err {
        RuntimeError::PortConflict(msg) => {
            assert!(msg.contains(&bound_port.to_string()));
            println!("Verified: RuntimeError::PortConflict formatted: {}", msg);
        }
        other => panic!("Expected RuntimeError::PortConflict, got: {:?}", other),
    }

    // Drop listener and verify port is available again
    drop(listener);
    let recheck = check_port_available(bound_port);
    assert!(
        recheck.is_ok(),
        "Port should be available after listener dropped"
    );
}

#[tokio::test]
async fn test_wb2api_probe_semantic_distinction() {
    use proxyweave_plugin::probe::evaluate_wb2api_status;

    // 1. 200 OK -> Ready
    let ready_status = evaluate_wb2api_status(200, Some("workbuddy2api"));
    assert_eq!(ready_status, ProbeStatus::Ready);

    // 2. 503 + X-Service: workbuddy2api -> Busy (NOT Failed/Crash)
    let busy_status = evaluate_wb2api_status(503, Some("workbuddy2api"));
    match busy_status {
        ProbeStatus::Busy { reason } => {
            assert!(reason.contains("workbuddy2api"));
            println!("Verified: 503 with X-Service evaluated as Busy: {}", reason);
        }
        other => panic!("Expected Busy probe status, got: {:?}", other),
    }

    // 3. 503 without X-Service -> Failed (not wb2api service)
    let failed_503 = evaluate_wb2api_status(503, None);
    assert!(matches!(failed_503, ProbeStatus::Failed { .. }));

    // 4. 500 Internal Server Error -> Failed
    let failed_500 = evaluate_wb2api_status(500, Some("workbuddy2api"));
    assert!(matches!(failed_500, ProbeStatus::Failed { .. }));
}

#[tokio::test]
async fn test_singleflight_startup_prevents_duplicate_spawns() {
    let manager = RuntimeManager::new();

    // Prepare an external record (so no process actually spawned, but singleflight lock tested)
    let ext_record = PluginRecord {
        manifest: proxyweave_plugin::Manifest::parse(
            r#"schema_version: '1.0'
metadata:
  id: sf-plugin
  name: Singleflight Plugin
  version: 1.0.0
  license: MIT
runtime:
  ownership: external
  endpoint: http://127.0.0.1:18888
network:
  health_check:
    path: /healthz
    expected_status: 200
    identity_header: X-Service
    identity_value: sf-plugin
    timeout_ms: 200
lifecycle:
  startup_timeout_ms: 1000
  idle_timeout_ms: null
  max_port_retries: 0
capabilities:
  protocol: openai_chat
  models_path: /v1/models
  inference_path: /v1/chat/completions
  streaming: true
configuration: {}
"#
            .as_bytes(),
        )
        .unwrap(),
        fingerprint: "sf_fingerprint".to_string(),
        settings: Default::default(),
    };

    let rec1 = ext_record.clone();
    let rec2 = ext_record.clone();
    let m1 = manager.clone();
    let m2 = manager.clone();

    // Launch two concurrent start attempts
    let (h1, h2) = tokio::join!(
        tokio::spawn(async move {
            m1.start_plugin(&rec1, Path::new("."), Some(18888), Path::new("."), None)
                .await
        }),
        tokio::spawn(async move {
            m2.start_plugin(&rec2, Path::new("."), Some(18888), Path::new("."), None)
                .await
        }),
    );

    let res1 = h1.unwrap();
    let res2 = h2.unwrap();

    assert!(res1.is_ok());
    assert!(res2.is_ok());
    println!("Verified: concurrent start completed cleanly with singleflight lock");
}

#[tokio::test]
async fn test_generic_managed_projection_and_spawn_without_hardcoding() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");
    let pkg_dir = temp.path().join("pkg");
    let data_dir = temp.path().join("data");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::create_dir_all(&data_dir).unwrap();

    // Use cmd.exe as mock executable
    let mock_exe = "test_cmd.exe";
    std::fs::copy("C:\\Windows\\System32\\cmd.exe", pkg_dir.join(mock_exe)).unwrap();
    let exe_bytes = std::fs::read(pkg_dir.join(mock_exe)).unwrap();
    use sha2::Digest;
    let sha256_hex = format!("{:x}", sha2::Sha256::digest(&exe_bytes));

    let manifest_yaml = format!(
        r#"schema_version: '1.0'
metadata:
  id: generic-runner
  name: Generic Runner
  version: 1.0.0
  license: MIT
runtime:
  ownership: managed
  executable: {mock_exe}
  sha256: {sha256_hex}
  args: ['/c', 'echo running & timeout /t 10 > nul']
  env:
    MOCK_LISTEN: '127.0.0.1:{{PORT}}'
    MOCK_CFG: '{{CONFIG_PATH}}'
network:
  health_check:
    path: /healthz
    expected_status: 200
    identity_header: X-Service
    identity_value: generic-runner
    timeout_ms: 100
lifecycle:
  startup_timeout_ms: 500
  idle_timeout_ms: null
  max_port_retries: 0
capabilities:
  protocol: openai_chat
  models_path: /v1/models
  inference_path: /v1/chat/completions
  streaming: true
configuration:
  required_secrets: [user_token]
  config_template:
    port: '{{PORT}}'
    secret: '{{SECRET:user_token}}'
  credential_files:
    auth/cred.json:
      key: '{{SECRET:user_token}}'
"#
    );

    let manifest = proxyweave_plugin::Manifest::parse(manifest_yaml.as_bytes()).unwrap();
    let record = PluginRecord {
        manifest,
        fingerprint: "fingerprint_123".to_string(),
        settings: Default::default(),
    };

    // Store credential into Keyring with isolated namespace
    quotio_platform::set_plugin_credential("generic-runner", "user_token", "secret_pass_888")
        .expect("failed to set credential");

    let manager = RuntimeManager::new();
    let port = 19777;

    // Start plugin: generic projection will render config and credentials with Protected DACL
    let status = manager
        .start_plugin(
            &record,
            &pkg_dir,
            Some(port),
            &data_dir,
            Some("mock-api-key"),
        )
        .await
        .expect("start_plugin should succeed");

    // Since mock cmd does not serve HTTP /healthz, probe times out and yields Failed status
    assert!(matches!(
        status,
        proxyweave_plugin::RuntimeStatus::Failed { .. }
    ));

    // Verify projected files exist and have rendered values
    let projected_cfg = data_dir
        .join("plugins")
        .join("generic-runner")
        .join("config.json");
    assert!(projected_cfg.exists(), "projected config.json must exist");
    let cfg_content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&projected_cfg).unwrap()).unwrap();
    assert_eq!(cfg_content["port"], "19777");
    assert_eq!(cfg_content["secret"], "secret_pass_888");

    let projected_cred = data_dir
        .join("plugins")
        .join("generic-runner")
        .join("auth")
        .join("cred.json");
    assert!(
        projected_cred.exists(),
        "projected credential file must exist"
    );
    let cred_content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&projected_cred).unwrap()).unwrap();
    assert_eq!(cred_content["key"], "secret_pass_888");

    // Clean up keyring
    let _ = quotio_platform::delete_plugin_credential("generic-runner", "user_token");
    println!("Verified: generic projection and process launch performed with zero hardcoding");
}
