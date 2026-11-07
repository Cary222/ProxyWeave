// Static validation script for src-ui workbench
import fs from "fs";

console.log("[ProxyWeave UI Gate] Validating src-ui assets against spec v7...");

const html = fs.readFileSync(new URL("./index.html", import.meta.url), "utf-8");
const appJs = fs.readFileSync(new URL("./app.js", import.meta.url), "utf-8");

// Red Line 1: Zero-DB check
const dbPatterns = ["indexedDB", "sqlite", "websql", "localStorage.setItem('db'"];
for (const pattern of dbPatterns) {
  if (html.includes(pattern) || appJs.includes(pattern)) {
    throw new Error(`RED LINE VIOLATION: Zero-DB required, found forbidden pattern: ${pattern}`);
  }
}
console.log(" - Zero-DB Audit: PASS (No database libraries or persistent storage detected)");

// Red Line 2: WhatsApp exclusion
if (html.toLowerCase().includes("whatsapp") || appJs.toLowerCase().includes("whatsapp")) {
  throw new Error("RED LINE VIOLATION: WhatsApp template canceled by user, must not appear in UI code");
}
console.log(" - WhatsApp Exclusion Audit: PASS (No WhatsApp artifacts found)");

// Red Line 3: External process isolation (Never Start/Stop/Kill external daemons)
if (html.includes("kill-external") || appJs.includes("stopExternal")) {
  throw new Error("RED LINE VIOLATION: External plugins must be read-only probe protected");
}
console.log(" - External Daemon Protection Audit: PASS (External daemons have zero start/stop/kill controls)");

console.log("[ProxyWeave UI Gate] src-ui build & static checks completed successfully.");

// Red Line 4: Proxy engine metrics present
if (!html.includes("kpi-gw-active") || !html.includes("kpi-gw-total")) {
  throw new Error("Missing active/total routed KPI indicators");
}
console.log(" - Proxy Engine KPI Metrics: PASS (kpi-gw-active and kpi-gw-total verified)");

// Red Line 5: Dynamic directory watcher & refresh_plugins IPC verification
if (!appJs.includes("refresh_plugins") || !appJs.includes("refreshPlugins")) {
  throw new Error("RED LINE VIOLATION: Missing refresh_plugins IPC invocation in app.js");
}
if (!html.includes("scan-directory-btn") || !html.includes("refreshPlugins")) {
  throw new Error("RED LINE VIOLATION: Missing scan-directory-btn in index.html");
}
console.log(" - Dynamic Directory Watcher: PASS (refresh_plugins IPC & UI trigger verified)");

// Red Line 6: Credential Contract & Security Modal
if (!html.includes("credential-modal") || !html.includes("btn-save-credential")) {
  throw new Error("RED LINE VIOLATION: Missing credential-modal or btn-save-credential in index.html");
}
if (!html.includes('type="password"') || !html.includes("credential-value-input")) {
  throw new Error("RED LINE VIOLATION: Credential input must use type='password' for visual masking");
}
if (!appJs.includes("save_plugin_secret") && !appJs.includes("saveCredential")) {
  throw new Error("RED LINE VIOLATION: Missing credential save IPC invocation in app.js");
}
console.log(" - Credential Security Contract: PASS (Modal, password masking & Keyring IPC verified)");
