// Preserve upstream window actions/dialogs mechanically; replace business body with Phase 0 status.
const fs = require('fs');
const path = require('path');
const root = 'D:/ProxyWeave/apps/desktop/src';
const source = fs.readFileSync('D:/temp-audit/quotio-desktop/apps/desktop/src/components/AppShell.tsx', 'utf8').split(/\r?\n/);
const slice = (first, last) => source.slice(first - 1, last).join('\n');
const header = `// Window behavior extracted from Quotio AppShell.tsx; see docs/verification/PHASE0.md.
import { useState } from "react";
import "../App.css";
import "./shell.css";
const t = (_key: string, fallback: string) => fallback;
${slice(115,119)}
export function AppShell() {
${slice(182,186)}
`;
// Source sentinels bind extraction to the actual pinned file.
const full = source.join('\n');
const closeDialogs = full.slice(full.indexOf('      {closeDialog ? ('), full.indexOf('      {props.isRefreshing'));
const actions = full.slice(full.indexOf('  // Quitting runs'),full.indexOf('  return (', full.indexOf('  // Quitting runs'))).replaceAll('quotio.closeAction','proxyweave.closeAction').replaceAll('quotio.minimizeAction','proxyweave.minimizeAction');
const titlebar = full.slice(full.indexOf('        <div className="sidebar-titlebar">'), full.indexOf('        <div className="rose-sidebar-profile"'));
const output = header + actions + `  return (<main className="app-shell app-shell--v2"><aside className="sidebar">${titlebar}<h1>ProxyWeave</h1><p>Local API plugin host</p></aside><section className="content"><h2>Phase 0 — Desktop baseline</h2><p>Window, tray and single-instance shell are available.</p><p>Gateway, plugins, protocol conversion and IDE handoff are not enabled.</p><p>Quotio v0.7.7 · MIT · Copyright (c) 2025 Trong Nguyen</p></section>${closeDialogs.replaceAll('Quotio','ProxyWeave')} {closing ? <p role="status">Closing…</p> : null}</main>);\n}\n`;
fs.writeFileSync(path.join(root,'components/AppShell.tsx'),output);
fs.writeFileSync(path.join(root,'App.tsx'),'import { AppShell } from "./components/AppShell";\nexport default function App() { return <AppShell />; }\n');
fs.writeFileSync(path.join(root,'lib/tauri.ts'),'// No browser mock and no provider access in Phase 0.\nexport { invoke } from "@tauri-apps/api/core";\n');
fs.writeFileSync(path.join(root,'MenuBarPanel.tsx'),`import "./App.css";
import "./menubar.css";
import { invoke } from "./lib/tauri";
export default function MenuBarPanel() {
  return <main className="floating-window"><h2>ProxyWeave</h2><p>Desktop baseline · no providers running</p><button onClick={() => void invoke("show_main_window")}>Open ProxyWeave</button><button onClick={() => void invoke("quit_app")}>Quit ProxyWeave</button></main>;
}
`);
// Exhaustive active source closure: remove disconnected business screens/tests after native check.
const keep = new Set(['App.tsx','main.tsx','MenuBarPanel.tsx','App.css','menubar.css','vite-env.d.ts','components/AppShell.tsx','components/shell.css','lib/tauri.ts']);
function prune(dir) {for(const entry of fs.readdirSync(dir,{withFileTypes:true})){const full=path.join(dir,entry.name);if(entry.isDirectory())prune(full);else {const rel=path.relative(root,full).replaceAll('\\','/');if(!keep.has(rel))fs.unlinkSync(full);}}}
prune(root);
console.log('Extracted window controls, close/minimize preferences and dialogs; removed disconnected business frontend.');
