import { beforeAll, afterAll, beforeEach, describe, it, expect } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require('D:/pi-web-0.8.9/pi-web-0.8.9/node_modules/playwright');
const esbuild = require('esbuild');
const desktop = process.cwd();
const evidence = path.resolve(desktop, '../../.pi/development/ide-theme-fix/browser');
let browser, server, base, page;

const ids = [
  'claude-code', 'claude-desktop', 'codex', 'dsh',
  'opencode', 'pi', 'grok-build', 'zcode',
  'kimi-code', 'openclaw', 'hermes'
];

beforeAll(async () => {
  fs.mkdirSync(evidence, { recursive: true });
  fs.writeFileSync(evidence + '/entry.tsx', `
    import React, { useState } from 'react';
    import { createRoot } from 'react-dom/client';
    import { IdePanel } from '${desktop.replaceAll('\\', '/')}/src/components/IdePanel';
    import { FlightPanel } from '${desktop.replaceAll('\\', '/')}/src/components/FlightPanel';

    window.calls = []; window.queue = []; window.copies = []; window.mode = 'ok'; window.rejectDetect = false;
    window.targets = ${JSON.stringify(ids)}.map((id, i) => ({
      id,
      label: id === 'claude-desktop' ? 'Claude Desktop for Windows' : id,
      installed: true,
      configured: id === 'opencode',
      supported: ![0, 1, 2, 6].includes(i),
      protocol: [0, 1].includes(i) ? 'Anthropic Messages' : [2, 6].includes(i) ? 'OpenAI Responses' : 'OpenAI Chat',
      config_path: 'C:/Users/User/.config/' + id + '/settings.json',
      version: id === 'pi' ? '0.8.9' : null,
      note: 'owned fixture'
    }));

    window.ipc = async (cmd, args) => {
      window.calls.push({ cmd, args });
      if (cmd === 'detect_agent_clients') {
        if (window.rejectDetect) throw Error('reject');
        return window.targets;
      }
      if (cmd === 'flight_list_models') return { models: [{ id: 'owned-model' }] };
      if (cmd === 'list_agent_backups') {
        return [{ id: '1775000000000', created_at: 1775000000000, path: 'owned/backup' }];
      }
      if (cmd === 'import_agent_client' || cmd === 'restore_agent_client') {
        return { models: args?.models ?? [], restored_files: 1 };
      }
      throw Error('Unexpected IPC ' + cmd);
    };

    function RootApp() {
      const [tab, setTab] = useState('ide');
      return (
        <div style={{ padding: 16 }}>
          <div style={{ marginBottom: 12, display: 'flex', gap: 8 }}>
            <button id="tab-ide" onClick={() => setTab('ide')}>IDE Panel</button>
            <button id="tab-flight" onClick={() => setTab('flight')}>Flight Panel</button>
          </div>
          {tab === 'ide' ? (
            <IdePanel
              gatewayStatus={{ is_running: true, port: 28319, master_key_preview: 'pw_live_preview' }}
              masterKey="owned-fixture-key"
            />
          ) : (
            <div id="flight-container" style={{ padding: 16 }}>
              <FlightPanel
                gatewayStatus={{ is_running: true, port: 28319, master_key_preview: 'pw_live_preview' }}
                masterKey="owned-fixture-key"
              />
            </div>
          )}
        </div>
      );
    }

    const container = document.getElementById('root');
    const root = createRoot(container);
    root.render(<RootApp />);
  `);

  await esbuild.build({
    entryPoints: [evidence + '/entry.tsx'],
    outfile: evidence + '/bundle.js',
    bundle: true,
    platform: 'browser',
    jsx: 'automatic',
    nodePaths: [desktop + '/node_modules'],
    loader: { '.css': 'empty', '.svg': 'dataurl', '.png': 'dataurl' },
    plugins: [{
      name: 'owned-ipc',
      setup(b) {
        b.onResolve({ filter: /lib\/tauri$/ }, () => ({ path: 'ipc', namespace: 'owned' }));
        b.onLoad({ filter: /.*/, namespace: 'owned' }, () => ({
          contents: 'export const invoke=(cmd,args)=>window.ipc(cmd,args); export class Channel {}'
        }));
      }
    }]
  });

  const cssName = fs.readdirSync(desktop + '/dist/assets').find(n => n.endsWith('.css'));
  fs.copyFileSync(desktop + '/dist/assets/' + cssName, evidence + '/tested-built.css');

  server = http.createServer((req, res) => {
    if (req.url === '/bundle.js') {
      res.setHeader('content-type', 'application/javascript');
      res.end(fs.readFileSync(evidence + '/bundle.js'));
    } else if (req.url === '/style.css') {
      res.setHeader('content-type', 'text/css');
      res.end(fs.readFileSync(evidence + '/tested-built.css'));
    } else {
      res.setHeader('content-type', 'text/html; charset=utf-8');
      res.end(`<!doctype html>
<html data-theme="light">
<head>
  <meta charset="utf-8">
  <link rel="stylesheet" href="/style.css">
  <style>
    body { margin: 0; padding: 0; font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif); }
  </style>
</head>
<body>
  <div id="root"></div>
  <script src="/bundle.js"></script>
</body>
</html>`);
    }
  });

  await new Promise(r => server.listen(0, '127.0.0.1', r));
  base = 'http://127.0.0.1:' + server.address().port;
  browser = await chromium.launch({ headless: true });
}, 30000);

afterAll(async () => {
  await browser?.close();
  if (server) await new Promise(r => server.close(r));
});

beforeEach(async () => {
  await page?.close();
  page = await browser.newPage({ viewport: { width: 1366, height: 768 } });
  await page.goto(base);
  await page.waitForFunction(() => document.querySelectorAll('[data-agent-target]').length === 11);
});

describe('IDE theme adaptation and layout boundary tests', () => {
  const themes = ['light', 'dark', 'glass'];
  const viewports = [
    { width: 1366, height: 768, name: '1366x768' },
    { width: 960, height: 640, name: '960x640' }
  ];

  for (const vp of viewports) {
    for (const theme of themes) {
      it(`theme=${theme} at ${vp.name}: all 11 cards and buttons stay within card bounds`, async () => {
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await page.evaluate(t => {
          document.documentElement.dataset.theme = t;
        }, theme);

        // Wait for rendering and styles to apply
        await page.waitForTimeout(50);

        // Verify all 11 cards exist
        const cards = page.locator('[data-agent-target]');
        expect(await cards.count()).toBe(11);

        // Check bounding box of every card and all its child buttons
        const overflowViolations = await page.evaluate(() => {
          const violations = [];
          const cardEls = document.querySelectorAll('[data-agent-target]');
          cardEls.forEach(c => {
            const cardId = c.getAttribute('data-agent-target');
            const cRect = c.getBoundingClientRect();
            const buttons = c.querySelectorAll('button');
            buttons.forEach(btn => {
              const bRect = btn.getBoundingClientRect();
              const text = btn.innerText.trim();
              // Check if button exceeds card's right boundary (allowing 1px sub-pixel tolerance)
              if (bRect.right > cRect.right + 1) {
                violations.push({
                  cardId,
                  buttonText: text,
                  cardRight: cRect.right,
                  btnRight: bRect.right,
                  overflow: bRect.right - cRect.right
                });
              }
              // Check if button exceeds card's left boundary
              if (bRect.left < cRect.left - 1) {
                violations.push({
                  cardId,
                  buttonText: text,
                  cardLeft: cRect.left,
                  btnLeft: bRect.left,
                  overflow: cRect.left - bRect.left
                });
              }
            });
          });
          return violations;
        });

        expect(overflowViolations).toEqual([]);

        // Take screenshot
        const screenshotPath = `${evidence}/${theme}-${vp.name}.png`;
        await page.screenshot({ path: screenshotPath, fullPage: true });
        expect(fs.existsSync(screenshotPath)).toBe(true);
      });
    }
  }

  it('computed tokens match host theme colors across light, dark, and glass', async () => {
    // 1. Light theme
    await page.evaluate(() => { document.documentElement.dataset.theme = 'light'; });
    let styles = await page.evaluate(() => {
      const btn = (() => { const btns = document.querySelectorAll('[data-agent-target="pi"] button'); return btns[btns.length - 1]; })();
      const card = document.querySelector('[data-agent-target="pi"]');
      const computedBtn = window.getComputedStyle(btn);
      const computedCard = window.getComputedStyle(card);
      return {
        btnBg: computedBtn.backgroundColor,
        btnColor: computedBtn.color,
        cardBg: computedCard.backgroundColor,
        cardBorder: computedCard.borderColor
      };
    });
    // Host blue in light theme is #2f6bff -> rgb(47, 107, 255)
    expect(styles.btnBg).toBe('rgb(47, 107, 255)');
    expect(styles.btnColor).toBe('rgb(255, 255, 255)');

    // 2. Dark theme
    await page.evaluate(() => { document.documentElement.dataset.theme = 'dark'; });
    await page.waitForTimeout(200);
    styles = await page.evaluate(() => {
      const btn = (() => { const btns = document.querySelectorAll('[data-agent-target="pi"] button'); return btns[btns.length - 1]; })();
      const card = document.querySelector('[data-agent-target="pi"]');
      const computedBtn = window.getComputedStyle(btn);
      const computedCard = window.getComputedStyle(card);
      return {
        btnBg: computedBtn.backgroundColor,
        btnColor: computedBtn.color,
        cardBg: computedCard.backgroundColor,
        cardBorder: computedCard.borderColor
      };
    });
    // Host blue in dark theme is #6f9bff -> rgb(111, 155, 255)
    expect(styles.btnBg).toBe('rgb(111, 155, 255)');
    // Host card in dark theme is #101013 -> rgb(16, 16, 19)
    expect(styles.cardBg).toBe('rgb(16, 16, 19)');

    // 3. Glass theme
    await page.evaluate(() => { document.documentElement.dataset.theme = 'glass'; });
    await page.waitForTimeout(200);
    styles = await page.evaluate(() => {
      const btn = (() => { const btns = document.querySelectorAll('[data-agent-target="pi"] button'); return btns[btns.length - 1]; })();
      const card = document.querySelector('[data-agent-target="pi"]');
      const computedBtn = window.getComputedStyle(btn);
      const computedCard = window.getComputedStyle(card);
      return {
        btnBg: computedBtn.backgroundColor,
        btnColor: computedBtn.color,
        cardBg: computedCard.backgroundColor,
        backdropFilter: computedCard.backdropFilter || computedCard.webkitBackdropFilter
      };
    });
    // In glass theme, primary button remains host blue
    expect(styles.btnBg).toBe('rgb(47, 107, 255)');
    expect(styles.backdropFilter).toContain('blur');
  });

  it('backup history dialog portal inherits host theme and renders correctly', async () => {
    for (const theme of ['light', 'dark', 'glass']) {
      await page.evaluate(t => { document.documentElement.dataset.theme = t; }, theme);

      // Open backup dialog for pi
      const backupBtn = page.locator('[data-agent-target="pi"]').getByRole('button', { name: '备份历史' });
      await backupBtn.click();

      // Wait for dialog content to appear
      const dialog = page.locator('[role="dialog"]');
      await dialog.waitFor();

      // Verify dialog has .ide-workbench and data-theme matching the root
      const dialogThemeInfo = await dialog.evaluate(el => ({
        hasWorkbenchClass: el.classList.contains('ide-workbench'),
        dataTheme: el.getAttribute('data-theme'),
        bg: window.getComputedStyle(el).backgroundColor,
        titleColor: window.getComputedStyle(el.querySelector('h2') || el).color
      }));

      expect(dialogThemeInfo.hasWorkbenchClass).toBe(true);
      expect(dialogThemeInfo.dataTheme).toBe(theme);

      // Screenshot dialog
      await page.screenshot({ path: `${evidence}/dialog-${theme}.png` });

      // Close dialog
      const closeBtn = dialog.getByRole('button', { name: '关闭' });
      await closeBtn.click();
      await dialog.waitFor({ state: 'detached' });
    }
  });

  it('other panels before computed tokens are unchanged (no prototype token leakage)', async () => {
    // Switch to flight tab
    await page.locator('#tab-flight').click();
    await page.locator('#flight-container').waitFor();

    // Verify host tokens on flight panel
    const hostTokens = await page.evaluate(() => {
      const el = document.getElementById('flight-container');
      const cs = window.getComputedStyle(document.documentElement);
      return {
        blue: cs.getPropertyValue('--blue').trim(),
        card: cs.getPropertyValue('--card').trim(),
        text: cs.getPropertyValue('--text').trim()
      };
    });

    expect(hostTokens.blue).toBe('#2f6bff');
    expect(hostTokens.card).toBe('#ffffff');

    // Switch to dark
    await page.evaluate(() => { document.documentElement.dataset.theme = 'dark'; });
    const darkTokens = await page.evaluate(() => {
      const cs = window.getComputedStyle(document.documentElement);
      return {
        blue: cs.getPropertyValue('--blue').trim(),
        card: cs.getPropertyValue('--card').trim()
      };
    });
    expect(darkTokens.blue).toBe('#6f9bff');
    expect(darkTokens.card).toBe('#101013');
  });
});
