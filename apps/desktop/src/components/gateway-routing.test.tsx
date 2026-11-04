import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { GatewayPanel } from "./GatewayPanel";
import { GatewayStatusInfo } from "../types";

vi.mock("../lib/tauri", () => {
  return {
    invoke: vi.fn().mockImplementation((cmd: string, args?: any) => {
      if (cmd === "set_gateway_route_strategy") {
        return Promise.resolve({
          is_running: true,
          port: 28319,
          master_key_preview: "sk-test",
          route_strategy: args?.strategy ?? "smart",
        });
      }
      return Promise.resolve(null);
    }),
  };
});

const mockRunningGateway: GatewayStatusInfo = {
  is_running: true,
  port: 28319,
  master_key_preview: "sk-test",
  route_strategy: "smart",
};

describe("GatewayPanel Routing & Prompt Cache Stickiness (Phase F5)", () => {
  it("renders 3 routing strategies: Smart, Order, Rotate", () => {
    const html = renderToStaticMarkup(
      <GatewayPanel
        gatewayStatus={mockRunningGateway}
        onRefresh={() => {}}
      />
    );

    expect(html).toContain("id=\"gwRoutingCard\"");
    expect(html).toContain("id=\"route-strat-smart\"");
    expect(html).toContain("id=\"route-strat-order\"");
    expect(html).toContain("id=\"route-strat-rotate\"");
    expect(html).toContain("智能优选 (Smart)");
    expect(html).toContain("顺序主备 (Order)");
    expect(html).toContain("轮询负载 (Rotate)");
  });

  it("marks active strategy based on gatewayStatus.route_strategy", () => {
    const htmlSmart = renderToStaticMarkup(
      <GatewayPanel
        gatewayStatus={{ ...mockRunningGateway, route_strategy: "smart" }}
        onRefresh={() => {}}
      />
    );
    expect(htmlSmart).toContain("id=\"route-strat-smart\" class=\"btn btn-sm btn-primary\"");
    expect(htmlSmart).toContain("id=\"route-strat-order\" class=\"btn btn-sm btn-ghost\"");

    const htmlOrder = renderToStaticMarkup(
      <GatewayPanel
        gatewayStatus={{ ...mockRunningGateway, route_strategy: "order" }}
        onRefresh={() => {}}
      />
    );
    expect(htmlOrder).toContain("id=\"route-strat-order\" class=\"btn btn-sm btn-primary\"");
    expect(htmlOrder).toContain("id=\"route-strat-smart\" class=\"btn btn-sm btn-ghost\"");
  });

  it("displays prompt cache session stickiness and fail-open guarantees", () => {
    const html = renderToStaticMarkup(
      <GatewayPanel
        gatewayStatus={mockRunningGateway}
        onRefresh={() => {}}
      />
    );

    expect(html).toContain("Prompt Cache 会话粘性");
    expect(html).toContain("5min 滑动窗口");
    expect(html).toContain("只重排不丢弃 (Fail-Open)");
  });
});
