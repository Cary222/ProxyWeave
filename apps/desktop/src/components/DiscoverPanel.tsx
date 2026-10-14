import React, { useState, useRef, useEffect } from "react";
import { invoke } from "../lib/tauri";
import { ImportResult, PluginRecord } from "../types";

interface DiscoverPanelProps {
  plugins?: PluginRecord[];
  onRefresh?: () => void;
  onNavigatePlugins?: () => void;
  onInstallSuccess?: () => void;
}

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
}) => {
  const [repoUrl, setRepoUrl] = useState("");
  const [installing, setInstalling] = useState(false);
  const [message, setMessage] = useState<{ text: string; error?: boolean } | null>(null);
  const [lastImportResult, setLastImportResult] = useState<ImportResult | null>(null);

  const mountedRef = useRef(true);
  const inFlightRef = useRef(false);
  const opIdRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const isInstalled = (id: string) =>
    plugins.some((p) => p.manifest?.metadata?.id === id);

  function handleUrlChange(newUrl: string) {
    setRepoUrl(newUrl);
    if (message) setMessage(null);
    if (lastImportResult) setLastImportResult(null);
    opIdRef.current++;
  }

  async function handleConfirmInstall() {
    const trimmedUrl = repoUrl.trim();
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

  return (
    <section className="panel" id="panel-discover">
      <div className="topbar">
        <div>
          <h2>插件发现</h2>
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
        <div className="card">
          <h3>从 GitHub 安装</h3>
          <div className="field" style={{ marginTop: 14 }}>
            <input
              className="input mono"
              id="repoUrl"
              value={repoUrl}
              onChange={(e) => handleUrlChange(e.target.value)}
              style={{ flex: 1, minWidth: 240 }}
              placeholder="https://github.com/owner/repository"
              disabled={installing}
            />
            <button
              className="btn btn-soft"
              id="parseRepo"
              type="button"
              disabled={true}
              title="后端暂未提供独立只读清单预检接口 (inspect_manifest)"
            >
              解析清单 (未实现)
            </button>
            <button
              className="btn btn-primary"
              id="confirmInstall"
              type="button"
              onClick={handleConfirmInstall}
              disabled={installing || !repoUrl.trim()}
            >
              {installing ? "正在克隆与导入..." : "克隆并安装"}
            </button>
          </div>

          <p className="sub" style={{ marginTop: 8, color: "var(--muted)" }}>
            只读预检未实现：当前后端未提供只读清单解析与沙箱预审接口，支持直接通过标准 GitHub URL 克隆并安全导入（需系统已安装 git）。
          </p>

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
    </section>
  );
};
