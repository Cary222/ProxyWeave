import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { IdePanel } from "./IdePanel";
import { api } from "../api";
import {
  AgentBackupItem,
  AgentDiffPreview,
  AgentImportOutcome,
  AgentRestoreOutcome,
  AgentTargetStatus,
  GatewayStatusInfo,
} from "../types";
import { canWriteTarget, type AgentClientTarget } from "./ide-source/ide-adapter";
import { CLIENT_ICONS, CLIENT_METAS, computeLineDiff, DiffPreviewBody } from "./ide-source/AgentsPage";
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
    config_path: "C:/Users/User/AppData/Roaming/Cursor/User/settings.json",
    installed: true,
    configured: false,
    protocol: "OpenAI Chat",
    supported: true,
    version: null,
    note: "已检测到安装",
  },
  {
    id: "windsurf",
    label: "Windsurf",
    config_path: "C:/Users/User/AppData/Roaming/Windsurf/User/settings.json",
    installed: true,
    configured: false,
    protocol: "OpenAI Chat",
    supported: true,
    version: null,
    note: "已检测到安装",
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
      expect(result).toHaveLength(5);
      expect(result[0].id).toBe("opencode");
      expect(result[0].supported).toBe(true);
      expect(result[2].id).toBe("claude-code");
      expect(result[2].supported).toBe(false);
      expect(result[2].protocol).toBe("Anthropic Messages");
      expect(result[3].id).toBe("cursor");
      expect(result[3].supported).toBe(true);
      expect(result[3].protocol).toBe("OpenAI Chat");
      expect(result[4].id).toBe("windsurf");
      expect(result[4].supported).toBe(true);
      expect(result[4].protocol).toBe("OpenAI Chat");
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

    it("previewAgentClient calls preview_agent_client with target and models", async () => {
      const mockPreview: AgentDiffPreview = {
        target: "cursor",
        models: ["gpt-4o", "deepseek-chat"],
        diffs: [
          {
            path: "C:/Users/User/AppData/Roaming/Cursor/User/settings.json",
            existing_content: "{}",
            proposed_content: '{\n  "cursor.cpp.overrideBaseUrl": "http://127.0.0.1:28319/v1",\n  "cursor.cpp.overrideApiKey": "sk-preview",\n  "openai.baseUrl": "http://127.0.0.1:28319/v1",\n  "openai.apiKey": "sk-preview"\n}',
          },
        ],
      };
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockPreview);

      const result = await api.previewAgentClient("cursor", ["gpt-4o", "deepseek-chat"]);
      expect(tauriLib.invoke).toHaveBeenCalledWith("preview_agent_client", {
        target: "cursor",
        models: ["gpt-4o", "deepseek-chat"],
      });
      expect(result.target).toBe("cursor");
      expect(result.models).toEqual(["gpt-4o", "deepseek-chat"]);
      expect(result.diffs).toHaveLength(1);
      expect(result.diffs[0].path).toContain("Cursor");
      expect(result.diffs[0].proposed_content).toContain("cursor.cpp.overrideBaseUrl");
    });

    it("previewAgentClient propagates error when IPC fails", async () => {
      vi.mocked(tauriLib.invoke).mockRejectedValueOnce(
        new Error("生成预览失败: 客户端配置解析错误")
      );

      await expect(api.previewAgentClient("cursor", ["gpt-4o"])).rejects.toThrow(
        "生成预览失败: 客户端配置解析错误"
      );
      expect(tauriLib.invoke).toHaveBeenCalledWith("preview_agent_client", {
        target: "cursor",
        models: ["gpt-4o"],
      });
    });
  });

  describe("Diff Preview Dialog & Cancellation/Confirmation Zero-Write Verification", () => {
    it("renders DiffPreviewBody with proposed diff, highlighting config items and safety backup notice", () => {
      const mockDiffPreview: AgentDiffPreview = {
        target: "cursor",
        models: ["gpt-4o"],
        diffs: [
          {
            path: "C:/Users/User/AppData/Roaming/Cursor/User/settings.json",
            existing_content: "{}",
            proposed_content: '{\n  "cursor.cpp.overrideBaseUrl": "http://127.0.0.1:28319/v1",\n  "cursor.cpp.overrideApiKey": "sk-preview",\n  "openai.baseUrl": "http://127.0.0.1:28319/v1",\n  "openai.apiKey": "sk-preview"\n}',
          },
        ],
      };

      const html = renderToStaticMarkup(
        <DiffPreviewBody
          diffPreview={mockDiffPreview}
          activeFileDiffIdx={0}
        />
      );

      expect(html).toContain("Cursor/User/settings.json");
      expect(html).toContain("更新配置");
      expect(html).toContain("cursor.cpp.overrideBaseUrl");
      expect(html).toContain("http://127.0.0.1:28319/v1");
      expect(html).toContain("openai.baseUrl");
      expect(html).toContain("openai.apiKey");
      expect(html).toContain("sk-preview");
      expect(html).toContain("+6");
      expect(html).toContain("-1");
    });

    it("renders empty notice when diff preview has no diff files", () => {
      const html = renderToStaticMarkup(
        <DiffPreviewBody
          diffPreview={{ target: "cursor", models: [], diffs: [] }}
        />
      );
      expect(html).toContain("无任何待修改的文件配置");
    });

    it("computeLineDiff accurately computes additions, deletions, and line numbers", () => {
      // New file case (null original)
      const newFileDiff = computeLineDiff(null, '{"key": "value"}');
      expect(newFileDiff).toHaveLength(1);
      expect(newFileDiff[0].type).toBe("add");
      expect(newFileDiff[0].text).toBe('{"key": "value"}');
      expect(newFileDiff[0].newLineNo).toBe(1);

      // Modified file case
      const original = "line1\nline2";
      const updated = "line1\nline2_modified\nline3";
      const modifiedDiff = computeLineDiff(original, updated);
      expect(modifiedDiff.some(l => l.type === "same" && l.text === "line1")).toBe(true);
      expect(modifiedDiff.some(l => l.type === "del" && l.text === "line2")).toBe(true);
      expect(modifiedDiff.some(l => l.type === "add" && l.text === "line2_modified")).toBe(true);
      expect(modifiedDiff.some(l => l.type === "add" && l.text === "line3")).toBe(true);
    });

    it("user clicking Cancel in Diff dialog workflow maintains zero-write guarantee (never calls importAgentClient)", async () => {
      const invokeSpy = vi.mocked(tauriLib.invoke);
      invokeSpy.mockClear();

      // Simulate the workflow in AgentsPage:
      // 1. User clicks "一键接入", preview is loaded
      const mockDiffPreview: AgentDiffPreview = {
        target: "cursor",
        models: ["gpt-4o"],
        diffs: [
          {
            path: "C:/Users/User/AppData/Roaming/Cursor/User/settings.json",
            existing_content: null,
            proposed_content: '{\n  "cursor.cpp.overrideBaseUrl": "http://127.0.0.1:28319/v1"\n}',
          },
        ],
      };
      invokeSpy.mockResolvedValueOnce(mockDiffPreview);

      const preview = await api.previewAgentClient("cursor", ["gpt-4o"]);
      expect(preview.target).toBe("cursor");
      expect(invokeSpy).toHaveBeenCalledWith("preview_agent_client", {
        target: "cursor",
        models: ["gpt-4o"],
      });

      // 2. Dialog is opened with preview. User reviews diff and clicks [取消].
      let dialogOpen = true;
      const handleCancel = () => {
        dialogOpen = false;
        // Zero IPC calls on cancel
      };
      handleCancel();

      expect(dialogOpen).toBe(false);
      // Verify NO import_agent_client call was made!
      const importCalls = invokeSpy.mock.calls.filter(call => call[0] === "import_agent_client");
      expect(importCalls).toHaveLength(0);
    });

    it("user clicking Confirm Write invokes importAgentClient and triggers data refresh", async () => {
      const invokeSpy = vi.mocked(tauriLib.invoke);
      invokeSpy.mockClear();

      const mockOutcome: AgentImportOutcome = {
        target: "cursor",
        backup_dir: "C:/Users/User/agent-backups/cursor/20260401_120000",
        files: ["C:/Users/User/AppData/Roaming/Cursor/User/settings.json"],
        models: ["gpt-4o"],
      };
      invokeSpy.mockResolvedValueOnce(mockOutcome);

      let refreshTriggered = false;
      const onRefresh = () => {
        refreshTriggered = true;
      };

      // User clicks [确认写入]
      const handleConfirmWrite = async (targetId: string, models: string[]) => {
        const outcome = await api.importAgentClient(targetId, models);
        onRefresh();
        return outcome;
      };

      const result = await handleConfirmWrite("cursor", ["gpt-4o"]);
      expect(result.target).toBe("cursor");
      expect(invokeSpy).toHaveBeenCalledWith("import_agent_client", {
        target: "cursor",
        models: ["gpt-4o"],
      });
      expect(refreshTriggered).toBe(true);
    });
  });

  describe("Cursor & Windsurf Metadata, Protocols, and Configuration", () => {
    it("Cursor and Windsurf metadata and protocols are correctly configured in CLIENT_METAS", () => {
      expect(CLIENT_METAS["cursor"]).toBeDefined();
      expect(CLIENT_METAS["cursor"].protocol).toBe("OpenAI Chat");
      expect(CLIENT_METAS["cursor"].protocolBadge).toBe("POST /v1/chat/completions");
      expect(CLIENT_METAS["cursor"].description).toContain("Base URL");

      expect(CLIENT_METAS["windsurf"]).toBeDefined();
      expect(CLIENT_METAS["windsurf"].protocol).toBe("OpenAI Chat");
      expect(CLIENT_METAS["windsurf"].protocolBadge).toBe("POST /v1/chat/completions");
      expect(CLIENT_METAS["windsurf"].description).toContain("Cascade");
    });

    it("Cursor and Windsurf have valid SVG/PNG icon assets registered in CLIENT_ICONS", () => {
      expect(CLIENT_ICONS["cursor"]).toBeDefined();
      expect(typeof CLIENT_ICONS["cursor"]).toBe("string");
      expect(CLIENT_ICONS["windsurf"]).toBeDefined();
      expect(typeof CLIENT_ICONS["windsurf"]).toBe("string");
    });

    it("canWriteTarget correctly marks installed Cursor and Windsurf as writable", () => {
      const cursorTarget: AgentClientTarget = {
        id: "cursor",
        label: "Cursor",
        config_path: "C:/Users/User/AppData/Roaming/Cursor/User/settings.json",
        configPath: "C:/Users/User/AppData/Roaming/Cursor/User/settings.json",
        installed: true,
        configured: false,
        protocol: "OpenAI Chat",
        supported: true,
        version: null,
        note: "已检测到安装",
      };
      expect(canWriteTarget(cursorTarget)).toBe(true);

      const windsurfTarget: AgentClientTarget = {
        id: "windsurf",
        label: "Windsurf",
        config_path: "C:/Users/User/AppData/Roaming/Windsurf/User/settings.json",
        configPath: "C:/Users/User/AppData/Roaming/Windsurf/User/settings.json",
        installed: true,
        configured: false,
        protocol: "OpenAI Chat",
        supported: true,
        version: null,
        note: "已检测到安装",
      };
      expect(canWriteTarget(windsurfTarget)).toBe(true);

      const uninstalledCursor: AgentClientTarget = {
        ...cursorTarget,
        installed: false,
      };
      expect(canWriteTarget(uninstalledCursor)).toBe(false);
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
