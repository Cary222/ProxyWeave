// Extracted from Quotio v0.7.7; see THIRD_PARTY_NOTICES.md and phase0-introduction.json.
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    utils::config::WindowEffectsConfig,
    window::Effect,
    AppHandle, Manager,
};

pub mod agent_import;
pub mod flight;
pub mod host;
pub mod yaml_lite;
use host::DesktopState;

#[tauri::command]
async fn quit_app(app: AppHandle, state: tauri::State<'_, DesktopState>) -> Result<(), String> {
    host::graceful_exit(&state, &app).await;
    Ok(())
}

#[tauri::command]
fn get_platform_info() -> quotio_types::PlatformInfo {
    quotio_platform::platform_info()
}

#[tauri::command]
fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    if let Some(panel) = app.get_webview_window("menubar") {
        let _ = panel.hide();
    }
    Ok(())
}

#[tauri::command]
fn show_menubar(app: AppHandle) {
    if let Some(panel) = app.get_webview_window("menubar") {
        position_menubar(&panel, None);
    }
}

/// Position the menu-bar panel near the tray icon (like macOS NSMenu), then
/// show + focus it.  Falls back to top-right if no tray rect is available.
fn position_menubar(panel: &tauri::WebviewWindow, tray_rect: Option<tauri::Rect>) {
    let panel_w = 360.0_f64;
    if let Some(rect) = tray_rect {
        if let Ok(Some(monitor)) = panel.primary_monitor() {
            let scale = monitor.scale_factor();
            let screen_h = monitor.size().height as f64 / scale;
            let screen_w = monitor.size().width as f64 / scale;
            let panel_h = panel
                .outer_size()
                .map(|s| s.height as f64 / scale)
                .unwrap_or(500.0);
            let icon_pos = rect.position.to_logical::<f64>(scale);
            let icon_size = rect.size.to_logical::<f64>(scale);
            let icon_cx = icon_pos.x + icon_size.width / 2.0;
            let icon_y = icon_pos.y;
            // Center horizontally on icon, clamp to screen
            let x = (icon_cx - panel_w / 2.0).clamp(8.0, screen_w - panel_w - 8.0);
            // Taskbar at bottom → place panel above; taskbar at top → place below
            let y = if icon_y > screen_h / 2.0 {
                (icon_y - panel_h - 8.0).max(8.0)
            } else {
                icon_y + icon_size.height + 8.0
            };
            let _ = panel.set_position(tauri::LogicalPosition::new(x, y));
        }
    } else if let Ok(Some(monitor)) = panel.primary_monitor() {
        let scale = monitor.scale_factor();
        let screen_w = monitor.size().width as f64 / scale;
        let x = (screen_w - panel_w - 12.0).max(0.0);
        let _ = panel.set_position(tauri::LogicalPosition::new(x, 20.0));
    }
    let _ = panel.show();
    let _ = panel.set_focus();
}

/// Show/hide the shell panel, anchored near the tray icon.
fn toggle_menubar(app: &AppHandle, tray_rect: Option<tauri::Rect>) {
    let Some(panel) = app.get_webview_window("menubar") else {
        return;
    };
    if panel.is_visible().unwrap_or(false) {
        let _ = panel.hide();
    } else {
        // The blur auto-hide may have just fired; re-show is fine — position_menubar
        // calls show() + set_focus() which will bring the panel back.
        position_menubar(&panel, tray_rect);
    }
}

pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();
    // Single-instance must be registered first: a second launch is forwarded to
    // the running instance (which we focus / restore from the tray) and then
    // exits, so the app never opens twice.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }
    builder
        .setup(|app| {
            let profile = std::env::var_os("PROXYWEAVE_PROFILE")
                .map(std::path::PathBuf::from)
                .map(Ok)
                .unwrap_or_else(|| app.path().app_data_dir())?;
            let registry = proxyweave_plugin::Registry::open(profile.join("plugins"))
                .map_err(std::io::Error::other)?;
            // Fail closed on corrupt registration without touching external processes.
            registry.list().map_err(std::io::Error::other)?;
            let desktop_state =
                DesktopState::new(profile, registry).map_err(std::io::Error::other)?;
            app.manage(desktop_state);
            let show = MenuItem::with_id(app, "show", "Open ProxyWeave", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let mut tray_builder = TrayIconBuilder::with_id("main-tray")
                .tooltip("ProxyWeave")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let _ = show_main_window(app.clone());
                    }
                    "quit" => {
                        let app_clone = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_clone.state::<DesktopState>();
                            host::graceful_exit(&state, &app_clone).await;
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        toggle_menubar(tray.app_handle(), Some(rect));
                    }
                });
            // Tauri's tray shows nothing without an explicit icon. Reuse the
            // app's embedded window icon so the tray is visible on Windows.
            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            }
            tray_builder.build(app)?;

            // Keep upstream Mica and persistent tray-panel behavior.
            if let Some(panel) = app.get_webview_window("menubar") {
                let _ = panel.set_effects(WindowEffectsConfig {
                    effects: vec![Effect::Mica],
                    ..Default::default()
                });
            }

            // Phase 0 has no background provider or quota work.

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            quit_app,
            show_menubar,
            get_platform_info,
            host::import_local_plugin,
            host::import_project_from_directory,
            host::import_project_from_github,
            host::list_local_plugins,
            host::save_plugin_settings,
            host::get_plugin_runtime_status,
            host::get_plugin_management,
            host::open_plugin_management,
            host::start_plugin,
            host::stop_plugin,
            host::restart_plugin,
            host::get_plugin_secrets_status,
            host::get_plugin_api_key,
            host::save_plugin_secret,
            host::delete_plugin_secret,
            host::get_gateway_status,
            host::get_gateway_master_key,
            host::regenerate_gateway_master_key,
            host::start_gateway,
            host::stop_gateway,
            host::set_gateway_port,
            host::select_plugin,
            host::get_host_events,
            host::test_gateway_models,
            host::flight_list_models,
            host::flight_chat_stream,
            host::flight_chat_nonstream,
            host::flight_cancel,
            host::detect_agent_clients,
            host::import_agent_client,
            host::restore_agent_client,
            host::list_agent_backups
        ])
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::Destroyed => {
                let app_handle = window.app_handle().clone();
                let label = window.label().to_string();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<DesktopState>();
                    state.flight().cancel_all_for_window(&label).await;
                });
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_, _| {});
}
