import React, { useState } from "react";
import { PluginRecord, PluginStatusInfo, SecretStatusInfo, GatewayStatusInfo } from "../types";
import { invoke } from "../lib/tauri";

interface PluginPanelProps {
  plugins: PluginRecord[];
  pluginStatuses: Record<string, PluginStatusInfo>;
  secretsStatuses: Record<string, SecretStatusInfo[]>;
  gatewayStatus: GatewayStatusInfo | null;
  onRefresh: () => void;
}

export const PluginPanel: React.FC<PluginPanelProps> = ({
  plugins,
  pluginStatuses,
  secretsStatuses,
  gatewayStatus,
  onRefresh,
}) => {
  const [manifestPath, setManifestPath] = useState("");
  const [importing, setImporting] = useState(false);
  const [actionLoading, setActionLoading] = useState<Record<string, boolean>>({});
  const [secretInputs, setSecretInputs] = useState<Record<string, Record<string, string>>>({});
  const [message, setMessage] = useState<{ text: string; error?: boolean } | null>(null);

  async function handleImport(e: React.FormEvent) {
    e.preventDefault();
    if (!manifestPath.trim()) return;

    setImporting(true);
    setMessage(null);
    try {
      const res: any = await invoke("import_local_plugin", { manifestPath: manifestPath.trim() });
      setMessage({
        text: res.already_registered
          ? `插件 ${res.record.manifest.metadata.name} (${res.record.manifest.metadata.id}) 已注册并通过安全校验。`
          : `成功导入并注册插件 ${res.record.manifest.metadata.name} (${res.record.manifest.metadata.id})！`,
      });
      setManifestPath("");
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `导入插件失败：${e}`, error: true });
    } finally {
      setImporting(false);
    }
  }

  async function handleStartPlugin(id: string) {
    setActionLoading((prev) => ({ ...prev, [id]: true }));
    setMessage(null);
    try {
      const res: PluginStatusInfo = await invoke("start_plugin", { id });
      if (res.status === "ready" || res.status === "busy") {
        setMessage({
          text: `插件 ${id} 已就绪，运行于端口 ${res.port} (PID: ${res.pid ?? "—"})${
            res.status === "busy" ? ` [状态: Busy, 原因: ${res.reason}]` : ""
          }`,
        });
      } else {
        setMessage({ text: `插件 ${id} 启动未就绪: ${res.error ?? res.status}`, error: true });
      }
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `启动失败: ${e}`, error: true });
    } finally {
      setActionLoading((prev) => ({ ...prev, [id]: false }));
    }
  }

  async function handleStopPlugin(id: string) {
    setActionLoading((prev) => ({ ...prev, [id]: true }));
    setMessage(null);
    try {
      await invoke("stop_plugin", { id });
      setMessage({ text: `插件 ${id} 已安全停止。` });
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `停止失败: ${e}`, error: true });
    } finally {
      setActionLoading((prev) => ({ ...prev, [id]: false }));
    }
  }

  async function handleRestartPlugin(id: string) {
    setActionLoading((prev) => ({ ...prev, [id]: true }));
    setMessage(null);
    try {
      const res: PluginStatusInfo = await invoke("restart_plugin", { id });
      setMessage({
        text: `插件 ${id} 重启完成，当前状态: ${res.status}${res.port ? ` (Port ${res.port})` : ""}`,
      });
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
      setMessage({ text: `已将插件 ${id} 设置为统一网关的活跃后端！` });
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `挂载后端失败: ${e}`, error: true });
    } finally {
      setActionLoading((prev) => ({ ...prev, [id]: false }));
    }
  }

  async function handleSaveSecret(pluginId: string, secretName: string) {
    const val = secretInputs[pluginId]?.[secretName] ?? "";
    if (!val.trim()) {
      setMessage({ text: "密钥内容不可为空", error: true });
      return;
    }

    setMessage(null);
    try {
      await invoke("save_plugin_secret", {
        id: pluginId,
        name: secretName,
        value: val.trim(),
      });
      setMessage({ text: `凭据 '${secretName}' 已成功安全保存到系统 Keyring！` });
      setSecretInputs((prev) => ({
        ...prev,
        [pluginId]: { ...(prev[pluginId] || {}), [secretName]: "" },
      }));
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `保存凭据失败: ${e}`, error: true });
    }
  }

  async function handleDeleteSecret(pluginId: string, secretName: string) {
    if (!window.confirm(`确定要从系统 Keyring 中删除插件 ${pluginId} 的凭据 '${secretName}' 吗？`)) {
      return;
    }

    setMessage(null);
    try {
      await invoke("delete_plugin_secret", { id: pluginId, name: secretName });
      setMessage({ text: `凭据 '${secretName}' 已从系统 Keyring 中移除。` });
      onRefresh();
    } catch (e: any) {
      setMessage({ text: `删除凭据失败: ${e}`, error: true });
    }
  }

  return (
    <div className="panel-section">
      <div className="section-header">
        <div>
          <h3 className="section-title">插件管理 (Plugins)</h3>
          <p className="section-desc">
            宿主以黑盒方式接入 2API 插件（Managed 托管进程与 External 外部服务），提供生命周期看护与凭据安全注入。
          </p>
        </div>
      </div>

      {message ? (
        <div className={`status-banner ${message.error ? "status-banner--error" : "status-banner--success"}`}>
          {message.text}
        </div>
      ) : null}

      {/* Import Form */}
      <div className="card-box">
        <h4 className="card-box-title">导入本地插件包 (Import Manifest)</h4>
        <p className="card-box-desc">
          输入已就绪插件的 <code>manifest.yaml</code> 绝对路径或工作区相对路径进行安全校验与注册。
        </p>
        <form onSubmit={handleImport} className="import-form">
          <input
            type="text"
            placeholder="例如: D:/temp-audit/proxyweave-f2-def006-review-package/manifest.yaml"
            value={manifestPath}
            onChange={(e) => setManifestPath(e.target.value)}
            disabled={importing}
            className="form-input form-input--wide"
          />
          <button type="submit" className="primary-action" disabled={importing || !manifestPath.trim()}>
            {importing ? "正在校验与导入..." : "导入并注册"}
          </button>
        </form>
      </div>

      {/* Plugin List */}
      <div className="plugin-list-container">
        <h4 className="card-box-title">已安装插件 ({plugins.length})</h4>

        {plugins.length === 0 ? (
          <div className="empty-state">
            <p>暂无已注册的插件。请在上方输入 manifest.yaml 路径进行导入。</p>
          </div>
        ) : (
          <div className="plugin-cards">
            {plugins.map((record) => {
              const meta = record.manifest.metadata;
              const id = meta.id;
              const isManaged = record.manifest.ownership === "managed";
              const statusInfo = pluginStatuses[id] || {
                status: "stopped",
                active_requests: 0,
              };
              const isRunning = statusInfo.status === "ready" || statusInfo.status === "busy";
              const isStarting = statusInfo.status === "starting";
              const isLoading = actionLoading[id] ?? false;
              const isCurrentBackend = gatewayStatus?.active_backend === id;
              const secrets = secretsStatuses[id] || [];

              return (
                <div key={id} className="plugin-card">
                  <div className="plugin-card-header">
                    <div>
                      <div className="plugin-title-row">
                        <strong className="plugin-name">{meta.name}</strong>
                        <span className="plugin-version">v{meta.version}</span>
                        <span
                          className={`ownership-tag ${
                            isManaged ? "ownership-tag--managed" : "ownership-tag--external"
                          }`}
                        >
                          {isManaged ? "Managed (托管进程)" : "External (外部服务·零杀伤)"}
                        </span>
                        {isCurrentBackend ? (
                          <span className="backend-tag">★ 当前网关挂载后端</span>
                        ) : null}
                      </div>
                      <div className="plugin-id-sub">
                        ID: <code>{id}</code> · 路径: <span className="path-text">{record.package_path}</span>
                      </div>
                    </div>

                    <div className="plugin-status-badge-area">
                      <span
                        className={`status-pill status-pill--${statusInfo.status}`}
                        title={statusInfo.error ? `错误: ${statusInfo.error}` : ""}
                      >
                        {statusInfo.status.toUpperCase()}
                        {statusInfo.port ? ` :${statusInfo.port}` : ""}
                        {statusInfo.pid ? ` (PID ${statusInfo.pid})` : ""}
                      </span>
                      {statusInfo.active_requests > 0 ? (
                        <span className="inflight-badge">
                          {statusInfo.active_requests} 活跃请求中
                        </span>
                      ) : null}
                    </div>
                  </div>

                  {meta.description ? <p className="plugin-desc">{meta.description}</p> : null}

                  {statusInfo.error ? (
                    <div className="plugin-error-banner">
                      <strong>异常原因:</strong> {statusInfo.error}
                    </div>
                  ) : null}

                  {/* Plugin Actions */}
                  <div className="plugin-actions-bar">
                    {isManaged ? (
                      <>
                        <button
                          type="button"
                          className="primary-action"
                          onClick={() => handleStartPlugin(id)}
                          disabled={isLoading || isRunning || isStarting}
                        >
                          {isStarting ? "启动中..." : "启动插件"}
                        </button>
                        <button
                          type="button"
                          className="secondary-action"
                          onClick={() => handleStopPlugin(id)}
                          disabled={isLoading || !isRunning}
                          title={
                            statusInfo.active_requests > 0
                              ? "插件正在处理请求，禁止强制终止"
                              : "停止托管进程"
                          }
                        >
                          停止
                        </button>
                        <button
                          type="button"
                          className="ghost-action"
                          onClick={() => handleRestartPlugin(id)}
                          disabled={isLoading || !isRunning}
                        >
                          重启
                        </button>
                      </>
                    ) : (
                      <span className="text-muted">（External 外部提供商由宿主透明探活，不执行进程杀死与启停）</span>
                    )}

                    <button
                      type="button"
                      className={`secondary-action ${isCurrentBackend ? "btn--active-backend" : ""}`}
                      onClick={() => handleSelectBackend(id)}
                      disabled={isLoading || isCurrentBackend}
                    >
                      {isCurrentBackend ? "已挂载为网关后端" : "挂载至统一网关"}
                    </button>
                  </div>

                  {/* Secret Configuration Section */}
                  {record.manifest.configuration.required_secrets &&
                  record.manifest.configuration.required_secrets.length > 0 ? (
                    <div className="plugin-secrets-box">
                      <div className="secrets-box-header">
                        <strong>安全凭据与授权 (OS Keyring 隔离存储)</strong>
                        <span className="secrets-note">
                          （需先停止插件才可修改敏感凭据，以防运行时配置撕裂）
                        </span>
                      </div>

                      <div className="secrets-grid">
                        {record.manifest.configuration.required_secrets.map((secretName) => {
                          const sec = secrets.find((s) => s.name === secretName);
                          const isConfigured = sec?.is_set ?? false;
                          const currentInput = secretInputs[id]?.[secretName] ?? "";

                          return (
                            <div key={secretName} className="secret-item-row">
                              <div className="secret-meta">
                                <code>{secretName}</code>
                                <span
                                  className={`secret-status-tag ${
                                    isConfigured ? "secret-status-tag--ok" : "secret-status-tag--missing"
                                  }`}
                                >
                                  {isConfigured ? "已配置" : "未配置"}
                                </span>
                              </div>

                              <div className="secret-form-row">
                                <input
                                  type="password"
                                  placeholder={isConfigured ? "输入新值以覆盖更新" : "输入凭据内容"}
                                  value={currentInput}
                                  onChange={(e) => {
                                    const val = e.target.value;
                                    setSecretInputs((prev) => ({
                                      ...prev,
                                      [id]: { ...(prev[id] || {}), [secretName]: val },
                                    }));
                                  }}
                                  disabled={isRunning || isStarting}
                                  className="form-input form-input--secret"
                                />
                                <button
                                  type="button"
                                  className="secondary-action"
                                  onClick={() => handleSaveSecret(id, secretName)}
                                  disabled={isRunning || isStarting || !currentInput.trim()}
                                >
                                  保存
                                </button>
                                {isConfigured ? (
                                  <button
                                    type="button"
                                    className="ghost-action text-danger"
                                    onClick={() => handleDeleteSecret(id, secretName)}
                                    disabled={isRunning || isStarting}
                                  >
                                    删除
                                  </button>
                                ) : null}
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  ) : null}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
