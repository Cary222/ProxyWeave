import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import App from "./App";
import MenuBarPanel from "./MenuBarPanel";
import { PluginPanel } from "./components/PluginPanel";
import { PluginRecord } from "./types";

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
});
