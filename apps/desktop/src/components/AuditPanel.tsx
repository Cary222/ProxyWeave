import React, { useEffect, useMemo, useRef, useState } from "react";
import { HostEvent, RequestTrace } from "../types";
import { invoke } from "../lib/tauri";

interface AuditPanelProps {
  events?: HostEvent[];
  traces?: RequestTrace[];
  onRefresh?: () => void | Promise<void>;
  onClearTraces?: () => void | Promise<void>;
}

interface SanitizedHostEvent {
  key: string;
  id: number | null;
  timestamp_ms: number | null;
  category: string;
  action: string;
  target: string;
  success: boolean | null;
  summary: string;
}

function sanitizeHostEvent(raw: unknown, index: number): SanitizedHostEvent {
  const item = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const id =
    typeof item.id === "number" && Number.isFinite(item.id) ? item.id : null;
  const timestamp_ms =
    typeof item.timestamp_ms === "number" &&
    Number.isFinite(item.timestamp_ms) &&
    item.timestamp_ms > 0
      ? item.timestamp_ms
      : null;
  const category =
    typeof item.category === "string" && item.category.trim().length > 0
      ? item.category.trim()
      : "unknown";
  const action =
    typeof item.action === "string" && item.action.trim().length > 0
      ? item.action.trim()
      : "unknown";
  const target =
    typeof item.target === "string" && item.target.trim().length > 0
      ? item.target.trim()
      : "unknown";
  const success = typeof item.success === "boolean" ? item.success : null;
  const summary =
    typeof item.summary === "string" && item.summary.trim().length > 0
      ? item.summary.trim()
      : "（无摘要）";

  return {
    key: id !== null ? `evt-${id}-${index}` : `evt-idx-${index}`,
    id,
    timestamp_ms,
    category,
    action,
    target,
    success,
    summary,
  };
}

interface SanitizedRequestTrace {
  id: string;
  timestamp_ms: number;
  client: string;
  client_ip: string;
  method: string;
  path: string;
  model: string | null;
  ingress_protocol: string;
  route_strategy: string;
  target_plugin: string | null;
  status_code: number;
  duration_ms: number;
  first_token_ms: number | null;
  input_tokens: number | null;
  output_tokens: number | null;
  cache_hit: boolean;
  fallback_trail: string[];
  error: string | null;
}

function sanitizeRequestTrace(raw: unknown, index: number): SanitizedRequestTrace {
  const item = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  const id =
    typeof item.id === "string" && item.id.trim().length > 0
      ? item.id.trim()
      : `trace-${index}`;
  const timestamp_ms =
    typeof item.timestamp_ms === "number" &&
    Number.isFinite(item.timestamp_ms) &&
    item.timestamp_ms > 0
      ? item.timestamp_ms
      : Date.now();
  const client =
    typeof item.client === "string" && item.client.trim().length > 0
      ? item.client.trim()
      : "Unknown";
  const client_ip = typeof item.client_ip === "string" ? item.client_ip : "";
  const method =
    typeof item.method === "string" && item.method.trim().length > 0
      ? item.method.trim().toUpperCase()
      : "POST";
  const path =
    typeof item.path === "string" && item.path.trim().length > 0
      ? item.path.trim()
      : "/v1/chat/completions";
  const model =
    typeof item.model === "string" && item.model.trim().length > 0
      ? item.model.trim()
      : null;
  const ingress_protocol =
    typeof item.ingress_protocol === "string" ? item.ingress_protocol : "openai";
  const route_strategy =
    typeof item.route_strategy === "string" ? item.route_strategy : "direct";
  const target_plugin =
    typeof item.target_plugin === "string" && item.target_plugin.trim().length > 0
      ? item.target_plugin.trim()
      : null;
  const status_code =
    typeof item.status_code === "number" && Number.isFinite(item.status_code)
      ? item.status_code
      : 200;
  const duration_ms =
    typeof item.duration_ms === "number" && Number.isFinite(item.duration_ms)
      ? item.duration_ms
      : 0;
  const first_token_ms =
    typeof item.first_token_ms === "number" && Number.isFinite(item.first_token_ms)
      ? item.first_token_ms
      : null;
  const input_tokens =
    typeof item.input_tokens === "number" && Number.isFinite(item.input_tokens)
      ? item.input_tokens
      : null;
  const output_tokens =
    typeof item.output_tokens === "number" && Number.isFinite(item.output_tokens)
      ? item.output_tokens
      : null;
  const cache_hit = item.cache_hit === true;
  const fallback_trail = Array.isArray(item.fallback_trail)
    ? item.fallback_trail.map(String)
    : [];
  const error =
    typeof item.error === "string" && item.error.trim().length > 0
      ? item.error.trim()
      : null;

  return {
    id,
    timestamp_ms,
    client,
    client_ip,
    method,
    path,
    model,
    ingress_protocol,
    route_strategy,
    target_plugin,
    status_code,
    duration_ms,
    first_token_ms,
    input_tokens,
    output_tokens,
    cache_hit,
    fallback_trail,
    error,
  };
}

function formatTimestamp(ts: number | null): string {
  if (ts === null) return "时间未知";
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return "时间未知";
  return d.toISOString().replace("T", " ").slice(0, 19);
}

export const AuditPanel: React.FC<AuditPanelProps> = ({
  events,
  traces,
  onRefresh,
  onClearTraces,
}) => {
  const isEventsUnknown = events === undefined || events === null;
  const safeEvents = useMemo(() => {
    if (!Array.isArray(events)) return [];
    return events.map((ev, idx) => sanitizeHostEvent(ev, idx));
  }, [events]);

  const [categoryFilter, setCategoryFilter] = useState<string>("all");
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [exportFeedback, setExportFeedback] = useState<{
    text: string;
    error?: boolean;
  } | null>(null);

  const exportTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (exportTimerRef.current !== null) {
        clearTimeout(exportTimerRef.current);
      }
    };
  }, []);

  const availableCategories = useMemo(() => {
    const set = new Set<string>();
    for (const ev of safeEvents) {
      set.add(ev.category);
    }
    return Array.from(set);
  }, [safeEvents]);

  useEffect(() => {
    if (categoryFilter !== "all" && !availableCategories.includes(categoryFilter)) {
      setCategoryFilter("all");
    }
  }, [availableCategories, categoryFilter]);

  const filteredEvents = useMemo(() => {
    const effectiveCategory =
      categoryFilter !== "all" && availableCategories.includes(categoryFilter)
        ? categoryFilter
        : "all";
    return effectiveCategory === "all"
      ? safeEvents
      : safeEvents.filter((ev) => ev.category === effectiveCategory);
  }, [safeEvents, categoryFilter, availableCategories]);

  const activeEvent = useMemo(() => {
    if (filteredEvents.length === 0) return null;
    if (selectedKey !== null) {
      const found = filteredEvents.find((ev) => ev.key === selectedKey);
      if (found) return found;
    }
    return filteredEvents[0];
  }, [filteredEvents, selectedKey]);

  useEffect(() => {
    setSelectedKey(activeEvent ? activeEvent.key : null);
  }, [activeEvent]);
  const [internalTraces, setInternalTraces] = useState<RequestTrace[]>([]);

  const loadTraces = async () => {
    try {
      const list = await invoke<RequestTrace[]>("get_gateway_traces", { limit: 50 });
      setInternalTraces(list);
    } catch {
      // ignore
    }
  };

  useEffect(() => {
    if (traces === undefined) {
      void loadTraces();
    }
  }, [traces]);

  const safeTraces = useMemo(() => {
    const list = traces !== undefined ? traces : internalTraces;
    if (!Array.isArray(list)) return [];
    return list.map((tr, idx) => sanitizeRequestTrace(tr, idx));
  }, [traces, internalTraces]);

  const [traceClientFilter, setTraceClientFilter] = useState<string>("all");
  const [selectedTraceId, setSelectedTraceId] = useState<string | null>(null);

  const availableTraceClients = useMemo(() => {
    const set = new Set<string>();
    for (const tr of safeTraces) {
      set.add(tr.client);
    }
    return Array.from(set);
  }, [safeTraces]);

  const filteredTraces = useMemo(() => {
    if (traceClientFilter === "all") return safeTraces;
    return safeTraces.filter((tr) => tr.client === traceClientFilter);
  }, [safeTraces, traceClientFilter]);

  const activeTrace = useMemo(() => {
    if (filteredTraces.length === 0) return null;
    if (selectedTraceId !== null) {
      const found = filteredTraces.find((tr) => tr.id === selectedTraceId);
      if (found) return found;
    }
    return filteredTraces[0];
  }, [filteredTraces, selectedTraceId]);

  function renderStatusCodePill(status: number) {
    if (status >= 200 && status < 300) {
      return <span className="pill pill-green">{status} OK</span>;
    }
    if (status >= 400 && status < 500) {
      return <span className="pill pill-amber">{status}</span>;
    }
    return <span className="pill pill-red">{status}</span>;
  }

  function renderClientPill(client: string) {
    const c = client.toLowerCase();
    if (c.includes("cursor")) return <span className="pill pill-blue">Cursor</span>;
    if (c.includes("windsurf")) return <span className="pill pill-teal">Windsurf</span>;
    if (c.includes("claude")) return <span className="pill pill-amber">Claude Code</span>;
    if (c.includes("aider") || c.includes("cline") || c.includes("roo"))
      return <span className="pill pill-violet">{client}</span>;
    return <span className="pill pill-gray">{client}</span>;
  }

  function renderResultPill(success: boolean | null) {
    if (success === true) {
      return <span className="pill pill-green">成功</span>;
    }
    if (success === false) {
      return <span className="pill pill-red">失败</span>;
    }
    return <span className="pill pill-gray">未知</span>;
  }

  function handleExportDiagnostics() {
    if (exportTimerRef.current !== null) {
      clearTimeout(exportTimerRef.current);
      exportTimerRef.current = null;
    }

    try {
      if (
        typeof URL === "undefined" ||
        typeof URL.createObjectURL !== "function" ||
        typeof document === "undefined"
      ) {
        throw new Error("当前环境不支持浏览器 Blob 下载接口");
      }

      const diagnosticPayload = {
        schema: "proxyweave.host-events-diagnostics.v1",
        exported_at: new Date().toISOString(),
        events_state: isEventsUnknown ? "unknown" : "loaded",
        request_tracing_status: "not_implemented",
        redaction_verified: false,
        host_events: safeEvents.map((ev) => ({
          id: ev.id,
          timestamp_ms: ev.timestamp_ms,
          category: ev.category,
          action: ev.action,
          target: ev.target,
          success: ev.success,
          summary: ev.summary,
        })),
      };

      const blob = new Blob([JSON.stringify(diagnosticPayload, null, 2)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `proxyweave-host-events-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);

      setExportFeedback({
        text: "已发起诊断 JSON 下载（仅含当前宿主控制事件白名单字段，不含 HTTP 请求链路，未验证本地落盘）",
      });
      exportTimerRef.current = setTimeout(() => {
        exportTimerRef.current = null;
        setExportFeedback(null);
      }, 3000);
    } catch (e: any) {
      setExportFeedback({
        text: `发起诊断导出失败: ${typeof e === "string" ? e : e?.message || String(e)}`,
        error: true,
      });
    }
  }

  return (
    <section className="panel" id="panel-audit">
      <div className="topbar">
        <div>
          <h2>审计 · API Doctor</h2>
        </div>
        <div className="actions" style={{ marginTop: 0 }}>
          {onRefresh ? (
            <button
              className="btn btn-soft btn-sm"
              id="refreshAuditBtn"
              type="button"
              onClick={async () => {
                if (onRefresh) await onRefresh();
                if (traces === undefined) await loadTraces();
              }}
            >
              刷新事件
            </button>
          ) : null}
          {safeTraces.length > 0 ? (
            <button
              className="btn btn-soft btn-sm"
              id="clearTracesBtn"
              type="button"
              onClick={async () => {
                if (onClearTraces) {
                  await onClearTraces();
                } else {
                  try {
                    await invoke<void>("clear_gateway_traces");
                    setInternalTraces([]);
                  } catch {
                    // ignore
                  }
                }
              }}
              style={{ color: "var(--red)" }}
            >
              清空调用轨迹
            </button>
          ) : null}
          <button
            className="btn btn-ghost"
            id="exportDiagnosticsBtn"
            type="button"
            onClick={handleExportDiagnostics}
          >
            导出宿主事件 JSON
          </button>
        </div>
      </div>

      {exportFeedback ? (
        <div
          id="auditExportFeedback"
          role={exportFeedback.error ? "alert" : "status"}
          style={{
            marginBottom: 16,
            padding: "10px 14px",
            borderRadius: 8,
            fontSize: 13,
            background: exportFeedback.error
              ? "rgba(239, 68, 68, 0.12)"
              : "rgba(47, 107, 255, 0.12)",
            color: exportFeedback.error ? "var(--red)" : "var(--blue)",
            border: `1px solid ${
              exportFeedback.error
                ? "rgba(239, 68, 68, 0.3)"
                : "rgba(47, 107, 255, 0.3)"
            }`,
            lineHeight: 1.6,
            wordBreak: "break-word",
          }}
        >
          {exportFeedback.text}
        </div>
      ) : null}

      {/* 实时网关调用与路由链路看板 (DEF-Phase F2: Live Request Traces) */}
      <div className="card" style={{ marginBottom: 16 }} id="gatewayTracesCard">
        <div className="section-head">
          <div>
            <h3>网关调用链路与选路观测 (Live Request Traces)</h3>
            <span className="sub">
              实时捕获经由统一网关发起的客户端请求，透明呈现真实耗时、首字时间(TTFT)、状态码、Token消耗与插件路由
            </span>
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <select
              className="input"
              id="traceClientFilter"
              aria-label="按客户端筛选调用轨迹"
              value={traceClientFilter}
              onChange={(e) => setTraceClientFilter(e.target.value)}
              style={{ width: "auto", minWidth: 130 }}
            >
              <option value="all">全部客户端 ({safeTraces.length})</option>
              {availableTraceClients.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </div>
        </div>

        {safeTraces.length === 0 ? (
          <div
            id="tracesEmptyState"
            className="sub"
            style={{ padding: "18px 4px", lineHeight: 1.7 }}
          >
            暂无网关实时调用轨迹（等待客户端如 Cursor、Windsurf、Claude Code 或试飞面板发起请求后自动实时记录）。
          </div>
        ) : filteredTraces.length === 0 ? (
          <div
            id="tracesEmptyFilteredState"
            className="sub"
            style={{ padding: "18px 4px", lineHeight: 1.7 }}
          >
            当前筛选条件下暂无调用轨迹记录（0 条）。
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "140px 100px 180px 1fr 90px 90px 100px",
                padding: "8px 12px",
                fontWeight: 600,
                fontSize: 12,
                color: "var(--text2)",
                borderBottom: "1px solid var(--border)",
              }}
            >
              <div>时间</div>
              <div>客户端</div>
              <div>模型</div>
              <div>目标插件 / 选路</div>
              <div>状态</div>
              <div>耗时 (TTFT)</div>
              <div>Token (入/出)</div>
            </div>

            {filteredTraces.map((tr) => {
              const isSelected = activeTrace?.id === tr.id;
              return (
                <div
                  key={tr.id}
                  data-trace-id={tr.id}
                  onClick={() => setSelectedTraceId(tr.id)}
                  style={{
                    display: "grid",
                    gridTemplateColumns: "140px 100px 180px 1fr 90px 90px 100px",
                    padding: "10px 12px",
                    borderRadius: 6,
                    fontSize: 13,
                    alignItems: "center",
                    cursor: "pointer",
                    background: isSelected
                      ? "rgba(47, 107, 255, 0.08)"
                      : "var(--bg-subtle, rgba(255, 255, 255, 0.02))",
                    border: isSelected
                      ? "1px solid rgba(47, 107, 255, 0.3)"
                      : "1px solid var(--border)",
                    transition: "all 0.15s ease",
                  }}
                >
                  <div className="mono" style={{ fontSize: 12, color: "var(--text2)" }}>
                    {formatTimestamp(tr.timestamp_ms)}
                  </div>
                  <div>{renderClientPill(tr.client)}</div>
                  <div
                    className="mono"
                    style={{
                      fontWeight: 500,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {tr.model || "默认模型"}
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, overflow: "hidden" }}>
                    <span className="mono" style={{ color: "var(--text)" }}>
                      {tr.target_plugin ? `→ ${tr.target_plugin}` : "未路由"}
                    </span>
                    <span className="pill pill-gray" style={{ fontSize: 11 }}>
                      {tr.route_strategy}
                    </span>
                    {tr.cache_hit && (
                      <span className="pill pill-violet" style={{ fontSize: 11 }}>
                        Cache
                      </span>
                    )}
                  </div>
                  <div>{renderStatusCodePill(tr.status_code)}</div>
                  <div className="mono" style={{ fontSize: 12 }}>
                    <b>{tr.duration_ms}ms</b>
                    {tr.first_token_ms !== null && (
                      <span style={{ color: "var(--text2)", marginLeft: 4 }}>
                        ({tr.first_token_ms}ms)
                      </span>
                    )}
                  </div>
                  <div className="mono sub" style={{ fontSize: 12 }}>
                    {tr.input_tokens !== null || tr.output_tokens !== null
                      ? `${tr.input_tokens ?? 0} / ${tr.output_tokens ?? 0}`
                      : "—"}
                  </div>
                </div>
              );
            })}

            {activeTrace && (
              <div
                style={{
                  marginTop: 8,
                  padding: "12px 14px",
                  borderRadius: 6,
                  background: "var(--card-bg, rgba(0, 0, 0, 0.2))",
                  border: "1px dashed var(--border)",
                  fontSize: 12,
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
                  <b>Trace 链路详情: <span className="mono">{activeTrace.id}</span></b>
                  <span className="sub mono">
                    {activeTrace.method} {activeTrace.path}
                  </span>
                </div>
                <div
                  style={{
                    display: "flex",
                    gap: 16,
                    flexWrap: "wrap",
                    marginBottom: 8,
                    color: "var(--text2)",
                  }}
                >
                  <div>客户端 IP: <span className="mono">{activeTrace.client_ip || "127.0.0.1"}</span></div>
                  <div>协议: <span className="mono">{activeTrace.ingress_protocol}</span></div>
                  <div>选路模式: <span className="mono">{activeTrace.route_strategy}</span></div>
                  <div>目标插件: <span className="mono">{activeTrace.target_plugin || "none"}</span></div>
                  {activeTrace.fallback_trail.length > 0 && (
                    <div>
                      Fallback 链: <span className="mono">{activeTrace.fallback_trail.join(" → ")}</span>
                    </div>
                  )}
                  {activeTrace.error && (
                    <div style={{ color: "var(--red)" }}>错误: {activeTrace.error}</div>
                  )}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
      <div className="card" style={{ marginBottom: 16 }}>
        <div className="section-head">
          <div>
            <h3>宿主控制事件记录 (Host Events)</h3>
            <span className="sub">
              HTTP 请求链路追踪与实时 API Doctor 尚未实现；以下仅展示宿主控制面事件（白名单字段，未做自动脱敏校验）
            </span>
          </div>
          <select
            className="input"
            id="auditCategoryFilter"
            aria-label="按事件类别筛选"
            value={
              categoryFilter !== "all" && availableCategories.includes(categoryFilter)
                ? categoryFilter
                : "all"
            }
            onChange={(e) => setCategoryFilter(e.target.value)}
            style={{ width: "auto", minWidth: 140 }}
          >
            <option value="all">全部类别 ({safeEvents.length})</option>
            {availableCategories.map((cat) => (
              <option key={cat} value={cat}>
                {cat}
              </option>
            ))}
          </select>
        </div>

        {isEventsUnknown ? (
          <div
            id="auditEmptyState"
            className="sub"
            style={{ padding: "18px 4px", lineHeight: 1.7 }}
          >
            宿主事件数据状态未知（尚未传入 events 数据）；HTTP 请求链路追踪（状态码 / IR 转换 / TTFT / SSE 归因）尚未实现。
          </div>
        ) : filteredEvents.length === 0 ? (
          <div
            id="auditEmptyState"
            className="sub"
            style={{ padding: "18px 4px", lineHeight: 1.7 }}
          >
            暂无宿主控制事件记录（0 条）。HTTP 请求链路追踪（状态码 / IR 转换 / TTFT / SSE 归因）尚未实现。
          </div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>时间</th>
                <th>类别 / 动作</th>
                <th>对象</th>
                <th>执行结果</th>
                <th>宿主事件摘要</th>
              </tr>
            </thead>
            <tbody>
              {filteredEvents.map((ev) => (
                <tr
                  key={ev.key}
                  data-event-id={ev.id ?? undefined}
                  onClick={() => setSelectedKey(ev.key)}
                  style={{
                    cursor: "pointer",
                    background:
                      activeEvent?.key === ev.key
                        ? "rgba(47, 107, 255, 0.08)"
                        : undefined,
                  }}
                >
                  <td className="mono">{formatTimestamp(ev.timestamp_ms)}</td>
                  <td>
                    <b>{ev.category}</b>
                    <p className="sub mono">{ev.action}</p>
                  </td>
                  <td className="mono">{ev.target}</td>
                  <td>{renderResultPill(ev.success)}</td>
                  <td className="sub">{ev.summary}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="grid2">
        <div className="card">
          <h3>事件详情（仅宿主白名单字段）</h3>
          <pre className="code-light" id="auditEventDetails" style={{ minHeight: 140 }}>
            {activeEvent
              ? JSON.stringify(
                  {
                    event_type: "host_control_event",
                    id: activeEvent.id,
                    timestamp_ms: activeEvent.timestamp_ms,
                    category: activeEvent.category,
                    action: activeEvent.action,
                    target: activeEvent.target,
                    success: activeEvent.success,
                    summary: activeEvent.summary,
                  },
                  null,
                  2
                )
              : isEventsUnknown
              ? "当前未读取到宿主控制事件（状态未知）。\nHTTP 请求链路追踪（Request Trace / TTFT / IR / SSE 归因）未实现。"
              : "当前无选中的宿主控制事件。\nHTTP 请求链路追踪（Request Trace / TTFT / IR / SSE 归因）未实现。"}
          </pre>
        </div>
        <div className="card">
          <h3>常见状态码一般说明（静态参考 · 未接入实时诊断）</h3>
          <p className="sub" style={{ marginBottom: 10 }}>
            以下为静态排障手册说明，当前未实现实时 HTTP 请求捕获、自动 Doctor 归因或自动 Fallback。
          </p>
          <div className="listrow">
            <span className="pill pill-gray">401</span>
            <div>
              <b style={{ fontSize: 14 }}>鉴权失败（静态说明）</b>
              <p className="sub">请人工核对客户端配置的网关 Master Key 或插件上游凭据</p>
            </div>
          </div>
          <div className="listrow">
            <span className="pill pill-gray">429</span>
            <div>
              <b style={{ fontSize: 14 }}>上游限流（静态说明）</b>
              <p className="sub">表示上游返回速率限制；自动 Fallback 备选链与自动退避尚未实现</p>
            </div>
          </div>
          <div className="listrow">
            <span className="pill pill-gray">502</span>
            <div>
              <b style={{ fontSize: 14 }}>上游中断（静态说明）</b>
              <p className="sub">请人工检查插件运行状态与健康探针，可在插件页手动重启</p>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
