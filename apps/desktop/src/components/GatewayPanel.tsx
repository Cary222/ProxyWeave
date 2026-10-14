import React, { useState } from "react";
import { GatewayStatusInfo, PluginRecord, TestCallResult } from "../types";
import { invoke } from "../lib/tauri";
import { Icon } from "./Icon";

interface GatewayPanelProps {
  gatewayStatus?: GatewayStatusInfo | null;
  status?: GatewayStatusInfo | null;
  plugins?: PluginRecord[];
  masterKey?: string | null;
  onRefresh: () => void | Promise<void>;
}

export const GatewayPanel: React.FC<GatewayPanelProps> = ({
  gatewayStatus: explicitStatus,
  status: altStatus,
  plugins: _plugins,
  masterKey: initialMasterKey,
  onRefresh,
}) => {
  const gatewayStatus = explicitStatus !== undefined ? explicitStatus : altStatus ?? null;
  const [targetPort, setTargetPort] = useState<number>(gatewayStatus?.port ?? 28319);
  const [portLoading, setPortLoading] = useState(false);
  const [toggleLoading, setToggleLoading] = useState(false);
  const [testLoading, setTestLoading] = useState(false);
  const [testResult, setTestResult] = useState<TestCallResult | null>(null);
  const [masterKeyFull, setMasterKeyFull] = useState<string | null>(initialMasterKey ?? null);
  const [message, setMessage] = useState<{ text: string; error?: boolean } | null>(null);

  React.useEffect(() => {
    if (initialMasterKey) {
      setMasterKeyFull(initialMasterKey);
    }
  }, [initialMasterKey]);

  React.useEffect(() => {
    if (gatewayStatus) {
      setTargetPort(gatewayStatus.port);
    }
  }, [gatewayStatus]);

  async function handleToggleGateway() {
    setToggleLoading(true);
    setMessage(null);
    try {
      if (gatewayStatus?.is_running) {
        await invoke("stop_gateway");
        setMessage({ text: "统一网关已安全停止，活跃连接已释放。" });
      } else {
        await invoke("start_gateway");
        setMessage({ text: "统一网关已成功启动并就绪。" });
      }
      onRefresh();
    } catch (e: any) {
      setMessage({ text: String(e), error: true });
    } finally {
      setToggleLoading(false);
    }
  }

  async function handleSavePort(e: React.FormEvent) {
    e.preventDefault();
    setPortLoading(true);
    setMessage(null);
    try {
      await invoke("set_gateway_port", { port: Number(targetPort) });
      setMessage({ text: `网关监听端口已更新为 ${targetPort}` });
      onRefresh();
    } catch (e: any) {
      setMessage({ text: String(e), error: true });
    } finally {
      setPortLoading(false);
    }
  }

  async function handleViewMasterKey() {
    try {
      const key: string = await invoke("get_gateway_master_key");
      setMasterKeyFull(key);
    } catch (e: any) {
      setMessage({ text: String(e), error: true });
    }
  }

  async function handleRegenerateMasterKey() {
    if (!window.confirm("确定要重新生成本地网关 Master Key 吗？已有客户端需要同步更新 Bearer Token。")) {
      return;
    }
    setMessage(null);
    try {
      const newKey: string = await invoke("regenerate_gateway_master_key");
      setMasterKeyFull(newKey);
      setMessage({ text: "Master Key 已重新生成并保存到系统安全凭据库！" });
      onRefresh();
    } catch (e: any) {
      setMessage({ text: String(e), error: true });
    }
  }

  async function handleTestModels() {
    setTestLoading(true);
    setMessage(null);
    setTestResult(null);
    try {
      const res: TestCallResult = await invoke("test_gateway_models");
      setTestResult(res);
      if (res.success) {
        setMessage({ text: `探测成功！耗时: ${res.latency_ms}ms, HTTP: ${res.status}` });
      } else {
        setMessage({ text: `探测失败: HTTP ${res.status}`, error: true });
      }
    } catch (e: any) {
      setMessage({ text: String(e), error: true });
    } finally {
      setTestLoading(false);
    }
  }

  const isRunning = gatewayStatus?.is_running ?? false;

  return (
    <section className="panel" id="panel-gateway">
      <div className="topbar">
        <div>
          <h2>统一网关</h2>
        </div>
        <button
          className={`btn ${isRunning ? "btn-danger-ghost" : "btn-primary"}`}
          id="gwToggle"
          type="button"
          onClick={handleToggleGateway}
          disabled={toggleLoading || gatewayStatus === null}
        >
          {toggleLoading ? "切换中..." : isRunning ? "停止网关" : "启动网关"}
        </button>
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

      <div className="grid3">
        <div className="card kpi">
          <label>状态</label>
          <div className="big">
            <span
              className={`pill ${isRunning ? "pill-green" : "pill-gray"}`}
              id="gwStatusPill"
              style={{ fontSize: 13, padding: "6px 12px" }}
            >
              <span className="dot"></span>
              {gatewayStatus === null ? "未知" : isRunning ? "Running" : "Stopped"}
            </span>
          </div>
          <div className="note" id="gwStatusNote">
            {isRunning ? "监听中 · 接受请求" : "未启动"}
          </div>
        </div>

        <div className="card kpi">
          <label>监听地址</label>
          <div className="big mono" id="gwAddr">
            {isRunning ? (gatewayStatus?.listen_address ?? `127.0.0.1:${gatewayStatus?.port}`) : "—"}
          </div>
          <div className="note">Loopback 127.0.0.1 only</div>
        </div>

        <div className="card kpi">
          <label>挂载插件</label>
          <div className="big" style={{ fontSize: 16 }}>
            {gatewayStatus?.active_backend ? gatewayStatus.active_backend : "（未指定后端）"}
          </div>
          <div className="note">OpenAI 协议透传</div>
        </div>
      </div>

      <div style={{ height: 16 }}></div>

      <div className="stack">
        <div className="card">
          <h3>Master Key</h3>
          <p className="sub">Keyring 托管 · 永不落盘</p>
          <div className="keybox">
            <code id="masterKey">
              {masterKeyFull
                ? masterKeyFull
                : gatewayStatus?.master_key_preview
                ? `pw-sk-••••••••${gatewayStatus.master_key_preview.slice(-4)}`
                : "pw-sk-••••••••••••••••"}
            </code>
            {!masterKeyFull ? (
              <button
                className="btn btn-ghost btn-sm"
                id="revealKey"
                type="button"
                onClick={handleViewMasterKey}
              >
                <Icon name="eye" className="ic sm" />
                查看完整 Key
              </button>
            ) : null}
            <button
              className="btn btn-ghost btn-sm"
              id="copyKey"
              type="button"
              onClick={() => {
                const textToCopy = masterKeyFull ?? gatewayStatus?.master_key_preview ?? "";
                if (textToCopy) {
                  void navigator.clipboard.writeText(textToCopy).then(
                    () => setMessage({ text: "Master Key 已复制到剪贴板！" }),
                    () => setMessage({ text: "复制失败", error: true })
                  );
                }
              }}
            >
              <Icon name="copy" className="ic sm" />
              复制
            </button>
            <button
              className="btn btn-ghost btn-sm"
              id="regenKey"
              style={{ color: "var(--red)" }}
              type="button"
              onClick={handleRegenerateMasterKey}
              disabled={isRunning || gatewayStatus === null}
              title={isRunning ? "需先停止网关方可重置" : "重新生成随机密钥"}
            >
              重新生成
            </button>
          </div>
        </div>

        <div className="card">
          <h3>端口设置</h3>
          <form onSubmit={handleSavePort} className="field" style={{ marginTop: 10 }}>
            <label htmlFor="portInput" style={{ fontWeight: 500, fontSize: 14 }}>
              端口：
            </label>
            <input
              className="input mono"
              id="portInput"
              type="number"
              min={1024}
              max={65535}
              value={targetPort}
              onChange={(e) => setTargetPort(Number(e.target.value))}
              disabled={isRunning || portLoading || gatewayStatus === null}
              style={{ width: 120 }}
            />
            <button
              className="btn btn-soft btn-sm"
              id="savePort"
              type="submit"
              disabled={isRunning || portLoading || targetPort === gatewayStatus?.port}
            >
              {portLoading ? "保存中..." : "保存"}
            </button>
            <span className="sub">
              {isRunning ? "网关运行中不可修改端口" : "127.0.0.1 only"}
            </span>
          </form>
        </div>

        <div className="card">
          <div className="section-head" style={{ margin: 0 }}>
            <div>
              <h3>连通性自测</h3>
              <p className="sub">向本机统一网关发起 GET /v1/models 探测请求</p>
            </div>
            <button
              className="btn btn-primary"
              id="probeBtn"
              type="button"
              onClick={handleTestModels}
              disabled={!isRunning || testLoading}
            >
              {testLoading ? "探测中..." : "探测 /v1/models"}
            </button>
          </div>

          {testResult ? (
            <div
              id="probeResult"
              style={{
                marginTop: 14,
                background: "var(--well)",
                border: "1px solid var(--line)",
                borderRadius: 12,
                padding: "14px 16px",
              }}
            >
              <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 10 }}>
                <span className={`pill ${testResult.success ? "pill-green" : "pill-red"}`}>
                  HTTP {testResult.status} {testResult.success ? "OK" : "Error"}
                </span>
                <span className="sub">耗时: {testResult.latency_ms} ms</span>
              </div>
              <pre
                style={{
                  margin: 0,
                  fontSize: 12,
                  fontFamily: "var(--mono)",
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-all",
                  maxHeight: 200,
                  overflowY: "auto",
                }}
              >
                {(() => {
                  try {
                    return JSON.stringify(JSON.parse(testResult.response_body), null, 2);
                  } catch {
                    return testResult.response_body;
                  }
                })()}
              </pre>
            </div>
          ) : null}
        </div>

        <div className="card">
          <h3>模型路由</h3>
          <table>
            <thead>
              <tr>
                <th>虚拟别名 Alias</th>
                <th>首选路由 Primary</th>
                <th>Fallback 链</th>
                <th>协议</th>
                <th>状态</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td className="mono">
                  <b>pw/coding</b>
                </td>
                <td>{gatewayStatus?.active_backend ?? "wb2api"} · workbuddy-code</td>
                <td className="sub">→ external-cpa · gpt-5-codex</td>
                <td>
                  <span className="tag">Chat ⇄ IR ⇄ Messages</span>
                </td>
                <td>
                  <span className="pill pill-green">
                    <span className="dot"></span>健康
                  </span>
                </td>
              </tr>
              <tr>
                <td className="mono">
                  <b>pw/chat</b>
                </td>
                <td>external-cpa · gpt-5</td>
                <td className="sub">→ wb2api · workbuddy-chat</td>
                <td>
                  <span className="tag">透传 Passthrough</span>
                </td>
                <td>
                  <span className="pill pill-green">
                    <span className="dot"></span>健康
                  </span>
                </td>
              </tr>
              <tr>
                <td className="mono">
                  <b>pw/wild</b>
                </td>
                <td>wild-work · wild-agent</td>
                <td className="sub">—（无备用，明确拒绝不降级伪造）</td>
                <td>
                  <span className="tag">Responses ⇄ Chat</span>
                </td>
                <td>
                  <span className="pill pill-amber">
                    <span className="dot"></span>休眠待唤醒
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
          <p className="sub" style={{ marginTop: 12 }}>
            兼容策略：透传 / 转换 / 降级 / 拒绝 / 未验证
          </p>
        </div>
      </div>
    </section>
  );
};
