import React, { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api";
import {
  FlightDiscoveredModel,
  FlightMessageInput,
  FlightStreamEvent,
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
}

interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  reasoning?: string;
}

type CheckStatus = "untested" | "passed" | "failed";

export const FlightPanel: React.FC<FlightPanelProps> = ({
  gatewayStatus,
  targetPluginId,
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

  // Clean cancellation on unmount
  useEffect(() => {
    return () => {
      if (activeRequestIdRef.current) {
        const reqId = activeRequestIdRef.current;
        activeRequestIdRef.current = null;
        api.flightCancel(reqId).catch(() => {});
      }
    };
  }, []);

  const handleCancel = useCallback(async () => {
    if (activeRequestIdRef.current) {
      const reqId = activeRequestIdRef.current;
      try {
        await api.flightCancel(reqId);
      } catch (err) {
        console.warn("Cancel request failed:", err);
      }
      setIsStreaming(false);
      activeRequestIdRef.current = null;
      setError("已取消试飞推理");
    }
  }, []);

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    const prompt = input.trim();
    if (!prompt || !selectedModel || !isGatewayRunning || isStreaming) {
      return;
    }

    // Cancel any previous active session before starting a new one
    if (activeRequestIdRef.current) {
      api.flightCancel(activeRequestIdRef.current).catch(() => {});
      activeRequestIdRef.current = null;
    }

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
    setIsStreaming(true);

    const flightMessages: FlightMessageInput[] = nextMessages.map((m) => ({
      role: m.role,
      content: m.content,
    }));

    if (streamMode) {
      try {
        const startResult = await api.flightChatStream(
          {
            model: selectedModel,
            messages: flightMessages,
            temperature: temperature / 10,
            stream: true,
          },
          (event: FlightStreamEvent) => {
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
              case "completed":
                setMessages((prev) => {
                  const copy = [...prev];
                  const last = copy[copy.length - 1];
                  if (last && last.role === "assistant") {
                    copy[copy.length - 1] = {
                      ...last,
                      content: event.total_text,
                      reasoning: event.total_reasoning || last.reasoning,
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
                setIsStreaming(false);
                activeRequestIdRef.current = null;
                setCompatStatus((prev) => ({
                  ...prev,
                  chat: "passed",
                  sse: "passed",
                  multiturn: nextMessages.length > 2 ? "passed" : prev.multiturn,
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
        activeRequestIdRef.current = startResult.request_id;
      } catch (err) {
        setIsStreaming(false);
        activeRequestIdRef.current = null;
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
        });
        if (res.first_content_ms != null) {
          setTtft(res.first_content_ms);
        }
        setTotalElapsed(res.elapsed_ms);
        if (res.usage) {
          setTokenUsage(res.usage);
        }
        setMessages((prev) => {
          const copy = [...prev];
          const last = copy[copy.length - 1];
          if (last && last.role === "assistant") {
            copy[copy.length - 1] = {
              ...last,
              content: res.text || res.content || "",
              reasoning: res.reasoning || undefined,
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
        }));
      } catch (err) {
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
  const isInputDisabled = !canSend || isStreaming;
  const isSendDisabled = !canSend || isStreaming || !input.trim();

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
                style={{ marginRight: 6 }}
              >
                取消
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
        </div>
      </div>
    </section>
  );
};
