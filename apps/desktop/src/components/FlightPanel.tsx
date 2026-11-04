import React, { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api";
import {
  FlightCancelResult,
  FlightDiscoveredModel,
  FlightMessageInput,
  FlightStreamEvent,
  FlightToolCall,
  FlightUsage,
  GatewayStatusInfo,
  PluginRecord,
  PluginStatusInfo,
} from "../types";

interface FlightPanelProps {
  gatewayStatus: GatewayStatusInfo | null;
  targetPluginId?: string | null;
  plugins?: PluginRecord[];
  pluginStatuses?: Record<string, PluginStatusInfo>;
  initialModel?: string | null;
}

interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  reasoning?: string;
  toolCalls?: FlightToolCall[];
}

type CheckStatus = "untested" | "passed" | "failed";

// Preserve an unresolved unmount cancellation across navigation without retaining component state.
let detachedFlight: {
  requestId: string;
  message: string;
  pending: Promise<void> | null;
} | null = null;
let pendingFlightStart: Promise<void> | null = null;
const noActiveSession = "未找到活跃的飞行会话";
const cancelReasons = new Set([
  noActiveSession,
  "请求 ID 不匹配或该会话已结束",
  "会话已在结束流程中",
]);

async function requestFlightCancellation(requestId: string): Promise<FlightCancelResult> {
  try {
    const result = await api.flightCancel(requestId);
    return result.cancelled
      ? { cancelled: true }
      : {
          cancelled: false,
          message: `取消未完成：${result.message && cancelReasons.has(result.message) ? result.message : "后端未确认取消"}`,
        };
  } catch {
    // IPC errors can contain private data; never render or log raw rejection values.
    return { cancelled: false, message: "取消请求失败：无法确认后端状态，请重试" };
  }
}

function rememberDetachedCancellation(requestId: string, result: FlightCancelResult) {
  if (detachedFlight?.requestId !== requestId) return;
  if (result.cancelled) {
    detachedFlight = null;
  } else {
    detachedFlight.message = result.message ?? "取消未完成：后端未确认取消";
  }
}

async function cancelDetachedFlight(requestId: string) {
  if (detachedFlight?.requestId === requestId && detachedFlight.pending) {
    return detachedFlight.pending;
  }
  const retained = { requestId, message: "上一试飞会话取消确认中", pending: null as Promise<void> | null };
  detachedFlight = retained;
  retained.pending = requestFlightCancellation(requestId).then((result) => {
    rememberDetachedCancellation(requestId, result);
    retained.pending = null;
  });
  return retained.pending;
}
export const FlightPanel: React.FC<FlightPanelProps> = ({
  gatewayStatus,
  targetPluginId,
  initialModel,
}) => {
  const [models, setModels] = useState<FlightDiscoveredModel[]>([]);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [loadingModels, setLoadingModels] = useState<boolean>(false);
  const [modelsError, setModelsError] = useState<string | null>(null);

  const [temperature, setTemperature] = useState<number>(7); // 0.7 default
  const [streamMode, setStreamMode] = useState<boolean>(true);

  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState<string>("");
  const [isStreaming, setIsStreaming] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  // Real metric collection - no fabricated or random numbers
  const [ttft, setTtft] = useState<number | null>(null);
  const [totalElapsed, setTotalElapsed] = useState<number | null>(null);
  const [tokenUsage, setTokenUsage] = useState<FlightUsage | null>(null);
  const [lastFinishReason, setLastFinishReason] = useState<string | null>(null);
  const [longContextBytes, setLongContextBytes] = useState<number>(8192);

  // Compatibility checklist items
  const [compatStatus, setCompatStatus] = useState<Record<string, CheckStatus>>({
    chat: "untested",
    sse: "untested",
    multiturn: "untested",
    tools: "untested",
    longtext: "untested",
    multimodal: "untested",
  });

  const activeRequestIdRef = useRef<string | null>(null);
  const mountedRef = useRef(true);
  const cancellingRef = useRef<object | null>(null);
  const sendingRef = useRef(false);
  const activeRunRef = useRef<{ terminal: boolean; requestId: string | null; cleanupStarted: boolean } | null>(null);
  const [isCancelling, setIsCancelling] = useState(false);
  const [isPreparing, setIsPreparing] = useState(false);

  const isGatewayRunning = Boolean(gatewayStatus?.is_running);

  // Model discovery on mount or when gateway state/backend changes
  useEffect(() => {
    let active = true;
    if (isGatewayRunning) {
      setLoadingModels(true);
      setModelsError(null);
      api
        .flightListModels()
        .then((res) => {
          if (!active) return;
          const list = res?.models || [];
          setModels(list);
          setLoadingModels(false);
          if (list.length > 0) {
            setSelectedModel((prev) => {
              if (initialModel && list.some((m) => m.id === initialModel)) return initialModel;
              if (prev && list.some((m) => m.id === prev)) return prev;
              return list[0].id;
            });
          } else {
            setSelectedModel("");
          }
        })
        .catch((err) => {
          if (!active) return;
          setLoadingModels(false);
          setModels([]);
          setSelectedModel("");
          const msg = err instanceof Error ? err.message : String(err);
          setModelsError(msg);
        });
    } else {
      setModels([]);
      setSelectedModel("");
      setLoadingModels(false);
      setModelsError(null);
    }
    return () => {
      active = false;
    };
  }, [isGatewayRunning, targetPluginId]);

  useEffect(() => {
    if (initialModel) {
      setSelectedModel(initialModel);
      const chatInput = document.getElementById("chatInput");
      if (chatInput instanceof HTMLInputElement) {
        chatInput.focus();
      }
    }
  }, [initialModel]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      const requestId = activeRequestIdRef.current;
      if (requestId) {
        if (activeRunRef.current) activeRunRef.current.cleanupStarted = true;
        void cancelDetachedFlight(requestId);
      }
    };
  }, []);

  const handleCancel = useCallback(async () => {
    const requestId = activeRequestIdRef.current;
    if (!requestId || cancellingRef.current) return;
    const cancellation = {};
    cancellingRef.current = cancellation;
    setIsCancelling(true);
    const result = await requestFlightCancellation(requestId);
    if (cancellingRef.current === cancellation) cancellingRef.current = null;
    if (!mountedRef.current) {
      rememberDetachedCancellation(requestId, result);
      return;
    }
    if (!cancellingRef.current) setIsCancelling(false);
    // A terminal event may have settled the request while cancellation was pending.
    if (activeRequestIdRef.current !== requestId) return;
    if (result.cancelled) {
      if (activeRunRef.current) activeRunRef.current.terminal = true;
      sendingRef.current = false;
      activeRequestIdRef.current = null;
      if (detachedFlight?.requestId === requestId) detachedFlight = null;
      setIsStreaming(false);
      setError("已取消试飞推理");
    } else {
      // The IPC did not confirm cancellation. Keep the existing stream state and ID.
      setError(result.message ?? "取消未完成：后端未确认取消");
    }
  }, []);
  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    const prompt = input.trim();
    if (!prompt || !selectedModel || !isGatewayRunning || isStreaming || sendingRef.current) {
      return;
    }
    sendingRef.current = true;
    setIsPreparing(true);
    if (pendingFlightStart) {
      await pendingFlightStart;
      if (!mountedRef.current) { sendingRef.current = false; return; }
    }
    // An unmount may have left an unconfirmed cancellation. Never overlap it.
    if (detachedFlight) {
      await cancelDetachedFlight(detachedFlight.requestId);
      if (!mountedRef.current) { sendingRef.current = false; return; }
      if (detachedFlight) {
        setError(detachedFlight.message);
        setIsPreparing(false);
        sendingRef.current = false;
        return;
      }
    }
    if (activeRequestIdRef.current) {
      const previousId = activeRequestIdRef.current;
      const result = await requestFlightCancellation(previousId);
      if (!mountedRef.current) { sendingRef.current = false; return; }
      if (activeRequestIdRef.current === previousId && !result.cancelled) {
        setError(result.message ?? "取消未完成：后端未确认取消");
        setIsPreparing(false);
        sendingRef.current = false;
        return;
      }
      if (activeRequestIdRef.current === previousId) activeRequestIdRef.current = null;
    }
    const run = { terminal: false, requestId: null as string | null, cleanupStarted: false };
    activeRunRef.current = run;
    setIsPreparing(false);
    const nextMessages: ChatMessage[] = [
      ...messages,
      { role: "user", content: prompt },
    ];
    setMessages([
      ...nextMessages,
      { role: "assistant", content: "", reasoning: "" },
    ]);
    setInput("");
    setError(null);
    setTtft(null);
    setTotalElapsed(null);
    setTokenUsage(null);
    setLastFinishReason(null);
    setIsStreaming(true);

    // Long-context quality test: bounded, user-configurable synthetic block.
    // No silent truncation: the size is visible and finish_reason/usage reflect the outcome.
    const lcBytes = Math.min(Math.max(longContextBytes, 0), 64 * 1024);
    const longContextNote =
      lcBytes > 0
        ? "\n\n[长上下文测试附加 " + lcBytes + " 字节] " + "A".repeat(lcBytes)
        : "";

    const flightMessages: FlightMessageInput[] = nextMessages.map((m, i) => ({
      role: m.role,
      content:
        i === nextMessages.length - 1 ? m.content + longContextNote : m.content,
    }));

    // Standard bounded get_weather tool definition to probe tool-calling support.
    const toolDefs = [
      {
        type: "function",
        function: {
          name: "get_weather",
          description: "Get current weather for a city",
          parameters: {
            type: "object",
            properties: { city: { type: "string" } },
            required: ["city"],
          },
        },
      },
    ];

    let resolvePending!: () => void;
    const pending = new Promise<void>((resolve) => { resolvePending = resolve; });
    pendingFlightStart = pending;
    try {
    if (streamMode) {
      try {
        const startResult = await api.flightChatStream(
          {
            model: selectedModel,
            messages: flightMessages,
            temperature: temperature / 10,
            stream: true,
            tools: toolDefs,
            tool_choice: "auto",
          },
          (event: FlightStreamEvent) => {
            if (activeRunRef.current !== run || run.terminal) return;
            if (run.requestId && event.request_id && run.requestId !== event.request_id) return;
            run.requestId = event.request_id ?? run.requestId;
            const terminal = ["completed", "done", "cancelled", "failed", "error"].includes(event.type);
            if (terminal) {
              run.terminal = true;
              sendingRef.current = false;
              cancellingRef.current = null;
              activeRequestIdRef.current = null;
              if (detachedFlight?.requestId === run.requestId) detachedFlight = null;
            }
            if (!mountedRef.current) {
              if (!terminal && event.type === "started") {
                run.cleanupStarted = true;
                void cancelDetachedFlight(event.request_id);
              }
              return;
            }
            if (terminal) setIsCancelling(false);
            if (terminal) setError(null);
            switch (event.type) {
              case "started":
                activeRequestIdRef.current = event.request_id;
                break;
              case "delta":
                setMessages((prev) => {
                  const copy = [...prev];
                  const last = copy[copy.length - 1];
                  if (last && last.role === "assistant") {
                    const deltaText = event.text ?? event.delta ?? "";
                    const deltaReasoning =
                      event.reasoning ?? event.reasoning_delta ?? "";
                    copy[copy.length - 1] = {
                      ...last,
                      content: last.content + deltaText,
                      reasoning:
                        last.reasoning || deltaReasoning
                          ? (last.reasoning || "") + deltaReasoning
                          : undefined,
                    };
                  }
                  return copy;
                });
                if (event.first_content_ms != null) {
                  setTtft((cur) => (cur === null ? event.first_content_ms! : cur));
                }
                if (event.elapsed_ms != null) {
                  setTotalElapsed(event.elapsed_ms);
                }
                break;
              case "tool_call_delta":
                if (event.elapsed_ms != null) {
                  setTotalElapsed(event.elapsed_ms);
                }
                break;
              case "completed":
                setMessages((prev) => {
                  const copy = [...prev];
                  const last = copy[copy.length - 1];
                  if (last && last.role === "assistant") {
                    copy[copy.length - 1] = {
                      ...last,
                      content: event.total_text,
                      reasoning: event.total_reasoning || last.reasoning,
                      toolCalls: event.total_tool_calls || undefined,
                    };
                  }
                  return copy;
                });
                if (event.first_content_ms != null) {
                  setTtft(event.first_content_ms);
                }
                setTotalElapsed(event.elapsed_ms);
                if (event.usage) {
                  setTokenUsage(event.usage);
                }
                setLastFinishReason(event.finish_reason ?? null);
                setIsStreaming(false);
                activeRequestIdRef.current = null;
                setCompatStatus((prev) => ({
                  ...prev,
                  chat: "passed",
                  sse: "passed",
                  multiturn: nextMessages.length > 2 ? "passed" : prev.multiturn,
                  tools:
                    event.finish_reason === "tool_calls" ||
                    (event.total_tool_calls && event.total_tool_calls.length > 0)
                      ? "passed"
                      : prev.tools,
                  longtext:
                    lcBytes > 0 && event.finish_reason && event.finish_reason !== ""
                      ? "passed"
                      : prev.longtext,
                }));
                break;
              case "done":
                if (event.first_content_ms != null) {
                  setTtft(event.first_content_ms);
                }
                setTotalElapsed(event.elapsed_ms);
                if (event.usage) {
                  setTokenUsage(event.usage);
                }
                setIsStreaming(false);
                activeRequestIdRef.current = null;
                setCompatStatus((prev) => ({
                  ...prev,
                  chat: "passed",
                  sse: "passed",
                }));
                break;
              case "cancelled":
                setIsStreaming(false);
                activeRequestIdRef.current = null;
                setTotalElapsed(event.elapsed_ms);
                setError("已取消试飞推理");
                break;
              case "failed":
              case "error":
                setIsStreaming(false);
                activeRequestIdRef.current = null;
                setTotalElapsed(event.elapsed_ms);
                setError(
                  ("error_message" in event && event.error_message) ||
                    ("error" in event && event.error) ||
                    "推理请求失败",
                );
                setCompatStatus((prev) => ({
                  ...prev,
                  chat: "failed",
                  sse: "failed",
                }));
                break;
            }
          },
        );
        if (!run.terminal && activeRunRef.current === run) {
          run.requestId = startResult.request_id;
          activeRequestIdRef.current = startResult.request_id;
          if (!mountedRef.current && !run.cleanupStarted) void cancelDetachedFlight(startResult.request_id);
        }
      } catch (err) {
        if (run.terminal || activeRunRef.current !== run) return;
        run.terminal = true;
        sendingRef.current = false;
        activeRequestIdRef.current = null;
        if (!mountedRef.current) return;
        setIsStreaming(false);
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        setCompatStatus((prev) => ({
          ...prev,
          chat: "failed",
          sse: "failed",
        }));
      }
    } else {
      // Non-streaming chat completion
      try {
        const res = await api.flightChatNonstream({
          model: selectedModel,
          messages: flightMessages,
          temperature: temperature / 10,
          stream: false,
          tools: toolDefs,
          tool_choice: "auto",
        });
        if (!mountedRef.current || activeRunRef.current !== run) return;
        if (res.first_content_ms != null) {
          setTtft(res.first_content_ms);
        }
        setTotalElapsed(res.elapsed_ms);
        if (res.usage) {
          setTokenUsage(res.usage);
        }
        setLastFinishReason(res.finish_reason ?? null);
        setMessages((prev) => {
          const copy = [...prev];
          const last = copy[copy.length - 1];
          if (last && last.role === "assistant") {
            copy[copy.length - 1] = {
              ...last,
              content: res.text || res.content || "",
              reasoning: res.reasoning || undefined,
              toolCalls: res.tool_calls || undefined,
            };
          }
          return copy;
        });
        setIsStreaming(false);
        activeRequestIdRef.current = null;
        setCompatStatus((prev) => ({
          ...prev,
          chat: "passed",
          multiturn: nextMessages.length > 2 ? "passed" : prev.multiturn,
          tools:
            res.finish_reason === "tool_calls" ||
            (res.tool_calls && res.tool_calls.length > 0)
              ? "passed"
              : prev.tools,
          longtext:
            lcBytes > 0 && res.finish_reason && res.finish_reason !== ""
              ? "passed"
              : prev.longtext,
        }));
      } catch (err) {
        if (!mountedRef.current || activeRunRef.current !== run) return;
        setIsStreaming(false);
        activeRequestIdRef.current = null;
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        setCompatStatus((prev) => ({
          ...prev,
          chat: "failed",
        }));
      }
    }
    } finally {
      if (!streamMode) sendingRef.current = false;
      if (pendingFlightStart === pending) pendingFlightStart = null;
      resolvePending();
    }
  };

  const gatewayLabel =
    gatewayStatus === null
      ? "网关状态未知"
      : gatewayStatus.is_running
        ? "网关运行中"
        : "网关已停止";

  const gatewayPillClass =
    gatewayStatus?.is_running ? "pill pill-green" : "pill pill-gray";

  // Checkbox & input readiness
  const canSend = Boolean(isGatewayRunning && selectedModel && !loadingModels);
  const isInputDisabled = !canSend || isStreaming || isPreparing;
  const isSendDisabled = !canSend || isStreaming || isPreparing || !input.trim();

  // Metrics string formatting
  const ttftDisplay =
    ttft !== null ? `${ttft} ms` : isStreaming ? "等待中…" : "未测试";
  const elapsedDisplay =
    totalElapsed !== null
      ? `${totalElapsed} ms`
      : isStreaming
        ? "计时中…"
        : "未测试";

  // Only compute TPS if real completion tokens and real elapsed time are available
  const tpsDisplay =
    tokenUsage && tokenUsage.completion_tokens > 0 && totalElapsed && totalElapsed > 0
      ? (tokenUsage.completion_tokens / (totalElapsed / 1000)).toFixed(1)
      : totalElapsed !== null
        ? "—"
        : "未测试";

  const checklistItems = [
    { key: "chat", name: "Chat", status: compatStatus.chat },
    { key: "sse", name: "SSE", status: compatStatus.sse },
    { key: "multiturn", name: "多轮映射", status: compatStatus.multiturn },
    { key: "tools", name: "Tool Calling", status: compatStatus.tools },
    { key: "longtext", name: "长文本", status: compatStatus.longtext },
    { key: "multimodal", name: "多模态", status: compatStatus.multimodal },
  ];

  const untestedCount = checklistItems.filter(
    (i) => i.status === "untested",
  ).length;
  // Capability disclosure: the current backend manifest does not declare any
  // multimodal capability field, so the multimodal check is never executed and
  // must not be shown as passed. It stays "untested" with an explicit note.
  const multimodalUnsupported = true;

  return (
    <section className="panel" id="panel-flight">
      <div className="topbar">
        <div>
          <h2>试飞工作台</h2>
        </div>
        <span className={gatewayPillClass}>
          <span className="dot"></span>
          {gatewayLabel}
        </span>
      </div>

      <div className="grid2" style={{ alignItems: "start" }}>
        <div className="card">
          <div className="field" style={{ justifyContent: "space-between" }}>
            <select
              className="input"
              id="flightModel"
              aria-label="试飞模型"
              disabled={isStreaming || !canSend}
              value={selectedModel}
              onChange={(e) => setSelectedModel(e.target.value)}
              style={{ fontWeight: 500, maxWidth: 280 }}
            >
              {gatewayStatus === null ? (
                <option value="">网关状态未知（无法探测模型）</option>
              ) : !gatewayStatus.is_running ? (
                <option value="">网关未启动（无法探测模型）</option>
              ) : loadingModels ? (
                <option value="">正在探测可用模型…</option>
              ) : modelsError ? (
                <option value="">模型探测失败: {modelsError}</option>
              ) : models.length === 0 ? (
                <option value="">暂无可用模型（统一网关未挂载后端或未发现模型）</option>
              ) : (
                models.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name || m.id}
                  </option>
                ))
              )}
            </select>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <label
                style={{
                  fontSize: 12,
                  display: "flex",
                  alignItems: "center",
                  gap: 4,
                  cursor: "pointer",
                }}
              >
                <input
                  type="checkbox"
                  id="streamToggle"
                  aria-label="流式传输"
                  checked={streamMode}
                  onChange={(e) => setStreamMode(e.target.checked)}
                  disabled={isStreaming}
                />
                <span className="sub">流式</span>
              </label>
              <span className="sub">
                温度 <b id="tempVal">{(temperature / 10).toFixed(1)}</b>
              </span>
              <input
                type="range"
                id="tempRange"
                aria-label="采样温度"
                min="0"
                max="20"
                value={temperature}
                onChange={(e) => setTemperature(Number(e.target.value))}
                disabled={isStreaming || !canSend}
                style={{ width: 110 }}
              />
              <label
                style={{
                  fontSize: 12,
                  display: "flex",
                  alignItems: "center",
                  gap: 4,
                }}
              >
                <span className="sub">长上下文</span>
                <select
                  className="input"
                  id="longContextBytes"
                  aria-label="长上下文附加字节数"
                  value={longContextBytes}
                  onChange={(e) => setLongContextBytes(Number(e.target.value))}
                  disabled={isStreaming}
                  style={{ width: 92, padding: "2px 4px", fontSize: 12 }}
                >
                  <option value={0}>关闭</option>
                  <option value={8192}>8 KB</option>
                  <option value={32768}>32 KB</option>
                  <option value={65536}>64 KB</option>
                </select>
              </label>
            </div>
          </div>

          <div
            className="chatbox"
            id="chatBox"
            style={{
              marginTop: 14,
              maxHeight: 320,
              overflowY: "auto",
              display: "flex",
              flexDirection: "column",
              gap: 10,
            }}
          >
            {messages.length === 0 ? (
              <p className="sub" role="status">
                {gatewayStatus === null
                  ? "本地统一网关状态未知，请先确认网关运行状态后再试飞。"
                  : !gatewayStatus.is_running
                    ? "本地统一网关未启动，请先在网关面板启动服务并挂载插件后端。"
                    : modelsError
                      ? `模型探测失败: ${modelsError}。请检查插件后端是否正常挂载。`
                      : models.length === 0
                        ? "统一网关已启动但暂未发现可用模型。请先在插件列表中挂载模型提供方后端。"
                        : "试飞环境已就绪。输入消息并点击发送开始真实推理测试。"}
              </p>
            ) : (
              messages.map((msg, idx) => (
                <div
                  key={idx}
                  className={`msg ${msg.role === "assistant" ? "ai" : "user"}`}
                  style={{
                    alignSelf:
                      msg.role === "assistant" ? "flex-start" : "flex-end",
                    maxWidth: "85%",
                    padding: "8px 12px",
                    borderRadius: 8,
                    background:
                      msg.role === "assistant"
                        ? "var(--card-bg, #f3f4f6)"
                        : "var(--blue, #2f6bff)",
                    color: msg.role === "assistant" ? "inherit" : "#fff",
                  }}
                >
                  {msg.reasoning ? (
                    <div
                      className="sub"
                      style={{
                        marginBottom: 6,
                        fontStyle: "italic",
                        whiteSpace: "pre-wrap",
                        fontSize: 12,
                        opacity: 0.85,
                      }}
                    >
                      {msg.reasoning}
                    </div>
                  ) : null}
                  <div style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
                    {msg.content ||
                      (isStreaming && idx === messages.length - 1
                        ? "等待回复中…"
                        : "")}
                  </div>
                  {msg.toolCalls && msg.toolCalls.length > 0 ? (
                    <div
                      className="sub"
                      style={{ marginTop: 6, fontSize: 12, opacity: 0.85 }}
                    >
                      {msg.toolCalls.map((tc) => (
                        <div
                          key={tc.id || tc.name}
                          style={{
                            border: "1px solid var(--border, #e5e7eb)",
                            borderRadius: 6,
                            padding: "4px 8px",
                            marginTop: 4,
                          }}
                        >
                          <b>工具调用</b> {tc.name}
                          <span style={{ opacity: 0.7 }}> ({tc.id})</span>
                          <div style={{ fontFamily: "monospace", fontSize: 11 }}>
                            {tc.arguments}
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : null}
                </div>
              ))
            )}
            {error ? (
              <div
                className="sub"
                role="alert"
                style={{ color: "var(--red, #ef4444)", marginTop: 4 }}
              >
                {error}
              </div>
            ) : null}
            {lastFinishReason === "length" ? (
              <div
                className="sub"
                role="status"
                style={{ color: "var(--yellow, #d97706)", marginTop: 4 }}
              >
                上游因长度限制截断了输出 (finish_reason=length)，未伪造完整结果。
              </div>
            ) : null}
          </div>

          <form onSubmit={handleSend} className="field" style={{ marginTop: 12 }}>
            <input
              className="input"
              id="chatInput"
              aria-label="试飞消息"
              placeholder={
                gatewayStatus === null
                  ? "网关状态未知，暂不能发送"
                  : !gatewayStatus.is_running
                    ? "本地统一网关未启动，暂不能发送"
                    : !selectedModel
                      ? "请先选择可用模型"
                      : "输入试飞提示词，回车或点击发送…"
              }
              value={input}
              onChange={(e) => setInput(e.target.value)}
              disabled={isInputDisabled}
              style={{ flex: 1 }}
            />
            {isStreaming ? (
              <button
                className="btn btn-secondary"
                id="cancelBtn"
                type="button"
                onClick={handleCancel}
                disabled={isCancelling}
                aria-busy={isCancelling}
                style={{ marginRight: 6 }}
              >
                {isCancelling ? "取消确认中…" : "取消"}
              </button>
            ) : null}
            <button
              className="btn btn-primary"
              id="sendBtn"
              type="submit"
              disabled={isSendDisabled}
            >
              {isStreaming ? "生成中…" : "发送"}
            </button>
          </form>

          <div className="grid3" style={{ marginTop: 14, gap: 10 }}>
            <div className="card metric" style={{ boxShadow: "none", padding: 12 }}>
              <b style={{ color: "var(--blue)" }}>{ttftDisplay}</b>
              <span>TTFT 首字延迟</span>
            </div>
            <div className="card metric" style={{ boxShadow: "none", padding: 12 }}>
              <b>{tpsDisplay}</b>
              <span>TPS · tok/s</span>
            </div>
            <div className="card metric" style={{ boxShadow: "none", padding: 12 }}>
              <b>{elapsedDisplay}</b>
              <span>总耗时</span>
            </div>
          </div>
        </div>

        <div className="card">
          <h3>Compatibility Flight</h3>
          <p className="sub" style={{ margin: "4px 0 12px" }}>
            {untestedCount === 6
              ? "尚未执行协议契约与边界兼容性测试"
              : `已执行测试，当前 ${6 - untestedCount} 项已验证`}
          </p>
          <div style={{ marginTop: 8 }}>
            {checklistItems.map((item, index) => (
              <div className="checkrow" key={item.name}>
                <div
                  className="check-ic"
                  style={{
                    background:
                      item.status === "passed"
                        ? "rgba(34, 197, 94, 0.15)"
                        : item.status === "failed"
                          ? "rgba(239, 68, 68, 0.15)"
                          : "var(--well)",
                    color:
                      item.status === "passed"
                        ? "var(--green)"
                        : item.status === "failed"
                          ? "var(--red)"
                          : "var(--muted)",
                  }}
                >
                  {item.status === "passed"
                    ? "✓"
                    : item.status === "failed"
                      ? "✗"
                      : "—"}
                </div>
                <div style={{ flex: 1 }}>
                  <b style={{ fontSize: 14.5 }}>
                    {index + 1} · {item.name}
                  </b>
                </div>
                <span
                  className={`pill ${
                    item.status === "passed"
                      ? "pill-green"
                      : item.status === "failed"
                        ? "pill-red"
                        : "pill-gray"
                  }`}
                >
                  {item.status === "passed"
                    ? "通过"
                    : item.status === "failed"
                      ? "未通过"
                      : "未测试"}
                </span>
              </div>
            ))}
          </div>
          <p className="sub" style={{ marginTop: 12 }}>
            <b>
              {untestedCount === 6
                ? "6 项未测试 · 无兼容性结论"
                : `${untestedCount} 项未测试 · 真实试飞结果`}
            </b>
          </p>
          {multimodalUnsupported ? (
            <p className="sub" style={{ marginTop: 4, fontSize: 12 }}>
              当前后端未声明多模态能力，多模态项保持未测试（不会伪造结果）。
            </p>
          ) : null}
        </div>
      </div>
    </section>
  );
};
