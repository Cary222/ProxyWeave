import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import App from "./App";
import MenuBarPanel from "./MenuBarPanel";

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
});
