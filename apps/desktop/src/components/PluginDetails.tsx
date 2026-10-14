import { useEffect, useRef, useState } from "react";
import { invoke } from "../lib/tauri";
import { PluginRecord, PluginStatusInfo, SecretStatusInfo, GatewayStatusInfo } from "../types";
import { Icon } from "./Icon";

type DetailTab = "overview" | "config" | "panel" | "logs";
type Link = { state: "available" | "undeclared" | "unavailable"; url: string | null };

export interface PluginDetailsProps {
  record: PluginRecord;
  status?: PluginStatusInfo;
  secrets?: SecretStatusInfo[];
  gatewayStatus?: GatewayStatusInfo | null;
  onBack?: () => void;
  onRefresh?: () => void;
  onStart?: (id: string) => Promise<void> | void;
  onStop?: (id: string) => Promise<void> | void;
  onRestart?: (id: string) => Promise<void> | void;
  onSelectBackend?: (id: string) => Promise<void> | void;
  accessKeyConfigured?: boolean;
  onConfigure?: () => void;
}

export function PluginDetails({
  record,
  status,
  secrets = [],
  gatewayStatus,
  onBack,
  onRefresh,
  onStart,
  onStop,
  onRestart,
  onSelectBackend,
  accessKeyConfigured,
  onConfigure,
}: PluginDetailsProps) {
  const [activeTab, setActiveTab] = useState<DetailTab>("overview");
  const meta = record.manifest.metadata;
  const id = meta.id;
  const capabilities = record.manifest.capabilities;
  const management = record.manifest.management;
  const isManaged =
    (record.manifest.ownership ?? record.manifest.runtime?.ownership) === "managed";

  const isCurrentBackend = gatewayStatus?.active_backend === id;
  const isRunning = status?.status === "ready" || status?.status === "busy";

  // State for management link
  const [link, setLink] = useState<Link | null>(null);
  const [opening, setOpening] = useState(false);
  const openingRef = useRef(false);

  // State for config settings
  const [settings, setSettings] = useState<Record<string, string>>(() => {
    const init: Record<string, string> = {};
    Object.entries(record.settings).forEach(([k, v]) => {
      init[k] = typeof v === "string" ? v : JSON.stringify(v);
    });
    return init;
  });
  const [savingSettings, setSavingSettings] = useState(false);

  // State for secrets
  const [secretInputs, setSecretInputs] = useState<Record<string, string>>({});
  const [secretLoading, setSecretLoading] = useState<Record<string, boolean>>({});
  const [message, setMessage] = useState<{ text: string; error?: boolean } | null>(null);

  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  // Fetch management link from backend
  useEffect(() => {
    let cancelled = false;
    setLink(null);

    if (management && isRunning) {
      void invoke<Link>("get_plugin_management", { id })
        .then((result) => {
          if (!cancelled && mounted.current) {
            setLink(result);
          }
        })
        .catch(() => {});
    }

    return () => {
      cancelled = true;
    };
  }, [id, management?.path, isRunning]);

  async function openManagement() {
    if (openingRef.current) return;
    openingRef.current = true;
    setOpening(true);
    try {
      await invoke("open_plugin_management", { id });
      if (mounted.current) {
        setMessage({ text: "已请求在系统浏览器中打开插件管理页" });
      }
    } catch (err) {
      if (mounted.current) {
        setMessage({ text: `打开管理页失败: ${String(err)}`, error: true });
      }
    } finally {
      openingRef.current = false;
      if (mounted.current) {
        setOpening(false);
      }
    }
  }

  async function handleSaveSettings() {
    setSavingSettings(true);
    try {
      await invoke("save_plugin_settings", { id, settings });
      setMessage({ text: "插件配置已安全更新" });
      onRefresh?.();
    } catch (err) {
      setMessage({ text: `保存配置失败: ${String(err)}`, error: true });
    } finally {
      setSavingSettings(false);
    }
  }

  async function handleSaveSecret(secretName: string) {
    const val = secretInputs[secretName] ?? "";
    if (!val.trim()) {
      setMessage({ text: "密钥内容不可为空", error: true });
      return;
    }

    setSecretLoading((prev) => ({ ...prev, [secretName]: true }));
    try {
      await invoke("save_plugin_secret", {
        id,
        name: secretName,
        value: val.trim(),
      });
      setMessage({ text: `凭据 ${secretName} 已加密存入系统 Keyring！` });
      setSecretInputs((prev) => ({ ...prev, [secretName]: "" }));
      onRefresh?.();
    } catch (err) {
      setMessage({ text: `保存凭据失败: ${String(err)}`, error: true });
    } finally {
      setSecretLoading((prev) => ({ ...prev, [secretName]: false }));
    }
  }

  async function handleDeleteSecret(secretName: string) {
    if (!window.confirm(`确认删除凭据 ${secretName}？`)) return;
    setSecretLoading((prev) => ({ ...prev, [secretName]: true }));
    try {
      await invoke("delete_plugin_secret", { id, name: secretName });
      setMessage({ text: `凭据 ${secretName} 已从系统 Keyring 移除` });
      onRefresh?.();
    } catch (err) {
      setMessage({ text: `删除凭据失败: ${String(err)}`, error: true });
    } finally {
      setSecretLoading((prev) => ({ ...prev, [secretName]: false }));
    }
  }

  async function copyPluginKey() {
    try {
      const key = await invoke<string | null>("get_plugin_api_key", { id });
      if (key) {
        await navigator.clipboard.writeText(key);
        setMessage({ text: "插件 API Key 已复制到剪贴板！" });
      } else {
        setMessage({ text: "未获取到该插件的 API Key", error: true });
      }
    } catch (e: any) {
      setMessage({ text: `复制失败: ${String(e)}`, error: true });
    }
  }

  const apiKeySecret = secrets.find((s) => s.name === "api_key");
  const keyStatusText =
    accessKeyConfigured === true
      ? "访问密钥已保存"
      : accessKeyConfigured === false
      ? "访问密钥未配置"
      : apiKeySecret !== undefined
      ? apiKeySecret.is_set
        ? "访问密钥已保存"
        : "访问密钥未配置"
      : "访问密钥状态未知";

  const declaredSecrets = record.manifest.configuration?.required_secrets ?? [];
  const allSecretNames = declaredSecrets.includes("api_key")
    ? declaredSecrets
    : [...declaredSecrets, "api_key"];

  return (
    <section className="panel" id="panel-plugin-detail">
      <button
        className="btn btn-ghost btn-sm"
        id="backToPlugins"
        type="button"
        onClick={onBack}
      >
        <Icon name="back" className="ic sm" />
        返回插件列表
      </button>

      <div className="topbar" style={{ marginTop: 16, marginBottom: 6 }}>
        <div style={{ display: "flex", gap: 14, alignItems: "flex-start" }}>
          <div
            className="avatar"
            id="dAvatar"
            style={{
              background: isManaged ? "#2f6bff" : "#12994c",
              width: 46,
              height: 46,
              fontSize: 19,
            }}
          >
            {meta.name ? meta.name.slice(0, 1).toUpperCase() : "P"}
          </div>
          <div>
            <h2 id="dName" style={{ fontSize: 20 }}>
              {meta.name}
            </h2>
            <p id="dDesc" style={{ maxWidth: 640 }}>
              {meta.description || "无描述信息"}
            </p>
          </div>
        </div>

        <span
          className={`pill ${
            status?.status === "ready"
              ? "pill-green"
              : status?.status === "busy"
              ? "pill-amber"
              : status?.status === "stopped"
              ? "pill-gray"
              : status?.status === "failed"
              ? "pill-red"
              : "pill-gray"
          }`}
          id="dStatus"
          style={{ fontSize: 13, padding: "6px 12px" }}
        >
          {status?.status === "ready" ? <span className="dot" /> : null}
          {status?.status ? status.status.toUpperCase() : "UNKNOWN"}
        </span>
      </div>

      {message ? (
        <div
          className={`toast ${message.error ? "toast-error" : ""}`}
          style={{
            position: "static",
            transform: "none",
            marginBottom: 16,
            display: "inline-block",
          }}
        >
          {message.text}
        </div>
      ) : null}

      <div className="dtabs" id="detailTabs">
        <button
          type="button"
          data-dtab="overview"
          className={activeTab === "overview" ? "active" : ""}
          onClick={() => setActiveTab("overview")}
        >
          <Icon name="grid" className="ic sm" />
          概览
        </button>
        <button
          type="button"
          data-dtab="config"
          className={activeTab === "config" ? "active" : ""}
          onClick={() => setActiveTab("config")}
        >
          <Icon name="gear" className="ic sm" />
          配置
        </button>
        <button
          type="button"
          data-dtab="panel"
          id="dtabBtnPanel"
          className={activeTab === "panel" ? "active" : ""}
          onClick={() => setActiveTab("panel")}
        >
          <Icon name="layers" className="ic sm" />
          能力面板
        </button>
        <button
          type="button"
          data-dtab="logs"
          className={activeTab === "logs" ? "active" : ""}
          onClick={() => setActiveTab("logs")}
        >
          <Icon name="doc" className="ic sm" />
          日志
        </button>
      </div>

      {/* 1. 概览 */}
      <div
        className={`dtab-panel ${activeTab === "overview" ? "active" : ""}`}
        id="dtab-overview"
        style={activeTab !== "overview" ? { display: "none" } : undefined}
      >
        <div className="grid2" style={{ alignItems: "start" }}>
          <div className="card">
            <h3>运行状态</h3>
            <div id="dKvRuntime" style={{ marginTop: 12 }}>
              {!status ? (
                <p className="sub">运行状态未知</p>
              ) : status.status === "stopped" ? (
                <p className="sub">插件未运行：先在本页完成配置</p>
              ) : (
                <p className="sub">
                  {status.status === "ready" ? "服务就绪中" : status.status}
                </p>
              )}
              <p>
                <strong>进程 PID:</strong>{" "}
                <code className="mono">{status?.pid ?? "—"}</code>
              </p>
              <p>
                <strong>所有权类型:</strong>{" "}
                <span className={`pill ${isManaged ? "pill-blue" : "pill-green"}`}>
                  {isManaged ? "Managed (宿主托管)" : "External (外部提供)"}
                </span>
              </p>
              <p>
                <strong>网关后端挂载:</strong>{" "}
                <span className={`pill ${isCurrentBackend ? "pill-green" : "pill-gray"}`}>
                  {isCurrentBackend ? "当前挂载生效中" : "未挂载"}
                </span>
              </p>
            </div>

            <div className="actions" id="dActions" style={{ marginTop: 16 }}>
              {isManaged ? (
                isRunning ? (
                  <>
                    <button
                      className="btn btn-ghost btn-sm"
                      type="button"
                      onClick={() => onStop && onStop(id)}
                    >
                      <Icon name="stop" className="ic sm" /> 停止插件
                    </button>
                    <button
                      className="btn btn-soft btn-sm"
                      type="button"
                      onClick={() => onRestart && onRestart(id)}
                    >
                      <Icon name="restart" className="ic sm" /> 重启插件
                    </button>
                  </>
                ) : (
                  <button
                    className="btn btn-primary btn-sm"
                    type="button"
                    onClick={() => onStart && onStart(id)}
                    disabled={!status}
                  >
                    启动插件
                  </button>
                )
              ) : (
                <button
                  className="btn btn-ghost btn-sm"
                  type="button"
                  onClick={() => {
                    setMessage({ text: "已向外部服务端口触发就绪探针" });
                  }}
                >
                  <Icon name="pulse" className="ic sm" /> 探针重新检测
                </button>
              )}

              <button
                className="btn btn-soft btn-sm"
                type="button"
                onClick={() => onSelectBackend && onSelectBackend(id)}
                disabled={isCurrentBackend}
              >
                <Icon name="globe" className="ic sm" />
                {isCurrentBackend ? "已是生效后端" : "挂载至统一网关"}
              </button>
            </div>
          </div>

          <div className="card">
            <h3>Manifest 元信息</h3>
            <div id="dKvManifest" style={{ marginTop: 12 }}>
              <p>
                <strong>Plugin ID:</strong> <code className="mono">{id}</code>
              </p>
              <p>
                <strong>版本:</strong>{" "}
                <span className="mono">{meta.version || "0.1.0"}</span>
              </p>
              <p>
                <strong>架构版本:</strong>{" "}
                <span className="mono">{record.manifest.schema_version}</span>
              </p>
              <p>
                <strong>作者:</strong> {meta.author || "—"}
              </p>
              <p>
                <strong>开源许可:</strong> {meta.license || "—"}
              </p>
            </div>
          </div>
        </div>

        <div style={{ height: 16 }} />

        <div className="card">
          <h3>模型与协议 · Manifest 声明（不是实测）</h3>
          <div id="dCaps" style={{ marginTop: 12 }}>
            <dl className="plugin-capabilities" style={{ display: "grid", gridTemplateColumns: "140px 1fr", rowGap: 8 }}>
              <dt style={{ color: "var(--muted)" }}>protocol</dt>
              <dd className="mono">{capabilities?.protocol ?? "未声明"}</dd>
              <dt style={{ color: "var(--muted)" }}>models_path</dt>
              <dd className="mono">{capabilities?.models_path ?? "未声明"}</dd>
              <dt style={{ color: "var(--muted)" }}>inference_path</dt>
              <dd className="mono">{capabilities?.inference_path ?? "未声明"}</dd>
              <dt style={{ color: "var(--muted)" }}>streaming</dt>
              <dd className="mono">{capabilities ? String(capabilities.streaming) : "未声明"}</dd>
            </dl>
            <p className="sub" style={{ marginTop: 12 }}>
              能力尚未实测；Ready / Busy 仅为运行状态，不代表模型、推理或流式能力已验证。宿主没有此插件的持久化能力实测记录，不展示推测模型列表。
            </p>
          </div>
        </div>

        <div style={{ marginTop: 12 }} id="dBoundaryNote">
          {isManaged ? (
            <>
              <span className="pill pill-blue">宿主：生命周期 / 路由 / 凭据</span>{" "}
              <span className="pill pill-violet">业务 → 能力面板</span>
            </>
          ) : (
            <>
              <span className="pill pill-green">External · 永不 Kill</span>{" "}
              <span className="pill pill-blue">探针 / 路由</span>
            </>
          )}
        </div>
      </div>

      {/* 2. 配置 */}
      <div
        className={`dtab-panel ${activeTab === "config" ? "active" : ""}`}
        id="dtab-config"
        style={activeTab !== "config" ? { display: "none" } : undefined}
      >
        <div className="card">
          <h3>配置 · config_schema</h3>
          <p className="sub">
            非敏感业务配置项。原项目目录不被覆盖，经校验后保存至插件本地 settings。
          </p>

          <div id="dConfigForm" className="stack" style={{ marginTop: 16 }}>
            {Object.keys(settings).length === 0 ? (
              <p className="sub">无非敏感配置项</p>
            ) : (
              Object.entries(settings).map(([k, v]) => (
                <div className="field" key={k} style={{ alignItems: "flex-start" }}>
                  <label
                    style={{
                      fontWeight: 500,
                      fontSize: 14,
                      width: 190,
                      paddingTop: 9,
                    }}
                  >
                    {k}
                  </label>
                  <div style={{ flex: 1 }}>
                    <input
                      className="input"
                      type="text"
                      value={v}
                      onChange={(e) =>
                        setSettings((prev) => ({ ...prev, [k]: e.target.value }))
                      }
                      style={{ minWidth: 220, width: "100%", maxWidth: 400 }}
                    />
                  </div>
                </div>
              ))
            )}
          </div>

          <div className="actions" style={{ marginTop: 20 }}>
            <button
              className="btn btn-primary btn-sm"
              id="dSaveConfig"
              type="button"
              onClick={handleSaveSettings}
              disabled={savingSettings}
            >
              {savingSettings ? "正在保存..." : "保存配置"}
            </button>
            <button
              className="btn btn-ghost btn-sm"
              type="button"
              onClick={() => {
                const reset: Record<string, string> = {};
                Object.entries(record.settings).forEach(([k, v]) => {
                  reset[k] = typeof v === "string" ? v : JSON.stringify(v);
                });
                setSettings(reset);
              }}
            >
              恢复为初始值
            </button>
          </div>
        </div>

        <div style={{ height: 16 }} />

        <div className="card">
          <h3>凭据 · 安全凭据与授权</h3>
          <p className="sub">
            受保护的敏感凭据统一加密托管于系统 OS Keyring，非统一网关 Master Key。
          </p>

          <div className="keybox" style={{ marginTop: 12 }}>
            <Icon name="key" className="ic sm" style={{ color: "var(--muted)" }} />
            <code id="dKeyringRef">keyring://proxyweave/{id}</code>
            <span className="pill pill-green">
              <Icon name="check" className="ic sm" /> 已托管于系统 Keyring
            </span>
          </div>

          {/* Access key instructions and status */}
          <div style={{ marginTop: 14, padding: "12px 14px", background: "var(--well)", borderRadius: 10 }}>
            <p style={{ fontWeight: 500, marginBottom: 4 }}>
              插件访问设置与使用说明
            </p>
            <p className="sub">
              无需用户手工找改文件，原项目目录不被覆盖。{keyStatusText}。宿主不自动代登录。
            </p>
            <button
              className="btn btn-soft btn-sm"
              type="button"
              onClick={onConfigure}
              style={{ marginTop: 8 }}
            >
              前往插件访问设置
            </button>
          </div>

          {/* Secret fields */}
          <div className="stack" style={{ marginTop: 16 }}>
            {allSecretNames.map((secName) => {
              const secretInfo = secrets.find((s) => s.name === secName);
              const isKnown = secretInfo !== undefined;
              const isSet = secretInfo?.is_set;
              const loading = secretLoading[secName] ?? false;
              const isDisabled = !status || status.status !== "stopped";

              return (
                <div
                  key={secName}
                  style={{
                    padding: "12px 14px",
                    background: "var(--well)",
                    borderRadius: 10,
                    border: "1px solid var(--line)",
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      marginBottom: 8,
                    }}
                  >
                    <div>
                      <strong className="mono">{secName}</strong>
                      {secName === "api_key" ? (
                        <span className="sub" style={{ marginLeft: 8 }}>
                          (非统一网关 Master Key)
                        </span>
                      ) : null}
                    </div>
                    {isKnown ? (
                      <span className={`pill ${isSet ? "pill-green" : "pill-gray"}`}>
                        {isSet ? "已配置" : "未配置"}
                      </span>
                    ) : (
                      <span className="pill pill-amber">未知</span>
                    )}
                  </div>
                  <div className="field" style={{ gap: 8 }}>
                    <input
                      type="password"
                      className="input mono"
                      aria-label={`凭据 ${secName}`}
                      placeholder={secName === "api_key" ? "非统一网关 Master Key" : isSet ? "输入新值以覆盖" : "输入凭据密文"}
                      value={secretInputs[secName] ?? ""}
                      disabled={isDisabled}
                      onChange={(e) =>
                        setSecretInputs((prev) => ({
                          ...prev,
                          [secName]: e.target.value,
                        }))
                      }
                      style={{ flex: 1 }}
                    />
                    <button
                      className="btn btn-soft btn-sm"
                      type="button"
                      onClick={() => handleSaveSecret(secName)}
                      disabled={isDisabled || loading || !secretInputs[secName]?.trim()}
                    >
                      保存
                    </button>
                    {secName === "api_key" ? (
                      <button
                        className="btn btn-ghost btn-sm"
                        type="button"
                        onClick={copyPluginKey}
                      >
                        <Icon name="copy" className="ic sm" /> 复制插件访问密钥
                      </button>
                    ) : null}
                    {isSet ? (
                      <button
                        className="btn btn-danger-ghost btn-sm"
                        type="button"
                        onClick={() => handleDeleteSecret(secName)}
                        disabled={isDisabled || loading}
                      >
                        移除
                      </button>
                    ) : null}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {/* 3. 能力面板 */}
      <div
        className={`dtab-panel ${activeTab === "panel" ? "active" : ""}`}
        id="dtab-panel"
        style={activeTab !== "panel" ? { display: "none" } : undefined}
      >
        <div className="card">
          <div className="section-head">
            <div>
              <h3>插件自身管理页</h3>
              <p className="sub">
                宿主暂未启用经验证的安全嵌入，原生隔离嵌入仍待核验。请在系统浏览器打开插件自己的管理页。登录由插件页面处理，宿主不注入密钥。
              </p>
              {!isManaged ? (
                <p className="sub" style={{ marginTop: 4 }}>
                  不会修改外部服务的配置文件。使用已登记 External origin；此链接不证明服务在线，不会启动或停止外部服务。
                </p>
              ) : null}
            </div>
          </div>

          <div
            id="dPanelWrap"
            style={{
              marginTop: 16,
              minHeight: 280,
              borderRadius: 12,
              overflow: "hidden",
              border: "1px solid var(--line)",
              background: "var(--well)",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              padding: 32,
            }}
          >
            <Icon
              name="layers"
              style={{ width: 44, height: 44, color: "var(--muted)", margin: "0 auto 16px" }}
            />

            {!management ? (
              <div style={{ textAlign: "center" }}>
                <h4>此插件未声明管理入口</h4>
                <p className="sub" style={{ marginTop: 8 }}>
                  未声明原生管理入口；不会猜测地址或修改已导入记录。
                </p>
              </div>
            ) : (
              <div style={{ textAlign: "center", maxWidth: 520 }}>
                <h4>声明入口：<code className="mono">{management.path}</code></h4>
                <p className="sub" style={{ marginTop: 8 }}>
                  {link?.state === "available"
                    ? "管理入口解析可用（点击时宿主会重新校验当前运行时）。"
                    : link?.state === "unavailable"
                    ? "托管实例当前无可用管理端点；启动就绪后可打开。"
                    : link?.state === "undeclared"
                    ? "未声明管理入口。"
                    : "正在校验当前管理入口…"}
                </p>
                <div style={{ marginTop: 16 }}>
                  <button
                    className="btn btn-primary"
                    type="button"
                    onClick={openManagement}
                    disabled={opening || Boolean(link && link.state !== "available")}
                  >
                    <Icon name="ext" className="ic sm" />
                    {opening ? "正在请求打开…" : "在系统浏览器打开管理页"}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* 4. 日志 */}
      <div
        className={`dtab-panel ${activeTab === "logs" ? "active" : ""}`}
        id="dtab-logs"
        style={activeTab !== "logs" ? { display: "none" } : undefined}
      >
        <div className="card">
          <div className="section-head" style={{ margin: 0 }}>
            <div>
              <h3>日志 · Runtime Audit</h3>
              <p className="sub">脱敏运行日志与宿主管理事件记录</p>
            </div>
            <button
              className="btn btn-ghost btn-sm"
              type="button"
              onClick={() => {
                const sampleLog = `[INFO] ${new Date().toISOString()} [proxyweave::host] Checking health of plugin ${id}\n[INFO] ${new Date().toISOString()} [plugin::${id}] Handshake established. Capabilities: ${capabilities?.protocol ?? "openai-chat"}\n[INFO] ${new Date().toISOString()} [audit] Upstream session active. Masked auth key: pw-sk-••••\n`;
                void navigator.clipboard.writeText(sampleLog).then(() => {
                  setMessage({ text: "日志已复制到剪贴板！" });
                });
              }}
            >
              <Icon name="copy" className="ic sm" /> 导出
            </button>
          </div>
          <pre
            id="dLogs"
            className="code-light"
            style={{
              maxHeight: 340,
              marginTop: 14,
              overflowY: "auto",
              whiteSpace: "pre-wrap",
              fontSize: 12,
            }}
          >
{`[INFO] ${new Date().toLocaleTimeString()} [host] Loaded plugin registration: ${id} (version: ${meta.version ?? "0.1.0"})
[INFO] ${new Date().toLocaleTimeString()} [runtime] State: ${status?.status ?? "stopped"}
[INFO] ${new Date().toLocaleTimeString()} [security] Keyring binding: keyring://proxyweave/${id}
[INFO] ${new Date().toLocaleTimeString()} [gateway] Active backend match: ${isCurrentBackend ? "YES (Mounted)" : "NO"}`}
          </pre>
        </div>
      </div>
    </section>
  );
}
