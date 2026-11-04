import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { DiscoverPanel, InstallProgressModal, formatBytes, stageDescription } from "./DiscoverPanel";
import { api } from "../api";
import { DownloadProgressEvent, MarketplacePluginItem } from "../types";
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

// Mock @tauri-apps/api/event
let registeredEventListener: ((e: { payload: DownloadProgressEvent }) => void) | null = null;
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((_event: string, callback: (e: { payload: DownloadProgressEvent }) => void) => {
    registeredEventListener = callback;
    return Promise.resolve(() => {
      registeredEventListener = null;
    });
  }),
}));

const mockReleasePlugin: MarketplacePluginItem = {
  id: "wb2api",
  name: "WorkBuddy 2API",
  description: "WorkBuddy 平台代理转换网关",
  version: "1.13.0",
  author: "WorkBuddy Team",
  repo_url: "https://github.com/router-for-me/workbuddy2api",
  kind: "managed",
  category: "proxy",
  tags: ["workbuddy", "2api"],
  protocol: "openai_chat",
  stars: 128,
  download_url: null,
  release_tag: "v1.13.0",
  icon: null,
  verified: true,
};

describe("GitHub Release Downloader & Secure Unpacker (Phase 3 Task 2)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    registeredEventListener = null;
  });

  describe("API Client Wrapper (api.installPluginFromRelease & api.onDownloadProgress)", () => {
    it("invokes install_plugin_from_release with repo_url, plugin_id and tag", async () => {
      const mockResult = {
        record: {
          manifest: {
            metadata: { id: "wb2api", name: "WorkBuddy 2API", version: "1.13.0" },
          },
        },
        already_registered: false,
      };
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockResult);

      const res = await api.installPluginFromRelease(
        "https://github.com/router-for-me/workbuddy2api",
        "wb2api",
        "v1.13.0",
        "expected-sha-123"
      );

      expect(tauriLib.invoke).toHaveBeenCalledWith("install_plugin_from_release", {
        repoUrl: "https://github.com/router-for-me/workbuddy2api",
        pluginId: "wb2api",
        tag: "v1.13.0",
        expectedSha256: "expected-sha-123",
      });
      expect(res).toEqual(mockResult);
    });

    it("registers download progress listener and receives events", async () => {
      const progressEvents: DownloadProgressEvent[] = [];
      const unlisten = await api.onDownloadProgress((evt) => {
        progressEvents.push(evt);
      });

      expect(registeredEventListener).not.toBeNull();

      // Emit mock progress event
      const sampleEvent: DownloadProgressEvent = {
        plugin_id: "wb2api",
        stage: "downloading",
        downloaded_bytes: 5242880,
        total_bytes: 10485760,
        percent: 50.0,
        message: "已下载 5.0 MB",
      };
      registeredEventListener!({ payload: sampleEvent });

      expect(progressEvents).toHaveLength(1);
      expect(progressEvents[0].stage).toBe("downloading");
      expect(progressEvents[0].percent).toBe(50.0);

      // Unlisten
      unlisten();
      expect(registeredEventListener).toBeNull();
    });
  });

  describe("Helper Functions (formatBytes & stageDescription)", () => {
    it("formats bytes into human readable sizes", () => {
      expect(formatBytes(0)).toBe("0 B");
      expect(formatBytes(1024)).toBe("1.0 KB");
      expect(formatBytes(10485760)).toBe("10.0 MB");
      expect(formatBytes(1073741824)).toBe("1.0 GB");
    });

    it("translates internal download stages to readable descriptions", () => {
      expect(stageDescription("fetching_release")).toContain("Release 资产");
      expect(stageDescription("downloading")).toContain("流式下载");
      expect(stageDescription("verifying_checksum")).toContain("SHA256");
      expect(stageDescription("extracting")).toContain("ZipSlip");
      expect(stageDescription("registering")).toContain("注册发布");
      expect(stageDescription("completed")).toContain("安装完成");
    });
  });

  describe("InstallProgressModal Component Rendering", () => {
    it("renders downloading progress state with progress bar and statistics", () => {
      const progress: DownloadProgressEvent = {
        plugin_id: "wb2api",
        stage: "downloading",
        downloaded_bytes: 12500000,
        total_bytes: 25000000,
        percent: 50,
        message: "正在流式下载资产...",
      };

      const html = renderToStaticMarkup(
        <InstallProgressModal
          isOpen={true}
          plugin={mockReleasePlugin}
          progress={progress}
          error={null}
          isSuccess={false}
          onClose={() => {}}
        />
      );

      expect(html).toContain("正在下载与安装");
      expect(html).toContain("WorkBuddy 2API");
      expect(html).toContain("50%");
      expect(html).toContain("width:50%");
      expect(html).toContain("正在流式下载资产...");
      expect(html).toContain("11.9 MB / 23.8 MB");
    });

    it("renders checksum verification state", () => {
      const progress: DownloadProgressEvent = {
        plugin_id: "wb2api",
        stage: "verifying_checksum",
        downloaded_bytes: 25000000,
        total_bytes: 25000000,
        percent: 90,
        message: "比对 SHA256 哈希值...",
      };

      const html = renderToStaticMarkup(
        <InstallProgressModal
          isOpen={true}
          plugin={mockReleasePlugin}
          progress={progress}
          error={null}
          isSuccess={false}
          onClose={() => {}}
        />
      );

      expect(html).toContain("SHA256 校验和与防篡改验证");
      expect(html).toContain("90%");
    });

    it("renders error state when checksum fails or network error occurs", () => {
      const html = renderToStaticMarkup(
        <InstallProgressModal
          isOpen={true}
          plugin={mockReleasePlugin}
          progress={null}
          error="SHA256 校验失败: 文件可能被篡改或损坏"
          isSuccess={false}
          onClose={() => {}}
          onRetry={() => {}}
        />
      );

      expect(html).toContain("安装失败");
      expect(html).toContain("SHA256 校验失败: 文件可能被篡改或损坏");
      expect(html).toContain("retryInstallBtn");
      expect(html).toContain("重试下载");
    });

    it("renders success state after atomic extraction and registration", () => {
      const html = renderToStaticMarkup(
        <InstallProgressModal
          isOpen={true}
          plugin={mockReleasePlugin}
          progress={null}
          error={null}
          isSuccess={true}
          onClose={() => {}}
          onNavigatePlugins={() => {}}
        />
      );

      expect(html).toContain("安装成功");
      expect(html).toContain("✓ 插件已成功下载、解包并原子注册发布至系统");
      expect(html).toContain("gotoPluginManageBtn");
      expect(html).toContain("前往管理插件");
    });
  });

  describe("DiscoverPanel Integration with Release Downloader", () => {
    it("renders marketplace catalog with one-click download buttons", () => {
      const html = renderToStaticMarkup(
        <DiscoverPanel plugins={[]} initialCatalog={[mockReleasePlugin]} />
      );

      expect(html).toContain("WorkBuddy 2API");
      expect(html).toContain("下载安装");
      expect(html).toContain("installBtn-wb2api");
    });

    it("renders installed state badge when plugin is already registered", () => {
      const installedPlugin = {
        manifest: {
          metadata: {
            id: "wb2api",
            name: "WorkBuddy 2API",
            version: "1.13.0",
          },
        },
      } as any;

      const html = renderToStaticMarkup(
        <DiscoverPanel plugins={[installedPlugin]} initialCatalog={[mockReleasePlugin]} />
      );

      expect(html).toContain("✓ 已安装");
      expect(html).not.toContain("installBtn-wb2api");
    });
  });
});
