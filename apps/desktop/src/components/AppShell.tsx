import { useState, useEffect, useCallback } from "react";
import { invoke } from "../lib/tauri";
import {
  PluginRecord,
  PluginStatusInfo,
  SecretStatusInfo,
  GatewayStatusInfo,
  HostEvent,
} from "../types";
import { GatewayPanel } from "./GatewayPanel";
import { PluginPanel } from "./PluginPanel";
import { EventsPanel } from "./EventsPanel";
import "../App.css";
import "./shell.css";
import "./dashboard.css";

const t = (_key: string, fallback: string) => fallback;

async function toggleMaximize() {
  if (!("__TAURI_INTERNALS__" in window)) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().toggleMaximize();
}

type TabType = "gateway" | "plugins" | "events" | "about";

export function AppShell() {
  const [activeTab, setActiveTab] = useState<TabType>("gateway");
  const [closeDialog, setCloseDialog] = useState(false);
  const [rememberClose, setRememberClose] = useState(false);
  const [minimizeDialog, setMinimizeDialog] = useState(false);
  const [rememberMinimize, setRememberMinimize] = useState(false);
  const [closing, setClosing] = useState(false);

  // Core application data state
  const [gatewayStatus, setGatewayStatus] = useState<GatewayStatusInfo | null>(null);
  const [plugins, setPlugins] = useState<PluginRecord[]>([]);
  const [pluginStatuses, setPluginStatuses] = useState<Record<string, PluginStatusInfo>>({});
  const [secretsStatuses, setSecretsStatuses] = useState<Record<string, SecretStatusInfo[]>>({});
  const [events, setEvents] = useState<HostEvent[]>([]);

  const refreshAll = useCallback(async () => {
    try {
      // 1. Gateway status
      const gw: GatewayStatusInfo = await invoke("get_gateway_status");
      setGatewayStatus(gw);

      // 2. Plugins list
      const pl: PluginRecord[] = await invoke("list_local_plugins");
      setPlugins(pl);

      // 3. Status and secrets per plugin
      const statuses: Record<string, PluginStatusInfo> = {};
      const secrets: Record<string, SecretStatusInfo[]> = {};

      for (const p of pl) {
        const id = p.manifest.metadata.id;
        try {
          const st: PluginStatusInfo = await invoke("get_plugin_runtime_status", { id });
          statuses[id] = st;
        } catch {
          // ignore individual fetch errors
        }

        try {
          const sec: SecretStatusInfo[] = await invoke("get_plugin_secrets_status", { id });
          secrets[id] = sec;
        } catch {
          // ignore individual fetch errors
        }
      }

      setPluginStatuses(statuses);
      setSecretsStatuses(secrets);

      // 4. Host events
      const ev: HostEvent[] = await invoke("get_host_events");
      setEvents(ev);
    } catch (e) {
      console.warn("Polling error:", e);
    }
  }, []);

  useEffect(() => {
    refreshAll();
    const timer = setInterval(refreshAll, 3000);
    return () => clearInterval(timer);
  }, [refreshAll]);

  function doQuit() {
    setClosing(true);
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        void (async () => {
          if (!("__TAURI_INTERNALS__" in window)) return;
          await invoke("quit_app");
        })();
      }),
    );
  }

  async function requestClose() {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let saved: string | null = null;
    try {
      saved = localStorage.getItem("proxyweave.closeAction");
    } catch {
      /* storage unavailable */
    }
    if (saved === "quit") return doQuit();
    if (saved === "tray") return minimizeToTray();
    setCloseDialog(true);
  }

  function chooseClose(choice: "quit" | "tray") {
    if (rememberClose) {
      try {
        localStorage.setItem("proxyweave.closeAction", choice);
      } catch {
        /* storage unavailable */
      }
    }
    setCloseDialog(false);
    if (choice === "tray") {
      void minimizeToTray();
    } else {
      doQuit();
    }
  }

  async function minimizeToTray() {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().hide();
    void invoke("show_menubar");
  }

  async function minimizeToTaskbar() {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  }

  async function requestMinimize() {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let saved: string | null = null;
    try {
      saved = localStorage.getItem("proxyweave.minimizeAction");
    } catch {
      /* storage unavailable */
    }
    if (saved === "tray") return minimizeToTray();
    if (saved === "taskbar") return minimizeToTaskbar();
    setMinimizeDialog(true);
  }

  function chooseMinimize(choice: "tray" | "taskbar") {
    if (rememberMinimize) {
      try {
        localStorage.setItem("proxyweave.minimizeAction", choice);
      } catch {
        /* storage unavailable */
      }
    }
    setMinimizeDialog(false);
    if (choice === "tray") {
      void minimizeToTray();
    } else {
      void minimizeToTaskbar();
    }
  }

  const isGwRunning = gatewayStatus?.is_running ?? false;
  const runningPluginCount = Object.values(pluginStatuses).filter(
    (s) => s.status === "ready" || s.status === "busy",
  ).length;

  return (
    <main className="app-shell app-shell--v2">
      {/* Left Glass Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-titlebar">
          <div className="window-controls">
            <button
              type="button"
              className="win-dot win-dot--close"
              onClick={() => void requestClose()}
              aria-label="关闭"
              title="关闭"
            >
              <svg className="win-dot-icon" viewBox="0 0 12 12">
                <path
                  d="M3.172 3.172a.5.5 0 0 1 .707 0L6 5.293l2.121-2.121a.5.5 0 1 1 .707.707L6.707 6l2.121 2.121a.5.5 0 0 1-.707.707L6 6.707 3.879 8.828a.5.5 0 1 1-.707-.707L5.293 6 3.172 3.879a.5.5 0 0 1 0-.707Z"
                  fill="currentColor"
                />
              </svg>
            </button>
            <button
              type="button"
              className="win-dot win-dot--min"
              onClick={() => void requestMinimize()}
              aria-label="最小化"
              title="最小化"
            >
              <svg className="win-dot-icon" viewBox="0 0 12 12">
                <rect x="2" y="5.25" width="8" height="1.5" rx=".75" fill="currentColor" />
              </svg>
            </button>
            <button
              type="button"
              className="win-dot win-dot--max"
              onClick={() => void toggleMaximize()}
              aria-label="最大化"
              title="最大化"
            >
              <svg className="win-dot-icon" viewBox="0 0 12 12">
                <path
                  d="M2 4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V4Zm2-.75a.75.75 0 0 0-.75.75v4c0 .414.336.75.75.75h4a.75.75 0 0 0 .75-.75V4a.75.75 0 0 0-.75-.75H4Z"
                  fill="currentColor"
                />
              </svg>
            </button>
          </div>
          <div className="titlebar-drag" data-tauri-drag-region />
        </div>

        <div className="sidebar-brand">
          <h1 className="brand-title">ProxyWeave</h1>
          <p className="brand-subtitle">AI 编织网关宿主</p>
        </div>

        {/* Navigation List */}
        <nav className="nav-list">
          <button
            type="button"
            className={activeTab === "gateway" ? "active" : ""}
            onClick={() => setActiveTab("gateway")}
          >
            <span className="nav-symbol">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="9" />
                <path d="M12 3v18" />
                <path d="M3 12h18" />
              </svg>
            </span>
            <span className="nav-label">统一网关</span>
            <span className={`nav-badge ${isGwRunning ? "badge--online" : "badge--offline"}`}>
              {isGwRunning ? "ONLINE" : "OFF"}
            </span>
          </button>

          <button
            type="button"
            className={activeTab === "plugins" ? "active" : ""}
            onClick={() => setActiveTab("plugins")}
          >
            <span className="nav-symbol">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="3" width="7" height="7" rx="1" />
                <rect x="14" y="3" width="7" height="7" rx="1" />
                <rect x="14" y="14" width="7" height="7" rx="1" />
                <rect x="3" y="14" width="7" height="7" rx="1" />
              </svg>
            </span>
            <span className="nav-label">插件管理</span>
            {plugins.length > 0 ? (
              <span className="nav-badge">
                {runningPluginCount}/{plugins.length}
              </span>
            ) : null}
          </button>

          <button
            type="button"
            className={activeTab === "events" ? "active" : ""}
            onClick={() => setActiveTab("events")}
          >
            <span className="nav-symbol">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
              </svg>
            </span>
            <span className="nav-label">审计轨迹</span>
            {events.length > 0 ? <span className="nav-badge">{events.length}</span> : null}
          </button>

          <button
            type="button"
            className={activeTab === "about" ? "active" : ""}
            onClick={() => setActiveTab("about")}
          >
            <span className="nav-symbol">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="16" x2="12" y2="12" />
                <line x1="12" y1="8" x2="12.01" y2="8" />
              </svg>
            </span>
            <span className="nav-label">系统概览</span>
          </button>
        </nav>

        {/* Sidebar Footer Status Card */}
        <div className="sidebar-footer">
          <div
            className="proxy-card"
            onClick={() => setActiveTab("gateway")}
            title="点击切换到网关面板"
          >
            <div
              className={`proxy-card-icon ${
                isGwRunning ? "proxy-card-icon--good" : "proxy-card-icon--warn"
              }`}
            >
              {isGwRunning ? "⚡" : "⏸"}
            </div>
            <div className="proxy-card-text">
              <strong>{isGwRunning ? "网关运行中" : "网关已停止"}</strong>
              <span>
                {isGwRunning
                  ? `端口 :${gatewayStatus?.port ?? 28319}`
                  : "点击启动服务"}
              </span>
            </div>
            <span className="proxy-card-chevron">›</span>
          </div>
        </div>
      </aside>

      {/* Main Content Area */}
      <section className="content">
        {activeTab === "gateway" ? (
          <GatewayPanel gatewayStatus={gatewayStatus} onRefresh={refreshAll} />
        ) : null}

        {activeTab === "plugins" ? (
          <PluginPanel
            plugins={plugins}
            pluginStatuses={pluginStatuses}
            secretsStatuses={secretsStatuses}
            gatewayStatus={gatewayStatus}
            onRefresh={refreshAll}
          />
        ) : null}

        {activeTab === "events" ? (
          <EventsPanel events={events} onRefresh={refreshAll} />
        ) : null}

        {activeTab === "about" ? (
          <div className="panel-section">
            <div className="section-header">
              <div>
                <h3 className="section-title">系统概览与架构基线 (Overview)</h3>
                <p className="section-desc">
                  ProxyWeave 桌面网关宿主核心架构与运行状态基准。
                </p>
              </div>
            </div>

            <div className="card-box">
              <h4 className="card-box-title">研发三大不可逾越之红线</h4>
              <ul className="rules-list">
                <li>
                  <strong>上游易逝，胶水永存：</strong> 宿主自身不硬编码任何具体平台的逆向逻辑，所有 2API 均作为外部黑盒插件运行。
                </li>
                <li>
                  <strong>Direct-Reuse First：</strong> 非平凡系统机制（进程看护 Job Object、Windows DACL、Keyring、动态端口、Axum/Hyper 透传）直接复用工业级成熟组件，杜绝凭空手写。
                </li>
                <li>
                  <strong>外部服务零伤害：</strong> 对 External 外部服务仅执行透明反向代理挂载与健康探活，绝对禁止发送 Kill 信号。
                </li>
              </ul>
            </div>

            <div className="card-box">
              <h4 className="card-box-title">版本与许可证基线</h4>
              <p>
                ProxyWeave Desktop v0.1.0 · 基于 Quotio v0.7.7 (MIT) 架构重塑与业务解耦 · Axum 0.8.9 / Hyper 1.10.1 · Tokio 1.52
              </p>
            </div>
          </div>
        ) : null}
      </section>

      {/* Modal Dialogs */}
      {closeDialog ? (
        <div className="modal-overlay" onClick={() => setCloseDialog(false)}>
          <div className="close-dialog" onClick={(event) => event.stopPropagation()}>
            <strong className="close-dialog-title">{t("close.title", "关闭 ProxyWeave")}</strong>
            <p className="close-dialog-desc">
              {t("close.desc", "退出程序,还是最小化到托盘继续后台运行?")}
            </p>
            <label className="close-dialog-remember">
              <input
                type="checkbox"
                checked={rememberClose}
                onChange={(event) => setRememberClose(event.target.checked)}
              />
              <span>{t("close.remember", "记住我的选择")}</span>
            </label>
            <div className="close-dialog-actions">
              <button
                type="button"
                className="ghost-action"
                onClick={() => setCloseDialog(false)}
              >
                {t("close.cancel", "取消")}
              </button>
              <button
                type="button"
                className="secondary-action"
                onClick={() => chooseClose("tray")}
              >
                {t("close.tray", "最小化到托盘")}
              </button>
              <button
                type="button"
                className="danger-action"
                onClick={() => chooseClose("quit")}
              >
                {t("close.quit", "退出程序")}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {minimizeDialog ? (
        <div className="modal-overlay" onClick={() => setMinimizeDialog(false)}>
          <div className="close-dialog" onClick={(event) => event.stopPropagation()}>
            <strong className="close-dialog-title">最小化 ProxyWeave</strong>
            <p className="close-dialog-desc">隐藏到托盘并弹出悬浮窗，还是最小化到任务栏？</p>
            <label className="close-dialog-remember">
              <input
                type="checkbox"
                checked={rememberMinimize}
                onChange={(event) => setRememberMinimize(event.target.checked)}
              />
              <span>记住我的选择</span>
            </label>
            <div className="close-dialog-actions">
              <button
                type="button"
                className="ghost-action"
                onClick={() => setMinimizeDialog(false)}
              >
                取消
              </button>
              <button
                type="button"
                className="secondary-action"
                onClick={() => chooseMinimize("taskbar")}
              >
                最小化到任务栏
              </button>
              <button
                type="button"
                className="primary-action"
                onClick={() => chooseMinimize("tray")}
              >
                隐藏到托盘
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {closing ? <p role="status">Closing…</p> : null}
    </main>
  );
}
