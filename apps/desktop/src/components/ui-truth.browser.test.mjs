import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";
import http from "node:http";
import crypto from "node:crypto";
import { createRequire } from "node:module";

// Reuse the installed reviewer browser, not a new dependency or a native acceptance claim.
const require = createRequire(import.meta.url);
const { chromium } = require("D:/pi-web-0.8.9/pi-web-0.8.9/node_modules/playwright");
const esbuild = require("esbuild");
const desktop = process.cwd();
const evidence = path.resolve(desktop, "../../.pi/development/ui-truth-wave1/browser");
let server, browser, page, base;

beforeAll(async () => {
  fs.mkdirSync(evidence + "/snapshot/components", { recursive: true });
  const hashes = {};
  for (const name of ["FlightPanel.tsx", "IdePanel.tsx"]) {
    const bytes = fs.readFileSync(desktop + "/src/components/" + name);
    fs.writeFileSync(evidence + "/snapshot/components/" + name, bytes);
    hashes[name] = crypto.createHash("sha256").update(bytes).digest("hex");
  }
  fs.writeFileSync(evidence + "/snapshot/types.ts", fs.readFileSync(desktop + "/src/types.ts"));
  fs.writeFileSync(evidence + "/snapshot/api.ts", fs.readFileSync(desktop + "/src/api.ts"));
  fs.cpSync(desktop + "/src/components/ide-source", evidence + "/snapshot/components/ide-source", { recursive: true });
  fs.cpSync(desktop + "/src/assets", evidence + "/snapshot/assets", { recursive: true });
  fs.writeFileSync(evidence + "/tested-source-sha.json", JSON.stringify(hashes, null, 2));
  fs.writeFileSync(evidence + "/entry.tsx", `
    import React from 'react';
    import { createRoot } from 'react-dom/client';
    import { FlightPanel } from './snapshot/components/FlightPanel';
    import { IdePanel } from './snapshot/components/IdePanel';
    window.__calls = []; window.__copies = []; window.__pendingCopies = [];
    window.__timers = new Set();
    const set = window.setTimeout.bind(window), clear = window.clearTimeout.bind(window);
    window.setTimeout = (fn, delay, ...args) => {
      const timer = set(() => { window.__timers.delete(timer); fn(...args); }, delay);
      if (delay === 2000) window.__timers.add(timer);
      return timer;
    };
    window.clearTimeout = timer => { window.__timers.delete(timer); clear(timer); };
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: {
      writeText: text => { window.__copies.push(text); return new Promise((resolve, reject) => window.__pendingCopies.push({resolve, reject})); }
    }});
    window.__settle = success => { const pending = window.__pendingCopies.shift(); if (!pending) throw Error('No pending owned copy'); success ? pending.resolve() : pending.reject(Error('owned clipboard rejection')); };
    let root = createRoot(document.getElementById('root'));
    window.__mount = (panel, state, key) => {
      if (!root) root = createRoot(document.getElementById('root'));
      const status = state === 'unknown' ? null : {is_running: state === 'running', port: 28319, master_key_preview: 'owned-preview'};
      root.render(panel === 'flight' ? <FlightPanel gatewayStatus={status} targetPluginId='not-a-model'/> : <IdePanel gatewayStatus={status} masterKey={key}/>);
    };
    window.__unmount = () => { root.unmount(); root = null; };
    window.__mount('ide', 'running', 'owned-fixture-key');
  `);
  await esbuild.build({
    entryPoints: [evidence + "/entry.tsx"], outfile: evidence + "/bundle.js", bundle: true,
    platform: "browser", jsx: "automatic", nodePaths: [desktop + "/node_modules"], logLevel: "silent", loader: { ".css": "empty", ".svg": "dataurl", ".png": "dataurl" },
    plugins: [{ name: "owned-ipc-rejection", setup(build) {
      build.onResolve({ filter: /lib\/tauri$|@tauri-apps\/api\/core$/ }, () => ({ path: "owned-ipc", namespace: "owned" }));
      build.onLoad({ filter: /.*/, namespace: "owned" }, () => ({ contents: "export class Channel { constructor() { this.onmessage = null; } } export async function invoke(cmd) { window.__calls.push(cmd); throw Error('owned IPC rejection'); }" }));
      build.onResolve({ filter: /@tauri-apps\/api\/window$/ }, () => ({ path: "owned-window", namespace: "owned-window" }));
      build.onLoad({ filter: /.*/, namespace: "owned-window" }, () => ({ contents: "export function getCurrentWindow() { return null; }" }));
    } }],
  });
  server = http.createServer((req, res) => {
    res.setHeader("content-type", req.url === "/bundle.js" ? "application/javascript" : "text/html; charset=utf-8");
    res.end(req.url === "/bundle.js" ? fs.readFileSync(evidence + "/bundle.js") : '<html><meta charset="utf-8"><div id="root"></div><script src="/bundle.js"></script></html>');
  });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  base = "http://127.0.0.1:" + server.address().port;
  browser = await chromium.launch({ headless: true });
}, 30000);

beforeEach(async () => {
  if (page) await page.close();
  page = await browser.newPage();
  await page.goto(base);
  await page.locator("#panel-ide").waitFor();
});
afterAll(async () => {
  if (browser) await browser.close();
  if (server) await new Promise(resolve => server.close(resolve));
});

const mount = async (panel, state = "running", key = "owned-fixture-key") => {
  await page.evaluate(([panel, state, key]) => window.__mount(panel, state, key), [panel, state, key]);
  await page.locator("#panel-" + panel).waitFor();
  // React updates to the same panel need an observable status gate.
  if (state === "unknown") await page.getByText(/网关状态未知/).first().waitFor();
  if (state === "stopped") await page.getByText(/网关已停止/).first().waitFor();
};
const body = () => page.locator("body").innerText();
const settle = async success => { await page.evaluate(success => window.__settle(success), success); };
const idle = async () => { await page.waitForFunction(() => !document.body.innerText.includes("复制中…")); };
const assertNoIpc = async () => expect(await page.evaluate(() => window.__calls)).toEqual([]);

describe("owned browser Flight / IDE rejection regressions (no real clipboard or native app)", () => {
  it.each(["unknown", "stopped", "running"])("Flight %s blocks submit even with a rejecting IPC seam", async state => {
    // Initial IDE mount legitimately detects now. Isolate Flight assertions to
    // its own mount rather than suppressing the new detection IPC.
    await page.evaluate(() => { window.__unmount(); window.__calls = []; });
    await mount("flight", state);
    if (state === "running") {
      await page.waitForFunction(() => document.body.innerText.includes("模型探测失败") || document.querySelector("#flightModel")?.innerText.includes("模型探测失败"));
    }
    expect(await page.locator("#sendBtn").isDisabled()).toBe(true);
    expect(await page.locator("#chatInput").isDisabled()).toBe(true);
    await page.locator("form").evaluate(form => form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })));
    await page.locator("#sendBtn").evaluate(button => button.click());
    expect(await page.locator(".chatbox .msg").count()).toBe(0);
    expect(await page.locator(".metric b").allTextContents()).toEqual(["未测试", "未测试", "未测试"]);
    expect(await page.locator(".checkrow .pill").allTextContents()).toEqual(Array(6).fill("未测试"));
    expect(await body()).not.toMatch(/pw\/coding|pw\/chat|pw\/wild|验证测试流|通过|降级/);
    if (state === "running") {
      expect(await page.evaluate(() => window.__calls)).toEqual(["flight_list_models"]);
    } else {
      await assertNoIpc();
    }
  });

  // Retired endpoint/key clipboard controls were deliberately removed with the
  // source-page transplant. Path-copy promise/rejection/retry/unmount and write
  // rejections are exercised by ide-source.browser.test.mjs instead, never skipped.
  it.each(["unknown", "stopped", "running"])("IDE %s detects without secrets, shows IPC rejection and never fabricates cards", async state => {
    await mount("ide", state, null);
    await page.getByText("检测失败", { exact: true }).first().waitFor();
    expect(await body()).toContain("检测 IPC 被拒绝");
    expect(await page.locator('[data-agent-target]').count()).toBe(0);
    expect(await page.locator('[data-agent-stat]').count()).toBe(4);
    expect(await page.getByRole("button", { name: "一键更新所有已安装智能体" }).isDisabled()).toBe(true);
    expect(await page.getByRole("button", { name: "复制 Master Key", exact: true }).count()).toBe(0);
    expect(await body()).not.toMatch(/写入成功|pw-sk-proxyweave-master-dev|pw\/coding|ANTHROPIC_AUTH_TOKEN/);
    expect(await page.evaluate(() => window.__copies)).toEqual([]);
    const calls = await page.evaluate(() => window.__calls);
    expect(calls).toContain("detect_agent_clients");
    expect(calls).not.toContain("get_gateway_master_key");
    expect(calls).not.toContain("import_agent_client");
    await page.getByRole('button', { name: '重试检测' }).click();
    await page.getByText("检测失败", { exact: true }).first().waitFor();
    expect((await page.evaluate(() => window.__calls)).filter(c => c === 'detect_agent_clients').length).toBeGreaterThan(1);
  });
});
