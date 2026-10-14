import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";
import http from "node:http";
import crypto from "node:crypto";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { chromium } = require("D:/pi-web-0.8.9/pi-web-0.8.9/node_modules/playwright");
const esbuild = require("esbuild");
const desktop = process.cwd();
const evidence = path.resolve(desktop, "../../.pi/development/ui-truth-wave2/browser");
let server, browser, page, base;

beforeAll(async () => {
  fs.mkdirSync(evidence + "/snapshot/components", { recursive: true });
  const hashes = {};
  for (const name of ["DiscoverPanel.tsx", "AuditPanel.tsx", "SettingsPanel.tsx"]) {
    const bytes = fs.readFileSync(desktop + "/src/components/" + name);
    fs.writeFileSync(evidence + "/snapshot/components/" + name, bytes);
    hashes[name] = crypto.createHash("sha256").update(bytes).digest("hex");
  }
  fs.writeFileSync(evidence + "/snapshot/types.ts", fs.readFileSync(desktop + "/src/types.ts"));
  fs.writeFileSync(evidence + "/tested-source-sha.json", JSON.stringify(hashes, null, 2));

  fs.writeFileSync(evidence + "/entry.tsx", `
    import React, { useState } from 'react';
    import { createRoot } from 'react-dom/client';
    import { DiscoverPanel } from './snapshot/components/DiscoverPanel';
    import { AuditPanel } from './snapshot/components/AuditPanel';
    import { SettingsPanel } from './snapshot/components/SettingsPanel';

    window.__calls = [];
    window.__ipcBehavior = { mode: 'reject', delayMs: 0 };
    window.__pendingIpc = [];
    window.__downloads = [];
    window.__timers = new Set();

    const origSetTimeout = window.setTimeout.bind(window);
    const origClearTimeout = window.clearTimeout.bind(window);
    window.setTimeout = (fn, delay, ...args) => {
      const timer = origSetTimeout(() => {
        window.__timers.delete(timer);
        fn(...args);
      }, delay);
      window.__timers.add(timer);
      return timer;
    };
    window.clearTimeout = (timer) => {
      window.__timers.delete(timer);
      origClearTimeout(timer);
    };

    // Intercept blob object URL and link click downloads
    let throwOnDownload = false;
    window.__setThrowOnDownload = (val) => { throwOnDownload = val; };
    const origCreateObjectURL = URL.createObjectURL;
    URL.createObjectURL = (blob) => {
      if (throwOnDownload) throw new Error('simulated Blob URL failure');
      window.__downloads.push({ type: blob.type, size: blob.size });
      return 'blob:mock-download-' + Math.random().toString(36).slice(2);
    };
    URL.revokeObjectURL = () => {};

    let root = createRoot(document.getElementById('root'));

    function TestApp() {
      const [currentPanel, setCurrentPanel] = useState('discover');
      const [theme, setTheme] = useState('sky');
      const [accent, setAccent] = useState('#2f6bff');
      const [events, setEvents] = useState([
        { id: 1, timestamp_ms: 1700000000000, category: 'plugin', action: 'install', target: 'wb2api', success: true, summary: '安装成功' },
        { id: 2, timestamp_ms: 1700000001000, category: 'gateway', action: 'stop', target: 'gateway', success: false, summary: '停止失败' }
      ]);
      const [plugins, setPlugins] = useState([
        { manifest: { metadata: { id: 'wb2api', name: 'WorkBuddy Bridge', version: '0.2.0' } } }
      ]);

      window.__setTestPanel = (panel, data) => {
        if (data && data.events !== undefined) setEvents(data.events);
        if (data && data.plugins !== undefined) setPlugins(data.plugins);
        if (data && data.theme !== undefined) setTheme(data.theme);
        setCurrentPanel(panel);
      };

      if (currentPanel === 'discover') {
        return (
          <DiscoverPanel
            plugins={plugins}
            onRefresh={() => window.__calls.push({ cmd: 'discover:refresh' })}
            onNavigatePlugins={() => window.__calls.push({ cmd: 'discover:navigate_plugins' })}
            onInstallSuccess={() => window.__calls.push({ cmd: 'discover:install_success' })}
          />
        );
      }
      if (currentPanel === 'audit') {
        return <AuditPanel events={events} onRefresh={() => window.__calls.push({ cmd: 'audit:refresh' })} />;
      }
      return (
        <SettingsPanel
          theme={theme}
          onThemeChange={setTheme}
          accentColor={accent}
          onAccentColorChange={setAccent}
        />
      );
    }

    window.__mount = () => {
      if (!root) root = createRoot(document.getElementById('root'));
      root.render(<TestApp />);
    };
    window.__unmount = () => {
      if (root) {
        root.unmount();
        root = null;
      }
    };
    window.__mount();
  `);

  await esbuild.build({
    entryPoints: [evidence + "/entry.tsx"],
    outfile: evidence + "/bundle.js",
    bundle: true,
    platform: "browser",
    jsx: "automatic",
    nodePaths: [desktop + "/node_modules"],
    logLevel: "silent",
    plugins: [
      {
        name: "owned-ipc-control",
        setup(build) {
          build.onResolve({ filter: /lib\/tauri$|@tauri-apps\/api\/core$/ }, () => ({
            path: "owned-ipc",
            namespace: "owned",
          }));
          build.onLoad({ filter: /.*/, namespace: "owned" }, () => ({
            contents: `
              export async function invoke(cmd, args) {
                window.__calls.push({ cmd, args });
                const behavior = window.__ipcBehavior || { mode: 'reject' };
                if (behavior.mode === 'pending') {
                  return new Promise((resolve, reject) => {
                    window.__pendingIpc.push({ cmd, args, resolve, reject });
                  });
                }
                if (behavior.delayMs) {
                  await new Promise(r => setTimeout(r, behavior.delayMs));
                }
                if (behavior.mode === 'resolve') {
                  return behavior.result;
                }
                throw new Error(behavior.errorMessage || 'owned IPC rejection: ' + cmd);
              }
            `,
          }));
        },
      },
    ],
  });

  server = http.createServer((req, res) => {
    res.setHeader(
      "content-type",
      req.url === "/bundle.js"
        ? "application/javascript"
        : "text/html; charset=utf-8"
    );
    res.end(
      req.url === "/bundle.js"
        ? fs.readFileSync(evidence + "/bundle.js")
        : '<html><meta charset="utf-8"><div id="root"></div><script src="/bundle.js"></script></html>'
    );
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  base = "http://127.0.0.1:" + server.address().port;
  browser = await chromium.launch({ headless: true });
}, 30000);

beforeEach(async () => {
  if (page) await page.close();
  page = await browser.newPage();
  await page.goto(base);
  await page.locator("#panel-discover").waitFor();
  await page.evaluate(() => {
    window.__calls = [];
    window.__pendingIpc = [];
    window.__downloads = [];
    window.__ipcBehavior = { mode: "reject", delayMs: 0 };
  });
});

afterAll(async () => {
  if (browser) await browser.close();
  if (server) await new Promise((resolve) => server.close(resolve));
});

const switchTo = async (panel, data) => {
  await page.evaluate(([p, d]) => window.__setTestPanel(p, d), [panel, data]);
  await page.locator("#panel-" + panel).waitFor();
};

describe("Wave2 browser regressions: Discover / Audit / Settings operational honesty (no native app)", () => {
  describe("DiscoverPanel browser behavior (DEF015)", () => {
    it("disallows empty or whitespace URL and keeps confirm install disabled", async () => {
      const confirmBtn = page.locator("#confirmInstall");
      expect(await confirmBtn.isDisabled()).toBe(true);
      await page.locator("#repoUrl").fill("   ");
      expect(await confirmBtn.isDisabled()).toBe(true);
    });

    it("rejects non-standard or malicious repo URLs with immediate error feedback and 0 IPC calls", async () => {
      const input = page.locator("#repoUrl");
      const confirmBtn = page.locator("#confirmInstall");
      await input.fill("ftp://github.com/bad/repo");
      expect(await confirmBtn.isDisabled()).toBe(false);
      await confirmBtn.click();

      const feedback = page.locator("#discoverFeedback");
      await feedback.waitFor();
      expect(await feedback.innerText()).toContain("当前仅支持标准 GitHub 仓库 URL");
      const calls = await page.evaluate(() => window.__calls);
      expect(calls.filter((c) => c.cmd === "import_project_from_github")).toHaveLength(0);
    });

    it("handles IPC rejection gracefully without creating fake parsed records", async () => {
      await page.evaluate(() => {
        window.__ipcBehavior = {
          mode: "reject",
          errorMessage: "网络超时无法克隆",
        };
      });
      await page.locator("#repoUrl").fill("https://github.com/owner/sample-plugin");
      await page.locator("#confirmInstall").click();

      const feedback = page.locator("#discoverFeedback");
      await feedback.waitFor();
      expect(await feedback.innerText()).toContain("网络超时无法克隆");
      expect(await page.locator("#confirmInstall").isDisabled()).toBe(false);
      expect(await page.locator("#confirmInstall").innerText()).toBe("克隆并安装");
      expect(await page.locator("#parseResult").count()).toBe(0);
    });

    it("enforces in-flight lock during github clone import", async () => {
      await page.evaluate(() => {
        window.__ipcBehavior = { mode: "pending" };
      });
      await page.locator("#repoUrl").fill("https://github.com/owner/locked-plugin");
      await page.locator("#confirmInstall").click();

      expect(await page.locator("#confirmInstall").isDisabled()).toBe(true);
      expect(await page.locator("#confirmInstall").innerText()).toBe("正在克隆与导入...");

      // Attempt second click while in-flight
      await page.locator("#confirmInstall").evaluate((btn) => btn.click());

      // Settle IPC with rejection
      await page.evaluate(() => {
        const item = window.__pendingIpc.shift();
        if (item) item.reject(new Error("owned settled rejection"));
      });

      await page.waitForFunction(
        () => document.getElementById("confirmInstall").innerText === "克隆并安装"
      );
      const calls = await page.evaluate(() => window.__calls);
      expect(calls.filter((c) => c.cmd === "import_project_from_github")).toHaveLength(1);
    });

    it("handles successful github import with parsed manifest display and callbacks", async () => {
      await page.evaluate(() => {
        window.__ipcBehavior = {
          mode: "resolve",
          result: {
            record: {
              manifest: {
                metadata: {
                  id: "sample-ext",
                  name: "Sample Extension",
                  version: "1.1.0",
                },
                runtime: {
                  kind: "managed",
                  entrypoint: "main.exe",
                },
              },
            },
            already_registered: false,
          },
        };
      });
      await page.locator("#repoUrl").fill("https://github.com/owner/sample-ext");
      await page.locator("#confirmInstall").click();

      const parseResult = page.locator("#parseResult");
      await parseResult.waitFor();
      expect(await parseResult.innerText()).toContain("Sample Extension");
      expect(await parseResult.innerText()).toContain("sample-ext");
      expect(await parseResult.innerText()).toContain("v1.1.0");

      const feedback = page.locator("#discoverFeedback");
      expect(await feedback.innerText()).toContain("成功克隆并安全导入插件: Sample Extension");

      const calls = await page.evaluate(() => window.__calls);
      expect(calls.some((c) => c.cmd === "discover:refresh")).toBe(true);
      expect(calls.some((c) => c.cmd === "discover:install_success")).toBe(true);
    });

    it("ignores in-flight github import if unmounted before settlement", async () => {
      await page.evaluate(() => {
        window.__ipcBehavior = { mode: "pending" };
      });
      await page.locator("#repoUrl").fill("https://github.com/owner/delayed-plugin");
      await page.locator("#confirmInstall").click();

      // Switch panel away (unmounts DiscoverPanel)
      await switchTo("audit");

      // Settle IPC now
      await page.evaluate(() => {
        const item = window.__pendingIpc.shift();
        if (item) item.resolve({ record: { manifest: { metadata: { id: "ghost" } } } });
      });

      // Switch back to discover
      await switchTo("discover");
      expect(await page.locator("#parseResult").count()).toBe(0);
      expect(await page.locator("#discoverFeedback").count()).toBe(0);
    });
  });

  describe("AuditPanel browser behavior (DEF016)", () => {
    it("filters host control events by category and updates table rows", async () => {
      await switchTo("audit");
      const rows = page.locator("#panel-audit table tbody tr");
      expect(await rows.count()).toBe(2);

      const filter = page.locator("#auditCategoryFilter");
      await filter.selectOption("plugin");
      expect(await page.locator("#panel-audit table tbody tr").count()).toBe(1);
      expect(await page.locator("#panel-audit table tbody tr").first().innerText()).toContain("wb2api");

      await filter.selectOption("gateway");
      expect(await page.locator("#panel-audit table tbody tr").count()).toBe(1);
      expect(await page.locator("#panel-audit table tbody tr").first().innerText()).toContain("gateway");

      await filter.selectOption("all");
      expect(await page.locator("#panel-audit table tbody tr").count()).toBe(2);
    });

    it("updates event details JSON when clicking different rows", async () => {
      await switchTo("audit");
      const firstRow = page.locator("#panel-audit table tbody tr").first();
      const secondRow = page.locator("#panel-audit table tbody tr").nth(1);

      await secondRow.click();
      const detailsText = await page.locator("#auditEventDetails").innerText();
      expect(detailsText).toContain('"target": "gateway"');
      expect(detailsText).toContain('"action": "stop"');

      await firstRow.click();
      const detailsFirst = await page.locator("#auditEventDetails").innerText();
      expect(detailsFirst).toContain('"target": "wb2api"');
      expect(detailsFirst).toContain('"action": "install"');
    });

    it("triggers client-side diagnostics download with timer-cleared feedback", async () => {
      await switchTo("audit");
      const exportBtn = page.locator("#exportDiagnosticsBtn");
      await exportBtn.click();

      const feedback = page.locator("#auditExportFeedback");
      await feedback.waitFor();
      expect(await feedback.innerText()).toContain("已发起诊断 JSON 下载");

      const downloads = await page.evaluate(() => window.__downloads);
      expect(downloads.length).toBeGreaterThan(0);
      expect(downloads[0].type).toBe("application/json");

      // Verify timer cleanup on unmount
      expect(await page.evaluate(() => window.__timers.size)).toBe(1);
      await switchTo("settings");
      expect(await page.evaluate(() => window.__timers.size)).toBe(0);
    });

    it("handles export diagnostics failure gracefully with visible alert", async () => {
      await switchTo("audit");
      await page.evaluate(() => window.__setThrowOnDownload(true));
      const exportBtn = page.locator("#exportDiagnosticsBtn");
      await exportBtn.click();

      const feedback = page.locator("#auditExportFeedback");
      await feedback.waitFor();
      expect(await feedback.getAttribute("role")).toBe("alert");
      expect(await feedback.innerText()).toContain("发起诊断导出失败");
      await page.evaluate(() => window.__setThrowOnDownload(false));
    });
  });

  describe("SettingsPanel browser behavior (DEF017)", () => {
    it("handles theme selection and displays glass-specific sliders only for glass", async () => {
      await switchTo("settings", { theme: "sky" });
      expect(await page.locator(".glass-only").count()).toBe(0);

      // Click glass theme button
      await page.locator('button[data-theme-set="glass"]').click();
      await page.locator(".glass-only").waitFor();
      expect(await page.locator(".glass-only").count()).toBe(1);

      // Click dark theme button
      await page.locator('button[data-theme-set="dark"]').click();
      expect(await page.locator(".glass-only").count()).toBe(0);
    });

    it("handles color swatch clicks and palette reset without backend mutations", async () => {
      await switchTo("settings");
      const swatch = page.locator('div[data-swatches="accent"] button[data-color="#7c5cff"]');
      await swatch.click();
      expect(await swatch.getAttribute("class")).toContain("active");

      const resetBtn = page.locator("button[data-reset-palette]");
      await resetBtn.click();
      const defaultSwatch = page.locator('div[data-swatches="accent"] button[data-color="#2f6bff"]');
      expect(await defaultSwatch.getAttribute("class")).toContain("active");
    });

    it("exports theme configuration JSON via client Blob download", async () => {
      await switchTo("settings");
      const exportBtn = page.getByRole("button", { name: "导出主题配置" });
      await exportBtn.click();

      const toast = page.locator("#settingsToast");
      await toast.waitFor();
      expect(await toast.innerText()).toContain("已发起主题与调色板配置 JSON 下载");

      const downloads = await page.evaluate(() => window.__downloads);
      expect(downloads.length).toBeGreaterThan(0);
      expect(downloads[downloads.length - 1].type).toBe("application/json");

      // Verify timer cleanup on unmount
      expect(await page.evaluate(() => window.__timers.size)).toBe(1);
      await switchTo("audit");
      expect(await page.evaluate(() => window.__timers.size)).toBe(0);
    });

    it("handles theme export failure gracefully with visible toast", async () => {
      await switchTo("settings");
      await page.evaluate(() => window.__setThrowOnDownload(true));
      const exportBtn = page.getByRole("button", { name: "导出主题配置" });
      await exportBtn.click();

      const toast = page.locator("#settingsToast");
      await toast.waitFor();
      expect(await toast.innerText()).toContain("导出配置失败");
      await page.evaluate(() => window.__setThrowOnDownload(false));
    });
  });
});
