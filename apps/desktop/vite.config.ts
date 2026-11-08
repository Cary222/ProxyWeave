import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  // The browser-backed acceptance suites drive a real Chromium once per file.
  // Vitest's 5s default measures a warm desktop machine; on a 1-vCPU CI runner
  // one navigation plus a paint can exceed it, and the suite then fails for the
  // runner's speed rather than for the behaviour it asserts. These bounds only
  // cap failures: a passing assertion still returns the moment the browser
  // reports.
  test: {
    testTimeout: 120_000,
    hookTimeout: 60_000,
    // Each suite drives a real Chromium. On a 1-vCPU runner the default worker
    // pool runs several at once and the box swaps or OOMs before the assertions
    // matter; one file at a time keeps peak memory inside the runner.
    fileParallelism: false,
  },
}));
