'use strict';

// Cross-platform Playwright resolution for the browser-backed acceptance
// suites. They used to `require()` an absolute path from the machine that first
// wrote them (`D:/pi-web-0.8.9/.../node_modules/playwright`), so all seven files
// aborted on Linux/CI with MODULE_NOT_FOUND before a single assertion ran.
// `playwright` is now a declared devDependency of apps/desktop and resolved at
// run time instead.
//
// Resolution order:
//   1. the declared dependency, normal Node resolution (Windows/macOS/Linux);
//   2. PLAYWRIGHT_MODULE_PATH, for images that ship Playwright elsewhere;
//   3. playwright-core, for images that only install the driver.
// A missing driver fails loudly here, once, with the command that fixes it.

const { createRequire } = require('node:module');

const requireFromHere = createRequire(__filename);

function tryRequire(id) {
  try {
    return requireFromHere(id);
  } catch {
    return null;
  }
}

const candidates = [];

const declared = tryRequire('playwright');
if (declared) candidates.push(['playwright', declared]);

if (process.env.PLAYWRIGHT_MODULE_PATH) {
  const overridden = tryRequire(process.env.PLAYWRIGHT_MODULE_PATH);
  if (overridden) candidates.push([process.env.PLAYWRIGHT_MODULE_PATH, overridden]);
}

const core = tryRequire('playwright-core');
if (core) candidates.push(['playwright-core', core]);

const resolved = candidates.find(
  ([, mod]) => mod && mod.chromium && typeof mod.chromium.launch === 'function',
);

if (!resolved) {
  throw new Error(
    'playwright-unavailable: the browser acceptance suites need Playwright. ' +
      'Run `npm ci` (or `npm install playwright`) in apps/desktop, then ' +
      '`npx playwright install --with-deps chromium`. Set PLAYWRIGHT_MODULE_PATH ' +
      'to point at a pre-installed copy if the driver lives elsewhere.',
  );
}

module.exports = resolved[1];
