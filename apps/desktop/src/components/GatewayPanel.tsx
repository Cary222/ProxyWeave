import React, { useState } from "react";
import { GatewayStatusInfo, TestCallResult } from "../types";
import { invoke } from "../lib/tauri";

interface GatewayPanelProps {
  gatewayStatus: GatewayStatusInfo | null;
  onRefresh: () => void;
}

export const GatewayPanel: React.FC<GatewayPanelProps> = ({
  gatewayStatus,
  onRefresh,
}) => {
  const [targetPort, setTargetPort] = useState<number>(gatewayStatus?.port ?? 28319);
  const [portLoading, setPortLoading] = useState(false);
  const [toggleLoading, setToggleLoading] = useState(false);
  const [testLoading, setTestLoading] = useState(false);
  const [testResult, setTestResult] = useState<TestCallResult | null>(null);
  const [masterKeyFull, setMasterKeyFull] = useState<string | null>(null);
  const [message, setMessage] = useState<{ text: string; error?: boolean } | null>(null);

  // Sync port when gatewayStatus arrives
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
        setMessage({ text: `测试调用成功！耗时: ${res.latency_ms}ms, HTTP 状态: ${res.status}` });
      } else {
        setMessage({ text: `测试调用返回非 200: HTTP ${res.status}`, error: true });
      }
    } catch (e: any) {
      setMessage({ text: String(e), error: true });
    } finally {
      setTestLoading(false);
    }
  }

  const isRunning = gatewayStatus?.is_running ?? false;

  return (
    <div className="panel-section">
      <div className="section-header">
        <div>
          <h3 className="section-title">统一网关服务 (Gateway)</h3>
          <p className="section-desc">
            本地统一 HTTP/SSE 编织网关，提供标准化 OpenAI 协议入口，隔离上游插件生命周期与敏感凭据。
          </p>
        </div>
        <button
          type="button"
          className={isRunning ? "danger-action" : "primary-action"}
          onClick={handleToggleGateway}
          disabled={toggleLoading}
        >
          {toggleLoading ? "切换中..." : isRunning ? "停止网关" : "启动网关"}
        </button>
      </div>

      {message ? (
        <div className={`status-banner ${message.error ? "status-banner--error" : "status-banner--success"}`}>
          {message.text}
        </div>
      ) : null}

      <div className="card-grid">
        <div className="info-card">
          <span className="info-label">运行状态</span>
          <span className={`status-pill ${isRunning ? "status-pill--ready" : "status-pill--stopped"}`}>
            {isRunning ? "Running (运行中)" : "Stopped (已停止)"}
          </span>
        </div>
        <div className="info-card">
          <span className="info-label">监听地址</span>
          <span className="info-value">
            {isRunning ? gatewayStatus?.listen_address ?? `127.0.0.1:${gatewayStatus?.port}` : "—"}
          </span>
        </div>
        <div className="info-card">
          <span className="info-label">当前挂载插件</span>
          <span className="info-value">
            {gatewayStatus?.active_backend ? (
              <strong>{gatewayStatus.active_backend}</strong>
            ) : (
              <span className="text-muted">（未挂载就绪后端）</span>
            )}
          </span>
        </div>
      </div>

      {/* Master Key and Auth Configuration */}
      <div className="card-box">
        <h4 className="card-box-title">本地鉴权凭据 (Master Key)</h4>
        <p className="card-box-desc">
          访问本地网关需要携带 <code>Authorization: Bearer &lt;MASTER_KEY&gt;</code>。该密钥安全存放于 Windows Credential Manager，永不落盘。
        </p>
        <div className="token-row">
          <div className="token-display">
            <code>{masterKeyFull ? masterKeyFull : gatewayStatus?.master_key_preview ?? "••••••••"}</code>
          </div>
          <div className="token-actions">
            {!masterKeyFull ? (
              <button type="button" className="secondary-action" onClick={handleViewMasterKey}>
                查看完整 Key
              </button>
            ) : (
              <button
                type="button"
                className="secondary-action"
                onClick={() => {
                  navigator.clipboard.writeText(masterKeyFull);
                  setMessage({ text: "Master Key 已复制到剪贴板！" });
                }}
              >
                复制 Key
              </button>
            )}
            <button
              type="button"
              className="ghost-action"
              onClick={handleRegenerateMasterKey}
              disabled={isRunning}
              title={isRunning ? "网关运行中不可更换密钥，请先停止网关" : "重新生成安全随机 Key"}
            >
              重新生成
            </button>
          </div>
        </div>
      </div>

      {/* Port Configuration */}
      <div className="card-box">
        <h4 className="card-box-title">端口设置</h4>
        <form onSubmit={handleSavePort} className="port-form">
          <div className="form-group">
            <label htmlFor="gateway-port-input">监听端口 (Port):</label>
            <input
              id="gateway-port-input"
              type="number"
              min={1024}
              max={65535}
              value={targetPort}
              onChange={(e) => setTargetPort(Number(e.target.value))}
              disabled={isRunning || portLoading}
              className="form-input"
            />
          </div>
          <button
            type="submit"
            className="secondary-action"
            disabled={isRunning || portLoading || targetPort === gatewayStatus?.port}
          >
            {portLoading ? "保存中..." : "保存端口"}
          </button>
          {isRunning ? (
            <span className="form-hint">（网关正在运行中，需先停止网关方可修改端口）</span>
          ) : null}
        </form>
      </div>

      {/* Test Endpoint Trigger */}
      <div className="card-box">
        <div className="section-header">
          <div>
            <h4 className="card-box-title">网关连通性自测 (Test Gateway)</h4>
            <p className="card-box-desc">
              向本机统一网关发起 <code>GET /v1/models</code> 请求，验证鉴权、挂载后端路由透传及模型探测结果。
            </p>
          </div>
          <button
            type="button"
            className="primary-action"
            onClick={handleTestModels}
            disabled={!isRunning || testLoading}
          >
            {testLoading ? "请求中..." : "探测 /v1/models"}
          </button>
        </div>

        {testResult ? (
          <div className="test-result-box">
            <div className="test-result-meta">
              <span className={`status-pill ${testResult.success ? "status-pill--ready" : "status-pill--failed"}`}>
                HTTP {testResult.status} {testResult.success ? "OK" : "Error"}
              </span>
              <span className="test-latency">延迟: {testResult.latency_ms} ms</span>
            </div>
            <pre className="test-body-pre">
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
    </div>
  );
};
