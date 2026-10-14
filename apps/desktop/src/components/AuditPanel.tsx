import React, { useEffect, useMemo, useRef, useState } from "react";
import { HostEvent } from "../types";

interface AuditPanelProps {
  events?: HostEvent[];
  onRefresh?: () => void | Promise<void>;
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

function formatTimestamp(ts: number | null): string {
  if (ts === null) return "时间未知";
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return "时间未知";
  return d.toISOString().replace("T", " ").slice(0, 19);
}

export const AuditPanel: React.FC<AuditPanelProps> = ({ events, onRefresh }) => {
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
              onClick={() => void onRefresh()}
            >
              刷新事件
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
