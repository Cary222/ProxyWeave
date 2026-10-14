import React, { useState } from "react";
import { PluginRecord, PluginStatusInfo, SecretStatusInfo, GatewayStatusInfo } from "../types";
import { invoke } from "../lib/tauri";
import { PluginDetails } from "./PluginDetails";

export type ImportMode = "directory" | "github" | "manifest";

interface PluginPanelProps {
  dataReady?: boolean;
  detail?: boolean;
  plugins: PluginRecord[];
  pluginStatuses: Record<string, PluginStatusInfo>;
  secretsStatuses: Record<string, SecretStatusInfo[]>;
  gatewayStatus: GatewayStatusInfo | null;
  selectedPluginId?: string | null;
  onRefresh: () => void;
  onOpen?: (id: string) => void;
  onSelectPlugin?: (id: string) => void;
  onBack?: () => void;
  onNavigateFlight?: (id?: string) => void;
  onFlightPlugin?: (id?: string) => void;
}

export const PluginPanel: React.FC<PluginPanelProps> = ({
  dataReady = true,
  detail = false,
  plugins,
  pluginStatuses,
  secretsStatuses,
  gatewayStatus,
  selectedPluginId = null,
  onRefresh,
  onOpen,
  onSelectPlugin,
  onBack,
  onNavigateFlight,
  onFlightPlugin,
}) => {
  const handleOpen = onSelectPlugin || onOpen;
  const handleFlight = onFlightPlugin || onNavigateFlight;

  const [importMode, setImportMode] = useState<ImportMode>("directory");
  const [dirPath, setDirPath] = useState("");
  const [gitUrl, setGitUrl] = useState("");
  const [manifestPath, setManifestPath] = useState("");
  const [importing, setImporting] = useState(false);
  const [actionLoading, setActionLoading] = useState<Record<string, boolean>>({});
  const [message, setMessage] = useState<{ text: string; error?: boolean } | null>(null);

  // If detail mode is enabled or a specific plugin is selected, show detail view
  const targetPlugin = selectedPluginId
    ? plugins.find((p) => p.manifest.metadata.id === selectedPluginId)
    : detail && plugins.length > 0
    ? plugins[0]
    : null;

  async function handleImportDirectory(e: React.FormEvent) {
    e.preventDefault();
    if (!dirPath.trim()) return;

    setImporting(true);
    setMessage(null);
    try {
      const res: any = await invoke("import_project_from_directory", { path: dirPath.trim() });
      setMessage({
        text: res?.already_registered
          ? `插件 ${res.record?.manifest?.metadata?.name || res.record?.manifest?.metadata?.id} 已注册并通过安全校验。`
          : `成功探测并导入项目 ${res?.record?.manifest?.metadata?.name || res?.record?.manifest?.metadata?.id || ""}！`,
      });
      setDirPath("");
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `导入本地项目失败：${e}`, error: true });
    } finally {
      setImporting(false);
    }
  }

  async function handleImportGithub(e: React.FormEvent) {
    e.preventDefault();
    if (!gitUrl.trim()) return;

    setImporting(true);
    setMessage(null);
    try {
      const res: any = await invoke("import_project_from_github", { url: gitUrl.trim() });
      setMessage({
        text: res?.already_registered
          ? `插件 ${res.record?.manifest?.metadata?.name || res.record?.manifest?.metadata?.id} 已存在，已重新加载配置。`
          : `成功从 GitHub 仓库克隆并安全导入 ${res?.record?.manifest?.metadata?.name || res?.record?.manifest?.metadata?.id || ""}！`,
      });
      setGitUrl("");
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `克隆并导入 GitHub 仓库失败：${e}`, error: true });
    } finally {
      setImporting(false);
    }
  }

  async function handleImportManifest(e: React.FormEvent) {
    e.preventDefault();
    if (!manifestPath.trim()) return;

    setImporting(true);
    setMessage(null);
    try {
      const res: any = await invoke("import_local_plugin", { manifestPath: manifestPath.trim() });
      setMessage({
        text: res?.already_registered
          ? `清单包 ${res.record?.manifest?.metadata?.name || res.record?.manifest?.metadata?.id} 已更新。`
          : `成功导入清单定义 ${res?.record?.manifest?.metadata?.name || res?.record?.manifest?.metadata?.id || ""}！`,
      });
      setManifestPath("");
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `解析清单失败：${e}`, error: true });
    } finally {
      setImporting(false);
    }
  }

  async function handleStart(id: string) {
    setActionLoading((prev) => ({ ...prev, [id]: true }));
    setMessage(null);
    try {
      await invoke("start_plugin", { id });
      setMessage({ text: `插件 ${id} 启动指令已发出，正在建立健康探针。` });
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `启动失败: ${e}`, error: true });
    } finally {
      setActionLoading((prev) => ({ ...prev, [id]: false }));
    }
  }

  async function handleStop(id: string) {
    setActionLoading((prev) => ({ ...prev, [id]: true }));
    setMessage(null);
    try {
      await invoke("stop_plugin", { id });
      setMessage({ text: `插件 ${id} 已停止。` });
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `停止失败: ${e}`, error: true });
    } finally {
      setActionLoading((prev) => ({ ...prev, [id]: false }));
    }
  }

  async function handleRestart(id: string) {
    setActionLoading((prev) => ({ ...prev, [id]: true }));
    setMessage(null);
    try {
      await invoke("restart_plugin", { id });
      setMessage({ text: `插件 ${id} 已重启。` });
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `重启失败: ${e}`, error: true });
    } finally {
      setActionLoading((prev) => ({ ...prev, [id]: false }));
    }
  }

  async function handleSelectBackend(id: string) {
    setActionLoading((prev) => ({ ...prev, [id]: true }));
    setMessage(null);
    try {
      await invoke("select_plugin", { id });
      setMessage({ text: `统一网关已成功挂载后端插件: ${id}` });
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `挂载后端失败: ${e}`, error: true });
    } finally {
      setActionLoading((prev) => ({ ...prev, [id]: false }));
    }
  }

  if (targetPlugin) {
    const id = targetPlugin.manifest.metadata.id;
    return (
      <PluginDetails
        record={targetPlugin}
        status={pluginStatuses[id]}
        secrets={secretsStatuses[id] || []}
        gatewayStatus={gatewayStatus}
        onBack={() => {
          if (onBack) onBack();
          else if (handleOpen) handleOpen("");
        }}
        onRefresh={onRefresh}
        onStart={handleStart}
        onStop={handleStop}
        onRestart={handleRestart}
        onSelectBackend={handleSelectBackend}
      />
    );
  }

  return (
    <section className="panel" id="panel-plugins">
      <div className="topbar">
        <div>
          <h2>插件管理</h2>
        </div>
      </div>

      {message ? (
        <div
          style={{
            marginBottom: 16,
            padding: "10px 14px",
            borderRadius: 8,
            fontSize: 13,
            background: message.error ? "rgba(239, 68, 68, 0.12)" : "rgba(34, 197, 94, 0.12)",
            color: message.error ? "var(--red)" : "var(--green)",
            border: `1px solid ${message.error ? "rgba(239, 68, 68, 0.3)" : "rgba(34, 197, 94, 0.3)"}`,
          }}
        >
          {message.text}
        </div>
      ) : null}

      {/* 导入与注册插件项目 (Import Plugin) Wizard */}
      <div className="card" id="import-wizard">
        <div className="section-head" style={{ marginBottom: 4 }}>
          <div>
            <h3>导入与注册插件项目 (Import Plugin)</h3>
            <p className="sub">
              支持本地目录自动探测、GitHub 仓库一键克隆与离线 Manifest 导入。
            </p>
          </div>
        </div>

        <div className="dtabs" style={{ margin: "14px 0" }}>
          <button
            type="button"
            className={importMode === "directory" ? "active" : ""}
            onClick={() => setImportMode("directory")}
          >
            本地项目目录
          </button>
          <button
            type="button"
            className={importMode === "github" ? "active" : ""}
            onClick={() => setImportMode("github")}
          >
            GitHub 仓库 URL
          </button>
          <button
            type="button"
            className={importMode === "manifest" ? "active" : ""}
            onClick={() => setImportMode("manifest")}
          >
            标准 Manifest 包
          </button>
        </div>

        {importMode === "directory" ? (
          <form onSubmit={handleImportDirectory} className="stack">
            <div className="field">
              <input
                type="text"
                className="input mono"
                aria-label="本地项目目录路径"
                placeholder="例如: D:\projects\wb2api 或绝对路径"
                value={dirPath}
                onChange={(e) => setDirPath(e.target.value)}
                disabled={importing}
                style={{ flex: 1 }}
              />
              <button
                type="submit"
                className="btn btn-primary"
                disabled={importing || !dirPath.trim()}
              >
                {importing ? "正在探测..." : "探测并导入"}
              </button>
            </div>
            <div className="sub" style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
              <span>快速预填样例：</span>
              <button
                type="button"
                className="btn btn-ghost btn-sm"
                onClick={() => setDirPath("D:\\projects\\workbuddy2api-panel")}
              >
                workbuddy2api-panel
              </button>
              <button
                type="button"
                className="btn btn-ghost btn-sm"
                onClick={() => setDirPath("D:\\projects\\CLIProxyAPI")}
              >
                CLIProxyAPI
              </button>
            </div>
          </form>
        ) : null}

        {importMode === "github" ? (
          <form onSubmit={handleImportGithub} className="stack">
            <div className="field">
              <input
                type="text"
                className="input mono"
                placeholder="https://github.com/example/proxyweave-plugin-xxx"
                value={gitUrl}
                onChange={(e) => setGitUrl(e.target.value)}
                disabled={importing}
                style={{ flex: 1 }}
              />
              <button
                type="submit"
                className="btn btn-primary"
                disabled={importing || !gitUrl.trim()}
              >
                {importing ? "正在拉取与校验..." : "克隆并安装"}
              </button>
            </div>
          </form>
        ) : null}

        {importMode === "manifest" ? (
          <form onSubmit={handleImportManifest} className="stack">
            <div className="field">
              <input
                type="text"
                className="input mono"
                placeholder="如 C:\path\to\proxyweave.json 或 manifest.yaml"
                value={manifestPath}
                onChange={(e) => setManifestPath(e.target.value)}
                disabled={importing}
                style={{ flex: 1 }}
              />
              <button
                type="submit"
                className="btn btn-primary"
                disabled={importing || !manifestPath.trim()}
              >
                {importing ? "正在解析..." : "导入清单"}
              </button>
            </div>
          </form>
        ) : null}
      </div>

      <div style={{ height: 16 }}></div>

      {/* Grid of Plugin Cards */}
      <div className="grid3" style={{ gridTemplateColumns: "repeat(auto-fit,minmax(300px,1fr))" }}>
        {plugins.length === 0 ? (
          <div className="card" style={{ gridColumn: "1 / -1", textAlign: "center", padding: 36 }}>
            <p className="sub">
              {dataReady ? "暂无已注册插件。请在上方输入路径或 URL 导入项目。" : "正在读取插件注册表…"}
            </p>
          </div>
        ) : (
          plugins.map((record) => {
            const meta = record.manifest.metadata;
            const id = meta.id;
            const isManaged =
              (record.manifest.ownership ?? record.manifest.runtime?.ownership) === "managed";
            const currentStatus = pluginStatuses[id];
            const statusStr = currentStatus?.status ?? "unknown";
            const isRunning = statusStr === "ready" || statusStr === "busy";
            const isCurrentBackend = gatewayStatus?.active_backend === id;
            const loading = actionLoading[id] ?? false;

            const initialLetter = (meta.name || id).slice(0, 1).toUpperCase();
            const avatarBg = isManaged ? (id === "wb2api" ? "#2f6bff" : "#c77400") : "#12994c";

            const statusPillClass =
              statusStr === "ready"
                ? "pill-green"
                : statusStr === "busy"
                ? "pill-blue"
                : statusStr === "stopped"
                ? "pill-gray"
                : statusStr === "failed"
                ? "pill-red"
                : "pill-amber";

            return (
              <div key={id} className="card plugin-card">
                <div className="plugin-head">
                  <div className="avatar" style={{ background: avatarBg }}>
                    {initialLetter}
                  </div>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                      <b style={{ fontSize: 15 }} className="plugin-name">{meta.name || id}</b>
                      <span className="plugin-version sub mono">v{meta.version || "0.1.0"}</span>
                    </div>
                    <p className="sub plugin-desc" style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis", margin: "2px 0 0" }}>
                      {meta.description || "无详细描述"}
                    </p>
                  </div>
                  <span className={`pill ${statusPillClass} plugin-status`}>
                    <span className="dot"></span>
                    {statusStr.toUpperCase()}
                  </span>
                </div>

                <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 12, alignItems: "center" }}>
                  <span
                    className={`ownership-tag pill ${
                      isManaged ? "ownership-tag--managed pill-blue" : "ownership-tag--external pill-green"
                    }`}
                  >
                    {isManaged ? "Managed (托管进程)" : "External (外部服务·零杀伤)"}
                  </span>
                  <span className="pill pill-violet">
                    {isManaged ? "idle_timeout · 300s" : "永不 Kill"}
                  </span>
                  <span className="tag">{record.manifest.schema_version ?? "schema 1.0"}</span>
                  {isCurrentBackend ? (
                    <span className="pill pill-amber">★ 统一网关活跃后端</span>
                  ) : null}
                </div>

                <div className="sub mono" style={{ fontSize: 12, marginTop: 8, overflowWrap: "anywhere" }}>
                  路径:{" "}
                  <span className="path-text">
                    {record.package_path ||
                      (record.manifest.runtime?.endpoint
                        ? String(record.manifest.runtime.endpoint)
                        : "本地注册表受管")}
                  </span>
                </div>

                <div className="plugin-meta">
                  <div>
                    <b className="mono">{currentStatus?.port ? currentStatus.port : "—"}</b>
                    <span>端口</span>
                  </div>
                  <div>
                    <b>{isRunning ? "64 MB" : "0 MB"}</b>
                    <span>内存</span>
                  </div>
                  <div>
                    <b>{currentStatus?.active_requests ?? 0}</b>
                    <span>活跃请求</span>
                  </div>
                  <div>
                    <b>{isRunning ? "12,408" : "0"}</b>
                    <span>累计</span>
                  </div>
                </div>

                <div className="actions">
                  <button
                    className="btn btn-primary btn-sm"
                    type="button"
                    data-plugin={id}
                    onClick={() => handleOpen?.(id)}
                  >
                    查看详情 →
                  </button>

                  {isManaged ? (
                    <>
                      {isRunning ? (
                        <>
                          <button
                            className="btn btn-danger-ghost btn-sm act-stop"
                            type="button"
                            onClick={() => handleStop(id)}
                            disabled={loading}
                          >
                            停止
                          </button>
                          <button
                            className="btn btn-ghost btn-sm act-restart"
                            type="button"
                            onClick={() => handleRestart(id)}
                            disabled={loading}
                          >
                            重启
                          </button>
                        </>
                      ) : (
                        <button
                          className="btn btn-soft btn-sm act-start"
                          type="button"
                          onClick={() => handleStart(id)}
                          disabled={loading}
                        >
                          启动插件
                        </button>
                      )}
                    </>
                  ) : (
                    <button
                      className="btn btn-ghost btn-sm act-reprobe"
                      type="button"
                      onClick={() => onRefresh()}
                      disabled={loading}
                    >
                      重新探针
                    </button>
                  )}

                  <button
                    className={`btn btn-sm ${isCurrentBackend ? "btn-soft" : "btn-ghost"}`}
                    type="button"
                    onClick={() => handleSelectBackend(id)}
                    disabled={!isRunning || isCurrentBackend || loading}
                    title={isCurrentBackend ? "已挂载为统一网关后端" : "挂载至统一网关"}
                  >
                    {isCurrentBackend ? "已挂载" : "挂载至统一网关"}
                  </button>

                  <button
                    className="btn btn-soft btn-sm"
                    type="button"
                    data-goto="flight"
                    onClick={() => handleFlight?.(id)}
                  >
                    试飞
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>

      <div style={{ height: 16 }}></div>

      <div className="card">
        <h3>生命周期</h3>
        <div
          style={{
            display: "flex",
            gap: 8,
            flexWrap: "wrap",
            alignItems: "center",
            fontWeight: 500,
            fontSize: "13.5px",
            marginTop: 10,
          }}
        >
          <span className="pill pill-gray" style={{ padding: "8px 14px" }}>
            Stopped
          </span>
          →
          <span className="pill pill-amber" style={{ padding: "8px 14px" }}>
            Starting
          </span>
          →
          <span className="pill pill-green" style={{ padding: "8px 14px" }}>
            Ready
          </span>
          ⇄
          <span className="pill pill-blue" style={{ padding: "8px 14px" }}>
            Busy
          </span>
          <span style={{ color: "var(--muted)", fontWeight: 500, margin: "0 6px" }}>
            任一阶段异常 →
          </span>
          <span className="pill pill-red" style={{ padding: "8px 14px" }}>
            Failed
          </span>
        </div>
      </div>
    </section>
  );
};
