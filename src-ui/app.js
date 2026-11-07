// ProxyWeave Workbench Frontend Controller (spec v6)
// Zero-DB Invariant: All data remains in memory variables, strictly no IndexedDB/LocalStorage DBs.

class WorkbenchApp {
  constructor() {
    this.state = {
      plugins: [],
      routes: null,
      probeResults: {},
      isTauri: typeof window.__TAURI__ !== "undefined",
    };
  }

  async invoke(command, args = {}) {
    if (this.state.isTauri && window.__TAURI__.core && window.__TAURI__.core.invoke) {
      return await window.__TAURI__.core.invoke(command, args);
    }
    // Standalone / headless mock fallback for browser inspection
    if (command === "list_plugins" || command === "refresh_plugins") {
      return [
        { manifest: { metadata: { id: "wb2api", name: "WorkBuddy 2API Panel", version: "1.13.0" }, runtime: { ownership: "managed" } } },
        { manifest: { metadata: { id: "external-cpa", name: "External CPA Provider", version: "1.0.0" }, runtime: { ownership: "external", endpoint: "http://127.0.0.1:28317" } } },
      ];
    } else if (command === "probe_plugin") {
      return { id: args.id, status: "ready", healthy: true, latency_ms: Math.floor(Math.random() * 5) + 1, error: null };
    } else if (command === "get_route_status") {
      return { is_running: true, port: 28317, listen_address: "127.0.0.1:28317", active_backend: "wb2api", route_strategy: "SingleBackend", master_key_preview: "pk-pw-••••••••", active_requests: 0, total_routed: 42 };
    } else if (command === "get_plugin_secrets_status") {
      return [{ name: "API_KEY", is_set: true, masked_value: "sk-••••••" }];
    } else if (command === "save_plugin_secret" || command === "delete_plugin_secret") {
      return null;
    }
    return null;
  }

  async refreshPlugins() {
    console.log("[Workbench] Refreshing plugin directory (hot discovery)...");
    try {
      const plugins = await this.invoke("refresh_plugins");
      if (plugins) {
        this.state.plugins = plugins;
      }
      await this.refreshAll();
    } catch (err) {
      console.error("[Workbench] refresh_plugins failed:", err);
    }
  }

  async refreshAll() {
    console.log("[Workbench] Refreshing plugins and route state...");
    try {
      const [plugins, routeStatus] = await Promise.all([
        this.invoke("list_plugins"),
        this.invoke("get_route_status"),
      ]);

      this.state.plugins = plugins || [];
      this.state.routes = routeStatus;

      // Update UI elements
      document.getElementById("kpi-total-plugins").textContent = this.state.plugins.length;
      if (routeStatus) {
        document.getElementById("kpi-gw-port").textContent = routeStatus.port;
        document.getElementById("kpi-gw-status").textContent = routeStatus.is_running ? "● 活跃就绪 (" + (routeStatus.listen_address || "127.0.0.1") + ")" : "○ 已停止";
        if (document.getElementById("kpi-gw-active")) {
          document.getElementById("kpi-gw-active").textContent = routeStatus.active_requests ?? 0;
        }
        if (document.getElementById("kpi-gw-total")) {
          document.getElementById("kpi-gw-total").textContent = routeStatus.total_routed ?? 0;
        }
      }

      // Probe all plugins
      for (const p of this.state.plugins) {
        const id = p.manifest?.metadata?.id;
        if (id) {
          await this.probePlugin(id);
          await this.loadCredentialStatus(id);
        }
      }
    } catch (err) {
      console.error("[Workbench] Refresh failed:", err);
    }
  }

  async probePlugin(id) {
    try {
      const result = await this.invoke("probe_plugin", { id });
      this.state.probeResults[id] = result;
      const latencyEl = document.getElementById(`probe-latency-${id}`);
      const statusEl = document.getElementById(`probe-status-${id}`);
      if (latencyEl && result.latency_ms !== undefined) {
        latencyEl.textContent = `${result.latency_ms} ms`;
      }
      if (statusEl) {
        if (result.healthy) {
          statusEl.textContent = "● READY";
          statusEl.className = "pill pill-green";
        } else {
          statusEl.textContent = `● ${result.status.toUpperCase()}`;
          statusEl.className = "pill pill-red";
        }
      }
    } catch (err) {
      console.warn(`[Workbench] Probe failed for ${id}:`, err);
      const statusEl = document.getElementById(`probe-status-${id}`);
      if (statusEl) {
        statusEl.textContent = "● UNREACHABLE";
        statusEl.className = "pill pill-red";
      }
    }
  }


  openCredentialModal(pluginId = "wb2api") {
    const modal = document.getElementById("credential-modal");
    if (modal) {
      document.getElementById("credential-plugin-id").value = pluginId;
      modal.style.display = "flex";
      this.loadCredentialStatus(pluginId);
    }
  }

  closeCredentialModal() {
    const modal = document.getElementById("credential-modal");
    if (modal) {
      modal.style.display = "none";
      const feedback = document.getElementById("credential-feedback");
      if (feedback) feedback.textContent = "";
      const valInput = document.getElementById("credential-value-input");
      if (valInput) valInput.value = "";
    }
  }

  async loadCredentialStatus(pluginId) {
    try {
      const secrets = await this.invoke("get_plugin_secrets_status", { id: pluginId });
      const previewEl = document.getElementById("credential-masked-preview");
      const cardStatusEl = document.getElementById(`cred-status-${pluginId}`);
      if (secrets && secrets.length > 0) {
        const s = secrets[0];
        document.getElementById("credential-name-input").value = s.name;
        if (s.is_set) {
          if (previewEl) previewEl.textContent = `● 已配置 (${s.masked_value || "sk-••••••"})`;
          if (cardStatusEl) {
            cardStatusEl.textContent = `● 已安全配置 (${s.masked_value || "sk-••••••"})`;
            cardStatusEl.className = "val pill pill-green";
          }
        } else {
          if (previewEl) previewEl.textContent = "○ 未配置 (启动将阻断)";
          if (cardStatusEl) {
            cardStatusEl.textContent = "⚠️ 未配置 (启动阻断)";
            cardStatusEl.className = "val pill pill-amber";
          }
        }
      }
    } catch (e) {
      console.warn("[Workbench] loadCredentialStatus failed:", e);
    }
  }

  async saveCredential() {
    const pluginId = document.getElementById("credential-plugin-id").value;
    const name = document.getElementById("credential-name-input").value;
    const value = document.getElementById("credential-value-input").value;
    const feedback = document.getElementById("credential-feedback");

    if (!value || value.trim().length === 0) {
      if (feedback) {
        feedback.textContent = "⚠️ 凭据值不能为空";
        feedback.style.color = "var(--accent-amber)";
      }
      return;
    }

    try {
      await this.invoke("save_plugin_secret", { id: pluginId, name, value });
      if (feedback) {
        feedback.textContent = "✓ 凭据已安全存入 OS Keyring";
        feedback.style.color = "var(--accent-green)";
      }
      document.getElementById("credential-value-input").value = "";
      await this.loadCredentialStatus(pluginId);
    } catch (err) {
      if (feedback) {
        feedback.textContent = `❌ 保存失败: ${err}`;
        feedback.style.color = "var(--accent-red)";
      }
    }
  }

  async deleteCredential() {
    const pluginId = document.getElementById("credential-plugin-id").value;
    const name = document.getElementById("credential-name-input").value;
    const feedback = document.getElementById("credential-feedback");

    try {
      await this.invoke("delete_plugin_secret", { id: pluginId, name });
      if (feedback) {
        feedback.textContent = "✓ 凭据已从 OS Keyring 清除";
        feedback.style.color = "var(--accent-green)";
      }
      await this.loadCredentialStatus(pluginId);
    } catch (err) {
      if (feedback) {
        feedback.textContent = `❌ 删除失败: ${err}`;
        feedback.style.color = "var(--accent-red)";
      }
    }
  }

  async startPlugin(id) {
    console.log(`[Workbench] Start requested for Managed plugin ${id}`);
    await this.invoke("start_plugin", { id });
    await this.probePlugin(id);
          await this.loadCredentialStatus(id);
  }

  async stopPlugin(id) {
    console.log(`[Workbench] Stop requested for Managed plugin ${id}`);
    await this.invoke("stop_plugin", { id });
    await this.probePlugin(id);
          await this.loadCredentialStatus(id);
  }
}

window.workbenchApp = new WorkbenchApp();
window.addEventListener("DOMContentLoaded", () => {
  window.workbenchApp.refreshAll();
});
