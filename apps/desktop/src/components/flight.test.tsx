import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { FlightPanel } from "./FlightPanel";
import { api } from "../api";
import {
  FlightChatRequestInput,
  FlightStreamEvent,
  FlightToolCall,
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

      // Dispatch tool_call_delta event (fragment aggregation passthrough)
      capturedChannel!.onmessage!({
        type: "tool_call_delta",
        request_id: "req-stream-002",
        fragments: [
          { index: 0, id: "call_abc", name: "get_weather", arguments_delta: "{\"city\":\"" },
        ],
        elapsed_ms: 200,
      });

      // Dispatch completed event with total usage and assembled tool calls
      capturedChannel!.onmessage!({
        type: "completed",
        request_id: "req-stream-002",
        total_text: "第一段流式内容 全部完成",
        total_reasoning: "思考中... 完成",
        first_content_ms: 150,
        elapsed_ms: 500,
        saw_done: true,
        usage: { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 },
        finish_reason: "tool_calls",
        total_tool_calls: [
          { id: "call_abc", type: "function", name: "get_weather", arguments: "{\"city\":\"Beijing\"}" },
        ],
      });

      expect(events).toHaveLength(4);
      expect(events[0].type).toBe("started");
      expect(events[1].type).toBe("delta");
      expect(events[2].type).toBe("tool_call_delta");
      expect(events[3].type).toBe("completed");
      if (events[2].type === "tool_call_delta") {
        expect(events[2].fragments[0].name).toBe("get_weather");
        expect(events[2].fragments[0].arguments_delta).toContain("city");
      }
      if (events[3].type === "completed") {
        expect(events[3].usage?.completion_tokens).toBe(20);
        expect(events[3].elapsed_ms).toBe(500);
        expect(events[3].finish_reason).toBe("tool_calls");
        const calls = events[3].total_tool_calls as FlightToolCall[];
        expect(calls).toHaveLength(1);
        expect(calls[0].name).toBe("get_weather");
        expect(calls[0].arguments).toContain("Beijing");
        // Verify real TPS calculation formula: 20 / 0.5s = 40.0
        const tps = (events[3].usage!.completion_tokens / (events[3].elapsed_ms / 1000)).toFixed(1);
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

  describe("Tool-Calling Payload Passthrough", () => {
    it("forwards tools and tool_choice verbatim in streaming requests", async () => {
      const req: FlightChatRequestInput = {
        model: "gpt-4o",
        messages: [{ role: "user", content: "北京天气如何？" }],
        stream: true,
        tools: [
          {
            type: "function",
            function: {
              name: "get_weather",
              description: "Get current weather for a city",
              parameters: {
                type: "object",
                properties: { city: { type: "string" } },
                required: ["city"],
              },
            },
          },
        ],
        tool_choice: "auto",
      };
      let capturedArgs: unknown = null;
      vi.mocked(tauriLib.invoke).mockImplementation(async (cmd, args) => {
        if (cmd === "flight_chat_stream") {
          capturedArgs = args;
          return { request_id: "req-tools-004", model: "gpt-4o" };
        }
        throw new Error("unexpected command: " + cmd);
      });
      await api.flightChatStream(req, () => {});
      expect(capturedArgs).not.toBeNull();
      const input = (capturedArgs as { input: FlightChatRequestInput }).input;
      expect(Array.isArray(input.tools)).toBe(true);
      expect((input.tools as unknown[]).length).toBe(1);
      expect(input.tool_choice).toBe("auto");
    });

    it("forwards tools verbatim in non-streaming requests and parses tool_calls", async () => {
      const req: FlightChatRequestInput = {
        model: "gpt-4o",
        messages: [{ role: "user", content: "北京天气如何？" }],
        stream: false,
        tools: [
          { type: "function", function: { name: "get_weather", parameters: { type: "object" } } },
        ],
        tool_choice: "auto",
      };
      const mockResp = {
        request_id: "req-tools-005",
        model: "gpt-4o",
        text: "",
        first_content_ms: 90,
        elapsed_ms: 210,
        finish_reason: "tool_calls",
        tool_calls: [
          { id: "call_x", type: "function", name: "get_weather", arguments: "{\"city\":\"Beijing\"}" },
        ],
      };
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockResp);
      const result = await api.flightChatNonstream(req);
      expect(tauriLib.invoke).toHaveBeenCalledWith("flight_chat_nonstream", { input: req });
      expect(result.finish_reason).toBe("tool_calls");
      expect(result.tool_calls).toHaveLength(1);
      expect(result.tool_calls![0].name).toBe("get_weather");
      expect(result.tool_calls![0].arguments).toContain("Beijing");
    });

    it("supports multimodal content parts in message DTOs", async () => {
      const req: FlightChatRequestInput = {
        model: "gpt-4o-vision",
        messages: [
          {
            role: "user",
            content: "描述这张图",
            content_parts: [
              { type: "text", text: "描述这张图" },
              { type: "image_url", image_url: { url: "https://example.com/a.png" } },
            ],
          },
        ],
        stream: false,
      };
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce({
        request_id: "req-mm-006",
        model: "gpt-4o-vision",
        text: "图中是…",
        elapsed_ms: 100,
      });
      await api.flightChatNonstream(req);
      expect(tauriLib.invoke).toHaveBeenCalledWith("flight_chat_nonstream", { input: req });
      expect(req.messages[0].content_parts).toHaveLength(2);
      expect(req.messages[0].content_parts![1].type).toBe("image_url");
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
    // Multimodal stays untested and is annotated as not supported by the current backend (never faked as passed).
    expect(html).not.toContain("未测试 · 当前后端不支持");
    expect(html).toContain("当前后端未声明多模态能力");
    // Long-context selector present
    expect(html).toContain('id="longContextBytes"');
    expect(html).toContain("长上下文");
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
