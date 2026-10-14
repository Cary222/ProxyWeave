use proxyweave_plugin::{manifest::SUPPORTED_BACKEND_PROTOCOLS, Manifest};

fn manifest(protocol: &str) -> Manifest {
    serde_yaml_ng::from_str(&format!(
        "schema_version: '1.0'\nmetadata: {{id: protocol-fixture, name: Fixture, version: 1.0.0, license: MIT}}\nruntime: {{ownership: external, endpoint: 'http://127.0.0.1:19001'}}\nnetwork:\n  health_check: {{path: /health, expected_status: 200, identity_header: X-Fixture, identity_value: owned, timeout_ms: 1000}}\nlifecycle: {{startup_timeout_ms: 1000, idle_timeout_ms: null, max_port_retries: 0}}\ncapabilities: {{protocol: '{protocol}', models_path: /catalog/models, inference_path: /inference, streaming: true}}\n"
    )).unwrap()
}

#[test]
fn canonical_protocols_validate_without_changing_paths_or_fingerprints() {
    for protocol in SUPPORTED_BACKEND_PROTOCOLS {
        let source = manifest(protocol);
        source.validate().unwrap();
        let bytes = serde_yaml_ng::to_string(&source).unwrap();
        assert_eq!(Manifest::parse(bytes.as_bytes()).unwrap(), source);
        assert_eq!(source.capabilities.models_path, "/catalog/models");
        assert_eq!(source.capabilities.inference_path, "/inference");
    }
}

#[test]
fn unknown_case_variants_and_uncontracted_aliases_fail_closed() {
    for protocol in [
        "",
        "gemini",
        "messages",
        "anthropic",
        "responses",
        "chat_completions",
        "OPENAI_CHAT",
        " openai_chat",
    ] {
        let error = manifest(protocol).validate().unwrap_err();
        assert!(error.contains("unsupported backend protocol"));
    }
}
