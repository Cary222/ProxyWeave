import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ModelSearchModal } from "./ModelSearchModal";
import { FlightPanel } from "./FlightPanel";
import { AppShell } from "./AppShell";
import { GatewayStatusInfo } from "../types";

vi.mock("../lib/tauri", () => {
  class MockChannel<T> {
    onmessage: ((message: T) => void) | null = null;
  }
  return {
    invoke: vi.fn().mockImplementation((cmd: string) => {
      if (cmd === "flight_list_models") {
        return Promise.resolve({
          models: [
            { id: "claude-3-7-sonnet", name: "Claude 3.7 Sonnet", owned_by: "anthropic" },
            { id: "gpt-4o", name: "GPT-4o", owned_by: "openai" },
            { id: "deepseek-r1", name: "DeepSeek R1", owned_by: "deepseek" },
            { id: "custom-llama", name: "Llama 3 70B", owned_by: "meta" },
          ],
          active_backend: "wb2api",
        });
      }
      return Promise.resolve(null);
    }),
    Channel: MockChannel,
  };
});

const mockRunningGateway: GatewayStatusInfo = {
  is_running: true,
  port: 28319,
  master_key_preview: "sk-test",
};

const mockStoppedGateway: GatewayStatusInfo = {
  is_running: false,
  port: 28319,
  master_key_preview: "sk-test",
};

describe("ModelSearchModal & Global Ctrl+K Integration (Phase F3)", () => {
  it("renders null when isOpen is false", () => {
    const html = renderToStaticMarkup(
      <ModelSearchModal
        isOpen={false}
        onClose={() => {}}
        gatewayStatus={mockRunningGateway}
        onSelectModelFlight={() => {}}
      />
    );
    expect(html).toBe("");
  });

  it("renders stopped gateway warning and navigation prompt when gateway is off", () => {
    const html = renderToStaticMarkup(
      <ModelSearchModal
        isOpen={true}
        onClose={() => {}}
        gatewayStatus={mockStoppedGateway}
        onSelectModelFlight={() => {}}
        onNavigateGateway={() => {}}
      />
    );
    expect(html).toContain("全局模型与快速试飞检索");
    expect(html).toContain("本地统一网关服务未启动");
    expect(html).toContain("前往统一网关启动服务");
  });

  it("renders search input, category filters, and navigation shortcuts", () => {
    const html = renderToStaticMarkup(
      <ModelSearchModal
        isOpen={true}
        onClose={() => {}}
        gatewayStatus={mockRunningGateway}
        onSelectModelFlight={() => {}}
      />
    );
    expect(html).toContain('id="modelSearchInput"');
    expect(html).toContain("搜索模型名称、ID 或厂商");
    expect(html).toContain("Claude");
    expect(html).toContain("GPT / OpenAI");
    expect(html).toContain("DeepSeek");
    expect(html).toContain("其他模型");
    expect(html).toContain("Enter");
    expect(html).toContain("Esc");
  });

  it("AppShell renders quick model search trigger button with Ctrl+K badge", () => {
    const html = renderToStaticMarkup(<AppShell />);
    expect(html).toContain('id="quickModelSearchBtn"');
    expect(html).toContain("快速检索模型…");
    expect(html).toContain("Ctrl K");
  });

  it("FlightPanel respects initialModel prop and renders without crash", () => {
    const html = renderToStaticMarkup(
      <FlightPanel
        gatewayStatus={mockRunningGateway}
        initialModel="claude-3-7-sonnet"
      />
    );
    expect(html).toContain("试飞工作台");
  });
});
