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
const evidence = path.resolve(desktop, "../../.pi/development/ui-truth-shell/browser");
let server, browser, page, base;

function copyDirRecursive(srcDir, destDir) {
  fs.mkdirSync(destDir, { recursive: true });
  for (const entry of fs.readdirSync(srcDir, { withFileTypes: true })) {
    const srcPath = path.join(srcDir, entry.name);
    const destPath = path.join(destDir, entry.name);
    if (entry.isDirectory()) {
      copyDirRecursive(srcPath, destPath);
    } else if (
      !entry.name.endsWith(".test.tsx") &&
      !entry.name.endsWith(".test.ts") &&
      !entry.name.endsWith(".browser.test.mjs")
    ) {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

beforeAll(async () => {
  fs.mkdirSync(evidence + "/snapshot", { recursive: true });
  copyDirRecursive(desktop + "/src", evidence + "/snapshot");

  const hashes = {};
  for (const rel of ["api.ts", "components/AppShell.tsx", "components/Overview.tsx"]) {
    const bytes = fs.readFileSync(desktop + "/src/" + rel);
    hashes[rel] = crypto.createHash("sha256").update(bytes).digest("hex");
  }
  fs.writeFileSync(evidence + "/tested-source-sha.json", JSON.stringify(hashes, null, 2));

  fs.writeFileSync(
    evidence + "/entry.tsx",
    `
    import React from 'react';
    import { createRoot } from 'react-dom/client';
    import { AppShell } from './snapshot/components/AppShell';

    window.__calls = [];
    window.__intervals = new Map();

    const origSetInterval = window.setInterval.bind(window);
    const origClearInterval = window.clearInterval.bind(window);
    window.setInterval = (fn, delay, ...args) => {
      const id = origSetInterval(fn, delay, ...args);
      window.__intervals.set(id, { fn: () => fn(...args), delay });
      return id;
    };
    window.clearInterval = (id) => {
      window.__intervals.delete(id);
      return origClearInterval(id);
    };
    window.__triggerPoll = () => {
      for (const item of window.__intervals.values()) {
        item.fn();
      }
    };

    const defaultGateway = {
      is_running: true,
      port: 28319,
      active_backend: 'wb2api',
      master_key_preview: 'pw-sk-***'
    };
    const defaultPlugin = {
      manifest: {
        schema_version: '1.0',
        metadata: { id: 'wb2api', name: 'WorkBuddy Bridge', version: '0.2.0' },
        runtime: { ownership: 'managed', executable: 'wb2api.exe' },
        configuration: { required_secrets: [] }
      },
      settings: {}
    };
    const defaultEvent = {
      id: 101,
      timestamp_ms: 1700000000000,
      category: 'plugin',
      action: 'start',
      target: 'wb2api',
      success: true,
      summary: '受管插件已就绪'
    };

    window.__ipcConfig = {
      mode: 'success', // 'success' | 'reject' | 'empty' | 'partial-gw-fail' | 'manual-queue'
      windowErrorCmd: null,
      gateway: defaultGateway,
      plugins: [defaultPlugin],
      events: [defaultEvent],
      runtimeStatus: { status: 'ready', pid: 4321, port: 32123, active_requests: 0 }
    };
    window.__manualQueue = [];

    window.__handleIpc = (cmd, args) => {
      const cfg = window.__ipcConfig;
      window.__calls.push({ cmd, args: args || {}, mode: cfg.mode });

      if (cfg.windowErrorCmd === cmd) {
        return Promise.reject(new Error('模拟窗口命令拒绝: ' + cmd));
      }

      if (
        cmd === 'plugin:window|minimize' ||
        cmd === 'plugin:window|toggle_maximize' ||
        cmd === 'plugin:window|hide' ||
        cmd === 'quit_app' ||
        cmd === 'show_menubar'
      ) {
        return Promise.resolve();
      }

      if (cfg.mode === 'manual-queue') {
        return new Promise((resolve, reject) => {
          window.__manualQueue.push({ cmd, args, resolve, reject });
        });
      }

      if (cfg.mode === 'reject') {
        return Promise.reject(new Error('owned IPC rejection: ' + cmd));
      }

      if (cfg.mode === 'partial-gw-fail' && cmd === 'get_gateway_status') {
        return Promise.reject(new Error('gateway status read failed'));
      }

      switch (cmd) {
        case 'get_gateway_status':
          return Promise.resolve(
            cfg.mode === 'empty'
              ? { is_running: false, port: 28319, master_key_preview: 'pw-sk-***' }
              : cfg.gateway
          );
        case 'list_local_plugins':
          return Promise.resolve(cfg.mode === 'empty' ? [] : cfg.plugins);
        case 'get_host_events':
          return Promise.resolve(cfg.mode === 'empty' ? [] : cfg.events);
        case 'get_plugin_runtime_status':
          return Promise.resolve(cfg.runtimeStatus);
        case 'get_plugin_secrets_status':
          return Promise.resolve([]);
        default:
          return Promise.resolve(null);
      }
    };

    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: 'main' },
        currentWebview: { label: 'main' }
      },
      invoke: (cmd, args) => window.__handleIpc(cmd, args)
    };

    let root = null;
    window.__mount = (initialConfig) => {
      if (initialConfig) {
        window.__ipcConfig = { ...window.__ipcConfig, ...initialConfig };
      }
      if (root) {
        root.unmount();
      }
      root = createRoot(document.getElementById('root'));
      root.render(<AppShell />);
    };
    window.__unmount = () => {
      if (root) {
        root.unmount();
        root = null;
      }
    };
  `
  );

  await esbuild.build({
    entryPoints: [evidence + "/entry.tsx"],
    outfile: evidence + "/bundle.js",
    bundle: true,
    platform: "browser",
    jsx: "automatic",
    nodePaths: [desktop + "/node_modules"],
    logLevel: "silent",
    loader: { ".css": "css", ".svg": "dataurl", ".png": "dataurl" },
  });

  server = http.createServer((req, res) => {
    if (req.url === "/bundle.js") {
      res.setHeader("content-type", "application/javascript; charset=utf-8");
      res.end(fs.readFileSync(evidence + "/bundle.js"));
      return;
    }
    if (req.url === "/bundle.css") {
      res.setHeader("content-type", "text/css; charset=utf-8");
      res.end(fs.existsSync(evidence + "/bundle.css") ? fs.readFileSync(evidence + "/bundle.css") : "");
      return;
    }
    res.setHeader("content-type", "text/html; charset=utf-8");
    res.end(
      '<!doctype html><html><head><meta charset="utf-8"><link rel="stylesheet" href="/bundle.css"></head><body><div id="root"></div><script src="/bundle.js"></script></body></html>'
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
  await page.evaluate(() => {
    localStorage.clear();
    window.__calls = [];
    window.__manualQueue = [];
  });
});

afterAll(async () => {
  if (browser) await browser.close();
  if (server) await new Promise((resolve) => server.close(resolve));
});

describe("Wave 3 Owned Browser Shell & Native Window Control Regressions (DEF-018 / DEF-019)", () => {
  it("DEF-019: initial IPC rejection displays unknown/error states and never fabricates 0 plugins or OFF", async () => {
    await page.evaluate(() => window.__mount({ mode: "reject" }));
    await page.waitForFunction(() => window.__calls.length >= 3);
    await page.getByText("读取网关状态失败").waitFor();

    const overviewText = await page.locator("#panel-overview").innerText();
    expect(overviewText).toContain("状态未知");
    expect(overviewText).toContain("读取网关状态失败");
    expect(overviewText).toContain("审计事件读取失败");
    expect(overviewText).not.toContain("0 个");
    expect(overviewText).not.toContain("暂无审计事件记录");
    expect(overviewText).not.toContain("网关空闲");
    expect(overviewText).not.toContain("已停止");

    const sideNavText = await page.locator("#sideNav").innerText();
    expect(sideNavText).toContain("未知");
    expect(sideNavText).not.toContain("OFF");
    expect(sideNavText).not.toContain("0/0");

    const gwMiniText = await page.locator("#gwMini").innerText();
    expect(gwMiniText).toContain("网关状态未知");
    expect(gwMiniText).toContain("读取宿主状态失败");
  });

  it("DEF-019: retains last confirmed data with explicit stale indicator on refresh failure, then clears on recovery", async () => {
    await page.evaluate(() => window.__mount({ mode: "success" }));
    await page.getByText("1 个").waitFor();

    const beforeText = await page.locator("#panel-overview").innerText();
    expect(beforeText).toContain("网关运行中");
    expect(beforeText).toContain("1 个");
    expect(beforeText).toContain("1 / 1");
    expect(beforeText).toContain("受管插件已就绪");

    // 切换为拒绝模式并触发轮询刷新
    await page.evaluate(() => {
      window.__ipcConfig.mode = "reject";
      window.__triggerPoll();
    });

    await page.locator(".stale-warning").waitFor();
    const staleText = await page.locator("#panel-overview").innerText();
    expect(staleText).toContain("数据刷新失败，当前展示最后确认状态（数据可能已过期）");
    expect(staleText).toContain("1 个 (过期)");
    expect(staleText).toContain("运行中 (过期)");
    expect(staleText).toContain("127.0.0.1:28319 (刷新失败)");
    expect(staleText).not.toContain("0 个");

    const sideNavStale = await page.locator("#sideNav").innerText();
    expect(sideNavStale).toContain("ON (过期)");
    expect(sideNavStale).toContain("1/1 (过期)");

    // 恢复正常读取后应自动清除过期警告
    await page.evaluate(() => {
      window.__ipcConfig.mode = "success";
      window.__triggerPoll();
    });

    await page.waitForFunction(() => !document.querySelector(".stale-warning"));
    const recoveredText = await page.locator("#panel-overview").innerText();
    expect(recoveredText).toContain("网关运行中");
    expect(recoveredText).toContain("1 个");
    expect(recoveredText).not.toContain("(过期)");
    expect(recoveredText).not.toContain("(刷新失败)");
  });

  it("DEF-019: confirmed empty array [] truthfully displays 0 and clears outdated pluginStatuses", async () => {
    await page.evaluate(() => window.__mount({ mode: "success" }));
    await page.getByText("1 / 1").waitFor();

    // 当后续真实读取返回空数组 [] 时，应如实显示 0 个并清理旧 pluginStatuses
    await page.evaluate(() => {
      window.__ipcConfig.mode = "empty";
      window.__triggerPoll();
    });

    await page.getByText("0 / 0").waitFor();
    const emptyText = await page.locator("#panel-overview").innerText();
    expect(emptyText).toContain("0 个");
    expect(emptyText).toContain("0 / 0");
    expect(emptyText).toContain("已停止");
    expect(emptyText).toContain("暂无审计事件记录");

    const sideNavText = await page.locator("#sideNav").innerText();
    expect(sideNavText).toContain("0/0");
    expect(sideNavText).toContain("OFF");
  });

  it("DEF-019: isolates partial resource failure so successful resources still render", async () => {
    await page.evaluate(() => window.__mount({ mode: "partial-gw-fail" }));
    await page.getByText("读取网关状态失败").waitFor();

    const overviewText = await page.locator("#panel-overview").innerText();
    // 网关读取失败显示未知/失败，但插件和事件正常展示确认数据
    expect(overviewText).toContain("状态未知");
    expect(overviewText).toContain("读取网关状态失败");
    expect(overviewText).toContain("1 个");
    expect(overviewText).toContain("1 / 1");
    expect(overviewText).toContain("受管插件已就绪");
  });

  it("DEF-019: out-of-order concurrent refresh responses obey generational guard (latest wins)", async () => {
    await page.evaluate(() => window.__mount({ mode: "manual-queue" }));
    await page.waitForFunction(() => window.__manualQueue.length === 3);

    // 保留第 1 轮挂起请求，切换到 success 模式并发起第 2 轮最新刷新
    await page.evaluate(() => {
      window.__firstRound = window.__manualQueue.splice(0, 3);
      window.__ipcConfig.mode = "success";
      window.__triggerPoll();
    });

    // 第 2 轮已成功返回并更新为 1 个插件
    await page.getByText("1 个").waitFor();

    // 现在让旧的第 1 轮请求全部以空数组或拒绝结算，验证不会覆盖第 2 轮的最新状态
    await page.evaluate(() => {
      for (const item of window.__firstRound) {
        if (item.cmd === "list_local_plugins") {
          item.resolve([]);
        } else {
          item.reject(new Error("late outdated error"));
        }
      }
    });

    // 等待微任务结算后检查界面仍保持第 2 轮确认的 1 个插件且无 stale 警告
    await page.waitForTimeout(80);
    const overviewText = await page.locator("#panel-overview").innerText();
    expect(overviewText).toContain("1 个");
    expect(overviewText).toContain("1 / 1");
    expect(overviewText).not.toContain("0 个");
    expect(await page.locator(".stale-warning").count()).toBe(0);
  });

  it("DEF-019: cleans up polling interval on unmount and ignores post-unmount settlements", async () => {
    const pageErrors = [];
    page.on("pageerror", (err) => pageErrors.push(err.message));

    await page.evaluate(() => window.__mount({ mode: "manual-queue" }));
    await page.waitForFunction(() => window.__manualQueue.length === 3);
    expect(await page.evaluate(() => window.__intervals.size)).toBe(1);

    // 卸载组件
    await page.evaluate(() => window.__unmount());
    expect(await page.evaluate(() => window.__intervals.size)).toBe(0);

    // 卸载后结算挂起的 Promise，不应抛错
    await page.evaluate(() => {
      for (const item of window.__manualQueue) {
        item.resolve([]);
      }
    });
    await page.waitForTimeout(50);
    expect(pageErrors).toEqual([]);
  });

  it("persists theme selection to documentElement[data-theme] and localStorage across remounts", async () => {
    await page.evaluate(() => window.__mount({ mode: "success" }));
    await page.getByText("1 个").waitFor();

    expect(await page.evaluate(() => document.documentElement.getAttribute("data-theme"))).toBe("sky");

    await page.getByRole("button", { name: "玻璃" }).click();
    expect(await page.evaluate(() => document.documentElement.getAttribute("data-theme"))).toBe("glass");
    expect(await page.evaluate(() => localStorage.getItem("pw-theme"))).toBe("glass");

    await page.getByRole("button", { name: "深空" }).click();
    expect(await page.evaluate(() => document.documentElement.getAttribute("data-theme"))).toBe("dark");
    expect(await page.evaluate(() => localStorage.getItem("pw-theme"))).toBe("dark");

    // 重新挂载后应保持 dark 主题
    await page.evaluate(() => window.__mount({ mode: "success" }));
    await page.getByText("1 个").waitFor();
    expect(await page.evaluate(() => document.documentElement.getAttribute("data-theme"))).toBe("dark");
  });

  it("DEF-018: supports keyboard activation on minimize/maximize buttons and surfaces window errors", async () => {
    await page.evaluate(() => window.__mount({ mode: "success" }));
    await page.getByText("1 个").waitFor();

    // 键盘聚焦并通过 Enter 触发最小化
    const minBtn = page.getByRole("button", { name: "最小化" });
    await minBtn.focus();
    await page.keyboard.press("Enter");
    await page.waitForFunction(() =>
      window.__calls.some((c) => c.cmd === "plugin:window|minimize")
    );

    // 键盘聚焦并通过 Space 触发最大化/还原切换
    const maxBtn = page.getByRole("button", { name: "最大化" });
    await maxBtn.focus();
    await page.keyboard.press("Space");
    await page.waitForFunction(() =>
      window.__calls.some((c) => c.cmd === "plugin:window|toggle_maximize")
    );

    // 当底层窗口操作失败时，必须显式弹出错误告警而非静默忽略
    await page.evaluate(() => {
      window.__ipcConfig.windowErrorCmd = "plugin:window|minimize";
    });
    await minBtn.click();
    const alert = page.getByRole("alert");
    await alert.waitFor();
    expect(await alert.innerText()).toContain("窗口操作失败");
    expect(await alert.innerText()).toContain("plugin:window|minimize");
  });

  it("DEF-018: custom close confirmation modal supports cancel, tray, quit, remember preference, and quit failure feedback", async () => {
    await page.evaluate(() => window.__mount({ mode: "success" }));
    await page.getByText("1 个").waitFor();

    const closeBtn = page.getByRole("button", { name: "关闭" });
    await closeBtn.focus();
    await page.keyboard.press("Enter");

    // 弹出关闭确认对话框
    const dialog = page.getByRole("dialog", { name: "关闭 ProxyWeave" });
    await dialog.waitFor();
    expect(await dialog.innerText()).toContain("退出程序,还是最小化到托盘继续后台运行?");

    // 1. 点击“取消”关闭弹窗，不触发 hide 或 quit_app
    await dialog.getByRole("button", { name: "取消" }).click();
    expect(await dialog.count()).toBe(0);
    expect(
      await page.evaluate(() =>
        window.__calls.some((c) => c.cmd === "quit_app" || c.cmd === "plugin:window|hide")
      )
    ).toBe(false);

    // 2. 再次打开弹窗，勾选“记住我的选择”并选择“最小化到托盘”
    await closeBtn.click();
    await dialog.waitFor();
    await dialog.getByRole("checkbox").check();
    await dialog.getByRole("button", { name: "最小化到托盘" }).click();

    await page.waitForFunction(() =>
      window.__calls.some((c) => c.cmd === "plugin:window|hide") &&
      window.__calls.some((c) => c.cmd === "show_menubar")
    );
    expect(await page.evaluate(() => localStorage.getItem("proxyweave.closeAction"))).toBe("tray");

    // 3. 已记住 tray 偏好后，再次点击关闭按钮直接最小化到托盘，不再弹窗
    await page.evaluate(() => {
      window.__calls = [];
    });
    await closeBtn.click();
    await page.waitForFunction(() =>
      window.__calls.some((c) => c.cmd === "plugin:window|hide") &&
      window.__calls.some((c) => c.cmd === "show_menubar")
    );
    expect(await dialog.count()).toBe(0);

    // 4. 清除偏好后重新打开弹窗，测试“退出程序”及 quit_app 拒绝时的错误反馈
    await page.evaluate(() => {
      localStorage.removeItem("proxyweave.closeAction");
      window.__ipcConfig.windowErrorCmd = "quit_app";
      window.__calls = [];
    });
    await closeBtn.click();
    await dialog.waitFor();
    await dialog.getByRole("button", { name: "退出程序" }).click();

    const alert = page.getByRole("alert");
    await alert.waitFor();
    expect(await alert.innerText()).toContain("窗口操作失败");
    expect(await alert.innerText()).toContain("quit_app");
  });
});
