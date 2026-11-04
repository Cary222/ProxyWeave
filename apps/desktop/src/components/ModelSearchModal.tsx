import React, { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "../lib/tauri";
import { FlightDiscoveredModel, FlightModelListResult, GatewayStatusInfo, PluginRecord } from "../types";
import { Icon } from "./Icon";

export interface ModelSearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  gatewayStatus: GatewayStatusInfo | null;
  plugins?: PluginRecord[];
  onSelectModelFlight: (modelId: string) => void;
  onNavigateGateway?: () => void;
}

type ModelCategory = "all" | "claude" | "gpt" | "deepseek" | "other";

function categorizeModel(modelId: string): ModelCategory {
  const lower = modelId.toLowerCase();
  if (lower.includes("claude")) return "claude";
  if (lower.includes("gpt") || lower.includes("o1") || lower.includes("o3") || lower.includes("openai")) return "gpt";
  if (lower.includes("deepseek") || lower.includes("r1") || lower.includes("v3")) return "deepseek";
  return "other";
}

export const ModelSearchModal: React.FC<ModelSearchModalProps> = ({
  isOpen,
  onClose,
  gatewayStatus,
  onSelectModelFlight,
  onNavigateGateway,
}) => {
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<ModelCategory>("all");
  const [models, setModels] = useState<FlightDiscoveredModel[]>([]);
  const [activeBackend, setActiveBackend] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const isGatewayRunning = Boolean(gatewayStatus?.is_running);

  // Load models when opened
  useEffect(() => {
    if (!isOpen) {
      setQuery("");
      setSelectedIndex(0);
      setCopiedId(null);
      return;
    }

    // Auto-focus input
    const timer = setTimeout(() => {
      inputRef.current?.focus();
    }, 50);

    if (isGatewayRunning) {
      setLoading(true);
      setError(null);
      invoke<FlightModelListResult>("flight_list_models")
        .then((res) => {
          setLoading(false);
          const list = Array.isArray(res?.models) ? res.models : [];
          setModels(list);
          setActiveBackend(res?.active_backend ?? null);
        })
        .catch((err) => {
          setLoading(false);
          setModels([]);
          const msg = err instanceof Error ? err.message : String(err);
          setError(msg);
        });
    } else {
      setModels([]);
      setLoading(false);
      setError(null);
    }

    return () => clearTimeout(timer);
  }, [isOpen, isGatewayRunning]);

  // Filter models
  const filteredModels = useMemo(() => {
    let result = models;

    // Filter by category
    if (category !== "all") {
      result = result.filter((m) => categorizeModel(m.id) === category);
    }

    // Filter by search query
    if (query.trim()) {
      const q = query.trim().toLowerCase();
      result = result.filter(
        (m) =>
          m.id.toLowerCase().includes(q) ||
          m.name.toLowerCase().includes(q) ||
          (m.owned_by && m.owned_by.toLowerCase().includes(q))
      );
    }

    return result;
  }, [models, category, query]);

  // Reset selectedIndex when filter changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [filteredModels.length, query, category]);

  // Scroll active item into view
  useEffect(() => {
    if (!listRef.current) return;
    const activeEl = listRef.current.querySelector(`[data-index="${selectedIndex}"]`);
    if (activeEl instanceof HTMLElement) {
      activeEl.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  // Handle keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) => (filteredModels.length > 0 ? (prev + 1) % filteredModels.length : 0));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) =>
        filteredModels.length > 0 ? (prev - 1 + filteredModels.length) % filteredModels.length : 0
      );
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (filteredModels[selectedIndex]) {
        onSelectModelFlight(filteredModels[selectedIndex].id);
      }
    }
  };

  const handleCopy = (id: string, e?: React.MouseEvent) => {
    e?.stopPropagation();
    try {
      void navigator.clipboard.writeText(id);
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 1800);
    } catch {
      // ignore
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="modal-overlay"
      role="dialog"
      aria-label="全局模型与快速试飞检索"
      aria-modal="true"
      onClick={onClose}
      onKeyDown={handleKeyDown}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 1000,
        background: "rgba(15, 23, 42, 0.45)",
        backdropFilter: "blur(4px)",
        display: "flex",
        alignItems: "flex-start",
        justifyContent: "center",
        paddingTop: "10vh",
      }}
    >
      <div
        className="model-search-modal"
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 640,
          maxWidth: "92vw",
          maxHeight: "78vh",
          background: "var(--card)",
          border: "1px solid var(--line)",
          borderRadius: 12,
          boxShadow: "0 24px 60px rgba(0, 0, 0, 0.28)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {/* 顶部搜索框 */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 10,
            padding: "14px 18px",
            borderBottom: "1px solid var(--line)",
            background: "var(--well)",
          }}
        >
          <Icon name="search" style={{ width: 18, height: 18, color: "var(--muted)" }} />
          <input
            ref={inputRef}
            id="modelSearchInput"
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索模型名称、ID 或厂商 (支持 ↑ ↓ 导航，Enter 试飞，Esc 关闭)..."
            aria-label="搜索模型"
            style={{
              flex: 1,
              border: "none",
              outline: "none",
              background: "transparent",
              color: "var(--text)",
              fontSize: 14,
              fontFamily: "inherit",
            }}
          />
          {query ? (
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => setQuery("")}
              style={{ padding: "2px 6px", fontSize: 12, color: "var(--muted)" }}
              aria-label="清空输入"
            >
              ×
            </button>
          ) : null}
          <kbd
            style={{
              fontSize: 11,
              fontFamily: "var(--mono)",
              padding: "2px 6px",
              borderRadius: 4,
              background: "var(--card)",
              border: "1px solid var(--line)",
              color: "var(--muted)",
            }}
          >
            Esc
          </kbd>
        </div>

        {/* 分类过滤芯片 */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "8px 18px",
            borderBottom: "1px solid var(--line2)",
            background: "var(--well)",
            overflowX: "auto",
          }}
        >
          <span style={{ fontSize: 11, color: "var(--muted)", marginRight: 4 }}>分类:</span>
          {(
            [
              { id: "all", label: `全部 (${models.length})` },
              { id: "claude", label: "Claude" },
              { id: "gpt", label: "GPT / OpenAI" },
              { id: "deepseek", label: "DeepSeek" },
              { id: "other", label: "其他模型" },
            ] as const
          ).map((cat) => (
            <button
              key={cat.id}
              type="button"
              className={`pill ${category === cat.id ? "pill-blue" : "pill-gray"}`}
              onClick={() => setCategory(cat.id)}
              style={{
                cursor: "pointer",
                border: "none",
                fontSize: 11,
                padding: "2px 8px",
                fontWeight: category === cat.id ? 600 : 400,
              }}
            >
              {cat.label}
            </button>
          ))}
          {activeBackend ? (
            <span
              style={{
                marginLeft: "auto",
                fontSize: 11,
                color: "var(--green)",
                display: "flex",
                alignItems: "center",
                gap: 4,
              }}
            >
              <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--green)" }} />
              网关后端: {activeBackend}
            </span>
          ) : null}
        </div>

        {/* 结果列表区域 */}
        <div
          ref={listRef}
          id="modelSearchResults"
          style={{
            flex: 1,
            overflowY: "auto",
            padding: "6px 8px",
            minHeight: 260,
          }}
        >
          {!isGatewayRunning ? (
            <div style={{ padding: "40px 20px", textAlign: "center", color: "var(--muted)" }}>
              <div style={{ fontSize: 24, marginBottom: 8 }}>⚠️</div>
              <div style={{ fontWeight: 600, color: "var(--text)", marginBottom: 4 }}>
                本地统一网关服务未启动
              </div>
              <p style={{ fontSize: 13, marginBottom: 16 }}>
                请先启动统一网关服务，即可快速发现与测试全部可用插件模型。
              </p>
              {onNavigateGateway ? (
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => {
                    onClose();
                    onNavigateGateway();
                  }}
                >
                  前往统一网关启动服务
                </button>
              ) : null}
            </div>
          ) : loading ? (
            <div style={{ padding: "40px 20px", textAlign: "center", color: "var(--muted)", fontSize: 13 }}>
              正在检索统一网关模型列表…
            </div>
          ) : error ? (
            <div style={{ padding: "30px 20px", textAlign: "center" }}>
              <div style={{ color: "var(--red)", fontSize: 13, marginBottom: 8 }}>
                获取模型列表失败: {error}
              </div>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => {
                  setLoading(true);
                  setError(null);
                  invoke<FlightModelListResult>("flight_list_models")
                    .then((res) => {
                      setLoading(false);
                      setModels(Array.isArray(res?.models) ? res.models : []);
                      setActiveBackend(res?.active_backend ?? null);
                    })
                    .catch((err) => {
                      setLoading(false);
                      setError(err instanceof Error ? err.message : String(err));
                    });
                }}
              >
                重试加载
              </button>
            </div>
          ) : filteredModels.length === 0 ? (
            <div style={{ padding: "40px 20px", textAlign: "center", color: "var(--muted)", fontSize: 13 }}>
              {models.length === 0
                ? "统一网关尚未挂载任何模型后端，请先在插件管理中配置或启动插件。"
                : `未找到与 “${query}” 匹配的模型。`}
            </div>
          ) : (
            filteredModels.map((m, idx) => {
              const isSelected = idx === selectedIndex;
              const cat = categorizeModel(m.id);
              const pillColor =
                cat === "claude" ? "pill-amber" : cat === "deepseek" ? "pill-blue" : "pill-green";

              return (
                <div
                  key={m.id}
                  data-index={idx}
                  onClick={() => onSelectModelFlight(m.id)}
                  onMouseEnter={() => setSelectedIndex(idx)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 12,
                    padding: "9px 12px",
                    borderRadius: 8,
                    cursor: "pointer",
                    background: isSelected ? "var(--well)" : "transparent",
                    border: isSelected ? "1px solid var(--accent-line)" : "1px solid transparent",
                    transition: "background 0.1s, border-color 0.1s",
                    marginBottom: 2,
                  }}
                >
                  <div
                    style={{
                      width: 8,
                      height: 8,
                      borderRadius: "50%",
                      background: isSelected ? "var(--blue)" : "var(--green)",
                      flexShrink: 0,
                    }}
                  />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <span
                        style={{
                          fontWeight: 600,
                          fontSize: 13,
                          color: isSelected ? "var(--blue)" : "var(--text)",
                          fontFamily: "var(--mono)",
                        }}
                      >
                        {m.id}
                      </span>
                      <span className={`pill ${pillColor}`} style={{ fontSize: 10, padding: "1px 6px" }}>
                        {cat.toUpperCase()}
                      </span>
                      {m.owned_by ? (
                        <span style={{ fontSize: 11, color: "var(--muted)" }}>by {m.owned_by}</span>
                      ) : null}
                    </div>
                    {m.name && m.name !== m.id ? (
                      <div style={{ fontSize: 11, color: "var(--text2)", marginTop: 2 }}>{m.name}</div>
                    ) : null}
                  </div>

                  <div style={{ display: "flex", alignItems: "center", gap: 6, flexShrink: 0 }}>
                    <button
                      type="button"
                      className="btn btn-ghost"
                      onClick={(e) => handleCopy(m.id, e)}
                      title="复制模型 ID"
                      style={{ padding: "3px 8px", fontSize: 11 }}
                    >
                      {copiedId === m.id ? "✓ 已复制" : "复制"}
                    </button>
                    <button
                      type="button"
                      className="btn btn-primary"
                      onClick={(e) => {
                        e.stopPropagation();
                        onSelectModelFlight(m.id);
                      }}
                      title="直接进入试飞面板并载入此模型"
                      style={{ padding: "3px 10px", fontSize: 11 }}
                    >
                      试飞 →
                    </button>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* 底部快捷键指南 */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "8px 16px",
            borderTop: "1px solid var(--line)",
            background: "var(--well)",
            fontSize: 11,
            color: "var(--muted)",
          }}
        >
          <div style={{ display: "flex", gap: 12 }}>
            <span>
              <kbd style={{ fontFamily: "var(--mono)" }}>↑</kbd> <kbd style={{ fontFamily: "var(--mono)" }}>↓</kbd> 切换选中
            </span>
            <span>
              <kbd style={{ fontFamily: "var(--mono)" }}>Enter</kbd> 立即试飞
            </span>
            <span>
              <kbd style={{ fontFamily: "var(--mono)" }}>Esc</kbd> 关闭
            </span>
          </div>
          <div>
            已匹配 <b>{filteredModels.length}</b> 个模型
          </div>
        </div>
      </div>
    </div>
  );
};
