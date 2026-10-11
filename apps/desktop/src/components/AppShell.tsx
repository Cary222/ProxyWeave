// Window behavior extracted from Quotio AppShell.tsx; see docs/verification/PHASE0.md.
import { useState } from "react";
import { invoke } from "../lib/tauri";
import "../App.css";
import "./shell.css";
const t = (_key: string, fallback: string) => fallback;
async function toggleMaximize() {
  if (!("__TAURI_INTERNALS__" in window)) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().toggleMaximize();
}
export function AppShell() {
  const [closeDialog, setCloseDialog] = useState(false);
  const [rememberClose, setRememberClose] = useState(false);
  const [minimizeDialog, setMinimizeDialog] = useState(false);
  const [rememberMinimize, setRememberMinimize] = useState(false);
  const [closing, setClosing] = useState(false);
  // Keep the upstream paint-before-exit behavior; Phase 0 has no provider cleanup.
  function doQuit() {
    setClosing(true);
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        void (async () => {
          if (!("__TAURI_INTERNALS__" in window)) return;
          await invoke("quit_app");
        })();
      }),
    );
  }

  // Closing prompts whether to quit or hide to the tray, unless a remembered
  // choice exists (saved after ticking "记住我的选择").
  async function requestClose() {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let saved: string | null = null;
    try {
      saved = localStorage.getItem("proxyweave.closeAction");
    } catch {
      /* storage unavailable */
    }
    if (saved === "quit") return doQuit();
    if (saved === "tray") return minimizeToTray();
    setCloseDialog(true);
  }

  function chooseClose(choice: "quit" | "tray") {
    if (rememberClose) {
      try {
        localStorage.setItem("proxyweave.closeAction", choice);
      } catch {
        /* storage unavailable */
      }
    }
    setCloseDialog(false);
    if (choice === "tray") {
      void minimizeToTray();
    } else {
      doQuit();
    }
  }

  async function minimizeToTray() {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().hide();
    void invoke("show_menubar");
  }

  async function minimizeToTaskbar() {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  }

  async function requestMinimize() {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let saved: string | null = null;
    try {
      saved = localStorage.getItem("proxyweave.minimizeAction");
    } catch {
      /* storage unavailable */
    }
    if (saved === "tray") return minimizeToTray();
    if (saved === "taskbar") return minimizeToTaskbar();
    setMinimizeDialog(true);
  }

  function chooseMinimize(choice: "tray" | "taskbar") {
    if (rememberMinimize) {
      try {
        localStorage.setItem("proxyweave.minimizeAction", choice);
      } catch {
        /* storage unavailable */
      }
    }
    setMinimizeDialog(false);
    if (choice === "tray") {
      void minimizeToTray();
    } else {
      void minimizeToTaskbar();
    }
  }

  return (<main className="app-shell app-shell--v2"><aside className="sidebar">        <div className="sidebar-titlebar">
          <div className="window-controls">
            <button type="button" className="win-dot win-dot--close" onClick={() => void requestClose()} aria-label="关闭" title="关闭">
              <svg className="win-dot-icon" viewBox="0 0 12 12"><path d="M3.172 3.172a.5.5 0 0 1 .707 0L6 5.293l2.121-2.121a.5.5 0 1 1 .707.707L6.707 6l2.121 2.121a.5.5 0 0 1-.707.707L6 6.707 3.879 8.828a.5.5 0 1 1-.707-.707L5.293 6 3.172 3.879a.5.5 0 0 1 0-.707Z" fill="currentColor"/></svg>
            </button>
            <button type="button" className="win-dot win-dot--min" onClick={() => void requestMinimize()} aria-label="最小化" title="最小化">
              <svg className="win-dot-icon" viewBox="0 0 12 12"><rect x="2" y="5.25" width="8" height="1.5" rx=".75" fill="currentColor"/></svg>
            </button>
            <button type="button" className="win-dot win-dot--max" onClick={() => void toggleMaximize()} aria-label="最大化" title="最大化">
              <svg className="win-dot-icon" viewBox="0 0 12 12"><path d="M2 4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V4Zm2-.75a.75.75 0 0 0-.75.75v4c0 .414.336.75.75.75h4a.75.75 0 0 0 .75-.75V4a.75.75 0 0 0-.75-.75H4Z" fill="currentColor"/></svg>
            </button>
          </div>
          <div className="titlebar-drag" data-tauri-drag-region />
        </div>

<h1>ProxyWeave</h1><p>Local API plugin host</p></aside><section className="content"><h2>Phase 0 — Desktop baseline</h2><p>Window, tray and single-instance shell are available.</p><p>Gateway, plugins, protocol conversion and IDE handoff are not enabled.</p><p>Quotio v0.7.7 · MIT · Copyright (c) 2025 Trong Nguyen</p></section>      {closeDialog ? (
        <div className="modal-overlay" onClick={() => setCloseDialog(false)}>
          <div className="close-dialog" onClick={(event) => event.stopPropagation()}>
            <strong className="close-dialog-title">{t("close.title", "关闭 ProxyWeave")}</strong>
            <p className="close-dialog-desc">{t("close.desc", "退出程序,还是最小化到托盘继续后台运行?")}</p>
            <label className="close-dialog-remember">
              <input type="checkbox" checked={rememberClose} onChange={(event) => setRememberClose(event.target.checked)} />
              <span>{t("close.remember", "记住我的选择")}</span>
            </label>
            <div className="close-dialog-actions">
              <button type="button" className="ghost-action" onClick={() => setCloseDialog(false)}>
                {t("close.cancel", "取消")}
              </button>
              <button type="button" className="secondary-action" onClick={() => chooseClose("tray")}>
                {t("close.tray", "最小化到托盘")}
              </button>
              <button type="button" className="danger-action" onClick={() => chooseClose("quit")}>
                {t("close.quit", "退出程序")}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {minimizeDialog ? (
        <div className="modal-overlay" onClick={() => setMinimizeDialog(false)}>
          <div className="close-dialog" onClick={(event) => event.stopPropagation()}>
            <strong className="close-dialog-title">最小化 ProxyWeave</strong>
            <p className="close-dialog-desc">隐藏到托盘并弹出悬浮窗，还是最小化到任务栏？</p>
            <label className="close-dialog-remember">
              <input type="checkbox" checked={rememberMinimize} onChange={(event) => setRememberMinimize(event.target.checked)} />
              <span>记住我的选择</span>
            </label>
            <div className="close-dialog-actions">
              <button type="button" className="ghost-action" onClick={() => setMinimizeDialog(false)}>
                取消
              </button>
              <button type="button" className="secondary-action" onClick={() => chooseMinimize("taskbar")}>
                最小化到任务栏
              </button>
              <button type="button" className="primary-action" onClick={() => chooseMinimize("tray")}>
                隐藏到托盘
              </button>
            </div>
          </div>
        </div>
      ) : null}

 {closing ? <p role="status">Closing…</p> : null}</main>);
}
