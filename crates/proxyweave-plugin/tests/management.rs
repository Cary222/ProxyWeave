use proxyweave_plugin::{
    management::resolve_management, Management, ManagementPresentation, Manifest, Runtime,
    RuntimeStatus,
};
use sha2::{Digest, Sha256};

// Explicit pre-extension canonical JSON: omission of management MUST preserve these bytes/hash.
const LEGACY: &str = r#"{"schema_version":"1.0","metadata":{"id":"external-test","name":"External Test","version":"1.0.0","license":"MIT","source_commit":null},"runtime":{"ownership":"external","endpoint":"https://example.invalid/"},"network":{"health_check":{"path":"/healthz","expected_status":200,"identity_header":"X-Service","identity_value":"external-test","timeout_ms":500}},"lifecycle":{"startup_timeout_ms":2000,"idle_timeout_ms":null,"max_port_retries":0},"capabilities":{"protocol":"openai_chat","models_path":"/v1/models","inference_path":"/v1/chat/completions","streaming":true},"configuration":{"settings":{},"config_template":null,"credential_files":{},"required_secrets":[]}}"#;
fn legacy() -> Manifest {
    Manifest::parse(LEGACY.as_bytes()).unwrap()
}
fn with_entry() -> Manifest {
    let mut m = legacy();
    m.management = Some(Management {
        path: "/ui/index.html".into(),
        presentation: ManagementPresentation::Browser,
    });
    m
}
#[test]
fn legacy_canonical_bytes_and_fingerprint_unchanged() {
    let m = legacy();
    assert!(m.management.is_none());
    assert_eq!(serde_json::to_string(&m).unwrap(), LEGACY);
    assert_eq!(
        format!("{:x}", Sha256::digest(serde_json::to_vec(&m).unwrap())),
        "51db2859e3fed117a482e7a9884cc327ce56081763c979280e1ac9b28b50ee48"
    );
    let explicit_null = LEGACY.replace(
        "\"schema_version\"",
        "\"management\":null,\"schema_version\"",
    );
    assert_eq!(
        serde_json::to_string(&Manifest::parse(explicit_null.as_bytes()).unwrap()).unwrap(),
        LEGACY
    );
}
#[test]
fn old_record_load_duplicate_and_changed_entry_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("old.json");
    std::fs::write(&source, LEGACY).unwrap();
    let registry = proxyweave_plugin::Registry::open(tmp.path().join("registry")).unwrap();
    let result = registry.import(&source).unwrap();
    let before = std::fs::read(
        registry
            .package_dir("external-test")
            .unwrap()
            .join("record.json"),
    )
    .unwrap();
    assert!(!result.already_registered);
    assert!(registry
        .load("external-test")
        .unwrap()
        .manifest
        .management
        .is_none());
    assert!(registry.import(&source).unwrap().already_registered);
    std::fs::write(&source, serde_json::to_vec(&with_entry()).unwrap()).unwrap();
    assert!(registry
        .import(&source)
        .unwrap_err()
        .contains("different package contents"));
    assert!(registry
        .load("external-test")
        .unwrap()
        .manifest
        .management
        .is_none());
    assert_eq!(
        before,
        std::fs::read(
            registry
                .package_dir("external-test")
                .unwrap()
                .join("record.json")
        )
        .unwrap()
    );
}
#[test]
fn resolver_uses_current_port_or_registered_external_origin() {
    let external = with_entry();
    assert_eq!(
        resolve_management(&external, &RuntimeStatus::Stopped)
            .unwrap()
            .url
            .as_deref(),
        Some("https://example.invalid/ui/index.html")
    );
    let mut managed = external;
    managed.runtime = Runtime::Managed {
        executable: "bin/test.exe".into(),
        sha256: "a".repeat(64),
        args: vec![],
        env: Default::default(),
        resources: Default::default(),
    };
    managed.lifecycle.max_port_retries = 3;
    managed.configuration.config_template = serde_json::json!({});
    for port in [31111, 32222] {
        let expected = format!("http://127.0.0.1:{port}/ui/index.html");
        assert_eq!(
            resolve_management(
                &managed,
                &RuntimeStatus::Ready {
                    port,
                    pid: Some(123)
                }
            )
            .unwrap()
            .url
            .as_deref(),
            Some(expected.as_str())
        );
        assert_eq!(
            resolve_management(
                &managed,
                &RuntimeStatus::Busy {
                    port,
                    pid: Some(123),
                    reason: "busy".into()
                }
            )
            .unwrap()
            .url
            .as_deref(),
            Some(expected.as_str())
        );
    }
    for status in [
        RuntimeStatus::Stopped,
        RuntimeStatus::Starting,
        RuntimeStatus::Failed {
            error: "exit".into(),
        },
        RuntimeStatus::Ready { port: 0, pid: None },
    ] {
        let link = resolve_management(&managed, &status).unwrap();
        assert_eq!(link.state, "unavailable");
        assert_eq!(link.url, None);
    }
    assert_eq!(
        resolve_management(&legacy(), &RuntimeStatus::Stopped)
            .unwrap()
            .state,
        "undeclared"
    );
}
#[test]
fn strict_shape_and_origin_rejections() {
    for entry in [
        r#"{"path":"/admin","presentation":"iframe"}"#,
        r#"{"path":"/admin","presentation":"browser","token":"secret"}"#,
        r#"{"path":"/a","path":"/b","presentation":"browser"}"#,
        r#"{"path":"/admin?token=secret","presentation":"browser"}"#,
    ] {
        let json = LEGACY.replace(
            "\"schema_version\"",
            &format!("\"management\":{entry},\"schema_version\""),
        );
        assert!(Manifest::parse(json.as_bytes()).is_err());
    }
    for endpoint in [
        "javascript:alert(1)",
        "file:///admin",
        "data:text/html,x",
        "http://user:secret@example.invalid",
        "http://example.invalid/?token=secret",
        "http://example.invalid/#secret",
        "http://example.invalid/admin",
        "http://tauri.localhost",
        "https://tauri.localhost",
        "http://localhost:1420",
        "http://127.0.0.1:1420",
        "http://[::1]:1420",
    ] {
        let mut m = with_entry();
        m.runtime = Runtime::External {
            endpoint: endpoint.into(),
        };
        assert_eq!(
            resolve_management(&m, &RuntimeStatus::Stopped).unwrap_err(),
            "invalid management entry"
        );
    }
}
