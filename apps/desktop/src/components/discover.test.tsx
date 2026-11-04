import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { DiscoverPanel, MarketplaceDetailModal } from "./DiscoverPanel";
import { api, InspectProjectResult } from "../api";
import { MarketplaceCatalogResponse, MarketplacePluginItem, PluginRecord } from "../types";
import * as tauriLib from "../lib/tauri";

vi.mock("../lib/tauri", () => ({
  invoke: vi.fn(),
  Channel: class MockChannel<T> {
    onmessage: ((message: T) => void) | null = null;
  },
}));

const mockMarketCatalog: MarketplacePluginItem[] = [
  {
    id: "wb2api",
    name: "WorkBuddy 2API",
    description: "WorkBuddy 平台代理转换网关，提供标准 OpenAI 接口兼容与请求转换",
    version: "1.13.0",
    author: "WorkBuddy Team",
    repo_url: "https://github.com/router-for-me/workbuddy2api",
    kind: "managed",
    category: "proxy",
    tags: ["workbuddy", "2api", "openai", "managed"],
    protocol: "openai_chat",
    stars: null,
    download_url: null,
    release_tag: "v1.13.0",
    icon: null,
    verified: false,
  },
  {
    id: "external-cpa",
    name: "CLIProxyAPI",
    description: "高性能全协议反向代理与中继插件，支持多协议转换与管理面板挂载",
    version: "7.2.115",
    author: "router-for-me",
    repo_url: "https://github.com/router-for-me/CLIProxyAPI",
    kind: "external",
    category: "proxy",
    tags: ["cliproxyapi", "cpa", "multi-protocol", "external"],
    protocol: "multi",
    stars: null,
    download_url: null,
    release_tag: "v7.2.115",
    icon: null,
    verified: false,
  },
  {
    id: "ollama-bridge",
    name: "Ollama 兼容网桥",
    description: "本地 Ollama 实例连接网桥，无缝将本地大模型映射为标准 OpenAI API",
    version: "0.4.1",
    author: "ProxyWeave Community",
    repo_url: "https://github.com/proxyweave/ollama-bridge",
    kind: "managed",
    category: "local_model",
    tags: ["ollama", "local", "bridge"],
    protocol: "openai_chat",
    stars: null,
    download_url: null,
    release_tag: "v0.4.1",
    icon: null,
    verified: false,
  },
  {
    id: "vllm-external",
    name: "vLLM 本地高并发服务挂载",
    description: "局域网或本机高性能 vLLM 推理实例挂载，支持 PagedAttention 吞吐优化",
    version: "0.6.2",
    author: "vLLM Team",
    repo_url: "https://github.com/vllm-project/vllm",
    kind: "external",
    category: "local_model",
    tags: ["vllm", "gpu", "high-concurrency"],
    protocol: "openai_chat",
    stars: null,
    download_url: null,
    release_tag: "v0.6.2",
    icon: null,
    verified: false,
  },
  {
    id: "deepseek-harness",
    name: "DeepSeek 本地专属服务挂载",
    description: "DeepSeek 本地专属服务挂载，支持 R1/V3 推理加速与上下文管理",
    version: "1.2.0",
    author: "DeepSeek Harness Team",
    repo_url: "https://github.com/proxyweave/deepseek-harness",
    kind: "managed",
    category: "proxy",
    tags: ["deepseek", "reasoning", "managed"],
    protocol: "openai_chat",
    stars: null,
    download_url: null,
    release_tag: "v1.2.0",
    icon: null,
    verified: false,
  },
  {
    id: "open-webui-proxy",
    name: "Open WebUI 桥接插件",
    description: "Open WebUI 桥接插件，实现现代化 Web 界面与 ProxyWeave 网关打通",
    version: "0.3.8",
    author: "open-webui",
    repo_url: "https://github.com/open-webui/open-webui",
    kind: "external",
    category: "tools",
    tags: ["webui", "chat", "tools"],
    protocol: "multi",
    stars: null,
    download_url: null,
    release_tag: "v0.3.8",
    icon: null,
    verified: false,
  },
];

const mockCatalogResponse: MarketplaceCatalogResponse = {
  items: mockMarketCatalog,
  is_offline: true,
  notice: "已加载内置生态推荐清单 (离线模式)",
  total: 6,
};

const installedWb2apiPlugin: PluginRecord = {
  manifest: {
    schema_version: "1.0",
    metadata: {
      id: "wb2api",
      name: "WorkBuddy 2API",
      version: "1.13.0",
      description: "WorkBuddy managed plugin",
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

describe("Marketplace & DiscoverPanel (Phase 3 Task 1)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Marketplace Tauri API Wrappers", () => {
    it("getMarketplaceCatalog invokes get_marketplace_catalog IPC command", async () => {
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockCatalogResponse);

      const res = await api.getMarketplaceCatalog(false);
      expect(tauriLib.invoke).toHaveBeenCalledWith("get_marketplace_catalog", {
        refreshOnline: false,
      });
      expect(res.items).toHaveLength(6);
      expect(res.is_offline).toBe(true);
      expect(res.total).toBe(6);
    });

    it("searchMarketplacePlugins invokes search_marketplace_plugins IPC command with query and category", async () => {
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce({
        items: [mockMarketCatalog[0]],
        is_offline: true,
        notice: null,
        total: 1,
      });

      const res = await api.searchMarketplacePlugins("workbuddy", "proxy", true);
      expect(tauriLib.invoke).toHaveBeenCalledWith("search_marketplace_plugins", {
        query: "workbuddy",
        category: "proxy",
        refreshOnline: true,
      });
      expect(res.items).toHaveLength(1);
      expect(res.items[0].id).toBe("wb2api");
    });

    it("inspectProjectFromGithub invokes inspect_project_from_github IPC command with url", async () => {
      const mockResult: InspectProjectResult = {
        id: "wb2api",
        name: "WorkBuddy 2API",
        version: "1.13.0",
        description: "Test plugin",
        author: "linguo2625469",
        homepage: "https://github.com/linguo2625469/workbuddy2api-panel",
        license: "MIT",
        project_type: "manifest",
        runtime_kind: "managed",
        entrypoint: "bin/wb2api.exe",
        sha256: "38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485",
        protocols: ["openai_chat"],
        exported_models: ["gpt-4o"],
        requires_secrets: ["access_token"],
        permissions: ["独立子进程执行 (managed_process)"],
        notes: [],
        raw_manifest: {},
      };
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce(mockResult);

      const res = await api.inspectProjectFromGithub("https://github.com/router-for-me/workbuddy2api");
      expect(tauriLib.invoke).toHaveBeenCalledWith("inspect_project_from_github", {
        url: "https://github.com/router-for-me/workbuddy2api",
      });
      expect(res.id).toBe("wb2api");
      expect(res.runtime_kind).toBe("managed");
      expect(res.sha256).toBe("38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485");
    });
  });

  describe("DiscoverPanel Component Rendering", () => {
    it("renders marketplace layout with search box, categories, and refresh button", () => {
      const html = renderToStaticMarkup(
        <DiscoverPanel plugins={[]} initialCatalog={mockMarketCatalog} />
      );

      expect(html).toContain('id="panel-discover"');
      expect(html).toContain("插件生态与市场");
      expect(html).toContain('id="marketSearch"');
      expect(html).toContain('id="refreshMarket"');
      expect(html).toContain("刷新市场");
      expect(html).toContain('id="toggleGitClone"');
      expect(html).toContain("GitHub 源码安装");

      // Verify category capsules
      expect(html).toContain("全部");
      expect(html).toContain("Managed");
      expect(html).toContain("External");
      expect(html).toContain("代理转换");
      expect(html).toContain("本地模型");
      expect(html).toContain("工具接入");
    });

    it("loads and renders all marketplace plugin cards when initialCatalog is provided", () => {
      const html = renderToStaticMarkup(
        <DiscoverPanel plugins={[]} initialCatalog={mockMarketCatalog} />
      );

      expect(html).toContain('id="marketplaceCardsGrid"');
      expect(html).toContain("WorkBuddy 2API");
      expect(html).toContain("CLIProxyAPI");
      expect(html).toContain("Ollama 兼容网桥");
      expect(html).toContain("vLLM 本地高并发服务挂载");
      expect(html).toContain("DeepSeek 本地专属服务挂载");
      expect(html).toContain("Open WebUI 桥接插件");

      // Verify buttons
      expect(html).toContain('id="installBtn-wb2api"');
      expect(html).toContain('id="detailBtn-wb2api"');
      expect(html).toContain("下载安装");
      expect(html).toContain("详情");
      // DEF-032: built-in cards with null stars do not render star ratings
      expect(html).not.toContain("★");
    });

    it("marks installed plugin with ✓ 已安装 pill and 管理插件 button", () => {
      const html = renderToStaticMarkup(
        <DiscoverPanel
          plugins={[installedWb2apiPlugin]}
          initialCatalog={mockMarketCatalog}
        />
      );

      // wb2api is installed
      expect(html).toContain("✓ 已安装");
      expect(html).toContain("管理插件");
      // Other uninstalled plugins still show 下载安装
      expect(html).toContain('id="installBtn-external-cpa"');
    });

    it("renders empty state notice when initialCatalog is empty", () => {
      const html = renderToStaticMarkup(
        <DiscoverPanel plugins={[]} initialCatalog={[]} />
      );

      expect(html).toContain('id="marketplaceEmpty"');
      expect(html).toContain("未找到匹配的插件");
    });

    it("preserves legacy GitHub clone and Topic discovery section", () => {
      const html = renderToStaticMarkup(<DiscoverPanel plugins={[]} />);

      expect(html).toContain('id="repoUrl"');
      expect(html).toContain('id="parseRepo"');
      expect(html).toContain('id="confirmInstall"');
      expect(html).toContain("克隆并安装");
      expect(html).toContain("Topic 发现");
    });
  });

  describe("MarketplaceDetailModal Component", () => {
    it("returns null when isOpen is false", () => {
      const html = renderToStaticMarkup(
        <MarketplaceDetailModal
          isOpen={false}
          plugin={mockMarketCatalog[0]}
          onClose={() => {}}
        />
      );

      expect(html).toBe("");
    });

    it("renders full plugin details, author, repo URL, tags, and install action when opened", () => {
      const item = mockMarketCatalog[0];
      const html = renderToStaticMarkup(
        <MarketplaceDetailModal
          isOpen={true}
          plugin={item}
          onClose={() => {}}
          isInstalled={false}
        />
      );

      expect(html).toContain('id="marketplaceDetailModal"');
      expect(html).toContain(item.name);
      expect(html).toContain(`v${item.version}`);
      expect(html).toContain(item.author);
      expect(html).toContain(item.description);
      expect(html).toContain(item.repo_url);
      expect(html).toContain("Managed 进程托管");
      expect(html).toContain("openai_chat");
      expect(html).toContain("从仓库克隆安装");
      expect(html).toContain('id="modalConfirmInstall"');
      expect(html).toContain("安全审查与执行边界说明");
      // DEF-032: no fake verification badge
      expect(html).not.toContain("官方/社区认证");
    });

    it("renders 管理已安装插件 button when isInstalled is true", () => {
      const item = mockMarketCatalog[0];
      const html = renderToStaticMarkup(
        <MarketplaceDetailModal
          isOpen={true}
          plugin={item}
          onClose={() => {}}
          isInstalled={true}
        />
      );

      expect(html).toContain("管理已安装插件");
      expect(html).not.toContain('id="modalConfirmInstall"');
    });
  });
});
