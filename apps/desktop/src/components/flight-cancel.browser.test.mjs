import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";
import http from "node:http";
import crypto from "node:crypto";
import { createRequire } from "node:module";

// Owned IPC seam only: tests the real React component, not native backend cancellation.
const require = createRequire(import.meta.url);
const { chromium } = require("D:/pi-web-0.8.9/pi-web-0.8.9/node_modules/playwright");
const esbuild = require("esbuild");
const desktop = process.cwd();
const evidence = path.resolve(desktop, "../../.pi/development/phase2-dev/browser-cancel");
let server, browser, page, base;

beforeAll(async () => {
  fs.mkdirSync(evidence + "/snapshot/components", { recursive: true });
  const hashes = {};
  for (const name of ["components/FlightPanel.tsx", "types.ts", "api.ts"]) {
    const bytes = fs.readFileSync(desktop + "/src/" + name);
    fs.writeFileSync(evidence + "/snapshot/" + name, bytes);
    hashes[name] = crypto.createHash("sha256").update(bytes).digest("hex");
  }
  fs.writeFileSync(evidence + "/tested-source-sha.json", JSON.stringify(hashes, null, 2));
  fs.writeFileSync(evidence + "/entry.tsx", `
    import React from 'react';
    import { createRoot } from 'react-dom/client';
    import { FlightPanel } from './snapshot/components/FlightPanel';
    window.__calls = []; window.__cancelResults = []; window.__channels = {};
    let root = null;
    window.__mount = () => {
      if (!root) root = createRoot(document.getElementById('root'));
      root.render(<FlightPanel gatewayStatus={{is_running: true, port: 28319, master_key_preview: 'owned'}} />);
    };
    window.__unmount = () => { root.unmount(); root = null; };
    window.__complete = (id) => window.__channels[id].onmessage({type: 'completed', request_id: id,
      total_text: 'owned terminal result', elapsed_ms: 12, finish_reason: 'stop'});
  `);
  await esbuild.build({
    entryPoints: [evidence + "/entry.tsx"], outfile: evidence + "/bundle.js", bundle: true,
    platform: "browser", jsx: "automatic", nodePaths: [desktop + "/node_modules"], logLevel: "silent",
    plugins: [{ name: "owned-flight-ipc", setup(build) {
      build.onResolve({ filter: /lib\/tauri$|@tauri-apps\/api\/core$/ }, () => ({ path: "ipc", namespace: "owned" }));
      build.onLoad({ filter: /.*/, namespace: "owned" }, () => ({ contents: `
        export class Channel { constructor() { this.onmessage = null; } }
        export async function invoke(cmd, args) {
          window.__calls.push({cmd, args: cmd === 'flight_cancel' ? args : undefined});
          if (cmd === 'flight_list_models') return {models: [{id: 'owned-model', name: 'Owned Model'}]};
          if (cmd === 'flight_chat_stream') {
            const id = 'owned-' + window.__calls.filter(c => c.cmd === cmd).length;
            window.__channels[id] = args.channel;
            if (window.__deferStart) await new Promise(resolve => { window.__resolveStart = resolve; });
            args.channel.onmessage({type: 'started', request_id: id, model: 'owned-model'});
            if (window.__earlyComplete) window.__complete(id);
            return {request_id: id, model: 'owned-model'};
          }
          if (cmd === 'flight_cancel') {
            let value = window.__cancelResults.shift() ?? {cancelled: false};
            if (value.defer) value = await new Promise(resolve => { window.__resolveCancel = resolve; });
            if (value.reject) throw new Error(value.message || 'PRIVATE-REJECTION');
            return value;
          }
          return null;
        }
      ` }));
      build.onResolve({ filter: /@tauri-apps\/api\/window$/ }, () => ({ path: "window", namespace: "window" }));
      build.onLoad({ filter: /.*/, namespace: "window" }, () => ({ contents: "export function getCurrentWindow() { return null; }" }));
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
  page.setDefaultTimeout(5000);
  await page.goto(base);
  await page.waitForFunction(() => typeof window.__mount === "function");
});
afterAll(async () => {
  if (browser) await browser.close();
  if (server) await new Promise(resolve => server.close(resolve));
});
const mount = async () => {
  await page.evaluate(() => window.__mount());
  await page.waitForFunction(() => document.querySelector("#flightModel")?.value === "owned-model");
};
const send = async () => {
  await page.locator("#chatInput").fill("owned cancellation probe");
  await page.locator("#sendBtn").click();
};
const start = async () => {
  await mount(); await send();
  await page.waitForFunction(() => Boolean(window.__channels['owned-1']));
  await page.locator("#cancelBtn").waitFor();
};
const count = cmd => page.evaluate(cmd => window.__calls.filter(c => c.cmd === cmd).length, cmd);
const alert = () => page.locator('[role="alert"]').innerText();
const outcomeCases = [
  ["false", {cancelled: false, message: "未找到活跃的飞行会话"}, "取消未完成"],
  ["rejection", {reject: true, message: "PRIVATE-REJECTION"}, "取消请求失败"],
];

describe("DEF-027 cancellation and lifecycle truth (owned browser seam)", () => {
  it("true confirms cancellation; duplicate clicks stay single-flight", async () => {
    await start();
    await page.evaluate(() => { window.__cancelResults = [{defer: true}]; });
    await page.locator("#cancelBtn").click();
    expect(await page.locator("#cancelBtn").isDisabled()).toBe(true);
    expect(await count("flight_cancel")).toBe(1);
    await page.evaluate(() => window.__resolveCancel({cancelled: true}));
    await page.getByText("已取消试飞推理", {exact: true}).waitFor();
    expect(await page.locator("#cancelBtn").count()).toBe(0);
  });

  for (const [name, value, message] of outcomeCases) {
    it(`${name} preserves active id and allows a successful retry without raw rejection`, async () => {
      await start();
      await page.evaluate(value => { window.__cancelResults = [value, {cancelled: true}]; }, value);
      await page.locator("#cancelBtn").click();
      await page.waitForFunction(message => document.querySelector('[role="alert"]')?.textContent.includes(message), message);
      expect(await alert()).not.toContain("已取消试飞推理");
      expect(await alert()).not.toContain("PRIVATE-REJECTION");
      expect(await page.locator("#chatInput").isDisabled()).toBe(true);
      await page.locator("#cancelBtn").click();
      await page.getByText("已取消试飞推理", {exact: true}).waitFor();
      const ids = await page.evaluate(() => window.__calls.filter(c => c.cmd === 'flight_cancel').map(c => c.args.requestId));
      expect(ids).toEqual(["owned-1", "owned-1"]);
    });
  }

  for (const [name, value] of [["true", {cancelled: true}], ...outcomeCases]) {
    it(`late ${name} cancellation cannot override terminal completion or a new request`, async () => {
      await start();
      await page.evaluate(() => { window.__cancelResults = [{defer: true}]; });
      await page.locator("#cancelBtn").click();
      await page.evaluate(() => window.__complete('owned-1'));
      await page.getByText('owned terminal result', {exact: true}).waitFor();
      await send();
      await page.waitForFunction(() => Boolean(window.__channels['owned-2']));
      await page.evaluate(value => window.__resolveCancel(value), value);
      await page.waitForFunction(() => !document.querySelector('#cancelBtn')?.disabled);
      expect(await page.locator('#chatInput').isDisabled()).toBe(true);
      expect(await page.locator('[role="alert"]').count()).toBe(0);
      // An old channel must not settle the new request either.
      await page.evaluate(() => window.__complete('owned-1'));
      expect(await page.locator('#cancelBtn').count()).toBe(1);
      await page.evaluate(() => window.__complete('owned-2'));
    });
  }

  it("a terminal event supersedes an earlier cancel failure notice", async () => {
    await start();
    await page.evaluate(() => { window.__cancelResults = [{cancelled: false}]; });
    await page.locator('#cancelBtn').click();
    await page.waitForFunction(() => document.querySelector('[role="alert"]')?.textContent.includes('取消未完成'));
    await page.evaluate(() => window.__complete('owned-1'));
    await page.getByText('owned terminal result', {exact: true}).waitFor();
    expect(await page.locator('[role="alert"]').count()).toBe(0);
    expect(await page.locator('#cancelBtn').count()).toBe(0);
  });

  it("an old cancellation result cannot unlock a newer pending cancellation", async () => {
    await start();
    await page.evaluate(() => { window.__cancelResults = [{defer: true}]; });
    await page.locator('#cancelBtn').click();
    await page.evaluate(() => { window.__oldCancel = window.__resolveCancel; window.__complete('owned-1'); });
    await send();
    await page.waitForFunction(() => Boolean(window.__channels['owned-2']));
    await page.evaluate(() => { window.__cancelResults = [{defer: true}]; });
    await page.locator('#cancelBtn').click();
    await page.evaluate(() => window.__oldCancel({cancelled: false}));
    expect(await page.locator('#cancelBtn').isDisabled()).toBe(true);
    expect(await page.locator('[role="alert"]').count()).toBe(0);
    await page.evaluate(() => window.__resolveCancel({cancelled: true}));
    await page.getByText('已取消试飞推理', {exact: true}).waitFor();
  });

  it("terminal completion after failed unmount cancellation clears the retained id", async () => {
    await start();
    await page.evaluate(() => { window.__cancelResults = [{cancelled: false}]; window.__unmount(); });
    await page.waitForFunction(() => window.__calls.filter(c => c.cmd === 'flight_cancel').length === 1);
    await page.evaluate(() => window.__complete('owned-1'));
    await mount(); await send();
    await page.waitForFunction(() => Boolean(window.__channels['owned-2']));
    expect(await count('flight_cancel')).toBe(1);
  });
  for (const [name, value, message] of outcomeCases) {
    it(`unmount ${name} retains unresolved id; pre-send ${name} blocks overlap; retry succeeds`, async () => {
      await start();
      await page.evaluate(value => { window.__cancelResults = [value, value, {cancelled: true}]; window.__unmount(); }, value);
      await page.waitForFunction(() => window.__calls.filter(c => c.cmd === 'flight_cancel').length === 1);
      await mount(); await send();
      await page.waitForFunction(message => document.querySelector('[role="alert"]')?.textContent.includes(message), message);
      expect(await count('flight_chat_stream')).toBe(1);
      expect(await alert()).not.toContain('PRIVATE-REJECTION');
      await page.locator('#sendBtn').click();
      await page.waitForFunction(() => Boolean(window.__channels['owned-2']));
      const ids = await page.evaluate(() => window.__calls.filter(c => c.cmd === 'flight_cancel').map(c => c.args.requestId));
      expect(ids).toEqual(['owned-1', 'owned-1', 'owned-1']);
    });
  }

  it("unmount true clears retained session and starts a fresh request without a redundant cancel", async () => {
    await start();
    await page.evaluate(() => { window.__cancelResults = [{cancelled: true}]; window.__unmount(); });
    await page.waitForFunction(() => window.__calls.filter(c => c.cmd === 'flight_cancel').length === 1);
    await mount(); await send();
    await page.waitForFunction(() => Boolean(window.__channels['owned-2']));
    expect(await count('flight_cancel')).toBe(1);
  });

  it("terminal event before IPC start result must not resurrect the completed id", async () => {
    await page.evaluate(() => { window.__earlyComplete = true; });
    await mount(); await send();
    await page.getByText('owned terminal result', {exact: true}).waitFor();
    await page.evaluate(() => window.__unmount());
    expect(await count('flight_cancel')).toBe(0);
  });

  it("unmount while start IPC is pending cancels its eventual id before any next send", async () => {
    await page.evaluate(() => { window.__deferStart = true; });
    await start();
    await page.evaluate(() => { window.__cancelResults = [{cancelled: true}]; window.__unmount(); });
    await mount();
    await page.locator('#chatInput').fill('next owned request');
    await page.locator('#sendBtn').click();
    expect(await count('flight_chat_stream')).toBe(1);
    await page.evaluate(() => { window.__deferStart = false; window.__resolveStart(); });
    await page.waitForFunction(() => Boolean(window.__channels['owned-2']));
    expect(await count('flight_cancel')).toBe(1);
  });
});
