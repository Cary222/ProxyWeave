import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { DiscoverPanel } from "./DiscoverPanel";
import { AuditPanel } from "./AuditPanel";
import { SettingsPanel } from "./SettingsPanel";
import { HostEvent, PluginRecord } from "../types";
import { invoke } from "../lib/tauri";

vi.mock("../lib/tauri", () => ({
  invoke: vi.fn().mockRejectedValue(new Error("owned IPC rejection")),
}));

function expectNoWave2Fakes(html: string) {
  for (const fake of [
    "https://github.com/proxyweave/ollama-bridge",
    "SHA 8f3a1c2",
    "已通过基础沙箱审查",
    "1,240",
    "890",
    "620",
    "410",
    "官方维护",
    "C:/Projects/my-local-plugin",
    "wb2api → workbuddy-code",
    "external-cpa → claude-sonnet-4-5",
    "req_9f8a2c1e",
    "已脱敏",
    "已触发自动切换备选链",
    "诊断包已导出至",
    "(已校验)",
    "(就绪)",
    ".bak 健康",
    "配置已导出为 JSON ✓",
    "选择并导入已有配置文件",
  ]) {
    expect(html).not.toContain(fake);
  }
}

const samplePlugin: PluginRecord = {
  manifest: {
    metadata: {
      id: "wb2api",
      name: "WorkBuddy Bridge",
      version: "0.2.0",
      description: "Managed plugin",
    },
    runtime: {
      kind: "managed",
      entrypoint: "wb2api.exe",
    } as any,
  } as any,
  root_dir: "D:/owned-fixtures/wb2api",
  manifest_path: "D:/owned-fixtures/wb2api/plugin.yaml",
  enabled: true,
  settings: {},
};

describe("Wave2 Discover / Audit / Settings operational truth containment (SSR, not native acceptance)", () => {
  describe("DiscoverPanel static truth containment (DEF015)", () => {
    it("renders empty repo input, disabled manifest parse, and honest topic statuses", () => {
      const html = renderToStaticMarkup(<DiscoverPanel plugins={[]} />);
      expectNoWave2Fakes(html);
      expect(html).toContain('id="panel-discover"');
      expect(html).toContain('id="repoUrl"');
      expect(html).toContain('value=""');
      expect(html).toMatch(/id="parseRepo"[^>]*disabled=""/);
      expect(html).toContain("解析清单 (未实现)");
      expect(html).toMatch(/id="confirmInstall"[^>]*disabled=""/);
      expect(html).toContain("克隆并安装");
      expect(html).toContain("只读预检未实现");
      expect(html.match(/生态展示 · 待接入/g)).toHaveLength(4);
      expect(html.match(/尚未接入/g)).toHaveLength(4);
      expect(html).not.toContain('id="parseResult"');
      expect(invoke).not.toHaveBeenCalled();
    });

    it("marks matching topic row as installed when present in plugins prop", () => {
      const html = renderToStaticMarkup(<DiscoverPanel plugins={[samplePlugin]} />);
      expectNoWave2Fakes(html);
      expect(html).toContain("已安装");
      expect(html.match(/尚未接入/g)).toHaveLength(3);
    });
  });

  describe("AuditPanel static truth containment (DEF016)", () => {
    it("distinguishes unknown events state when events prop is omitted", () => {
      const html = renderToStaticMarkup(<AuditPanel />);
      expectNoWave2Fakes(html);
      expect(html).toContain('id="panel-audit"');
      expect(html).toContain("宿主事件数据状态未知（尚未传入 events 数据）");
      expect(html).toContain("HTTP 请求链路追踪与实时 API Doctor 尚未实现");
      expect(html).toContain("当前未读取到宿主控制事件（状态未知）");
      expect(html).toContain("常见状态码一般说明（静态参考 · 未接入实时诊断）");
      expect(invoke).not.toHaveBeenCalled();
    });

    it("distinguishes empty events list (0 items) from unknown state", () => {
      const html = renderToStaticMarkup(<AuditPanel events={[]} />);
      expectNoWave2Fakes(html);
      expect(html).toContain("暂无宿主控制事件记录（0 条）");
      expect(html).toContain("全部类别 (0)");
      expect(html).toContain("当前无选中的宿主控制事件。");
    });

    it("renders real HostEvent rows and sanitizes malformed event payloads", () => {
      const events: HostEvent[] = [
        {
          id: 101,
          timestamp_ms: 1700000000000,
          category: "plugin",
          action: "start",
          target: "wb2api",
          success: true,
          summary: "启动受管插件进程成功",
        },
        {
          id: Number.NaN as any,
          timestamp_ms: -5 as any,
          category: "   ",
          action: "",
          target: null as any,
          success: null as any,
          summary: "",
        },
      ];
      const html = renderToStaticMarkup(<AuditPanel events={events} />);
      expectNoWave2Fakes(html);
      expect(html).toContain("启动受管插件进程成功");
      expect(html).toContain("2023-11-14 22:13:20");
      expect(html).toContain("成功");
      expect(html).toContain("时间未知");
      expect(html).toContain("unknown");
      expect(html).toContain("（无摘要）");
      expect(html).toContain("&quot;event_type&quot;: &quot;host_control_event&quot;");
    });
  });

  describe("SettingsPanel static truth containment (DEF017)", () => {
    it("disables unbacked system settings, removes fake keyring/backup verification, and keeps theme controls", () => {
      const html = renderToStaticMarkup(
        <SettingsPanel theme="sky" onThemeChange={() => {}} />
      );
      expectNoWave2Fakes(html);
      expect(html).toContain('id="panel-settings"');
      expect(html).toContain("系统级自启状态未知；后端暂未提供自启动读取与设置 IPC");
      expect(html).toContain("未实现");
      expect(html).toContain("面板未直连");
      expect(html).toContain("简体中文 (固定)");
      expect(html).toContain("凭据状态未知 · 未探测");
      expect(html).toContain("~/.proxyweave/desktop_config.json（后端默认存储路径）");
      expect(html).toContain("打开目录 (未实现)");
      expect(html).toContain("未巡检");
      expect(html).toContain("导出主题配置");
      expect(html).toContain("导入… (未实现)");
      expect(html).toContain("重置调色板");
      expect(html).not.toContain("Phase 1");
      expect(html).not.toContain("Plan.md v3.1.0");
      expect(invoke).not.toHaveBeenCalled();
    });

    it("renders active theme card and shows glass sliders only in glass theme", () => {
      const skyHtml = renderToStaticMarkup(
        <SettingsPanel theme="sky" onThemeChange={() => {}} />
      );
      expect(skyHtml).toMatch(/theme-card active[^>]*data-theme-set="sky"/);
      expect(skyHtml).not.toContain("glass-only");

      const glassHtml = renderToStaticMarkup(
        <SettingsPanel theme="glass" onThemeChange={() => {}} />
      );
      expect(glassHtml).toMatch(/theme-card active[^>]*data-theme-set="glass"/);
      expect(glassHtml).toContain("glass-only");
      expect(glassHtml).toContain("磨砂强度");
      expect(glassHtml).toContain("卡片透明度");

      const darkHtml = renderToStaticMarkup(
        <SettingsPanel theme="dark" onThemeChange={() => {}} />
      );
      expect(darkHtml).toMatch(/theme-card active[^>]*data-theme-set="dark"/);
      expect(darkHtml).not.toContain("glass-only");
    });
  });
});
