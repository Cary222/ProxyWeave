import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { PluginDetails } from "./PluginDetails";
import { PluginPanel } from "./PluginPanel";
import { api } from "../api";
import { PluginRecord, InstalledSourceDto } from "../types";
import * as tauriLib from "../lib/tauri";

const hooks = vi.hoisted(() => ({ enabled: false, cursor: 0, cells: [] as unknown[], effects: [] as (() => unknown)[], first: true }));
vi.mock("react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react")>();
  return { ...actual,
    useState: (initial: unknown) => {
      if (!hooks.enabled) return actual.useState(initial);
      const slot = hooks.cursor++;
      if (hooks.first) hooks.cells[slot] = typeof initial === "function" ? initial() : initial;
      return [hooks.cells[slot], (next: unknown) => { hooks.cells[slot] = typeof next === "function" ? next(hooks.cells[slot]) : next; }];
    },
    useRef: (initial: unknown) => {
      if (!hooks.enabled) return actual.useRef(initial);
      const slot = hooks.cursor++;
      if (hooks.first) hooks.cells[slot] = { current: initial };
      return hooks.cells[slot];
    },
    useEffect: (effect: () => unknown, deps?: unknown[]) => {
      if (!hooks.enabled) return actual.useEffect(effect as () => void, deps);
      if (hooks.first) hooks.effects.push(effect);
    },
  };
});
vi.mock("../lib/tauri", () => ({
  invoke: vi.fn(),
  Channel: class MockChannel<T> {
    onmessage: ((message: T) => void) | null = null;
  },
}));

describe("Plugin Update & Source Evidence UI", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const mockSource: InstalledSourceDto = {
    owner: "router-for-me",
    repo: "workbuddy2api",
    release_id: 123456,
    tag_name: "v1.13.0",
    asset_id: 78910,
    asset_name: "wb2api-windows-x64.zip",
    target_os: "windows",
    target_arch: "x86_64",
    archive_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    expected_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  };

  const recordWithSource: PluginRecord = {
    manifest: {
      schema_version: "1.0",
      metadata: {
        id: "wb2api",
        name: "WorkBuddy 2API",
        version: "1.13.0",
        author: "WorkBuddy Team",
      },
      runtime: { ownership: "managed" },
      configuration: { required_secrets: [] },
    },
    settings: {},
    source: mockSource,
  };

  const recordLocal: PluginRecord = {
    manifest: {
      schema_version: "1.0",
      metadata: {
        id: "local-dev",
        name: "Local Dev Plugin",
        version: "0.2.0",
      },
      runtime: { ownership: "external" },
      configuration: { required_secrets: [] },
    },
    settings: {},
    source: null,
  };

  it("renders source evidence facts when plugin has installed source", () => {
    const html = renderToStaticMarkup(
      <PluginDetails record={recordWithSource} />
    );

    expect(html).toContain("安装来源与存证事实");
    expect(html).toContain("checkPluginUpdateBtn");
    expect(html).toContain("检查更新");
    expect(html).toContain("router-for-me/workbuddy2api");
    expect(html).toContain("v1.13.0");
    expect(html).toContain("wb2api-windows-x64.zip");
    expect(html).toContain("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    expect(html).toContain("windows (x86_64)");
    expect(html).not.toContain("本地/未绑定市场");
  });

  it("renders local/unbound market indicator when plugin lacks source", () => {
    const html = renderToStaticMarkup(
      <PluginDetails record={recordLocal} />
    );

    expect(html).toContain("安装来源与存证事实");
    expect(html).toContain("checkPluginUpdateBtn");
    expect(html).toContain("本地/未绑定市场");
    expect(html).not.toContain("router-for-me");
  });

  it("renders source provenance tag on PluginPanel cards", () => {
    const html = renderToStaticMarkup(
      <PluginPanel
        plugins={[recordWithSource, recordLocal]}
        pluginStatuses={{}}
        secretsStatuses={{}}
        gatewayStatus={null}
        onRefresh={() => {}}
      />
    );

    expect(html).toContain("来源: router-for-me/workbuddy2api@v1.13.0");
    expect(html).toContain("本地/未绑定市场");
  });

  it("api.checkPluginUpdate invokes check_plugin_update with pluginId", async () => {
    const mockTauri = vi.mocked(tauriLib.invoke);
    mockTauri.mockResolvedValueOnce({
      status: "up_to_date",
      owned_update_allowed: false,
      latest_tag: "v1.13.0",
      detail: "已是最新版本",
    });

    const res = await api.checkPluginUpdate("wb2api");
    expect(mockTauri).toHaveBeenCalledWith("check_plugin_update", { pluginId: "wb2api" });
    expect(res.status).toBe("up_to_date");
    expect(res.latest_tag).toBe("v1.13.0");
  });

  it("api.checkPluginUpdate returns update_available status", async () => {
    const mockTauri = vi.mocked(tauriLib.invoke);
    mockTauri.mockResolvedValueOnce({
      status: "update_available",
      owned_update_allowed: false,
      latest_tag: "v1.14.0",
      detail: "发现新版本",
    });

    const res = await api.checkPluginUpdate("wb2api");
    expect(res.status).toBe("update_available");
    expect(res.latest_tag).toBe("v1.14.0");
  });

  it("api.updatePluginToRelease uses the atomic update command and optional checksum", async () => {
    vi.mocked(tauriLib.invoke).mockResolvedValueOnce({ status: "blocked_active_requests", active_requests: 2 });
    await api.updatePluginToRelease("wb2api", "a".repeat(64));
    expect(tauriLib.invoke).toHaveBeenCalledWith("update_plugin_to_release", { id: "wb2api", expectedSha256: "a".repeat(64) });
  });

  // This repo has no DOM renderer dependency; exercise real event handlers with a
  // deterministic hook scheduler, then render the returned element for assertions.
  async function mountUpdate() {
    hooks.enabled = true; hooks.first = true; hooks.cursor = 0; hooks.cells = []; hooks.effects = [];
    const onRefresh = vi.fn();
    const draw = () => { hooks.cursor = 0; return PluginDetails({ record: recordWithSource, onRefresh }); };
    const find = (node: unknown, id: string): { onClick: () => unknown; disabled?: boolean } | undefined => {
      if (!node || typeof node !== "object") return;
      if (Array.isArray(node)) { for (const child of node) { const found = find(child, id); if (found) return found; } return; }
      const props = (node as { props?: { id?: string; children?: unknown; onClick: () => unknown; disabled?: boolean } }).props;
      if (props?.id === id) return props;
      return find(props?.children, id);
    };
    let tree = draw(); hooks.effects.forEach(effect => effect()); hooks.first = false;
    vi.mocked(tauriLib.invoke).mockResolvedValueOnce({ status: "update_available", owned_update_allowed: true, latest_tag: "v2.0.0", detail: "new" });
    await find(tree, "checkPluginUpdateBtn")!.onClick();
    tree = draw();
    return { draw, find, tree, onRefresh };
  }

  it("clicks update and disables the button while the update is pending", async () => {
    try {
      const view = await mountUpdate();
      let finish!: (value: unknown) => void;
      vi.mocked(tauriLib.invoke).mockReturnValueOnce(new Promise(resolve => { finish = resolve; }));
      expect(renderToStaticMarkup(view.tree)).toContain("立即更新");
      view.find(view.tree, "updatePluginBtn")!.onClick();
      const pending = view.draw();
      expect(view.find(pending, "updatePluginBtn")!.disabled).toBe(true);
      expect(renderToStaticMarkup(pending)).toContain("更新中");
      finish({ status: "blocked_active_requests", active_requests: 1 });
      await Promise.resolve(); await Promise.resolve();
    } finally { hooks.enabled = false; }
  });

  it("shows active lease warning without refreshing or starting anything", async () => {
    try {
      const view = await mountUpdate();
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce({ status: "blocked_active_requests", active_requests: 3 });
      view.find(view.tree, "updatePluginBtn")!.onClick();
      await Promise.resolve(); await Promise.resolve();
      const html = renderToStaticMarkup(view.draw());
      expect(html).toContain("3 个活跃流"); expect(html).toContain("请稍后重试");
      expect(view.onRefresh).not.toHaveBeenCalled();
    } finally { hooks.enabled = false; }
  });

  it("shows automatic rollback to the stable version and refreshes status", async () => {
    try {
      const view = await mountUpdate();
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce({ status: "rollback_success", error: "health timeout", rolled_back_to_version: "1.13.0" });
      view.find(view.tree, "updatePluginBtn")!.onClick();
      await Promise.resolve(); await Promise.resolve();
      const html = renderToStaticMarkup(view.draw());
      expect(html).toContain("已安全回滚至上一稳定版本 1.13.0");
      expect(view.onRefresh).toHaveBeenCalledOnce();
    } finally { hooks.enabled = false; }
  });

  it("refreshes details and status after a successful hot reload", async () => {
    try {
      const view = await mountUpdate();
      vi.mocked(tauriLib.invoke).mockResolvedValueOnce({ status: "success", old_version: "1.13.0", new_version: "2.0.0", restarted: true });
      view.find(view.tree, "updatePluginBtn")!.onClick();
      await Promise.resolve(); await Promise.resolve();
      const html = renderToStaticMarkup(view.draw());
      expect(html).toContain("更新成功"); expect(html).toContain("2.0.0"); expect(html).toContain("已安全重启");
      expect(view.onRefresh).toHaveBeenCalledOnce();
    } finally { hooks.enabled = false; }
  });
});
