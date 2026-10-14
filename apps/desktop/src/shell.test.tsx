import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import App from "./App";
import MenuBarPanel from "./MenuBarPanel";
import { PluginPanel } from "./components/PluginPanel";
import { PluginNavigation, reconcilePluginSelection } from "./components/PluginNavigation";
import { PluginDetails } from "./components/PluginDetails";
import { PluginRecord } from "./types";

import { defaultPalette, paletteStyles } from "./components/Appearance";
import { Overview } from "./components/Overview";
describe("ProxyWeave desktop shell", () => {
  it("renders the unified gateway dashboard without legacy core download", () => {
    const html = renderToStaticMarkup(<App />);
    expect(html).toContain("ProxyWeave");
    expect(html).toContain("AI 编织网关宿主");
    expect(html).toContain('aria-label="关闭"');
    expect(html).toContain("统一网关服务");
    expect(html).not.toContain("Download proxy core");
  });
  it("keeps tray-panel show and quit controls", () => {
    const html = renderToStaticMarkup(<MenuBarPanel />);
    expect(html).toContain("Open ProxyWeave");
    expect(html).toContain("Quit ProxyWeave");
    expect(html).toContain("no providers running");
  });

  it("renders managed plugin correctly based on manifest.runtime.ownership (DEF-007)", () => {
    const managedPlugin: PluginRecord = {
      manifest: {
        schema_version: "1.0",
        metadata: {
          id: "wb2api",
          name: "wb2api 托管插件",
          version: "0.1.0",
          description: "测试托管插件描述",
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

    const html = renderToStaticMarkup(
      <PluginPanel
        plugins={[managedPlugin]}
        pluginStatuses={{}}
        secretsStatuses={{}}
        gatewayStatus={null}
        onRefresh={() => {}}
      />
    );

    expect(html).toContain("Managed (托管进程)");
    expect(html).toContain("ownership-tag--managed");
    expect(html).toContain("启动插件");
    expect(html).not.toContain("External (外部服务·零杀伤)");
    expect(html).toContain("本地注册表受管");
    expect(html).not.toContain("undefined");
  });

  it("renders external plugin with endpoint fallback (DEF-007)", () => {
    const externalPlugin: PluginRecord = {
      manifest: {
        schema_version: "1.0",
        metadata: {
          id: "ext-service",
          name: "外部提供商",
          version: "1.0.0",
        },
        runtime: {
          ownership: "external",
          endpoint: "http://127.0.0.1:9090/v1",
        },
        configuration: {
          required_secrets: [],
        },
      },
      settings: {},
    };

    const html = renderToStaticMarkup(
      <PluginPanel
        plugins={[externalPlugin]}
        pluginStatuses={{}}
        secretsStatuses={{}}
        gatewayStatus={null}
        onRefresh={() => {}}
      />
    );

    expect(html).toContain("External (外部服务·零杀伤)");
    expect(html).toContain("ownership-tag--external");
    expect(html).not.toContain("启动插件");
    expect(html).toContain("http://127.0.0.1:9090/v1");
  });

  it("supports backward compatibility with top-level ownership (DEF-007)", () => {
    const legacyPlugin: PluginRecord = {
      manifest: {
        schema_version: "1.0",
        metadata: {
          id: "legacy-managed",
          name: "旧版托管插件",
          version: "0.0.1",
        },
        runtime: {
          ownership: "external", // runtime is external, but top-level ownership is managed
        },
        ownership: "managed",
        configuration: {
          required_secrets: [],
        },
      },
      settings: {},
    };

    const html = renderToStaticMarkup(
      <PluginPanel
        plugins={[legacyPlugin]}
        pluginStatuses={{}}
        secretsStatuses={{}}
        gatewayStatus={null}
        onRefresh={() => {}}
      />
    );

    expect(html).toContain("Managed (托管进程)");
    expect(html).toContain("启动插件");
  });

  it("renders arbitrary-ID plugin navigation with native accessible buttons", () => {
    const plugins: PluginRecord[] = ["arbitrary-one", "arbitrary-two"].map((id) => ({
      manifest: { schema_version: "1.0", metadata: { id, name: id, version: "1.0.0" }, runtime: { ownership: "external" }, configuration: { required_secrets: [] } }, settings: {},
    }));
    const html = renderToStaticMarkup(<PluginNavigation plugins={plugins} statuses={{}} selectedId="arbitrary-two" active onSelect={() => {}} onOpen={() => {}} />);
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain('aria-controls="plugin-navigation-items"');
    expect(html).toContain('aria-current="page"');
    expect(html).toContain("总览 / 导入");
    expect(html).toContain("arbitrary-one");
    expect(html).toContain("arbitrary-two");
    expect(html).not.toContain('role="tab"'); // disclosure + navigation, not incomplete ARIA tab semantics
    expect(reconcilePluginSelection("arbitrary-two", [...plugins, plugins[0]])).toBe("arbitrary-two");
    expect(reconcilePluginSelection("arbitrary-two", plugins.slice(0, 1))).toBeNull();
    expect(reconcilePluginSelection(null, plugins)).toBeNull();
  });

  it("details distinguish declarations from measurements and preserve generic controls", () => {
    const record: PluginRecord = { manifest: { schema_version: "1.0", metadata: { id: "some-other-id", name: "Generic", version: "1.0.0" }, runtime: { ownership: "managed" }, configuration: { required_secrets: ["access"] }, capabilities: { protocol: "openai_chat", models_path: "/declared/models", inference_path: "/declared/chat", streaming: false } }, settings: { label: "public" } };
    const html = renderToStaticMarkup(<PluginPanel detail plugins={[record]} pluginStatuses={{}} secretsStatuses={{}} gatewayStatus={null} onRefresh={() => {}} />);
    expect(html).toContain("Manifest 声明（不是实测）");
    expect(html).toContain("/declared/models");
    expect(html).toContain("/declared/chat");
    expect(html).toContain("false");
    expect(html).toContain("能力尚未实测");
    expect(html).toContain("此插件未声明管理入口");
    expect(html).toContain("启动插件");
    expect(html).toContain("挂载至统一网关");
    expect(html).toContain("安全凭据与授权");
    expect(html).not.toContain("Import Manifest");
    expect(html).not.toContain("iframe");
    expect(html).not.toContain("webview");
  });

  it("a declared management entry is metadata, never an automatic link or embedded page", () => {
    const record: PluginRecord = { manifest: { schema_version: "1.0", metadata: { id: "fixture-admin", name: "Fixture", version: "1.0.0" }, runtime: { ownership: "external", endpoint: "http://127.0.0.1:9876" }, configuration: { required_secrets: [] }, management: { path: "/ui", presentation: "browser" } }, settings: {} };
    const html = renderToStaticMarkup(<PluginDetails record={record} status={{ status: "stopped", active_requests: 0 }} />);
    expect(html).toContain("宿主暂未启用经验证的安全嵌入");
    expect(html).toContain("声明入口");
    expect(html).toContain("/ui");
    expect(html).toContain("未声明");
    expect(html).not.toContain("href=");
    expect(html).not.toContain("iframe");
    expect(html).not.toContain("9876"); // no frontend URL construction
  });
  it("uses independent session theme palettes and bounded glass controls", () => {
    const sky = defaultPalette(), dark = defaultPalette();
    sky.accent = "#ff0000";
    expect(dark.accent).toBeNull();
    expect(paletteStyles("sky", sky)["--blue"]).toBe("#ff0000");
    expect(paletteStyles("dark", dark)).toEqual({});
    expect(paletteStyles("glass", {...defaultPalette(), blur: 0, opacity: 20})["--glass-blur"]).toBe("0px");
    expect(paletteStyles("glass", {...defaultPalette(), blur: 32, opacity: 90})["--card"]).toBe("rgba(255,255,255,0.9)");
    expect(paletteStyles("sky", defaultPalette())).toEqual({});
  });
  it("does not invent stopped status or measured overview counts when IPC is unavailable", () => {
    const html = renderToStaticMarkup(<Overview gatewayStatus={null} plugins={[]} statuses={{}} dataReady={false} onNavigate={()=>{}}/>);
    expect(html).toContain("未知");
    expect(html).not.toContain("38ms");
    expect(html).not.toContain("已停止");
    const shell = renderToStaticMarkup(<App/>);
    expect(shell).toContain("网关状态未知");
    expect(shell).toContain("正在读取宿主状态");
    expect(shell).toContain("IDE 接入");
    expect(shell).toContain("系统设置");
  });
  it("keeps unavailable plugin and credential status unknown and labels sensitive inputs", () => {
    const record: PluginRecord = { manifest: { schema_version: "1.0", metadata: { id: "unknown-state", name: "Unknown", version: "1.0.0" }, runtime: { ownership: "managed" }, configuration: { required_secrets: ["dummy"] } }, settings: {} };
    const html = renderToStaticMarkup(<PluginPanel detail plugins={[record]} pluginStatuses={{}} secretsStatuses={{}} gatewayStatus={null} onRefresh={() => {}} />);
    expect(html).toContain("UNKNOWN");
    expect(html).toContain('aria-label="凭据 dummy"');
    expect(html).not.toContain("未配置");
    expect(html).not.toContain("STOPPED");
    expect(html).toMatch(/<button[^>]*disabled=""[^>]*>启动插件<\/button>/);
  });

  it("withdraws unverified iframe claims and never invents a management URL", () => {
    const runningManaged: PluginRecord = {
      manifest: {
        schema_version: "1.0",
        metadata: { id: "managed-live", name: "Managed Live Plugin", version: "1.0.0" },
        runtime: { ownership: "managed" },
        configuration: { required_secrets: [] },
        management: { path: "/dashboard", presentation: "browser" },
      },
      settings: {},
    };

    const htmlManaged = renderToStaticMarkup(
      <PluginDetails
        record={runningManaged}
        status={{ status: "ready", port: 18080, active_requests: 0 }}
      />
    );
    expect(htmlManaged).toContain("宿主暂未启用经验证的安全嵌入");
    expect(htmlManaged).toContain("原生隔离嵌入仍待核验");
    expect(htmlManaged).toContain("/dashboard");
    expect(htmlManaged).toContain("正在校验当前管理入口");
    expect(htmlManaged).not.toContain("18080");
    expect(htmlManaged).not.toContain("iframe");
    expect(htmlManaged).not.toContain("已在安全沙箱中内嵌");
    expect(htmlManaged).not.toContain("href=");

    const runningExternal: PluginRecord = {
      manifest: {
        schema_version: "1.0",
        metadata: { id: "external-live", name: "External Live Plugin", version: "1.0.0" },
        runtime: { ownership: "external", endpoint: "http://127.0.0.1:29090" },
        configuration: { required_secrets: [] },
        management: { path: "/console", presentation: "browser" },
      },
      settings: {},
    };

    const htmlExternal = renderToStaticMarkup(
      <PluginDetails
        record={runningExternal}
        status={{ status: "busy", reason: "processing", active_requests: 1 }}
      />
    );
    expect(htmlExternal).toContain("/console");
    expect(htmlExternal).toContain("不会修改外部服务的配置文件");
    expect(htmlExternal).not.toContain("29090");
    expect(htmlExternal).not.toContain("iframe");

    // Missing or stopped runtime never becomes a fabricated management endpoint.
    const htmlStopped = renderToStaticMarkup(
      <PluginDetails
        record={runningManaged}
        status={{ status: "stopped", active_requests: 0 }}
      />
    );
    expect(htmlStopped).not.toContain("iframe");
    expect(htmlStopped).toContain("插件未运行：先在本页完成配置");
    const unknown = renderToStaticMarkup(<PluginDetails record={runningManaged} />);
    expect(unknown).toContain("运行状态未知");
    expect(unknown).not.toContain("插件未运行：");
    const legacyRoot = {...runningManaged, manifest: {...runningManaged.manifest, metadata: {...runningManaged.manifest.metadata, id: "wb2api"}, management: {path: "/", presentation: "browser" as const}}};
    const rootHtml = renderToStaticMarkup(<PluginDetails record={legacyRoot} status={{status: "ready", port: 18080, active_requests: 0}}/>);
    expect(rootHtml).not.toContain("/panel/");
    expect(rootHtml).not.toContain("18080");
  });

  it("exposes plugin-local api_key even without required_secrets, never OAuth readback", () => {
    const record: PluginRecord = { manifest: { schema_version: "1.0", metadata: { id: "arbitrary-auth", name: "Auth", version: "1.0.0" }, runtime: { ownership: "managed" }, configuration: { required_secrets: ["access_token", "api_key"] } }, settings: {} };
    const render = (status: "stopped" | "busy" | "failed") => renderToStaticMarkup(<PluginPanel detail plugins={[record]} pluginStatuses={{"arbitrary-auth": {status, active_requests: 0}}} secretsStatuses={{"arbitrary-auth": [{name: "api_key", is_set: true}, {name: "access_token", is_set: true}]}} gatewayStatus={null} onRefresh={()=>{}}/>);
    const stopped = render("stopped");
    expect(stopped).toContain('aria-label="凭据 api_key"');
    expect(stopped).toContain("非统一网关 Master Key");
    expect(stopped.match(/复制插件访问密钥/g)).toHaveLength(1);
    expect(stopped.match(/aria-label="凭据 api_key"/g)).toHaveLength(1);
    expect(stopped).not.toContain("复制 access_token");
    expect(stopped).toMatch(/<input[^>]*aria-label="凭据 api_key"[^>]*value=""/);
    for (const html of [render("busy"), render("failed")]) {
      expect(html).toMatch(/<input[^>]*aria-label="凭据 api_key"[^>]*disabled=""/);
    }
    const noRequired = {...record, manifest: {...record.manifest, configuration: {required_secrets: []}}};
    expect(renderToStaticMarkup(<PluginPanel detail plugins={[noRequired]} pluginStatuses={{}} secretsStatuses={{}} gatewayStatus={null} onRefresh={()=>{}}/>)).toContain('aria-label="凭据 api_key"');
  });

  it("provides plugin config key instructions and distinguishes unknown, absent and saved values", () => {
    const record: PluginRecord = {manifest: {schema_version: "1.0", metadata: {id: "owned-guide", name: "Guide", version: "1.0.0"}, runtime: {ownership: "managed"}, configuration: {required_secrets: ["access_token"]}}, settings: {}};
    const panel = renderToStaticMarkup(<PluginPanel detail plugins={[record]} pluginStatuses={{"owned-guide": {status: "stopped", active_requests: 0}}} secretsStatuses={{"owned-guide": [{name: "api_key", is_set: false}]}} gatewayStatus={null} onRefresh={()=>{}}/>);
    expect(panel).toContain("插件访问设置与使用说明");
    expect(panel).toContain("无需用户手工找改文件");
    expect(panel).toContain("原项目目录不被覆盖");
    expect(panel).toContain("前往插件访问设置");
    expect(panel).toContain("访问密钥未配置");
    const configured = renderToStaticMarkup(<PluginDetails record={record} accessKeyConfigured onConfigure={()=>{}} />);
    expect(configured).toContain("访问密钥已保存");
    expect(configured).toContain("宿主不自动代登录");
    const unknown = renderToStaticMarkup(<PluginDetails record={record} />);
    expect(unknown).toContain("访问密钥状态未知");
    expect(unknown).not.toContain("访问密钥未配置");
    expect(configured).not.toContain('value="pw_');
  });

  it("renders multi-mode project import wizard tabs and presets in PluginPanel", () => {
    const html = renderToStaticMarkup(
      <PluginPanel
        plugins={[]}
        pluginStatuses={{}}
        secretsStatuses={{}}
        gatewayStatus={null}
        onRefresh={() => {}}
      />
    );

    expect(html).toContain("导入与注册插件项目 (Import Plugin)");
    expect(html).toContain("本地项目目录");
    expect(html).toContain("GitHub 仓库 URL");
    expect(html).toContain("标准 Manifest 包");
    expect(html).toContain("快速预填样例：");
    expect(html).toContain("workbuddy2api-panel");
    expect(html).toContain("CLIProxyAPI");
    expect(html).toContain("探测并导入");
    expect(html).toContain('aria-label="本地项目目录路径"');
  });
});
