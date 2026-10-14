import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { FlightPanel } from "./FlightPanel";
import { IdePanel } from "./IdePanel";
import { GatewayStatusInfo } from "../types";
import { invoke } from "../lib/tauri";

vi.mock("../lib/tauri", () => ({ invoke: vi.fn().mockRejectedValue(new Error("owned IPC rejection")) }));

const running: GatewayStatusInfo = { is_running: true, port: 28319, master_key_preview: "owned-preview" };
const states: [string, GatewayStatusInfo | null][] = [
  ["unknown", null],
  ["stopped", { ...running, is_running: false }],
  ["running", running],
];

function expectNoOperationalClaims(html: string) {
  for (const claim of ["已接入", "写入成功", "已复制", "pw-sk-proxyweave-master-dev", "pw/coding", "pw/chat", "pw/wild", "模型返回了验证测试流"]) {
    expect(html).not.toContain(claim);
  }
}

describe("Flight / IDE operational truth containment (not native acceptance)", () => {
  it.each(states)("Flight %s state has no fabricated inference or compatibility", (_, gatewayStatus) => {
    const html = renderToStaticMarkup(<FlightPanel gatewayStatus={gatewayStatus} targetPluginId="owned-backend-not-a-model" />);
    expect(html).toMatch(/网关状态未知|网关未启动|暂无可用模型/);
    expect(html).toContain("6 项未测试");
    expect(html).toMatch(/id="sendBtn"[^>]*disabled=""/);
    expect(html).toMatch(/id="chatInput"[^>]*disabled=""/);
    expect(html).not.toContain('class="msg ai"');
    expect(html).not.toContain("通过");
    expect(html).not.toContain("降级");
    expect(html).not.toContain("owned-backend-not-a-model");
    expect(html).not.toMatch(/342 ms|48\.6|141 tokens/);
    expectNoOperationalClaims(html);
    expect(invoke).not.toHaveBeenCalled(); // Static render never triggers IPC; no fabricated inference on mount.
  });

  it.each(states)("IDE %s state keeps connection unknown and all writes disabled", (_, gatewayStatus) => {
    const html = renderToStaticMarkup(<IdePanel gatewayStatus={gatewayStatus} masterKey={null} />);
    for (const claim of ["写入成功", "已复制", "pw-sk-proxyweave-master-dev", "pw/coding"]) expect(html).not.toContain(claim);
    expect(html.match(/data-agent-stat=/g)).toHaveLength(4);
    expect(html).not.toContain('data-agent-target=');
    expect(html).toMatch(/disabled=""[^>]*>.*?一键更新所有已安装智能体/s);
    expect(html).not.toContain("安全配置写入未实现");
    expect(html).not.toContain("ANTHROPIC_AUTH_TOKEN");
    expect(html).not.toContain("openai.apiKey");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("does not invent a port when gateway status is unavailable", () => {
    const html = renderToStaticMarkup(<IdePanel gatewayStatus={null} masterKey="owned-fixture-key" />);
    expect(html).toContain("网关状态未知");
    expect(html).not.toContain("8080");
    expect(html).not.toContain("owned-fixture-key");
    expect(html).not.toContain('aria-label="复制 Master Key"');
  });

  it.each(["", "   "])("treats an empty key %j as unavailable, not as a usable configuration", (masterKey) => {
    const html = renderToStaticMarkup(<IdePanel gatewayStatus={running} masterKey={masterKey} />);
    expect(html).toMatch(/disabled=""[^>]*>.*?一键更新所有已安装智能体/s);
    expect(html).not.toMatch(/写入成功|已复制|pw-sk-proxyweave-master-dev/);
  });

  it.each([0, -1, 65536, 1.5])( "rejects invalid endpoint port %s", (port) => {
    const html = renderToStaticMarkup(<IdePanel gatewayStatus={{ ...running, port }} masterKey="owned-fixture-key" />);
    expect(html).toMatch(/disabled=""[^>]*>.*?一键更新所有已安装智能体/s);
    expect(html).not.toMatch(/写入成功|已复制/);
  });

  it("preserves prototype panel/card/control seams without changing theme CSS", () => {
    const flight = renderToStaticMarkup(<FlightPanel gatewayStatus={null} />);
    const ide = renderToStaticMarkup(<IdePanel gatewayStatus={null} />);
    for (const seam of ['id="panel-flight"', 'class="grid2"', 'id="chatBox"', 'class="grid3"', "Compatibility Flight", "TTFT 首字延迟", "TPS · tok/s"]) expect(flight).toContain(seam);
    for (const seam of ['id="panel-ide"', 'class="ide-workbench"', "智能体工作台", "分发模型配置", "全部客户端", "本机已安装"]) expect(ide).toContain(seam);
  });
});
