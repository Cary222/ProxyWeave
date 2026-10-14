import { useState, useEffect, useRef } from "react";
import { Icon } from "./Icon";
import { Overview, ResourceReadState } from "./Overview";
import { GatewayPanel } from "./GatewayPanel";
import { PluginPanel } from "./PluginPanel";
import { DiscoverPanel } from "./DiscoverPanel";
import { FlightPanel } from "./FlightPanel";
import { AuditPanel } from "./AuditPanel";
import { IdePanel } from "./IdePanel";
import { SettingsPanel } from "./SettingsPanel";
import {
  PluginRecord,
  PluginStatusInfo,
  SecretStatusInfo,
  GatewayStatusInfo,
  HostEvent,
} from "../types";
import { api } from "../api";
import "./prototype.css";

export type NavTab =
  | "overview"
  | "gateway"
  | "plugins"
  | "discover"
  | "flight"
  | "audit"
  | "ide"
  | "settings";

export type ThemeMode = "sky" | "glass" | "dark";

export function AppShell() {
  const [currentTab, setCurrentTab] = useState<NavTab>("overview");
  const [theme, setTheme] = useState<ThemeMode>(() => {
    try {
      if (typeof localStorage !== "undefined") {
        const stored = localStorage.getItem("pw-theme");
        if (stored === "sky" || stored === "glass" || stored === "dark") {
          return stored;
        }
      }
    } catch {
      // ignore storage errors
    }
    return "sky";
  });
  const [selectedPluginId, setSelectedPluginId] = useState<string | null>(null);

  // Per-resource read states (DEF-019: loading / confirmed / error + stale preservation)
  const [gatewayState, setGatewayState] = useState<ResourceReadState<GatewayStatusInfo>>({
    status: "loading",
    data: null,
    isStale: false,
  });
  const [pluginsState, setPluginsState] = useState<ResourceReadState<PluginRecord[]>>({
    status: "loading",
    data: null,
    isStale: false,
  });
  const [eventsState, setEventsState] = useState<ResourceReadState<HostEvent[]>>({
    status: "loading",
    data: null,
    isStale: false,
  });

  const [pluginStatuses, setPluginStatuses] = useState<Record<string, PluginStatusInfo>>({});
  const [secretsStatuses, setSecretsStatuses] = useState<Record<string, SecretStatusInfo[]>>({});
  const [dataReady, setDataReady] = useState(false);

  // Native window control & close preference dialog states (DEF-018)
  const [closeDialog, setCloseDialog] = useState(false);
  const [rememberClose, setRememberClose] = useState(false);
  const [closing, setClosing] = useState(false);
  const [windowError, setWindowError] = useState<string | null>(null);

  // Generational & mount tracking for concurrent refresh safety (DEF-019)
  const generationRef = useRef(0);
  const isMountedRef = useRef(true);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  // Sync theme attribute to documentElement and localStorage
  useEffect(() => {
    if (typeof document !== "undefined") {
      document.documentElement.setAttribute("data-theme", theme);
    }
    try {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("pw-theme", theme);
      }
    } catch {
      // ignore storage errors
    }
  }, [theme]);

  // Initial & periodic data fetching with per-resource state and generation guard
  const refreshData = async () => {
    const currentGen = ++generationRef.current;

    const [gwRes, plgRes, evtRes] = await Promise.allSettled([
      api.getGatewayStatus(),
      api.listPlugins(),
      api.getHostEvents(),
    ]);

    if (!isMountedRef.current || currentGen !== generationRef.current) {
      return;
    }

    // 1. Gateway status
    if (gwRes.status === "fulfilled") {
      setGatewayState({
        status: "confirmed",
        data: gwRes.value,
        isStale: false,
        error: null,
      });
    } else {
      const errMsg =
        gwRes.reason instanceof Error ? gwRes.reason.message : String(gwRes.reason);
      setGatewayState((prev) => ({
        status: prev.data !== null ? "confirmed" : "error",
        data: prev.data,
        isStale: prev.data !== null,
        error: errMsg || "读取网关状态失败",
      }));
    }

    // 2. Plugins list
    let confirmedPlugins: PluginRecord[] | null = null;
    if (plgRes.status === "fulfilled") {
      confirmedPlugins = plgRes.value;
      setPluginsState({
        status: "confirmed",
        data: plgRes.value,
        isStale: false,
        error: null,
      });
    } else {
      const errMsg =
        plgRes.reason instanceof Error ? plgRes.reason.message : String(plgRes.reason);
      setPluginsState((prev) => ({
        status: prev.data !== null ? "confirmed" : "error",
        data: prev.data,
        isStale: prev.data !== null,
        error: errMsg || "读取插件列表失败",
      }));
    }

    // 3. Host events
    if (evtRes.status === "fulfilled") {
      setEventsState({
        status: "confirmed",
        data: evtRes.value,
        isStale: false,
        error: null,
      });
    } else {
      const errMsg =
        evtRes.reason instanceof Error ? evtRes.reason.message : String(evtRes.reason);
      setEventsState((prev) => ({
        status: prev.data !== null ? "confirmed" : "error",
        data: prev.data,
        isStale: prev.data !== null,
        error: errMsg || "读取审计事件失败",
      }));
    }

    // 4. Per-plugin runtime & secret statuses (only updated when plugin list read succeeded)
    if (confirmedPlugins !== null) {
      if (confirmedPlugins.length === 0) {
        setPluginStatuses({});
        setSecretsStatuses({});
      } else {
        const statuses: Record<string, PluginStatusInfo> = {};
        const secrets: Record<string, SecretStatusInfo[]> = {};

        await Promise.all(
          confirmedPlugins.map(async (p: PluginRecord) => {
            const id = p.manifest.metadata.id;
            const [stRes, scRes] = await Promise.allSettled([
              api.getPluginRuntimeStatus(id),
              api.getPluginSecretsStatus(id),
            ]);
            if (stRes.status === "fulfilled" && stRes.value) {
              statuses[id] = stRes.value;
            }
            if (scRes.status === "fulfilled" && scRes.value) {
              secrets[id] = scRes.value;
            }
            return null;
          })
        );

        if (isMountedRef.current && currentGen === generationRef.current) {
          setPluginStatuses(statuses);
          setSecretsStatuses(secrets);
        }
      }
    }

    if (isMountedRef.current && currentGen === generationRef.current) {
      setDataReady(true);
    }
  };

  useEffect(() => {
    refreshData();
    const interval = setInterval(refreshData, 3000);
    return () => clearInterval(interval);
  }, []);

  // Window control actions (DEF-018: Phase 0 verified close/minimize/maximize flow)
  async function doQuit() {
    setClosing(true);
    setWindowError(null);
    try {
      await api.quitApp();
    } catch (err) {
      if (isMountedRef.current) {
        setClosing(false);
        setWindowError(err instanceof Error ? err.message : String(err));
      }
    }
  }

  async function minimizeToTray() {
    setWindowError(null);
    try {
      await api.hideWindow();
      await api.showMenubar();
    } catch (err) {
      if (isMountedRef.current) {
        setWindowError(err instanceof Error ? err.message : String(err));
      }
    }
  }

  function requestClose() {
    setWindowError(null);
    let saved: string | null = null;
    try {
      saved = localStorage.getItem("proxyweave.closeAction");
    } catch {
      /* storage unavailable */
    }
    if (saved === "quit") {
      void doQuit();
      return;
    }
    if (saved === "tray") {
      void minimizeToTray();
      return;
    }
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
      void doQuit();
    }
  }

  async function handleMinimize() {
    setWindowError(null);
    try {
      await api.minimizeWindow();
    } catch (err) {
      if (isMountedRef.current) {
        setWindowError(err instanceof Error ? err.message : String(err));
      }
    }
  }

  async function handleToggleMaximize() {
    setWindowError(null);
    try {
      await api.toggleMaximizeWindow();
    } catch (err) {
      if (isMountedRef.current) {
        setWindowError(err instanceof Error ? err.message : String(err));
      }
    }
  }

  const gatewayStatus = gatewayState.data;
  const plugins = pluginsState.data ?? [];
  const events = eventsState.data ?? [];

  const readyPluginsCount = Object.values(pluginStatuses).filter(
    (s) => s.status === "ready"
  ).length;

  return (
    <div className="app">
      {/* 侧边栏 Sidebar */}
      <aside className="sidebar">
        <div className="dots win-dots" data-tauri-drag-region>
          <button
            type="button"
            className="win-dot win-dot--close"
            style={{
              background: "#ff5f57",
              cursor: "pointer",
              width: 12,
              height: 12,
              borderRadius: "50%",
              border: "none",
              padding: 0,
              display: "block",
            }}
            onClick={requestClose}
            title="关闭"
            aria-label="关闭"
          />
          <button
            type="button"
            className="win-dot win-dot--min"
            style={{
              background: "#febc2e",
              cursor: "pointer",
              width: 12,
              height: 12,
              borderRadius: "50%",
              border: "none",
              padding: 0,
              display: "block",
            }}
            onClick={() => void handleMinimize()}
            title="最小化"
            aria-label="最小化"
          />
          <button
            type="button"
            className="win-dot win-dot--max"
            style={{
              background: "#28c840",
              cursor: "pointer",
              width: 12,
              height: 12,
              borderRadius: "50%",
              border: "none",
              padding: 0,
              display: "block",
            }}
            onClick={() => void handleToggleMaximize()}
            title="最大化"
            aria-label="最大化"
          />
        </div>
        {windowError && (
          <div
            role="alert"
            className="window-error"
            style={{
              background: "var(--red-bg, rgba(239, 68, 68, 0.14))",
              color: "var(--red, #ef4444)",
              borderRadius: 6,
              padding: "6px 10px",
              marginBottom: 8,
              fontSize: 12,
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
            }}
          >
            <span>窗口操作失败: {windowError}</span>
            <button
              type="button"
              onClick={() => setWindowError(null)}
              aria-label="忽略窗口错误"
              style={{
                background: "transparent",
                border: "none",
                color: "inherit",
                cursor: "pointer",
                padding: "0 4px",
              }}
            >
              ×
            </button>
          </div>
        )}
        <div className="brand" data-tauri-drag-region>
          <h1>ProxyWeave</h1>
          <p>AI 编织网关宿主 · 统一网关服务</p>
        </div>

        {/* 8 大主导航 */}
        <nav className="nav" id="sideNav">
          <button
            type="button"
            className={currentTab === "overview" ? "active" : ""}
            onClick={() => {
              setCurrentTab("overview");
              setSelectedPluginId(null);
            }}
          >
            <Icon name="grid" className="ic" />
            <span>系统概览</span>
            <small className="gray">Phase 1</small>
          </button>

          <button
            type="button"
            className={currentTab === "gateway" ? "active" : ""}
            onClick={() => {
              setCurrentTab("gateway");
              setSelectedPluginId(null);
            }}
          >
            <Icon name="globe" className="ic" />
            <span>统一网关</span>
            <small className={gatewayStatus?.is_running ? "" : "gray"}>
              {gatewayState.status === "loading"
                ? "读取中"
                : gatewayState.status === "error" && gatewayState.data === null
                ? "未知"
                : gatewayState.isStale
                ? gatewayStatus?.is_running
                  ? "ON (过期)"
                  : "OFF (过期)"
                : gatewayStatus?.is_running
                ? "ON"
                : "OFF"}
            </small>
          </button>

          <button
            type="button"
            className={currentTab === "plugins" ? "active" : ""}
            onClick={() => {
              setCurrentTab("plugins");
              setSelectedPluginId(null);
            }}
          >
            <Icon name="plug" className="ic" />
            <span>插件管理</span>
            <small>
              {pluginsState.status === "loading"
                ? "读取中"
                : pluginsState.status === "error" && pluginsState.data === null
                ? "未知"
                : pluginsState.isStale
                ? `${readyPluginsCount}/${plugins.length} (过期)`
                : `${readyPluginsCount}/${plugins.length}`}
            </small>
          </button>

          <button
            type="button"
            className={currentTab === "discover" ? "active" : ""}
            onClick={() => {
              setCurrentTab("discover");
              setSelectedPluginId(null);
            }}
          >
            <Icon name="compass" className="ic" />
            <span>插件发现</span>
          </button>

          <button
            type="button"
            className={currentTab === "flight" ? "active" : ""}
            onClick={() => {
              setCurrentTab("flight");
              setSelectedPluginId(null);
            }}
          >
            <Icon name="send" className="ic" />
            <span>试飞工作台</span>
          </button>

          <button
            type="button"
            className={currentTab === "audit" ? "active" : ""}
            onClick={() => {
              setCurrentTab("audit");
              setSelectedPluginId(null);
            }}
          >
            <Icon name="pulse" className="ic" />
            <span>审计轨迹</span>
            <small>
              {eventsState.status === "loading"
                ? "读取中"
                : eventsState.status === "error" && eventsState.data === null
                ? "未知"
                : eventsState.isStale
                ? `${events.length} (过期)`
                : events.length}
            </small>
          </button>

          <button
            type="button"
            className={currentTab === "ide" ? "active" : ""}
            onClick={() => {
              setCurrentTab("ide");
              setSelectedPluginId(null);
            }}
          >
            <Icon name="term" className="ic" />
            <span>IDE 接入</span>
          </button>

          <button
            type="button"
            className={currentTab === "settings" ? "active" : ""}
            onClick={() => {
              setCurrentTab("settings");
              setSelectedPluginId(null);
            }}
          >
            <Icon name="sliders" className="ic" />
            <span>系统设置</span>
          </button>
        </nav>

        <div className="side-label">外观主题 THEME</div>
        <div className="theme-switch" role="group" aria-label="外观主题">
          <button
            type="button"
            className={theme === "sky" ? "active" : ""}
            onClick={() => setTheme("sky")}
            title="晴空蓝白 (默认)"
          >
            晴空
          </button>
          <button
            type="button"
            className={theme === "glass" ? "active" : ""}
            onClick={() => setTheme("glass")}
            title="玻璃拟态 (毛玻璃)"
          >
            玻璃
          </button>
          <button
            type="button"
            className={theme === "dark" ? "active" : ""}
            onClick={() => setTheme("dark")}
            title="深空暗色 (极简深色)"
          >
            深空
          </button>
        </div>

        <div
          className="gw-mini"
          id="gwMini"
          role="button"
          tabIndex={0}
          onClick={() => {
            setCurrentTab("gateway");
            setSelectedPluginId(null);
          }}
          title="点击跳转至统一网关设置"
        >
          <div
            className="pause"
            id="gwMiniIcon"
            style={{
              background: gatewayStatus?.is_running ? "var(--green-bg)" : "var(--amber-bg)",
              color: gatewayStatus?.is_running ? "var(--green)" : "var(--amber)",
            }}
          >
            <Icon name={gatewayStatus?.is_running ? "play" : "pause"} className="ic" />
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <b id="gwMiniTitle">
              {gatewayStatus === null
                ? "网关状态未知"
                : gatewayState.isStale
                ? gatewayStatus.is_running
                  ? "网关运行中 (过期)"
                  : "网关已停止 (过期)"
                : gatewayStatus.is_running
                ? "网关运行中"
                : "网关已停止"}
            </b>
            <span id="gwMiniSub">
              {gatewayState.status === "loading"
                ? "正在读取宿主状态"
                : gatewayState.status === "error" && gatewayStatus === null
                ? "读取宿主状态失败"
                : gatewayState.isStale
                ? gatewayStatus?.is_running
                  ? `127.0.0.1:${gatewayStatus.port} (刷新失败)`
                  : "服务停止 (刷新失败)"
                : gatewayStatus?.is_running
                ? `127.0.0.1:${gatewayStatus.port}`
                : "点击启动服务 · 127.0.0.1:8080"}
            </span>
          </div>
          <span style={{ color: "var(--muted)" }}>›</span>
        </div>
        <p style={{ fontSize: 11, color: "var(--muted)", textAlign: "center", margin: "12px 0 0" }}>
          上游易逝 · 平台长存
        </p>
      </aside>

      {/* 主视图内容区域 Main View */}
      <main className="main">
        {currentTab === "overview" && (
          <Overview
            gatewayStatus={gatewayStatus}
            plugins={plugins}
            pluginStatuses={pluginStatuses}
            events={events}
            dataReady={dataReady}
            gatewayState={gatewayState}
            pluginsState={pluginsState}
            eventsState={eventsState}
            onNavigate={(tab) => {
              setCurrentTab(tab);
              setSelectedPluginId(null);
            }}
            onSelectPlugin={(id) => {
              setSelectedPluginId(id);
              setCurrentTab("plugins");
            }}
          />
        )}

        {currentTab === "gateway" && (
          <GatewayPanel
            status={gatewayStatus}
            plugins={plugins}
            onRefresh={refreshData}
          />
        )}

        {currentTab === "plugins" && (
          <PluginPanel
            dataReady={pluginsState.status === "confirmed"}
            plugins={plugins}
            pluginStatuses={pluginStatuses}
            secretsStatuses={secretsStatuses}
            gatewayStatus={gatewayStatus}
            selectedPluginId={selectedPluginId}
            onRefresh={refreshData}
            onSelectPlugin={(id) => setSelectedPluginId(id)}
            onBack={() => setSelectedPluginId(null)}
            onNavigateFlight={() => {
              setCurrentTab("flight");
            }}
          />
        )}

        {currentTab === "discover" && (
          <DiscoverPanel
            onInstallSuccess={() => {
              refreshData();
              setCurrentTab("plugins");
            }}
          />
        )}

        {currentTab === "flight" && (
          <FlightPanel
            plugins={plugins}
            pluginStatuses={pluginStatuses}
            gatewayStatus={gatewayStatus}
          />
        )}

        {currentTab === "audit" && (
          <AuditPanel events={events} onRefresh={refreshData} />
        )}

        {currentTab === "ide" && <IdePanel gatewayStatus={gatewayStatus} onNavigateGateway={() => setCurrentTab("gateway")} />}

        {currentTab === "settings" && (
          <SettingsPanel currentTheme={theme} onThemeChange={setTheme} />
        )}
      </main>

      {/* 关闭确认对话框 (DEF-018: Phase 0 Custom Close Confirmation) */}
      {closeDialog ? (
        <div
          className="modal-overlay"
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(15, 23, 42, 0.45)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 1000,
          }}
          onClick={() => setCloseDialog(false)}
        >
          <div
            className="close-dialog"
            role="dialog"
            aria-modal="true"
            aria-label="关闭 ProxyWeave"
            style={{
              width: 360,
              maxWidth: "90vw",
              display: "flex",
              flexDirection: "column",
              gap: 12,
              padding: 20,
              borderRadius: 14,
              background: "var(--card, #ffffff)",
              border: "1px solid var(--line, #e2e8f0)",
              boxShadow: "0 20px 50px rgba(15, 23, 42, 0.28)",
              color: "var(--ink, #0f172a)",
            }}
            onClick={(event) => event.stopPropagation()}
          >
            <strong className="close-dialog-title" style={{ fontSize: 15, fontWeight: 700 }}>
              关闭 ProxyWeave
            </strong>
            <p className="close-dialog-desc" style={{ fontSize: 13, color: "var(--sub, #64748b)", margin: 0 }}>
              退出程序,还是最小化到托盘继续后台运行?
            </p>
            <label
              className="close-dialog-remember"
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                fontSize: 12.5,
                cursor: "pointer",
                userSelect: "none",
              }}
            >
              <input
                type="checkbox"
                checked={rememberClose}
                onChange={(event) => setRememberClose(event.target.checked)}
              />
              <span>记住我的选择</span>
            </label>
            <div
              className="close-dialog-actions"
              style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 4 }}
            >
              <button
                type="button"
                className="btn btn-soft btn-sm ghost-action"
                onClick={() => setCloseDialog(false)}
              >
                取消
              </button>
              <button
                type="button"
                className="btn btn-soft btn-sm secondary-action"
                onClick={() => chooseClose("tray")}
              >
                最小化到托盘
              </button>
              <button
                type="button"
                className="btn btn-primary btn-sm danger-action"
                style={{ background: "var(--red, #ef4444)", borderColor: "var(--red, #ef4444)", color: "#fff" }}
                onClick={() => chooseClose("quit")}
              >
                退出程序
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {closing ? <p role="status">Closing…</p> : null}
    </div>
  );
}
export default AppShell;
