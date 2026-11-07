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
    if (command === "list_plugins") {
      return [
        { manifest: { metadata: { id: "wb2api", name: "WorkBuddy 2API Panel", version: "1.13.0" }, runtime: { ownership: "managed" } } },
        { manifest: { metadata: { id: "external-cpa", name: "External CPA Provider", version: "1.0.0" }, runtime: { ownership: "external", endpoint: "http://127.0.0.1:28317" } } },
      ];
    } else if (command === "probe_plugin") {
      return { id: args.id, status: "ready", healthy: true, latency_ms: Math.floor(Math.random() * 5) + 1, error: null };
    } else if (command === "get_route_status") {
      return { is_running: true, port: 28317, listen_address: "127.0.0.1:28317", active_backend: "wb2api", route_strategy: "SingleBackend", master_key_preview: "pk-pw-••••••••", active_requests: 0, total_routed: 42 };
    }
    return null;
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

  async startPlugin(id) {
    console.log(`[Workbench] Start requested for Managed plugin ${id}`);
    await this.invoke("start_plugin", { id });
    await this.probePlugin(id);
  }

  async stopPlugin(id) {
    console.log(`[Workbench] Stop requested for Managed plugin ${id}`);
    await this.invoke("stop_plugin", { id });
    await this.probePlugin(id);
  }
}

window.workbenchApp = new WorkbenchApp();
window.addEventListener("DOMContentLoaded", () => {
  window.workbenchApp.refreshAll();
});
