import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { PluginPanel } from "./PluginPanel";
import type { Manifest, PluginRecord } from "../types";

vi.mock("../lib/tauri", () => ({ invoke: vi.fn() }));

function renderLifecycle(lifecycle?: Manifest["lifecycle"], ownership: "managed" | "external" = "managed") {
  const record: PluginRecord = {
    manifest: {
      schema_version: "1.0", metadata: { id: "owned-lifecycle", name: "Owned Lifecycle", version: "1.0.0" },
      runtime: { ownership }, configuration: { required_secrets: [] }, lifecycle,
    }, settings: {},
  };
  return renderToStaticMarkup(<PluginPanel plugins={[record]} pluginStatuses={{}}
    secretsStatuses={{}} gatewayStatus={null} onRefresh={() => {}} />);
}

describe("DEF-026 declared idle policy does not imply an active idle reaper", () => {
  it("null explicitly disables idle reclaim", () => {
    const html = renderLifecycle({ idle_timeout_ms: null });
    expect(html).toContain("不启用空闲回收");
    expect(html).not.toContain("300s");
  });
  it("positive timeout is declared but not effective", () => {
    expect(renderLifecycle({ idle_timeout_ms: 300000 })).toContain("配置空闲超时 300s（尚未生效）");
    expect(renderLifecycle({ idle_timeout_ms: 1500 })).toContain("配置空闲超时 1.5s（尚未生效）");
  });
  it("missing lifecycle or missing field is unknown, not null or an invented default", () => {
    for (const lifecycle of [undefined, null, {}]) {
      const html = renderLifecycle(lifecycle);
      expect(html).toContain("空闲策略未声明");
      expect(html).not.toContain("不启用空闲回收");
      expect(html).not.toContain("300s");
    }
  });
  it("External keeps zero-kill semantics regardless of idle configuration", () => {
    const html = renderLifecycle({ idle_timeout_ms: 300000 }, "external");
    expect(html).toContain("永不 Kill");
    expect(html).not.toContain("配置空闲超时");
  });
});
