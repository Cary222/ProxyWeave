import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { FlightPanel } from "./FlightPanel";
import { api } from "../api";
import {
  FlightChatRequestInput,
  FlightStreamEvent,
  GatewayStatusInfo,
} from "../types";
import * as tauriLib from "../lib/tauri";

// Mock invoke and Channel from lib/tauri
vi.mock("../lib/tauri", () => {
  class MockChannel<T> {
    onmessage: ((message: T) => void) | null = null;
  }
  return {
    invoke: vi.fn(),
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

describe("Flight Frontend API Wrapper", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Model Discovery (flightListModels)", () => {
    it("invokes flight_list_models command and returns discovered models", async () => {
      const mockResponse = {
        models: [
          { id: "gpt-4o", name: "GPT-4o (Real)" },
          { id: "claude-3-5-sonnet", name: "Claude 3.5 Sonnet (Real)" },
        ],
        active_backend: "wb2api",
      };
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockResponse);

      const result = await api.flightListModels();
      expect(tauriLib.invoke).toHaveBeenCalledWith("flight_list_models");
      expect(result).toEqual(mockResponse);
      expect(result.models).toHaveLength(2);
      expect(result.models[0].id).toBe("gpt-4o");
    });

    it("propagates error when backend is unmounted or gateway is stopped", async () => {
      vi.mocked(tauriLib.invoke).mockRejectedValueOnce(
        new Error("统一网关尚未挂载任何插件后端")
      );

      await expect(api.flightListModels()).rejects.toThrow(
        "统一网关尚未挂载任何插件后端"
      );
    });
  });

  describe("Non-streaming Inference (flightChatNonstream)", () => {
    it("invokes flight_chat_nonstream with complete request input and parses timing", async () => {
      const req: FlightChatRequestInput = {
        model: "gpt-4o",
        messages: [{ role: "user", content: "请输出非流式测试文本" }],
        temperature: 0.7,
        stream: false,
      };
      const mockResp = {
        request_id: "req-nonstream-001",
        model: "gpt-4o",
        text: "这是非流式真实回复文本",
        first_content_ms: 120,
        elapsed_ms: 300,
        usage: {
          prompt_tokens: 10,
          completion_tokens: 5,
          total_tokens: 15,
        },
      };
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockResp);

      const result = await api.flightChatNonstream(req);
      expect(tauriLib.invoke).toHaveBeenCalledWith("flight_chat_nonstream", {
        input: req,
      });
      expect(result).toEqual(mockResp);
      expect(result.first_content_ms).toBe(120);
      expect(result.elapsed_ms).toBe(300);
      expect(result.usage?.completion_tokens).toBe(5);
    });

    it("handles non-streaming error rejection cleanly", async () => {
      const req: FlightChatRequestInput = {
        model: "invalid-model",
        messages: [{ role: "user", content: "fail" }],
      };
      vi.mocked(tauriLib.invoke).mockRejectedValueOnce(
        new Error("上游模型不存在")
      );

      await expect(api.flightChatNonstream(req)).rejects.toThrow("上游模型不存在");
    });
  });

  describe("Streaming Inference & Channel Lifecycle (flightChatStream)", () => {
    it("binds Tauri Channel and streams delta, completed, and timing events", async () => {
      const req: FlightChatRequestInput = {
        model: "claude-3-5-sonnet",
        messages: [{ role: "user", content: "流式测试" }],
        temperature: 0.5,
        stream: true,
      };
      const startResult = {
        request_id: "req-stream-002",
        model: "claude-3-5-sonnet",
      };

      let capturedChannel: tauriLib.Channel<FlightStreamEvent> | null = null;
      vi.mocked(tauriLib.invoke).mockImplementation(async (cmd, args) => {
        if (cmd === "flight_chat_stream") {
          capturedChannel = (args as { channel: tauriLib.Channel<FlightStreamEvent> }).channel;
          return startResult;
        }
        throw new Error(`Unexpected command: ${cmd}`);
      });

      const events: FlightStreamEvent[] = [];
      const onEvent = (e: FlightStreamEvent) => events.push(e);

      const res = await api.flightChatStream(req, onEvent);
      expect(res).toEqual(startResult);
      expect(capturedChannel).not.toBeNull();

      // Dispatch started event
      capturedChannel!.onmessage!({
        type: "started",
        request_id: "req-stream-002",
        model: "claude-3-5-sonnet",
      });

      // Dispatch delta event with reasoning and TTFT
      capturedChannel!.onmessage!({
        type: "delta",
        request_id: "req-stream-002",
        text: "第一段流式内容",
        reasoning: "思考中...",
        first_content_ms: 150,
        elapsed_ms: 160,
      });

      // Dispatch completed event with total usage
      capturedChannel!.onmessage!({
        type: "completed",
        request_id: "req-stream-002",
        total_text: "第一段流式内容 全部完成",
        total_reasoning: "思考中... 完成",
        first_content_ms: 150,
        elapsed_ms: 500,
        saw_done: true,
        usage: { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
      });

      expect(events).toHaveLength(3);
      expect(events[0].type).toBe("started");
      expect(events[1].type).toBe("delta");
      expect(events[2].type).toBe("completed");
      if (events[2].type === "completed") {
        expect(events[2].usage?.completion_tokens).toBe(20);
        expect(events[2].elapsed_ms).toBe(500);
        // Verify real TPS calculation formula: 20 / 0.5s = 40.0
        const tps = (events[2].usage!.completion_tokens / (events[2].elapsed_ms / 1000)).toFixed(1);
        expect(tps).toBe("40.0");
      }
    });

    it("handles stream failure events", async () => {
      const req: FlightChatRequestInput = {
        model: "gpt-4o",
        messages: [{ role: "user", content: "错误测试" }],
      };
      let capturedChannel: tauriLib.Channel<FlightStreamEvent> | null = null;
      vi.mocked(tauriLib.invoke).mockImplementation(async (cmd, args) => {
        if (cmd === "flight_chat_stream") {
          capturedChannel = (args as { channel: tauriLib.Channel<FlightStreamEvent> }).channel;
          return { request_id: "req-fail-003", model: "gpt-4o" };
        }
        return null;
      });

      const events: FlightStreamEvent[] = [];
      await api.flightChatStream(req, (e) => events.push(e));

      capturedChannel!.onmessage!({
        type: "failed",
        request_id: "req-fail-003",
        error_message: "上游网关连接重置",
        elapsed_ms: 100,
      });

      expect(events).toHaveLength(1);
      expect(events[0].type).toBe("failed");
      if (events[0].type === "failed") {
        expect(events[0].error_message).toBe("上游网关连接重置");
      }
    });
  });

  describe("Cancellation Lifecycle (flightCancel)", () => {
    it("invokes flight_cancel with requestId and returns cancellation confirmation", async () => {
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce({
        cancelled: true,
        message: "已终止会话",
      });

      const result = await api.flightCancel("req-stream-002");
      expect(tauriLib.invoke).toHaveBeenCalledWith("flight_cancel", {
        requestId: "req-stream-002",
      });
      expect(result.cancelled).toBe(true);
    });
  });
});

describe("FlightPanel Static Markup & Truth Verification", () => {
  it("renders with stopped gateway showing truthful prompt and disabled controls", () => {
    const html = renderToStaticMarkup(
      <FlightPanel gatewayStatus={mockStoppedGateway} />
    );

    expect(html).toContain('id="panel-flight"');
    expect(html).toContain("统一网关未启动");
    expect(html).toContain("网关已停止");
    expect(html).toContain('id="flightModel"');
    expect(html).toContain('id="tempRange"');
    expect(html).toContain('id="tempVal"');
    expect(html).toContain('id="chatBox"');
    expect(html).toContain('id="chatInput"');
    expect(html).toContain('id="sendBtn"');
    // Ensure button and input are disabled when gateway is stopped
    expect(html).toMatch(/<button[^>]*id="sendBtn"[^>]*disabled/);
    expect(html).toMatch(/<input[^>]*id="chatInput"[^>]*disabled/);
    // Metrics must show untested
    expect(html).toContain("未测试");
    // Compatibility flight checklist should not claim verified
    expect(html).not.toContain("通过");
    expect(html).not.toContain("降级");
    // No fabricated conversation
    expect(html).not.toContain("pw/coding");
    expect(html).not.toContain("pw/chat");
  });

  it("renders with unknown gateway status showing unknown status message", () => {
    const html = renderToStaticMarkup(<FlightPanel gatewayStatus={null} />);
    expect(html).toContain("网关状态未知");
    expect(html).toMatch(/<button[^>]*id="sendBtn"[^>]*disabled/);
    expect(html).toMatch(/<input[^>]*id="chatInput"[^>]*disabled/);
  });

  it("renders initial state when gateway is running before discovery resolves", () => {
    const html = renderToStaticMarkup(
      <FlightPanel
        gatewayStatus={mockRunningGateway}
        targetPluginId="wb2api"
      />
    );

    expect(html).toContain('id="panel-flight"');
    expect(html).toMatch(/暂无可用模型|正在探测可用模型/);
    // Stream checkbox is checked by default
    expect(html).toContain('type="checkbox"');
    expect(html).toContain("流式传输");
    // Metric cards must have 3 metrics: TTFT, TPS, 总耗时
    expect(html).toContain("TTFT 首字延迟");
    expect(html).toContain("TPS · tok/s");
    expect(html).toContain("总耗时");
    // Checklist contains 6 compatibility checks
    expect(html).toContain("Chat");
    expect(html).toContain("SSE");
    expect(html).toContain("多轮映射");
    expect(html).toContain("Tool Calling");
    expect(html).toContain("长文本");
    expect(html).toContain("多模态");
  });

  it("includes all required ARIA labels and structural elements", () => {
    const html = renderToStaticMarkup(
      <FlightPanel gatewayStatus={mockRunningGateway} />
    );

    expect(html).toContain('aria-label="试飞模型"');
    expect(html).toContain('aria-label="采样温度"');
    expect(html).toContain('aria-label="试飞消息"');
    expect(html).toContain('role="status"');
    expect(html).toContain('id="flightModel"');
    expect(html).toContain('id="tempRange"');
    expect(html).toContain('id="chatInput"');
  });

  it("never displays fabricated metrics or random TPS", () => {
    const html = renderToStaticMarkup(
      <FlightPanel gatewayStatus={mockRunningGateway} />
    );

    // Ensure no static fake metrics like 342 ms, 48.6 TPS, etc.
    expect(html).not.toMatch(/342\s*ms/);
    expect(html).not.toMatch(/48\.6/);
    expect(html).not.toMatch(/141\s*tokens/);
    // All 3 metrics must say "未测试" initially
    const matches = html.match(/未测试/g) || [];
    expect(matches.length).toBeGreaterThanOrEqual(3);
  });
});
