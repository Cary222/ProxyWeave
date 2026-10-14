import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AppShell } from "./AppShell";
import { Overview, ResourceReadState } from "./Overview";
import { GatewayStatusInfo, HostEvent, PluginRecord } from "../types";
import { api } from "../api";
import { invoke } from "../lib/tauri";

vi.mock("../lib/tauri", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "quit_app") return Promise.reject(new Error("simulated quit_app rejection"));
    if (cmd.startsWith("plugin:window|")) return Promise.reject(new Error("simulated window rejection"));
    return Promise.reject(new Error("simulated IPC rejection"));
  }),
}));

const samplePlugin: PluginRecord = {
  manifest: {
    schema_version: "1.0",
    metadata: {
      id: "wb2api",
      name: "WorkBuddy Bridge",
      version: "0.2.0",
      description: "Managed plugin",
    },
    runtime: {
      ownership: "managed",
      executable: "wb2api.exe",
    },
    configuration: {
      required_secrets: [],
    },
  },
  settings: {},
};

const sampleEvent: HostEvent = {
  id: 1,
  timestamp_ms: 1700000000000,
  category: "plugin",
  action: "start",
  target: "wb2api",
  success: true,
  summary: "启动受管插件进程成功",
};

describe("DEF-018 / DEF-019 UI Truth Shell & Native Window Controls (SSR & Unit)", () => {
  describe("DEF-018: Native Window Control Accessibility & Semantics", () => {
    it("renders window dots as accessible buttons with proper aria-label and type='button'", () => {
      const html = renderToStaticMarkup(<AppShell />);
      // 必须是真实的 button 标签，且具备明确的 aria-label
      expect(html).toContain('<button type="button" class="win-dot win-dot--close"');
      expect(html).toContain('<button type="button" class="win-dot win-dot--min"');
      expect(html).toContain('<button type="button" class="win-dot win-dot--max"');
      expect(html).toContain('aria-label="关闭"');
      expect(html).toContain('aria-label="最小化"');
      expect(html).toContain('aria-label="最大化"');
      // 必须保留外层 .dots / .win-dots 视觉结构
      expect(html).toContain('class="dots win-dots"');
    });

    it("api.ts provides real window control functions and does not swallow quit errors", async () => {
      expect(typeof api.minimizeWindow).toBe("function");
      expect(typeof api.toggleMaximizeWindow).toBe("function");
      expect(typeof api.hideWindow).toBe("function");
      expect(typeof api.quitApp).toBe("function");
      expect(typeof api.showMenubar).toBe("function");

      // quitApp 应当透传底层错误，不被静默吞掉
      await expect(api.quitApp()).rejects.toThrow("simulated quit_app rejection");
      expect(invoke).toHaveBeenCalledWith("quit_app");
    });
  });

  describe("DEF-019: Resource Read State Truth & Roadmap Honesty", () => {
    it("renders unknown/loading state on initial fetch failure, never fake 0 or empty", () => {
      const loadingGw: ResourceReadState<GatewayStatusInfo> = {
        status: "loading",
        data: null,
      };
      const loadingPlgs: ResourceReadState<PluginRecord[]> = {
        status: "loading",
        data: null,
      };
      const loadingEvts: ResourceReadState<HostEvent[]> = {
        status: "loading",
        data: null,
      };

      const html = renderToStaticMarkup(
        <Overview
          dataReady={false}
          gatewayState={loadingGw}
          pluginsState={loadingPlgs}
          eventsState={loadingEvts}
          onNavigate={() => {}}
        />
      );

      // 网关与插件卡片必须如实显示未知/读取中
      expect(html).toContain("未知");
      expect(html).toContain("读取中");
      expect(html).toContain("正在读取宿主审计事件…");
      // 绝不能伪造为已停止、网关空闲或 0 个插件
      expect(html).not.toContain("已停止");
      expect(html).not.toContain("0 个");
      expect(html).not.toContain("暂无审计事件记录");
    });

    it("renders explicit error state when initial resource fetch fails", () => {
      const errorGw: ResourceReadState<GatewayStatusInfo> = {
        status: "error",
        data: null,
        error: "网络连接断开",
      };
      const errorPlgs: ResourceReadState<PluginRecord[]> = {
        status: "error",
        data: null,
        error: "IPC 调用失败",
      };
      const errorEvts: ResourceReadState<HostEvent[]> = {
        status: "error",
        data: null,
        error: "审计数据损坏",
      };

      const html = renderToStaticMarkup(
        <Overview
          dataReady={true}
          gatewayState={errorGw}
          pluginsState={errorPlgs}
          eventsState={errorEvts}
          onNavigate={() => {}}
        />
      );

      expect(html).toContain("状态未知");
      expect(html).toContain("读取网关状态失败");
      expect(html).toContain("审计事件读取失败");
      expect(html).not.toContain("0 个");
      expect(html).not.toContain("暂无审计事件记录");
      expect(html).not.toContain("网关空闲");
    });

    it("retains last confirmed data on polling failure and displays stale warning", () => {
      const staleGw: ResourceReadState<GatewayStatusInfo> = {
        status: "confirmed",
        data: { is_running: true, port: 28319, active_backend: "wb2api", master_key_preview: "pw-sk-***" },
        isStale: true,
        error: "轮询失败",
      };
      const stalePlgs: ResourceReadState<PluginRecord[]> = {
        status: "confirmed",
        data: [samplePlugin],
        isStale: true,
        error: "轮询失败",
      };
      const staleEvts: ResourceReadState<HostEvent[]> = {
        status: "confirmed",
        data: [sampleEvent],
        isStale: true,
        error: "轮询失败",
      };

      const html = renderToStaticMarkup(
        <Overview
          dataReady={true}
          gatewayState={staleGw}
          pluginsState={stalePlgs}
          eventsState={staleEvts}
          onNavigate={() => {}}
        />
      );

      // 必须保留已确认的数据（1 个插件、运行中）
      expect(html).toContain("1 个 (过期)");
      expect(html).toContain("运行中 (过期)");
      expect(html).toContain("127.0.0.1:28319 (刷新失败)");
      expect(html).toContain("数据刷新失败，当前展示最后确认状态（数据可能已过期）");
      // 绝不能在刷新失败时被清空为 0 个
      expect(html).not.toContain("0 个");
    });

    it("truthfully renders 0 plugins and empty events when confirmed read returns empty array", () => {
      const emptyPlgs: ResourceReadState<PluginRecord[]> = {
        status: "confirmed",
        data: [],
        isStale: false,
      };
      const emptyEvts: ResourceReadState<HostEvent[]> = {
        status: "confirmed",
        data: [],
        isStale: false,
      };
      const stoppedGw: ResourceReadState<GatewayStatusInfo> = {
        status: "confirmed",
        data: { is_running: false, port: 28319, master_key_preview: "pw-sk-***" },
        isStale: false,
      };

      const html = renderToStaticMarkup(
        <Overview
          dataReady={true}
          gatewayState={stoppedGw}
          pluginsState={emptyPlgs}
          eventsState={emptyEvts}
          onNavigate={() => {}}
        />
      );

      // 真正成功返回空数组时，如实显示 0 个与暂无记录
      expect(html).toContain("0 个");
      expect(html).toContain("已停止");
      expect(html).toContain("暂无审计事件记录");
      expect(html).not.toContain("stale-warning");
    });

    it("supports partial resource failure where successful resource displays normally", () => {
      const errorGw: ResourceReadState<GatewayStatusInfo> = {
        status: "error",
        data: null,
      };
      const confirmedPlgs: ResourceReadState<PluginRecord[]> = {
        status: "confirmed",
        data: [samplePlugin],
        isStale: false,
      };

      const html = renderToStaticMarkup(
        <Overview
          dataReady={true}
          gatewayState={errorGw}
          pluginsState={confirmedPlgs}
          onNavigate={() => {}}
        />
      );

      // 网关为错误/未知，但插件正常展示 1 个
      expect(html).toContain("读取网关状态失败");
      expect(html).toContain("1 个");
    });

    it("Overview roadmap truthfully reflects Plan.md phases without fabricated percentages", () => {
      const html = renderToStaticMarkup(<Overview onNavigate={() => {}} />);
      // 必须如实反映 Plan.md 权威阶段
      expect(html).toContain("Phase -1 · 治理与契约");
      expect(html).toContain("Phase 0 · 基座与双模插件");
      expect(html).toContain("Phase 1 · 核心微内核与宿主闭环");
      expect(html).toContain("Phase 2 · 协议编解码与高级流式管道");
      expect(html).toContain("Phase 3 · 桌面控制台与插件市场");
      expect(html).toContain("Phase 4 · 发布与全平台流水线");
      // 严禁虚构 95% 或随意重命名里程碑
      expect(html).not.toContain("95%");
      expect(html).not.toContain("90%");
      expect(html).not.toContain("99%");
    });
  });
});
