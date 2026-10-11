// Mechanical extraction from the pinned, immutable upstream. No equivalent reimplementation.
const fs = require('fs');
const path = require('path');
const root = 'D:/ProxyWeave';
const upstream = 'D:/temp-audit/quotio-desktop';
const original = fs.readFileSync(path.join(upstream, 'apps/desktop/src-tauri/src/lib.rs'), 'utf8');
const lines = original.split(/\r?\n/);
const slice = (first, last) => lines.slice(first - 1, last).join('\n');
const windowCommands = slice(635, 651) + '\n\n' + slice(652, 658);
const positioning = slice(1579, 1629).replace('#[cfg_attr(mobile, tauri::mobile_entry_point)]\n', '');
let run = slice(1785, 2008);
run = run.replace(/        \/\/ 自动更新[\s\S]*?\.plugin\(tauri_plugin_process::init\(\)\);\n/, '');
run = run.replace(/        \.plugin\(tauri_plugin_opener::init\(\)\)[\s\S]*?        \.setup/, '        .setup');
run = run.replace(/            if let Ok\(proxy_resource_root\)[\s\S]*?            let show =/, '            let show =');
run = run.replace('打开 Quotio', 'Open ProxyWeave').replace('.tooltip("Quotio")', '.tooltip("ProxyWeave")');
run = run.replace('spawn_usage_collector(app.handle().clone());', '// Phase 0 has no background provider or quota work.');
run = run.replace(/        \.invoke_handler\(tauri::generate_handler!\[[\s\S]*?        \]\)/, '        .invoke_handler(tauri::generate_handler![show_main_window, quit_app, show_menubar, get_platform_info])');
run = run.replace(/        \.run\([\s\S]*$/, '        .run(|_, _| {});\n}\n');
// Native close requests must hide without destroying the reusable main webview.
run = run.replace('        .build(tauri::generate_context!())', `        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())`);
const imports = `// Extracted from Quotio v0.7.7; see THIRD_PARTY_NOTICES.md and phase0-introduction.json.
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    utils::config::WindowEffectsConfig,
    window::Effect,
    AppHandle, Manager,
};

#[tauri::command]
fn get_platform_info() -> quotio_types::PlatformInfo {
    quotio_platform::platform_info()
}

`;
fs.writeFileSync(path.join(root, 'apps/desktop/src-tauri/src/lib.rs'), imports + windowCommands + '\n\n' + positioning + '\n\n' + run);
console.log('Extracted native shell; platform_info is the only new IPC glue; no AppCore state needed.');
