import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import App from "./App";
import MenuBarPanel from "./MenuBarPanel";

describe("Phase 0 shell", () => {
  it("renders the baseline without provider state or browser mocks", () => {
    const html = renderToStaticMarkup(<App />);
    expect(html).toContain("ProxyWeave");
    expect(html).toContain("Phase 0");
    expect(html).toContain('aria-label="关闭"');
    expect(html).toContain("not enabled");
    expect(html).not.toContain("Download proxy core");
  });
  it("keeps tray-panel show and quit controls", () => {
    const html = renderToStaticMarkup(<MenuBarPanel />);
    expect(html).toContain("Open ProxyWeave");
    expect(html).toContain("Quit ProxyWeave");
    expect(html).toContain("no providers running");
  });
});
