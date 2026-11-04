import React, { useState, useRef, useEffect, useMemo } from "react";
import { invoke } from "../lib/tauri";
import "./DiscoverPanel.css";
import {
  DownloadProgressEvent,
  ImportResult,
  MarketplaceCatalogResponse,
  MarketplacePluginItem,
  PluginRecord,
} from "../types";
import { InspectProjectResult } from "../api";

export interface DiscoverPanelProps {
  plugins?: PluginRecord[];
  onRefresh?: () => void;
  onNavigatePlugins?: () => void;
  onInstallSuccess?: () => void;
  initialCatalog?: MarketplacePluginItem[];
  initialInspectResult?: InspectProjectResult | null;
  initialRepoUrl?: string;
}

export interface MarketplaceDetailModalProps {
  plugin: MarketplacePluginItem | null;
  isOpen: boolean;
  onClose: () => void;
  onInstall?: (plugin: MarketplacePluginItem) => void;
  onNavigatePlugins?: () => void;
  isInstalled?: boolean;
  installing?: boolean;
}

export interface InstallProgressModalProps {
  isOpen: boolean;
  plugin: MarketplacePluginItem | null;
  progress: DownloadProgressEvent | null;
  error: string | null;
  isSuccess: boolean;
  onClose: () => void;
  onRetry?: () => void;
  onNavigatePlugins?: () => void;
}

export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const val = (bytes / Math.pow(1024, i)).toFixed(1);
  return `${val} ${units[i] || "MB"}`;
}

export function stageDescription(stage?: string): string {
  switch (stage) {
    case "fetching_release":
      return "正在获取 Release 资产与元数据...";
    case "downloading":
      return "正在流式下载资产二进制包...";
    case "verifying_checksum":
      return "正在比对 SHA256 校验和与防篡改验证...";
    case "extracting":
      return "正在进行安全解包 (ZipSlip 防护)...";
    case "registering":
      return "正在校验清单并注册发布插件...";
    case "completed":
      return "插件安装完成！";
    case "error":
      return "安装遇到错误";
    default:
      return stage || "准备中...";
  }
}

export const InstallProgressModal: React.FC<InstallProgressModalProps> = ({
  isOpen,
  plugin,
  progress,
  error,
  isSuccess,
  onClose,
  onRetry,
  onNavigatePlugins,
}) => {
  if (!isOpen || !plugin) return null;

  const percent = progress ? Math.min(100, Math.max(0, Math.round(progress.percent))) : (isSuccess ? 100 : 0);
  const isBusy = !error && !isSuccess;

  return (
    <div
      className="modal-backdrop"
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: "rgba(0, 0, 0, 0.65)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1100,
        backdropFilter: "blur(4px)",
      }}
      onClick={!isBusy ? onClose : undefined}
    >
      <div
        className="card"
        style={{
          width: "90%",
          maxWidth: 480,
          padding: 24,
          position: "relative",
          boxShadow: "0 25px 30px -5px rgba(0, 0, 0, 0.4)",
        }}
        onClick={(e) => e.stopPropagation()}
        id="installProgressModal"
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
          <h3 style={{ margin: 0, fontSize: 17, display: "flex", alignItems: "center", gap: 8 }}>
            <span>📦</span>
            <span>{isSuccess ? "安装成功" : error ? "安装失败" : "正在下载与安装"}</span>
          </h3>
          {!isBusy ? (
            <button
              className="btn btn-ghost btn-sm"
              onClick={onClose}
              aria-label="关闭"
              style={{ fontSize: 18, lineHeight: 1 }}
            >
              ×
            </button>
          ) : null}
        </div>

        <div style={{ marginBottom: 16, display: "flex", alignItems: "center", gap: 10 }}>
          <div
            style={{
              width: 38,
              height: 38,
              borderRadius: 8,
              background: "linear-gradient(135deg, #3b82f6, #8b5cf6)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "#fff",
              fontWeight: 700,
              fontSize: 16,
              flexShrink: 0,
            }}
          >
            {plugin.name.slice(0, 1).toUpperCase()}
          </div>
          <div style={{ overflow: "hidden" }}>
            <div style={{ fontWeight: 600, fontSize: 15, textOverflow: "ellipsis", overflow: "hidden", whiteSpace: "nowrap" }}>
              {plugin.name}
            </div>
            <div className="sub" style={{ fontSize: 12, color: "var(--muted)" }}>
              版本: {plugin.version} · ID: <span className="mono">{plugin.id}</span>
            </div>
          </div>
        </div>

        {/* Progress or Status */}
        <div style={{ marginBottom: 20 }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 13, marginBottom: 8 }}>
            <span style={{ fontWeight: 500, color: error ? "var(--red, #ef4444)" : "var(--text)" }} id="installStageDesc">
              {error ? "安装中止" : stageDescription(progress?.stage)}
            </span>
            <span className="mono" style={{ color: "var(--muted)" }} id="installPercentText">
              {percent}%
            </span>
          </div>

          {/* Progress Bar Container */}
          <div
            style={{
              width: "100%",
              height: 10,
              background: "var(--border, rgba(255,255,255,0.1))",
              borderRadius: 6,
              overflow: "hidden",
              position: "relative",
            }}
          >
            <div
              id="installProgressBar"
              style={{
                width: `${percent}%`,
                height: "100%",
                background: error
                  ? "var(--red, #ef4444)"
                  : isSuccess
                  ? "var(--green, #22c55e)"
                  : "linear-gradient(90deg, #3b82f6, #60a5fa)",
                borderRadius: 6,
                transition: "width 0.25s ease-in-out",
              }}
            />
          </div>

          {/* Byte statistics & message */}
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, color: "var(--muted)", marginTop: 6 }}>
            <span id="installMessageText" style={{ textOverflow: "ellipsis", overflow: "hidden", whiteSpace: "nowrap", maxWidth: "60%" }}>
              {progress?.message || (isSuccess ? "插件已就绪" : "正在连接 GitHub...")}
            </span>
            {progress && progress.total_bytes > 0 ? (
              <span className="mono" id="installBytesStats">
                {formatBytes(progress.downloaded_bytes)} / {formatBytes(progress.total_bytes)}
              </span>
            ) : null}
          </div>
        </div>

        {/* Error Box */}
        {error ? (
          <div
            id="installErrorBox"
            style={{
              marginBottom: 16,
              padding: "10px 14px",
              borderRadius: 8,
              background: "rgba(239, 68, 68, 0.1)",
              border: "1px solid rgba(239, 68, 68, 0.3)",
              fontSize: 13,
              color: "var(--red, #ef4444)",
              lineHeight: 1.5,
              wordBreak: "break-word",
            }}
          >
            <b>安装失败:</b> {error}
          </div>
        ) : null}

        {/* Success Box */}
        {isSuccess ? (
          <div
            id="installSuccessBox"
            style={{
              marginBottom: 16,
              padding: "10px 14px",
              borderRadius: 8,
              background: "rgba(34, 197, 94, 0.1)",
              border: "1px solid rgba(34, 197, 94, 0.3)",
              fontSize: 13,
              color: "var(--green, #22c55e)",
              lineHeight: 1.5,
            }}
          >
            ✓ 插件已成功下载、解包并原子注册发布至系统，随时可以启动与挂载！
          </div>
        ) : null}

        {/* Action Buttons */}
        <div style={{ display: "flex", justifyContent: "flex-end", gap: 10 }}>
          {error ? (
            <>
              <button className="btn btn-soft" onClick={onClose} type="button" id="closeInstallErrorBtn">
                关闭
              </button>
              {onRetry ? (
                <button className="btn btn-primary" onClick={onRetry} type="button" id="retryInstallBtn">
                  重试下载
                </button>
              ) : null}
            </>
          ) : isSuccess ? (
            <>
              <button className="btn btn-soft" onClick={onClose} type="button" id="finishInstallBtn">
                留在市场
              </button>
              {onNavigatePlugins ? (
                <button
                  className="btn btn-primary"
                  onClick={() => {
                    onClose();
                    onNavigatePlugins();
                  }}
                  type="button"
                  id="gotoPluginManageBtn"
                >
                  前往管理插件
                </button>
              ) : null}
            </>
          ) : (
            <button className="btn btn-soft" disabled={true} type="button">
              正在处理...
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

const CATEGORIES = [
  { key: "all", label: "全部" },
  { key: "managed", label: "Managed" },
  { key: "external", label: "External" },
  { key: "proxy", label: "代理转换" },
  { key: "local_model", label: "本地模型" },
  { key: "tools", label: "工具接入" },
];

function categoryLabel(cat: string): string {
  switch (cat) {
    case "proxy":
      return "代理转换";
    case "local_model":
      return "本地模型";
    case "tools":
      return "工具接入";
    case "protocol":
      return "协议适配";
    default:
      return cat;
  }
}

export const MarketplaceDetailModal: React.FC<MarketplaceDetailModalProps> = ({
  plugin,
  isOpen,
  onClose,
  onInstall,
  onNavigatePlugins,
  isInstalled = false,
  installing = false,
}) => {
  if (!isOpen || !plugin) return null;

  return (
    <div
      className="modal-backdrop"
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: "rgba(0, 0, 0, 0.55)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
        backdropFilter: "blur(4px)",
      }}
      onClick={onClose}
    >
      <div
        className="card"
        style={{
          width: "90%",
          maxWidth: 620,
          maxHeight: "85vh",
          overflowY: "auto",
          padding: 24,
          position: "relative",
          boxShadow: "0 20px 25px -5px rgba(0, 0, 0, 0.3)",
        }}
        onClick={(e) => e.stopPropagation()}
        id="marketplaceDetailModal"
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 16 }}>
          <div style={{ display: "flex", gap: 14, alignItems: "center" }}>
            {plugin.icon ? (
              <img
                src={plugin.icon}
                alt={plugin.name}
                style={{ width: 44, height: 44, borderRadius: 8, objectFit: "cover" }}
              />
            ) : (
              <div
                style={{
                  width: 44,
                  height: 44,
                  borderRadius: 8,
                  background: "linear-gradient(135deg, #3b82f6, #8b5cf6)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  color: "#fff",
                  fontWeight: 700,
                  fontSize: 18,
                }}
              >
                {plugin.name.slice(0, 1).toUpperCase()}
              </div>
            )}
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <h3 style={{ margin: 0, fontSize: 18 }}>{plugin.name}</h3>
                <span className="pill pill-gray" style={{ fontSize: 11 }}>v{plugin.version}</span>
              </div>
              <p className="sub" style={{ margin: "4px 0 0 0", color: "var(--muted)" }}>
                ID: <span className="mono">{plugin.id}</span> · 作者: {plugin.author}
              </p>
            </div>
          </div>
          <button
            className="btn btn-ghost btn-sm"
            onClick={onClose}
            aria-label="关闭详情"
            style={{ fontSize: 18, lineHeight: 1 }}
          >
            ×
          </button>
        </div>

        <div style={{ display: "flex", gap: 8, marginBottom: 16, flexWrap: "wrap" }}>
          <span className={`pill ${plugin.kind === "managed" ? "pill-blue" : "pill-violet"}`}>
            {plugin.kind === "managed" ? "Managed 进程托管" : "External 外部挂载"}
          </span>
          <span className="pill pill-gray">
            分类: {categoryLabel(plugin.category)}
          </span>
          <span className="pill pill-gray">
            协议: {plugin.protocol}
          </span>
          {plugin.stars != null ? (
            <span className="pill pill-amber">
              ★ {plugin.stars}
            </span>
          ) : null}
        </div>

        <div style={{ marginBottom: 16 }}>
          <h4 style={{ margin: "0 0 6px 0", fontSize: 13, color: "var(--muted)" }}>插件描述</h4>
          <p style={{ margin: 0, lineHeight: 1.6, fontSize: 14 }}>{plugin.description}</p>
        </div>

        <div style={{ marginBottom: 16 }}>
          <h4 style={{ margin: "0 0 6px 0", fontSize: 13, color: "var(--muted)" }}>代码仓库</h4>
          <a
            href={plugin.repo_url}
            target="_blank"
            rel="noreferrer"
            className="mono"
            style={{ color: "var(--blue)", fontSize: 13, wordBreak: "break-all" }}
          >
            {plugin.repo_url}
          </a>
        </div>

        {plugin.tags && plugin.tags.length > 0 ? (
          <div style={{ marginBottom: 16 }}>
            <h4 style={{ margin: "0 0 6px 0", fontSize: 13, color: "var(--muted)" }}>检索标签</h4>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {plugin.tags.map((t) => (
                <span key={t} className="tag" style={{ fontSize: 11 }}>
                  #{t}
                </span>
              ))}
            </div>
          </div>
        ) : null}

        <div
          style={{
            marginBottom: 20,
            padding: "10px 14px",
            borderRadius: 8,
            background: "rgba(59, 130, 246, 0.08)",
            border: "1px solid rgba(59, 130, 246, 0.2)",
            fontSize: 12,
            color: "var(--text2)",
            lineHeight: 1.6,
          }}
        >
          <b>安全审查与执行边界说明：</b> 插件由 ProxyWeave Win32 Job Object 实施受管进程沙箱隔离（KILL_ON_JOB_CLOSE）或 External 零杀伤防护，遵循最小权限访问与安全网络反向代理。
        </div>

        <div style={{ display: "flex", justifyContent: "flex-end", gap: 10 }}>
          <button className="btn btn-soft" onClick={onClose} type="button">
            关闭
          </button>
          {isInstalled ? (
            <button
              className="btn btn-soft"
              onClick={() => {
                onClose();
                onNavigatePlugins?.();
              }}
              type="button"
            >
              管理已安装插件
            </button>
          ) : (
            <button
              className="btn btn-primary"
              onClick={() => {
                onInstall?.(plugin);
              }}
              disabled={installing}
              type="button"
              id="modalConfirmInstall"
            >
              {installing ? "正在克隆与导入..." : "从仓库克隆安装"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

function validateGithubUrl(url: string): { valid: boolean; error?: string } {
  const trimmed = url.trim();
  if (!trimmed) {
    return { valid: false, error: "GitHub 仓库 URL 不能为空" };
  }
  if (
    trimmed.startsWith("-") ||
    trimmed.includes("\0") ||
    trimmed.includes("\n") ||
    trimmed.includes("\r")
  ) {
    return { valid: false, error: "仓库地址格式非法" };
  }
  if (
    !trimmed.startsWith("https://github.com/") &&
    !trimmed.startsWith("http://github.com/") &&
    !trimmed.startsWith("git@github.com:")
  ) {
    return {
      valid: false,
      error: "当前仅支持标准 GitHub 仓库 URL (如 https://github.com/...)",
    };
  }
  return { valid: true };
}

export const DiscoverPanel: React.FC<DiscoverPanelProps> = ({
  plugins = [],
  onRefresh,
  onNavigatePlugins,
  onInstallSuccess,
  initialCatalog,
  initialInspectResult,
  initialRepoUrl,
}) => {
  const [repoUrl, setRepoUrl] = useState(initialRepoUrl ?? "");
  const [installing, setInstalling] = useState(false);
  const [message, setMessage] = useState<{ text: string; error?: boolean } | null>(null);
  const [lastImportResult, setLastImportResult] = useState<ImportResult | null>(null);
  const [inspecting, setInspecting] = useState(false);
  const [inspectResult, setInspectResult] = useState<InspectProjectResult | null>(
    initialInspectResult ?? null
  );
  const [inspectError, setInspectError] = useState<string | null>(null);

  // Marketplace states
  const [marketItems, setMarketItems] = useState<MarketplacePluginItem[]>(initialCatalog ?? []);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedCategory, setSelectedCategory] = useState("all");
  const [refreshing, setRefreshing] = useState(false);
  const [viewMode, setViewMode] = useState<"cards" | "list">("cards");
  const [notice, setNotice] = useState<string | null>(null);
  const [selectedPlugin, setSelectedPlugin] = useState<MarketplacePluginItem | null>(null);
  const [showGitClone, setShowGitClone] = useState(true);

  // Release downloader modal states
  const [downloadModalOpen, setDownloadModalOpen] = useState(false);
  const [downloadTargetPlugin, setDownloadTargetPlugin] = useState<MarketplacePluginItem | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgressEvent | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [downloadSuccess, setDownloadSuccess] = useState(false);

  const mountedRef = useRef(true);
  const inFlightRef = useRef(false);
  const opIdRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    let isCancelled = false;

    // Load initial marketplace catalog from backend if not already supplied
    if (!initialCatalog) {
      invoke<MarketplaceCatalogResponse>("get_marketplace_catalog", {
        refreshOnline: false,
      })
        .then((res) => {
          if (!isCancelled && mountedRef.current && res) {
            setMarketItems(res.items || []);
            setNotice(res.notice || null);
          }
        })
        .catch(() => {
          // Keep offline state gracefully without breaking UI
        });
    }

    return () => {
      isCancelled = true;
      mountedRef.current = false;
    };
  }, [initialCatalog]);

  const isInstalled = (id: string) =>
    plugins.some((p) => p.manifest?.metadata?.id === id);

  function handleUrlChange(newUrl: string) {
    setRepoUrl(newUrl);
    if (message) setMessage(null);
    if (lastImportResult) setLastImportResult(null);
    if (inspectResult) setInspectResult(null);
    if (inspectError) setInspectError(null);
    opIdRef.current++;
  }

  async function handleRefreshMarket() {
    if (refreshing) return;
    setRefreshing(true);
    setNotice(null);
    try {
      const res = await invoke<MarketplaceCatalogResponse>("get_marketplace_catalog", {
        refreshOnline: true,
      });
      if (mountedRef.current && res) {
        setMarketItems(res.items || []);
        setNotice(res.notice || (res.is_offline ? "网络离线，已降级展示内置生态推荐清单" : "已完成 GitHub 在线市场索引刷新"));
      }
    } catch (e: any) {
      if (mountedRef.current) {
        setNotice(`刷新失败: ${typeof e === "string" ? e : e?.message || String(e)}`);
      }
    } finally {
      if (mountedRef.current) {
        setRefreshing(false);
      }
    }
  }

  async function handleInstallFromUrl(targetUrl: string) {
    const trimmedUrl = targetUrl.trim();
    const validation = validateGithubUrl(trimmedUrl);
    if (!validation.valid) {
      setMessage({ text: validation.error || "仓库地址格式非法", error: true });
      return;
    }
    if (inFlightRef.current || installing) return;

    inFlightRef.current = true;
    setInstalling(true);
    setMessage(null);
    const currentOpId = ++opIdRef.current;

    try {
      const res = await invoke<ImportResult>("import_project_from_github", {
        url: trimmedUrl,
      });
      if (!mountedRef.current || currentOpId !== opIdRef.current) return;

      setLastImportResult(res);
      setInspectResult(null);
      setInspectError(null);
      const pluginName =
        res?.record?.manifest?.metadata?.name ||
        res?.record?.manifest?.metadata?.id ||
        "未知插件";
      const pluginVer = res?.record?.manifest?.metadata?.version || "未知";
      setMessage({
        text: res?.already_registered
          ? `插件 ${pluginName} 已注册，已重新加载配置。`
          : `成功克隆并安全导入插件: ${pluginName} (v${pluginVer})！`,
      });
      onRefresh?.();
      onInstallSuccess?.();
      setSelectedPlugin(null);
    } catch (e: any) {
      if (!mountedRef.current || currentOpId !== opIdRef.current) return;
      setLastImportResult(null);
      setMessage({
        text: `克隆 GitHub 仓库并导入失败: ${
          typeof e === "string" ? e : e?.message || String(e)
        }`,
        error: true,
      });
    } finally {
      inFlightRef.current = false;
      if (mountedRef.current) {
        setInstalling(false);
      }
    }
  }

  async function handleConfirmInstall() {
    await handleInstallFromUrl(repoUrl);
  }

  async function handleInspectRepo() {
    const trimmedUrl = repoUrl.trim();
    const validation = validateGithubUrl(trimmedUrl);
    if (!validation.valid) {
      setMessage({ text: validation.error || "仓库地址格式非法", error: true });
      return;
    }
    if (inFlightRef.current || installing || inspecting) return;

    setInspecting(true);
    setInspectError(null);
    setInspectResult(null);
    setMessage(null);
    const currentOpId = ++opIdRef.current;

    try {
      const res = await invoke<InspectProjectResult>("inspect_project_from_github", {
        url: trimmedUrl,
      });
      if (!mountedRef.current || currentOpId !== opIdRef.current) return;

      setInspectResult(res);
      setMessage({
        text: `成功解析清单: ${res.name} (v${res.version})`,
      });
    } catch (e: any) {
      if (!mountedRef.current || currentOpId !== opIdRef.current) return;
      const errMsg = typeof e === "string" ? e : e?.message || String(e);
      setInspectError(errMsg);
      setMessage({
        text: `解析清单失败: ${errMsg}`,
        error: true,
      });
    } finally {
      if (mountedRef.current) {
        setInspecting(false);
      }
    }
  }

  async function triggerReleaseInstall(item: MarketplacePluginItem) {
    if (inFlightRef.current || installing) return;
    setDownloadTargetPlugin(item);
    setDownloadModalOpen(true);
    setDownloadError(null);
    setDownloadSuccess(false);
    setDownloadProgress({
      plugin_id: item.id,
      stage: "fetching_release",
      downloaded_bytes: 0,
      total_bytes: 0,
      percent: 0,
      message: "准备连接 GitHub 获取最新 Release 资产...",
    });

    inFlightRef.current = true;
    setInstalling(true);
    setMessage(null);
    const currentOpId = ++opIdRef.current;

    let unlisten: (() => void) | null = null;
    try {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen<DownloadProgressEvent>(
          "plugin_download_progress",
          (e) => {
            if (!mountedRef.current || currentOpId !== opIdRef.current) return;
            if (e.payload.plugin_id === item.id) {
              setDownloadProgress(e.payload);
            }
          }
        );
      } catch {
        // Fallback for non-Tauri / test environments
      }

      const res = await invoke<ImportResult>("install_plugin_from_release", {
        repoUrl: item.repo_url,
        pluginId: item.id,
        tag: item.release_tag || null,
        expectedSha256: null,
      });

      if (!mountedRef.current || currentOpId !== opIdRef.current) return;

      setLastImportResult(res);
      setDownloadSuccess(true);
      setDownloadProgress({
        plugin_id: item.id,
        stage: "completed",
        downloaded_bytes: 0,
        total_bytes: 0,
        percent: 100,
        message: "插件已成功安装并注册！",
      });

      const pluginName =
        res?.record?.manifest?.metadata?.name || item.name || item.id;
      const pluginVer = res?.record?.manifest?.metadata?.version || item.version;
      setMessage({
        text: res?.already_registered
          ? `插件 ${pluginName} 已注册，已重新加载配置。`
          : `成功下载并安全安装插件: ${pluginName} (v${pluginVer})！`,
      });
      onRefresh?.();
      onInstallSuccess?.();
    } catch (e: any) {
      if (!mountedRef.current || currentOpId !== opIdRef.current) return;
      const errMsg = typeof e === "string" ? e : e?.message || String(e);
      setDownloadError(errMsg);
      setDownloadProgress((prev) =>
        prev
          ? { ...prev, stage: "error", message: `安装失败: ${errMsg}` }
          : {
              plugin_id: item.id,
              stage: "error",
              downloaded_bytes: 0,
              total_bytes: 0,
              percent: 0,
              message: `安装失败: ${errMsg}`,
            }
      );
    } finally {
      if (unlisten) {
        try {
          unlisten();
        } catch {
          // ignore unlisten error
        }
      }
      inFlightRef.current = false;
      if (mountedRef.current) {
        setInstalling(false);
      }
    }
  }

  function handleMarketplaceInstall(item: MarketplacePluginItem) {
    triggerReleaseInstall(item);
  }

  // Filtered market items
  const filteredMarketItems = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const cat = selectedCategory.trim().toLowerCase();

    return marketItems.filter((item) => {
      // Category filter
      if (cat !== "all" && cat !== "全部") {
        const matchesKind = (cat === "managed" || cat === "external") && item.kind.toLowerCase() === cat;
        const matchesCat = item.category.toLowerCase() === cat;
        if (!matchesKind && !matchesCat) return false;
      }

      // Query filter
      if (q) {
        const inId = item.id.toLowerCase().includes(q);
        const inName = item.name.toLowerCase().includes(q);
        const inDesc = item.description.toLowerCase().includes(q);
        const inAuthor = item.author.toLowerCase().includes(q);
        const inTags = item.tags.some((t) => t.toLowerCase().includes(q));
        if (!inId && !inName && !inDesc && !inAuthor && !inTags) return false;
      }

      return true;
    });
  }, [marketItems, searchQuery, selectedCategory]);

  return (
    <section className="panel" id="panel-discover">
      <div className="topbar">
        <div>
          <h2>插件生态与市场</h2>
          <p className="sub">探索、检索与安装 ProxyWeave 官方与社区插件生态</p>
        </div>
        <div className="discover-actions">
          <button
            className="btn btn-soft"
            id="refreshMarket"
            type="button"
            onClick={handleRefreshMarket}
            disabled={refreshing}
          >
            {refreshing ? "正在刷新市场..." : "刷新市场"}
          </button>
          <button
            className="btn btn-soft"
            id="toggleGitClone"
            type="button"
            onClick={() => setShowGitClone(!showGitClone)}
          >
            {showGitClone ? "收起 GitHub 源码安装" : "从 GitHub 源码安装"}
          </button>
        </div>
      </div>

      {message ? (
        <div
          id="discoverFeedback"
          style={{
            marginBottom: 16,
            padding: "10px 14px",
            borderRadius: 8,
            fontSize: 13,
            background: message.error
              ? "rgba(239, 68, 68, 0.12)"
              : "rgba(34, 197, 94, 0.12)",
            color: message.error ? "var(--red)" : "var(--green)",
            border: `1px solid ${
              message.error ? "rgba(239, 68, 68, 0.3)" : "rgba(34, 197, 94, 0.3)"
            }`,
            lineHeight: 1.6,
            wordBreak: "break-word",
          }}
        >
          {message.text}
        </div>
      ) : null}

      <div className="stack">
        {/* Marketplace Section */}
        <div className="card">
          <div className="discover-filters">
            <div className="discover-search">
              <input
                className="input"
                id="marketSearch"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="搜索插件名称、标签、描述或作者..."
                style={{ width: "100%", paddingRight: searchQuery ? 32 : 12 }}
              />
              {searchQuery ? (
                <button
                  type="button"
                  id="clearMarketSearch"
                  onClick={() => setSearchQuery("")}
                  style={{
                    position: "absolute",
                    right: 8,
                    top: "50%",
                    transform: "translateY(-50%)",
                    background: "transparent",
                    border: "none",
                    cursor: "pointer",
                    color: "var(--muted)",
                    fontSize: 16,
                    padding: "2px 6px",
                  }}
                  title="清空搜索"
                >
                  ×
                </button>
              ) : null}
            </div>

            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {CATEGORIES.map((cat) => (
                <button
                  key={cat.key}
                  id={`cat-filter-${cat.key}`}
                  className={`btn btn-sm ${selectedCategory === cat.key ? "btn-primary" : "btn-soft"}`}
                  type="button"
                  onClick={() => setSelectedCategory(cat.key)}
                >
                  {cat.label}
                </button>
              ))}
            </div>
          </div>

          {notice ? (
            <div
              id="marketplaceNotice"
              style={{
                marginBottom: 16,
                padding: "8px 12px",
                borderRadius: 6,
                background: "rgba(59, 130, 246, 0.08)",
                border: "1px solid rgba(59, 130, 246, 0.2)",
                fontSize: 12,
                color: "var(--text2)",
                overflowWrap: "anywhere",
              }}
            >
              ℹ {notice}
            </div>
          ) : null}

          <div className="discover-view-toolbar">
            <span className="sub" role="status">共 {filteredMarketItems.length} 个结果</span>
            <div className="discover-view-switch" role="group" aria-label="发现页显示模式">
              <button type="button" className={`btn btn-sm ${viewMode === "list" ? "btn-primary" : "btn-soft"}`}
                aria-pressed={viewMode === "list"} onClick={() => setViewMode("list")}>简易列表</button>
              <button type="button" className={`btn btn-sm ${viewMode === "cards" ? "btn-primary" : "btn-soft"}`}
                aria-pressed={viewMode === "cards"} onClick={() => setViewMode("cards")}>详情卡片</button>
            </div>
          </div>
          {filteredMarketItems.length > 0 ? (
            <div className={`discover-results discover-results--${viewMode}`} id="marketplaceCardsGrid" data-view={viewMode}>
              {filteredMarketItems.map((item) => {
                const installed = isInstalled(item.id);
                return (
                  <div
                    className="card discover-item"
                    key={item.id}
                    data-plugin-id={item.id}
                    style={{
                      margin: 0,
                      display: "flex",
                      flexDirection: "column",
                      justifyContent: "space-between",
                      transition: "box-shadow 0.2s ease",
                      border: "1px solid var(--border)",
                    }}
                  >
                    <div className="discover-item-content">
                      {/* Card Header */}
                      <div className="discover-item-header">
                        <div className="discover-item-identity">
                          {item.icon ? (
                            <img
                              src={item.icon}
                              alt={item.name}
                              style={{ width: 36, height: 36, borderRadius: 6, objectFit: "cover" }}
                            />
                          ) : (
                            <div
                              style={{
                                width: 36,
                                height: 36,
                                borderRadius: 6,
                                background: item.kind === "managed" ? "var(--blue)" : "var(--violet)",
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                                color: "#fff",
                                fontWeight: 700,
                                fontSize: 15,
                              }}
                            >
                              {item.name.slice(0, 1).toUpperCase()}
                            </div>
                          )}
                          <div className="discover-item-title">
                            <div className="discover-item-name">
                              <b style={{ fontSize: 15 }}>{item.name}</b>
                              <span className="pill pill-gray" style={{ fontSize: 10 }}>v{item.version}</span>
                            </div>
                            <span className="sub" style={{ fontSize: 11, color: "var(--muted)" }}>
                              {item.author}
                            </span>
                          </div>
                        </div>

                        <div className="discover-item-badges">
                          <span className={`tag ${item.kind === "managed" ? "tag-blue" : "tag-purple"}`} style={{ fontSize: 10 }}>
                            {item.kind === "managed" ? "Managed" : "External"}
                          </span>
                          {item.stars != null ? (
                            <span className="pill pill-amber" style={{ fontSize: 10 }}>
                              ★ {item.stars}
                            </span>
                          ) : null}
                        </div>
                      </div>

                      {/* Card Body */}
                      <p
                        className="sub"
                        data-market-description
                        style={{
                          margin: "6px 0 12px 0",
                          lineHeight: 1.5,
                          fontSize: 12,
                          color: "var(--text2)",
                          display: "-webkit-box",
                          WebkitLineClamp: 2,
                          WebkitBoxOrient: "vertical",
                          overflow: "hidden",
                        }}
                      >
                        {item.description}
                      </p>

                      <div className="discover-item-tags" style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 12 }}>
                        <span className="pill pill-gray" style={{ fontSize: 10 }}>
                          {categoryLabel(item.category)}
                        </span>
                        <span className="pill pill-gray" style={{ fontSize: 10 }}>
                          {item.protocol}
                        </span>
                      </div>
                    </div>

                    {/* Card Footer */}
                    <div
                      className="discover-item-footer"
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                        paddingTop: 10,
                        borderTop: "1px solid var(--border)",
                      }}
                    >
                      {installed ? (
                        <span className="pill pill-green" style={{ fontSize: 11 }}>
                          ✓ 已安装
                        </span>
                      ) : (
                        <button
                          className="btn btn-primary btn-sm"
                          id={`installBtn-${item.id}`}
                          type="button"
                          onClick={() => handleMarketplaceInstall(item)}
                          disabled={installing}
                        >
                          下载安装
                        </button>
                      )}

                      <div style={{ display: "flex", gap: 6 }}>
                        {installed ? (
                          <button
                            className="btn btn-soft btn-sm"
                            type="button"
                            onClick={() => onNavigatePlugins?.()}
                          >
                            管理插件
                          </button>
                        ) : null}
                        <button
                          className="btn btn-soft btn-sm"
                          id={`detailBtn-${item.id}`}
                          type="button"
                          onClick={() => setSelectedPlugin(item)}
                        >
                          详情
                        </button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div
              id="marketplaceEmpty"
              style={{
                textAlign: "center",
                padding: "24px 16px",
                color: "var(--muted)",
                fontSize: 13,
              }}
            >
              未找到匹配的插件
              {searchQuery || selectedCategory !== "all" ? (
                <div style={{ marginTop: 8 }}>
                  <button
                    className="btn btn-soft btn-sm"
                    type="button"
                    onClick={() => {
                      setSearchQuery("");
                      setSelectedCategory("all");
                    }}
                  >
                    重置筛选条件
                  </button>
                </div>
              ) : null}
            </div>
          )}
        </div>

        {/* GitHub Clone Section (Collapsible) */}
        {showGitClone ? (
          <div className="card" id="gitCloneSection">
            <h3>从 GitHub 安装</h3>
            <div className="field" style={{ marginTop: 14 }}>
              <input
                className="input mono"
                id="repoUrl"
                value={repoUrl}
                onChange={(e) => handleUrlChange(e.target.value)}
                style={{ flex: 1, minWidth: 0, width: "100%" }}
                placeholder="https://github.com/owner/repository"
                disabled={installing || inspecting}
              />
              <button
                className="btn btn-soft"
                id="parseRepo"
                type="button"
                onClick={handleInspectRepo}
                disabled={installing || inspecting || !repoUrl.trim()}
                title="在隔离临时沙箱中只读解析插件清单"
              >
                {inspecting ? "正在解析清单..." : "解析清单"}
              </button>
              <button
                className="btn btn-primary"
                id="confirmInstall"
                type="button"
                onClick={handleConfirmInstall}
                disabled={installing || inspecting || !repoUrl.trim()}
              >
                {installing ? "正在克隆与导入..." : "克隆并安装"}
              </button>
            </div>

            <p className="sub" style={{ marginTop: 8, color: "var(--muted)" }}>
              支持在正式克隆安装前执行只读沙箱预检，安全解析插件清单、运行方式与权限声明（需系统已安装 git）。
            </p>

            {inspectError ? (
              <div
                id="inspectError"
                style={{
                  marginTop: 12,
                  padding: "8px 12px",
                  borderRadius: "var(--radius-sm, 4px)",
                  background: "rgba(239, 68, 68, 0.1)",
                  border: "1px solid rgba(239, 68, 68, 0.3)",
                  fontSize: 13,
                  color: "var(--red, #ef4444)",
                }}
              >
                解析清单失败: {inspectError}
              </div>
            ) : null}

            {inspectResult ? (
              <div
                id="inspectResult"
                style={{
                  marginTop: 16,
                  padding: 16,
                  borderRadius: "var(--radius, 8px)",
                  background: "var(--bg-card, rgba(255, 255, 255, 0.04))",
                  border: "1px solid var(--border, rgba(255, 255, 255, 0.1))",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                    flexWrap: "wrap",
                    gap: 8,
                  }}
                >
                  <div>
                    <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                      <b style={{ fontSize: 16 }}>{inspectResult.name}</b>
                      <span className="pill pill-green">只读预检通过</span>
                      <span className={`pill ${inspectResult.runtime_kind === "managed" ? "pill-blue" : "pill-violet"}`}>
                        {inspectResult.runtime_kind === "managed" ? "托管进程 (Managed)" : "外部服务 (External)"}
                      </span>
                    </div>
                    <p className="sub" style={{ marginTop: 4, color: "var(--muted)" }}>
                      {inspectResult.description || "无项目描述"}
                    </p>
                  </div>
                  <div style={{ display: "flex", gap: 8 }}>
                    <button
                      className="btn btn-primary btn-sm"
                      id="inspectConfirmInstall"
                      type="button"
                      onClick={handleConfirmInstall}
                      disabled={installing || inspecting}
                    >
                      {installing ? "正在克隆与导入..." : "确认克隆并安装"}
                    </button>
                    <button
                      className="btn btn-soft btn-sm"
                      id="clearInspectResult"
                      type="button"
                      onClick={() => setInspectResult(null)}
                      disabled={installing || inspecting}
                    >
                      清除预览
                    </button>
                  </div>
                </div>

                <div className="grid2" style={{ marginTop: 14, gap: 12 }}>
                  <div>
                    <div style={{ fontSize: 13, lineHeight: 1.8 }}>
                      <div>
                        <b>插件 ID:</b> <code className="code-inline">{inspectResult.id}</code>
                      </div>
                      <div>
                        <b>版本:</b> v{inspectResult.version}
                      </div>
                      <div>
                        <b>作者:</b> {inspectResult.author || "未声明"}
                      </div>
                      {inspectResult.license ? (
                        <div>
                          <b>许可证:</b> {inspectResult.license}
                        </div>
                      ) : null}
                      {inspectResult.homepage ? (
                        <div>
                          <b>主页:</b>{" "}
                          <a
                            href={inspectResult.homepage}
                            target="_blank"
                            rel="noreferrer"
                            style={{ color: "var(--accent)" }}
                          >
                            {inspectResult.homepage}
                          </a>
                        </div>
                      ) : null}
                      <div>
                        <b>运行入口:</b>{" "}
                        <code className="code-inline">{inspectResult.entrypoint || "未指定"}</code>
                      </div>
                      <div>
                        <b>代码 SHA256:</b>{" "}
                        <code className="code-inline" title={inspectResult.sha256}>
                          {inspectResult.sha256 ? `${inspectResult.sha256.slice(0, 16)}...` : "无"}
                        </code>
                      </div>
                    </div>
                  </div>

                  <div>
                    <div style={{ fontSize: 13, lineHeight: 1.8 }}>
                      <div>
                        <b>协议能力:</b>{" "}
                        {inspectResult.protocols.length > 0 ? (
                          inspectResult.protocols.map((p) => (
                            <span key={p} className="pill" style={{ marginRight: 4 }}>
                              {p}
                            </span>
                          ))
                        ) : (
                          <span style={{ color: "var(--muted)" }}>未声明协议</span>
                        )}
                      </div>
                      <div style={{ marginTop: 4 }}>
                        <b>导出模型 ({inspectResult.exported_models.length}):</b>{" "}
                        {inspectResult.exported_models.length > 0 ? (
                          <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 2 }}>
                            {inspectResult.exported_models.map((m) => (
                              <span key={m} className="pill pill-soft">
                                {m}
                              </span>
                            ))}
                          </div>
                        ) : (
                          <span style={{ color: "var(--muted)" }}>未声明模型</span>
                        )}
                      </div>
                      <div style={{ marginTop: 4 }}>
                        <b>权限与安全声明:</b>{" "}
                        {inspectResult.permissions.length > 0 ? (
                          inspectResult.permissions.map((perm) => (
                            <span key={perm} className="pill pill-amber" style={{ marginRight: 4 }}>
                              {perm}
                            </span>
                          ))
                        ) : (
                          <span style={{ color: "var(--muted)" }}>无特殊权限声明</span>
                        )}
                      </div>
                      {inspectResult.requires_secrets.length > 0 ? (
                        <div style={{ marginTop: 4 }}>
                          <b>依赖凭据:</b>{" "}
                          {inspectResult.requires_secrets.map((s) => (
                            <span key={s} className="pill pill-red" style={{ marginRight: 4 }}>
                              {s}
                            </span>
                          ))}
                        </div>
                      ) : null}
                    </div>
                  </div>
                </div>

                <div
                  style={{
                    marginTop: 12,
                    padding: "8px 12px",
                    borderRadius: "var(--radius-sm, 4px)",
                    background: "rgba(234, 179, 8, 0.1)",
                    border: "1px solid rgba(234, 179, 8, 0.25)",
                    fontSize: 12,
                    color: "var(--amber, #eab308)",
                  }}
                >
                  提示：只读预检在隔离临时沙箱中执行，零写入、零持久化；独立子进程不等于操作系统底层安全沙箱。
                </div>
              </div>
            ) : null}

            {lastImportResult ? (
              <div id="parseResult" style={{ marginTop: 16 }}>
                <div className="grid2">
                  <div>
                    <b style={{ fontSize: 15 }}>
                      {lastImportResult.record?.manifest?.metadata?.name ||
                        lastImportResult.record?.manifest?.metadata?.id}
                    </b>{" "}
                    <span className="pill pill-green">已导入</span>
                    <p className="sub" style={{ marginTop: 8, lineHeight: 1.8 }}>
                      ID: {lastImportResult.record?.manifest?.metadata?.id} · 版本: v
                      {lastImportResult.record?.manifest?.metadata?.version}
                      <br />
                      类型: {String((lastImportResult.record?.manifest?.runtime as any)?.kind ?? "")}
                      <br />
                      入口: {String((lastImportResult.record?.manifest?.runtime as any)?.entrypoint ?? "")}
                    </p>
                  </div>
                  <div>
                    <pre className="code-light" style={{ margin: 0, fontSize: 12 }}>
                      {JSON.stringify(
                        {
                          metadata: lastImportResult.record?.manifest?.metadata,
                          runtime: lastImportResult.record?.manifest?.runtime,
                          already_registered: lastImportResult.already_registered,
                        },
                        null,
                        2
                      )}
                    </pre>
                  </div>
                </div>
              </div>
            ) : null}
          </div>
        ) : null}

        {/* Existing Topic Reference Table */}
        <div className="card">
          <div className="section-head">
            <div>
              <h3>
                Topic 发现 ·{" "}
                <span className="mono" style={{ color: "var(--blue)" }}>
                  #proxyweave-plugin
                </span>
              </h3>
            </div>
            <button
              className="btn btn-soft btn-sm"
              type="button"
              onClick={() => onRefresh?.()}
            >
              刷新发现
            </button>
          </div>
          <table>
            <thead>
              <tr>
                <th>插件</th>
                <th>说明</th>
                <th>生态状态</th>
                <th>类型</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <b>wb2api</b>
                  <p className="sub">WorkBuddy → OpenAI 兼容</p>
                </td>
                <td className="sub">官方首个 Managed 示例切片</td>
                <td>
                  <span className="pill pill-amber">生态展示 · 待接入</span>
                </td>
                <td>
                  <span className="tag">Managed</span>
                </td>
                <td>
                  {isInstalled("wb2api") ? (
                    <span className="pill pill-gray">已安装</span>
                  ) : (
                    <button
                      className="btn btn-soft btn-sm"
                      type="button"
                      disabled={true}
                      title="未接入在线发现中心索引，无法直接安装"
                    >
                      尚未接入
                    </button>
                  )}
                </td>
              </tr>
              <tr>
                <td>
                  <b>external-cpa</b>
                  <p className="sub">CLIProxyAPI 挂载清单</p>
                </td>
                <td className="sub">External 协议接入模板</td>
                <td>
                  <span className="pill pill-amber">生态展示 · 待接入</span>
                </td>
                <td>
                  <span className="tag">External</span>
                </td>
                <td>
                  {isInstalled("external-cpa") ? (
                    <span className="pill pill-gray">已安装</span>
                  ) : (
                    <button
                      className="btn btn-soft btn-sm"
                      type="button"
                      disabled={true}
                      title="未接入在线发现中心索引，无法直接安装"
                    >
                      尚未接入
                    </button>
                  )}
                </td>
              </tr>
              <tr>
                <td>
                  <b>ollama-bridge</b>
                  <p className="sub">本地 Ollama 模型桥接</p>
                </td>
                <td className="sub">社区插件模板</td>
                <td>
                  <span className="pill pill-amber">生态展示 · 待接入</span>
                </td>
                <td>
                  <span className="tag">Managed</span>
                </td>
                <td>
                  {isInstalled("ollama-bridge") ? (
                    <span className="pill pill-gray">已安装</span>
                  ) : (
                    <button
                      className="btn btn-soft btn-sm"
                      type="button"
                      disabled={true}
                      title="未接入在线发现中心索引，无法直接安装"
                    >
                      尚未接入
                    </button>
                  )}
                </td>
              </tr>
              <tr>
                <td>
                  <b>vllm-external</b>
                  <p className="sub">局域网 vLLM 服务挂载模板</p>
                </td>
                <td className="sub">External 插件模板</td>
                <td>
                  <span className="pill pill-amber">生态展示 · 待接入</span>
                </td>
                <td>
                  <span className="tag">External</span>
                </td>
                <td>
                  {isInstalled("vllm-external") ? (
                    <span className="pill pill-gray">已安装</span>
                  ) : (
                    <button
                      className="btn btn-soft btn-sm"
                      type="button"
                      disabled={true}
                      title="未接入在线发现中心索引，无法直接安装"
                    >
                      尚未接入
                    </button>
                  )}
                </td>
              </tr>
              <tr>
                <td>
                  <b>my-local-plugin</b>
                  <p className="sub">从本地目录导入</p>
                </td>
                <td className="sub">本地开发与测试工程</td>
                <td>
                  <span className="pill pill-violet">本地导入</span>
                </td>
                <td>
                  <span className="tag">Managed</span>
                </td>
                <td>
                  <button
                    className="btn btn-ghost btn-sm"
                    type="button"
                    onClick={() => onNavigatePlugins?.()}
                  >
                    选择目录…
                  </button>
                </td>
              </tr>
            </tbody>
          </table>

          <div
            style={{
              marginTop: 14,
              padding: "10px 14px",
              borderRadius: 8,
              background: "rgba(245, 158, 11, 0.08)",
              border: "1px solid rgba(245, 158, 11, 0.2)",
              fontSize: 13,
              color: "var(--text2)",
              lineHeight: 1.6,
              wordBreak: "break-word",
            }}
          >
            <b>发现功能缺口说明：</b> 当前后端未实现只读清单解析 (inspect_manifest)、安全沙箱审查与在线中心市场索引。列表中生态展示插件无法自动安装；请通过上方输入真实标准 GitHub 仓库 URL 直接导入，或在本地插件管理中导入本地目录。
          </div>
        </div>
      </div>

      <MarketplaceDetailModal
        plugin={selectedPlugin}
        isOpen={selectedPlugin !== null}
        onClose={() => setSelectedPlugin(null)}
        onInstall={handleMarketplaceInstall}
        onNavigatePlugins={onNavigatePlugins}
        isInstalled={selectedPlugin ? isInstalled(selectedPlugin.id) : false}
        installing={installing}
      />

      <InstallProgressModal
        isOpen={downloadModalOpen}
        plugin={downloadTargetPlugin}
        progress={downloadProgress}
        error={downloadError}
        isSuccess={downloadSuccess}
        onClose={() => {
          setDownloadModalOpen(false);
          setDownloadTargetPlugin(null);
          setDownloadError(null);
          setDownloadSuccess(false);
          setDownloadProgress(null);
        }}
        onRetry={
          downloadTargetPlugin
            ? () => triggerReleaseInstall(downloadTargetPlugin)
            : undefined
        }
        onNavigatePlugins={onNavigatePlugins}
      />
    </section>
  );
};
