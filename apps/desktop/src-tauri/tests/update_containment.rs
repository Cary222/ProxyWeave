//! Source-guard evidence only: no DesktopState construction, IPC invocation or Keyring access.

#[test]
fn update_ipc_signature_and_safety_guards_are_retained() {
    let source = include_str!("../src/host.rs");
    let function = source
        .split_once("pub async fn update_plugin_to_release(")
        .expect("update IPC signature retained")
        .1;
    let (signature, tail) = function
        .split_once(") -> Result<UpdateResultDto, String> {")
        .unwrap();
    for parameter in [
        "app_handle: AppHandle",
        "state: State<'_, DesktopState>",
        "id: String",
        "expected_sha256: Option<String>",
    ] {
        assert!(signature.contains(parameter));
    }
    let body = tail.split_once("\n#[tauri::command]").unwrap().0;
    // Singleflight mutex protection
    assert!(body.contains("state.plugin_operation(&clean_id).await"));
    // File lock protection
    assert!(body.contains("state.registry.acquire_plugin_lock(&clean_id)?"));
    // External rejection without kill
    assert!(body.contains("if record.manifest.ownership() != Ownership::Managed {"));
    assert!(body.contains("return Ok(UpdateResultDto::NotAllowedExternal);"));
    // Active requests pre-check
    // Active requests pre-check
    assert!(body.contains("active_request_count(&clean_id).await"));
    assert!(body.contains("return Ok(UpdateResultDto::BlockedActiveRequests {"));
    // Maintenance barrier drain
    assert!(body.contains("enter_maintenance_barrier(&clean_id"));
    // CAS activation transitions
    assert!(body.contains("envelope.prepare_candidate(candidate_ptr, txn_id)?"));
    assert!(body.contains("cas_advance_activation("));
    assert!(body.contains("envelope.switch_to_candidate()?"));
    assert!(body.contains("envelope.commit_active()?"));
    // Automatic rollback on start failure
    assert!(body.contains("envelope.mark_rollback_pending()?"));
    assert!(body.contains("env.complete_rollback(&container_dir)?"));
    assert!(body.contains("UpdateResultDto::RollbackSuccess"));
    assert!(body.contains("UpdateResultDto::RollbackFailed"));

    assert!(include_str!("../src/lib.rs").contains("host::update_plugin_to_release,"));
}

#[test]
fn readonly_check_and_fresh_install_entrypoints_are_retained() {
    let source = include_str!("../src/host.rs");
    let check = source
        .split_once("pub async fn check_plugin_update(")
        .unwrap()
        .1
        .split_once("\n}")
        .unwrap()
        .0;
    assert!(check.contains("state.registry.load(clean_id)?"));
    assert!(check.contains("updater::check_plugin_update(&record).await"));
    assert!(!check.contains("update_unavailable_pending_safety_validation"));
    assert!(source.contains("downloader::download_and_install_release("));
}

#[test]
fn production_compatibility_rejects_before_any_update_side_effect() {
    let source = include_str!("../src/host.rs");
    let body = source
        .split_once("pub async fn update_plugin_to_release(")
        .unwrap()
        .1
        .split_once("\n#[tauri::command]")
        .unwrap()
        .0;
    let reject = body.find("if !permit.is_compatible()").unwrap();
    assert!(body[..reject].contains("CompatibilityPermit::default_production()"));
    let return_error = body[reject..].find("\"CompatibilityUnknown:").unwrap() + reject;
    for side_effect in [
        "plugin_operation(",
        "acquire_plugin_lock(",
        "download_and_stage_release_package(",
        "store_version_package(",
        "initialize_legacy_activation(",
        "prepare_candidate(",
        "enter_maintenance_barrier(",
    ] {
        assert!(
            return_error < body.find(side_effect).unwrap(),
            "compatibility must precede {side_effect}"
        );
    }
    assert!(body.contains("stop_plugin_under_maintenance(&clean_id, token)"));
    assert!(body.contains("start_plugin_under_maintenance("));
    let release = body.find("exit_maintenance(token)").unwrap();
    assert!(release > body.rfind(".set_backend(").unwrap());
    assert!(release > body.rfind(".cas_advance_activation(").unwrap());
    assert!(body.contains("cancel_maintenance_barrier(token)"));
    assert!(!body.contains(".stop_plugin(&clean_id)"));
}

#[test]
fn ordinary_start_and_restart_use_coherent_active_package_snapshot() {
    let source = include_str!("../src/host.rs");
    for name in ["start_plugin", "restart_plugin"] {
        let marker = format!("pub async fn {name}(");
        let body = source
            .split_once(&marker)
            .unwrap()
            .1
            .split_once("\n}")
            .unwrap()
            .0;
        assert!(body.contains("resolve_active_package(&id)?"));
        assert!(!body.contains("registry.package_dir(&id)"));
    }
}
