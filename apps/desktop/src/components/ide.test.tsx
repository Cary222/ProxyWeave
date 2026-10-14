import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { IdePanel } from "./IdePanel";
import { api } from "../api";
import {
  AgentBackupItem,
  AgentImportOutcome,
  AgentRestoreOutcome,
  AgentTargetStatus,
  GatewayStatusInfo,
} from "../types";
import * as tauriLib from "../lib/tauri";

// Mock invoke from lib/tauri
vi.mock("../lib/tauri", () => ({
  invoke: vi.fn(),
  Channel: class MockChannel<T> {
    onmessage: ((message: T) => void) | null = null;
  },
}));

const mockRunningGateway: GatewayStatusInfo = {
  is_running: true,
  port: 28319,
  master_key_preview: "sk-preview",
};

const mockStoppedGateway: GatewayStatusInfo = {
  is_running: false,
  port: 28319,
  master_key_preview: "sk-preview",
};

const mockClients: AgentTargetStatus[] = [
  {
    id: "opencode",
    label: "OpenCode / Continue",
    config_path: "C:/Users/User/.config/opencode/opencode.json",
    installed: true,
    configured: false,
    protocol: "OpenAI Chat",
    supported: true,
    version: "0.9.200",
    note: "已检测到安装",
  },
  {
    id: "pi",
    label: "Pi Coding Agent",
    config_path: "C:/Users/User/.pi/agent/models.json",
    installed: true,
    configured: false,
    protocol: "OpenAI Chat",
    supported: true,
    version: "0.8.9",
    note: "已检测到安装",
  },
  {
    id: "claude-code",
    label: "Claude Code",
    config_path: "C:/Users/User/.claude/settings.json",
    installed: true,
    configured: false,
    protocol: "Anthropic Messages",
    supported: false,
    version: "1.0.0",
    note: "Claude Code 使用 Anthropic Messages，当前网关仅支持 OpenAI Chat，禁止假写入",
  },
  {
    id: "cursor",
    label: "Cursor",
    config_path: "Settings → Models → OpenAI API Key",
    installed: false,
    configured: false,
    protocol: "OpenAI Chat (Manual)",
    supported: false,
    version: null,
    note: "提供手动配置指引",
  },
];

describe("IDE / Agent Integration Module", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Tauri IPC API Wrapper", () => {
    it("detectAgentClients calls detect_agent_clients command and returns target statuses", async () => {
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockClients);

      const result = await api.detectAgentClients();
      expect(tauriLib.invoke).toHaveBeenCalledWith("detect_agent_clients");
      expect(result).toEqual(mockClients);
      expect(result).toHaveLength(4);
      expect(result[0].id).toBe("opencode");
      expect(result[0].supported).toBe(true);
      expect(result[2].id).toBe("claude-code");
      expect(result[2].supported).toBe(false);
      expect(result[2].protocol).toBe("Anthropic Messages");
    });

    it("detectAgentClients propagates rejection when IPC fails", async () => {
      vi.mocked(tauriLib.invoke).mockRejectedValueOnce(
        new Error("网关连接失败")
      );

      await expect(api.detectAgentClients()).rejects.toThrow("网关连接失败");
      expect(tauriLib.invoke).toHaveBeenCalledWith("detect_agent_clients");
    });

    it("importAgentClient calls import_agent_client with target and models", async () => {
      const mockOutcome: AgentImportOutcome = {
        target: "opencode",
        backup_dir: "D:/Data/agent-backups/opencode/20260401_120000",
        files: ["C:/Users/User/.config/opencode/opencode.json"],
        models: ["deepseek-v4-flash", "gpt-4o"],
      };
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockOutcome);

      const result = await api.importAgentClient("opencode", ["deepseek-v4-flash", "gpt-4o"]);
      expect(tauriLib.invoke).toHaveBeenCalledWith("import_agent_client", {
        target: "opencode",
        models: ["deepseek-v4-flash", "gpt-4o"],
      });
      expect(result.target).toBe("opencode");
      expect(result.backup_dir).toContain("agent-backups");
      expect(result.files).toHaveLength(1);
      expect(result.models).toEqual(["deepseek-v4-flash", "gpt-4o"]);
    });

    it("importAgentClient propagates error on protocol unsupported or failure", async () => {
      vi.mocked(tauriLib.invoke).mockRejectedValueOnce(
        new Error("目标客户端 claude-code 当前协议为 Anthropic Messages，ProxyWeave 网关仅支持 OpenAI Chat，禁止假写入")
      );

      await expect(api.importAgentClient("claude-code", ["gpt-4o"])).rejects.toThrow(
        "Anthropic Messages"
      );
      expect(tauriLib.invoke).toHaveBeenCalledWith("import_agent_client", {
        target: "claude-code",
        models: ["gpt-4o"],
      });
    });

    it("restoreAgentClient calls restore_agent_client with target and optional backup_id", async () => {
      const mockOutcome: AgentRestoreOutcome = {
        target: "opencode",
        backup_id: "20260401_120000",
        restored_files: 1,
      };
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockOutcome);

      const result = await api.restoreAgentClient("opencode", "20260401_120000");
      expect(tauriLib.invoke).toHaveBeenCalledWith("restore_agent_client", {
        target: "opencode",
        backupId: "20260401_120000",
      });
      expect(result.target).toBe("opencode");
      expect(result.backup_id).toBe("20260401_120000");
      expect(result.restored_files).toBe(1);
    });

    it("restoreAgentClient without backup_id defaults to latest backup", async () => {
      const mockOutcome: AgentRestoreOutcome = {
        target: "pi",
        backup_id: "20260401_120500",
        restored_files: 1,
      };
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockOutcome);

      const result = await api.restoreAgentClient("pi");
      expect(tauriLib.invoke).toHaveBeenCalledWith("restore_agent_client", {
        target: "pi",
        backupId: undefined,
      });
      expect(result.target).toBe("pi");
      expect(result.restored_files).toBe(1);
    });

    it("listAgentBackups calls list_agent_backups and returns backup history items", async () => {
      const mockBackups: AgentBackupItem[] = [
        {
          id: "20260401_120000",
          created_at: 1775000000,
          path: "D:/Data/agent-backups/opencode/20260401_120000",
        },
        {
          id: "20260331_090000",
          created_at: 1774900000,
          path: "D:/Data/agent-backups/opencode/20260331_090000",
        },
      ];
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockBackups);

      const result = await api.listAgentBackups("opencode");
      expect(tauriLib.invoke).toHaveBeenCalledWith("list_agent_backups", {
        target: "opencode",
      });
      expect(result).toHaveLength(2);
      expect(result[0].id).toBe("20260401_120000");
      expect(result[0].created_at).toBe(1775000000);
      expect(result[0].path).toContain("agent-backups");
    });
  });

  describe("IdePanel Static Rendering & Protocol Boundary Verification", () => {
    it("renders with stopped gateway showing truthful stopped prompt", () => {
      const html = renderToStaticMarkup(
        <IdePanel gatewayStatus={mockStoppedGateway} masterKey={null} />
      );

      expect(html).toContain('id="panel-ide"');
      expect(html).toContain("网关已停止");
      expect(html).toContain("网关尚未配置 API Key");
      expect(html).not.toContain("Claude Code 本地配置写入已接入");
      expect(html).not.toContain("网关协议能力未知");
      // No secret-copy feature; only real detection produces target cards.
      expect(html).not.toContain('aria-label="复制 Master Key"');
      expect(html).toMatch(/disabled=""[^>]*>.*?一键更新所有已安装智能体/s);
      expect(html).toContain("检测中");
      expect(html).not.toContain('data-agent-target=');
      expect(html).not.toContain("写入成功");
      expect(html).not.toContain("ANTHROPIC_AUTH_TOKEN");
    });

    it("renders with running gateway but null masterKey keeping writes disabled", () => {
      const html = renderToStaticMarkup(
        <IdePanel gatewayStatus={mockRunningGateway} masterKey={null} />
      );

      expect(html).toContain('id="panel-ide"');
      expect(html).toContain("端口 :28319");
      expect(html).toContain("网关尚未配置 API Key");
      expect(html).not.toContain('aria-label="复制 Master Key"');
      expect(html).toMatch(/disabled=""[^>]*>.*?一键更新所有已安装智能体/s);
      // Missing host capability DTO is unknown, not a Chat-only gateway claim.
      expect(html).not.toContain("网关协议能力未知");
      expect(html).not.toContain("Claude Code 本地配置写入已接入");
    });

    it("never renders key plaintext or fake fallback targets even with a provided key", () => {
      const html = renderToStaticMarkup(
        <IdePanel
          gatewayStatus={mockRunningGateway}
          masterKey="pw-sk-real-master-key-12345678"
        />
      );

      expect(html).toContain('id="panel-ide"');
      expect(html).toContain("端口 :28319");
      expect(html).not.toContain("pw-sk-real-master-key-12345678");
      expect(html).not.toContain('aria-label="复制 Master Key"');
      expect(html).not.toContain('data-agent-target=');
      expect(html).not.toContain("Cursor");
      expect(html).not.toContain("Windsurf");
      for (const block of ["智能体工作台", "本地配置匹配", "本机已安装", "分发模型配置", "全部客户端"]) expect(html).toContain(block);
    });

    it("renders clean workbench layout without technical disclaimer block", () => {
      const html = renderToStaticMarkup(
        <IdePanel
          gatewayStatus={mockRunningGateway}
          masterKey="pw-sk-real-master-key-12345678"
        />
      );

      expect(html).not.toContain("配置可写 ≠ 真实客户端已验收");
      expect(html).not.toContain("Claude Code 本地配置写入已接入，实际调用待验证");
      expect(html).not.toContain("Claude Desktop 客户端模型映射未验证");
      expect(html).not.toContain("Codex / Grok 接入契约待验证，仍禁止写入");
      expect(html).toContain("智能体工作台");
    });
  });
});
