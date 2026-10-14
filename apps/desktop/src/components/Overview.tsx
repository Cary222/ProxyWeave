import { GatewayStatusInfo, HostEvent, PluginRecord, PluginStatusInfo } from "../types";

export function PageHeading({ title, description }: { title: string; description: string }) {
  return (
    <div className="section-header">
      <div>
        <p className="eyebrow">PROXYWEAVE WORKSPACE</p>
        <h3 className="section-title">{title}</h3>
        <p className="section-desc">{description}</p>
      </div>
    </div>
  );
}

export interface ResourceReadState<T> {
  status: "loading" | "confirmed" | "error";
  data: T | null;
  error?: string | null;
  isStale?: boolean;
}

export interface OverviewProps {
  gatewayStatus?: GatewayStatusInfo | null;
  plugins?: PluginRecord[];
  statuses?: Record<string, PluginStatusInfo>;
  pluginStatuses?: Record<string, PluginStatusInfo>;
  events?: HostEvent[];
  dataReady?: boolean;
  gatewayState?: ResourceReadState<GatewayStatusInfo>;
  pluginsState?: ResourceReadState<PluginRecord[]>;
  eventsState?: ResourceReadState<HostEvent[]>;
  onNavigate: (tab: any) => void;
  onSelectPlugin?: (id: string) => void;
}

export function Overview({
  gatewayStatus = null,
  plugins = [],
  statuses,
  pluginStatuses,
  events = [],
  dataReady = true,
  gatewayState,
  pluginsState,
  eventsState,
  onNavigate,
  onSelectPlugin: _onSelectPlugin,
}: OverviewProps) {
  const effectiveStatuses = statuses || pluginStatuses || {};

  const effectiveGatewayState: ResourceReadState<GatewayStatusInfo> = gatewayState ?? {
    status: !dataReady ? "loading" : gatewayStatus ? "confirmed" : "error",
    data: gatewayStatus ?? null,
    isStale: false,
  };

  const effectivePluginsState: ResourceReadState<PluginRecord[]> = pluginsState ?? {
    status: !dataReady ? "loading" : plugins ? "confirmed" : "error",
    data: dataReady ? plugins : null,
    isStale: false,
  };

  const effectiveEventsState: ResourceReadState<HostEvent[]> = eventsState ?? {
    status: !dataReady ? "loading" : events ? "confirmed" : "error",
    data: dataReady ? events : null,
    isStale: false,
  };

  const isGwRunning = effectiveGatewayState.data?.is_running ?? false;
  const gwStateText =
    effectiveGatewayState.status === "loading"
      ? "未知"
      : effectiveGatewayState.status === "error" && effectiveGatewayState.data === null
      ? "未知"
      : isGwRunning
      ? effectiveGatewayState.isStale
        ? "运行中 (过期)"
        : "运行中"
      : effectiveGatewayState.isStale
      ? "已停止 (过期)"
      : "已停止";

  let pillClass = "pill-gray";
  let pillText = "网关空闲";
  if (effectiveGatewayState.status === "loading") {
    pillClass = "pill-amber";
    pillText = "读取中";
  } else if (effectiveGatewayState.status === "error" && effectiveGatewayState.data === null) {
    pillClass = "pill-red";
    pillText = "状态未知";
  } else if (effectiveGatewayState.isStale) {
    pillClass = "pill-amber";
    pillText = isGwRunning ? "网关运行中 (过期)" : "网关空闲 (过期)";
  } else if (isGwRunning) {
    pillClass = "pill-green";
    pillText = "网关运行中";
  } else {
    pillClass = "pill-gray";
    pillText = "网关空闲";
  }

  const pluginsList = effectivePluginsState.data ?? [];
  const managedCount = pluginsList.filter(
    (p) => (p.manifest.runtime?.ownership ?? p.manifest.ownership) === "managed"
  ).length;
  const externalCount = pluginsList.filter(
    (p) => (p.manifest.runtime?.ownership ?? p.manifest.ownership) === "external"
  ).length;
  const activeCount = Object.values(effectiveStatuses).filter(
    (s) => s.status === "ready" || s.status === "busy"
  ).length;

  return (
    <section className="panel" id="panel-overview">
      <div className="topbar">
        <div>
          <h2>系统概览</h2>
        </div>
        <span className={`pill ${pillClass}`}>
          <span className="dot"></span>
          {pillText}
        </span>
      </div>

      {(effectiveGatewayState.isStale ||
        effectivePluginsState.isStale ||
        effectiveEventsState.isStale) && (
        <div
          role="status"
          className="stale-warning"
          style={{
            background: "var(--amber-bg)",
            color: "var(--amber)",
            border: "1px solid var(--accent-line)",
            borderRadius: 8,
            padding: "8px 12px",
            fontSize: 12.5,
            marginBottom: 14,
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <span>数据刷新失败，当前展示最后确认状态（数据可能已过期）</span>
        </div>
      )}

      <div className="grid4">
        <div className="card kpi">
          <label>网关</label>
          <div
            className="big"
            style={{
              color:
                effectiveGatewayState.data === null
                  ? "var(--muted)"
                  : isGwRunning
                  ? "var(--green)"
                  : "var(--amber)",
            }}
          >
            {gwStateText}
          </div>
          <div className="note">
            {effectiveGatewayState.data?.port
              ? `127.0.0.1:${effectiveGatewayState.data.port}${
                  effectiveGatewayState.isStale ? " (刷新失败)" : ""
                }`
              : effectiveGatewayState.status === "error" && effectiveGatewayState.data === null
              ? "读取网关状态失败"
              : "等待宿主状态"}
          </div>
        </div>

        <div className="card kpi">
          <label>监听端口</label>
          <div className="big mono">
            {effectiveGatewayState.data?.port ? `127.0.0.1:${effectiveGatewayState.data.port}` : "—"}
          </div>
          <div className="note">Loopback 仅本地绑定</div>
        </div>

        <div className="card kpi">
          <label>已注册插件</label>
          <div className="big">
            {effectivePluginsState.status === "loading"
              ? "未知"
              : effectivePluginsState.status === "error" && effectivePluginsState.data === null
              ? "未知"
              : effectivePluginsState.isStale
              ? `${pluginsList.length} 个 (过期)`
              : `${pluginsList.length} 个`}
          </div>
          <div className="progress green">
            <i style={{ width: `${Math.min(100, pluginsList.length * 25)}%` }}></i>
          </div>
        </div>

        <div className="card kpi">
          <label>活跃 / 就绪插件</label>
          <div className="big">
            {effectivePluginsState.status === "loading"
              ? "未知"
              : effectivePluginsState.status === "error" && effectivePluginsState.data === null
              ? "未知"
              : `${activeCount} / ${pluginsList.length}`}
          </div>
          <div className="note">
            {effectivePluginsState.data === null
              ? "Managed: — · External: —"
              : `Managed: ${managedCount} · External: ${externalCount}`}
          </div>
        </div>
      </div>

      <div style={{ height: 16 }}></div>

      <div className="grid2">
        <div className="card">
          <div className="section-head">
            <h3>插件状态简报</h3>
            <button className="btn btn-soft btn-sm" type="button" onClick={() => onNavigate("plugins")}>
              管理 →
            </button>
          </div>
          <div className="grid2" style={{ marginTop: 14, gap: 10 }}>
            <div style={{ background: "var(--well)", borderRadius: 12, padding: 14 }}>
              <b style={{ fontSize: 20 }}>{effectivePluginsState.data !== null ? managedCount : "—"}</b>{" "}
              <span className="pill pill-blue">Managed 托管</span>
              <p className="sub" style={{ marginTop: 6 }}>宿主管理生命周期</p>
            </div>
            <div style={{ background: "var(--well)", borderRadius: 12, padding: 14 }}>
              <b style={{ fontSize: 20 }}>{effectivePluginsState.data !== null ? externalCount : "—"}</b>{" "}
              <span className="pill pill-green">External 外部</span>
              <p className="sub" style={{ marginTop: 6 }}>零杀伤 · 独立运行</p>
            </div>
            <div style={{ background: "var(--well)", borderRadius: 12, padding: 14 }}>
              <b style={{ fontSize: 20 }}>{effectivePluginsState.data !== null ? activeCount : "—"}</b>{" "}
              <span className="pill pill-violet">Ready / Busy</span>
              <p className="sub" style={{ marginTop: 6 }}>就绪监听中</p>
            </div>
            <div style={{ background: "var(--well)", borderRadius: 12, padding: 14 }}>
              <b style={{ fontSize: 20 }}>
                {effectiveGatewayState.data !== null
                  ? effectiveGatewayState.data.active_backend ?? "未指定"
                  : "—"}
              </b>{" "}
              <span className="pill pill-amber">当前挂载</span>
              <p className="sub" style={{ marginTop: 6 }}>统一网关后端</p>
            </div>
          </div>
        </div>

        <div className="card">
          <h3>路线图 · Plan.md v3.1.0</h3>
          <div className="road" style={{ marginTop: 14 }}>
            <div className="road-item">
              <div className="road-dot rd-done">✓</div>
              <div>
                <b>Phase -1 · 治理与契约</b> <span className="pill pill-green">Done</span>
              </div>
            </div>
            <div className="road-item">
              <div className="road-dot rd-done">✓</div>
              <div>
                <b>Phase 0 · 基座与双模插件</b> <span className="pill pill-green">Passed</span>
              </div>
            </div>
            <div className="road-item">
              <div className="road-dot rd-now">1</div>
              <div>
                <b>Phase 1 · 核心微内核与宿主闭环</b> <span className="pill pill-blue">进行中 (F1-F3 已验证, F4 交付中)</span>
              </div>
            </div>
            <div className="road-item">
              <div className="road-dot rd-todo">2</div>
              <div>
                <b>Phase 2 · 协议编解码与高级流式管道</b> <span className="pill pill-gray">待启动</span>
              </div>
            </div>
            <div className="road-item">
              <div className="road-dot rd-todo">3</div>
              <div>
                <b>Phase 3 · 桌面控制台与插件市场</b> <span className="pill pill-gray">待启动</span>
              </div>
            </div>
            <div className="road-item">
              <div className="road-dot rd-todo">4</div>
              <div>
                <b>Phase 4 · 发布与全平台流水线</b> <span className="pill pill-gray">待启动</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div style={{ height: 16 }}></div>

      <div className="card">
        <div className="section-head">
          <h3>最近事件</h3>
          <button className="btn btn-soft btn-sm" type="button" onClick={() => onNavigate("audit")}>
            全部 →
          </button>
        </div>
        {effectiveEventsState.status === "loading" ? (
          <p className="sub" style={{ margin: "12px 0" }}>
            正在读取宿主审计事件…
          </p>
        ) : effectiveEventsState.status === "error" && effectiveEventsState.data === null ? (
          <p className="sub" style={{ margin: "12px 0", color: "var(--red, #ef4444)" }}>
            审计事件读取失败
          </p>
        ) : (effectiveEventsState.data?.length ?? 0) === 0 ? (
          <p className="sub" style={{ margin: "12px 0" }}>
            暂无审计事件记录
          </p>
        ) : (
          effectiveEventsState.data
            ?.slice()
            .reverse()
            .slice(0, 5)
            .map((ev) => (
              <div key={ev.id} className="listrow">
                <span className={`pill ${ev.success ? "pill-green" : "pill-red"}`}>
                  {ev.success ? "OK" : "ERR"}
                </span>
                <div style={{ flex: 1 }}>
                  <b style={{ fontSize: 14 }}>
                    {ev.target} · {ev.action}
                  </b>
                  <p className="sub">
                    {new Date(ev.timestamp_ms).toLocaleTimeString()} · {ev.summary}
                  </p>
                </div>
              </div>
            ))
        )}
      </div>
    </section>
  );
}
