import React, { useEffect, useRef, useState } from "react";
import { ThemeMode } from "../types";

interface SettingsPanelProps {
  theme?: ThemeMode;
  currentTheme?: ThemeMode;
  onThemeChange: (theme: ThemeMode) => void;
  accentColor?: string;
  onAccentColorChange?: (color: string) => void;
  bgColor?: string;
  onBgColorChange?: (color: string) => void;
  blur?: number;
  onBlurChange?: (blur: number) => void;
  cardOpacity?: number;
  onCardOpacityChange?: (opacity: number) => void;
  onResetPalette?: () => void;
}

export const SettingsPanel: React.FC<SettingsPanelProps> = ({
  theme: explicitTheme,
  currentTheme,
  onThemeChange,
  accentColor: initialAccentColor = "#2f6bff",
  onAccentColorChange,
  bgColor: initialBgColor = "#fafafa",
  onBgColorChange,
  blur: initialBlur = 18,
  onBlurChange,
  cardOpacity: initialCardOpacity = 0.72,
  onCardOpacityChange,
  onResetPalette,
}) => {
  const theme = explicitTheme || currentTheme || "sky";
  const [internalAccent, setInternalAccent] = useState(initialAccentColor);
  const [internalBg, setInternalBg] = useState(initialBgColor);
  const [internalBlur, setInternalBlur] = useState(initialBlur);
  const [internalOpacity, setInternalOpacity] = useState(initialCardOpacity);

  const accentColor = onAccentColorChange ? initialAccentColor : internalAccent;
  const bgColor = onBgColorChange ? initialBgColor : internalBg;
  const blur = onBlurChange ? initialBlur : internalBlur;
  const cardOpacity = onCardOpacityChange ? initialCardOpacity : internalOpacity;

  const handleAccentChange = (col: string) => {
    setInternalAccent(col);
    onAccentColorChange?.(col);
  };
  const handleBgChange = (col: string) => {
    setInternalBg(col);
    onBgColorChange?.(col);
  };
  const handleBlurChange = (val: number) => {
    setInternalBlur(val);
    onBlurChange?.(val);
  };
  const handleOpacityChange = (val: number) => {
    setInternalOpacity(val);
    onCardOpacityChange?.(val);
  };
  const handleReset = () => {
    setInternalAccent("#2f6bff");
    setInternalBg("#fafafa");
    setInternalBlur(18);
    setInternalOpacity(0.72);
    onAccentColorChange?.("#2f6bff");
    onBgColorChange?.("#fafafa");
    onBlurChange?.(18);
    onCardOpacityChange?.(0.72);
    onResetPalette?.();
  };

  const [toast, setToast] = useState<string | null>(null);
  const toastTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (toastTimerRef.current !== null) {
        clearTimeout(toastTimerRef.current);
      }
    };
  }, []);

  const accentSwatches = [
    "#2f6bff",
    "#7c5cff",
    "#0ea5a4",
    "#12994c",
    "#f59e0b",
    "#ec4899",
    "#111827",
  ];

  const bgSwatches = [
    "#f2f5fa",
    "#e0eaff",
    "#fce7f3",
    "#dcfce7",
    "#fef3c7",
    "#0a0b0e",
    "#1e1b4b",
  ];

  function showNotification(msg: string) {
    if (toastTimerRef.current !== null) {
      clearTimeout(toastTimerRef.current);
    }
    setToast(msg);
    toastTimerRef.current = setTimeout(() => {
      toastTimerRef.current = null;
      setToast(null);
    }, 2500);
  }

  function handleExportConfig() {
    try {
      if (
        typeof URL === "undefined" ||
        typeof URL.createObjectURL !== "function" ||
        typeof document === "undefined"
      ) {
        throw new Error("当前环境不支持浏览器 Blob 下载接口");
      }
      const config = {
        theme,
        accentColor,
        bgColor,
        blur,
        cardOpacity,
        system_preferences_note: "仅包含当前页面主题与调色板内存状态，系统级配置尚未接入",
        exportedAt: new Date().toISOString(),
      };
      const blob = new Blob([JSON.stringify(config, null, 2)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "proxyweave-theme-config.json";
      a.click();
      URL.revokeObjectURL(url);
      showNotification("已发起主题与调色板配置 JSON 下载（未验证本地落盘）");
    } catch (e: any) {
      showNotification(`导出配置失败: ${typeof e === "string" ? e : e?.message || String(e)}`);
    }
  }

  return (
    <section className="panel" id="panel-settings">
      <div className="topbar">
        <div>
          <h2>系统设置</h2>
        </div>
      </div>

      {toast ? (
        <div
          id="settingsToast"
          role="status"
          style={{
            marginBottom: 16,
            padding: "8px 14px",
            borderRadius: 8,
            fontSize: 13,
            background: "rgba(47, 107, 255, 0.12)",
            color: "var(--blue)",
            border: "1px solid rgba(47, 107, 255, 0.3)",
            wordBreak: "break-word",
            lineHeight: 1.6,
          }}
        >
          {toast}
        </div>
      ) : null}

      <div className="stack">
        {/* 1. 主题卡片 */}
        <div className="card">
          <h3>主题</h3>
          <div className="grid3" style={{ marginTop: 12 }}>
            <button
              className={`card theme-card ${theme === "sky" ? "active" : ""}`}
              data-theme-set="sky"
              type="button"
              onClick={() => onThemeChange("sky")}
              style={{
                boxShadow: "none",
                textAlign: "left",
                cursor: "pointer",
                padding: "16px 18px",
                borderColor: theme === "sky" ? "var(--blue)" : undefined,
                background: theme === "sky" ? "rgba(47, 107, 255, 0.06)" : undefined,
              }}
            >
              <b style={{ fontSize: 15 }}>晴空蓝白</b>
              <div style={{ display: "flex", gap: 6, margin: "10px 0 0" }}>
                <i
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 7,
                    background: "#2f6bff",
                    display: "block",
                  }}
                />
                <i
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 7,
                    background: "#e9f1ff",
                    display: "block",
                  }}
                />
                <i
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 7,
                    background: "#ffffff",
                    border: "1px solid var(--line)",
                    display: "block",
                  }}
                />
              </div>
            </button>

            <button
              className={`card theme-card ${theme === "glass" ? "active" : ""}`}
              data-theme-set="glass"
              type="button"
              onClick={() => onThemeChange("glass")}
              style={{
                boxShadow: "none",
                textAlign: "left",
                cursor: "pointer",
                padding: "16px 18px",
                borderColor: theme === "glass" ? "var(--blue)" : undefined,
                background: theme === "glass" ? "rgba(47, 107, 255, 0.06)" : undefined,
              }}
            >
              <b style={{ fontSize: 15 }}>玻璃拟态</b>
              <div style={{ display: "flex", gap: 6, margin: "10px 0 0" }}>
                <i
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 7,
                    background: "linear-gradient(135deg,#ffffff,#e4e4e7)",
                    border: "1px solid var(--line)",
                    display: "block",
                  }}
                />
                <i
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 7,
                    background: "linear-gradient(135deg,#f4f4f5,#d4d4d8)",
                    display: "block",
                  }}
                />
                <i
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 7,
                    background: "rgba(255,255,255,.55)",
                    border: "1px solid var(--line)",
                    display: "block",
                  }}
                />
              </div>
            </button>

            <button
              className={`card theme-card ${theme === "dark" ? "active" : ""}`}
              data-theme-set="dark"
              type="button"
              onClick={() => onThemeChange("dark")}
              style={{
                boxShadow: "none",
                textAlign: "left",
                cursor: "pointer",
                padding: "16px 18px",
                borderColor: theme === "dark" ? "var(--blue)" : undefined,
                background: theme === "dark" ? "rgba(47, 107, 255, 0.06)" : undefined,
              }}
            >
              <b style={{ fontSize: 15 }}>深空暗色</b>
              <div style={{ display: "flex", gap: 6, margin: "10px 0 0" }}>
                <i
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 7,
                    background: "#0e1014",
                    display: "block",
                  }}
                />
                <i
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 7,
                    background: "#23262f",
                    display: "block",
                  }}
                />
                <i
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 7,
                    background: "#6f9bff",
                    display: "block",
                  }}
                />
              </div>
            </button>
          </div>

          {/* 自定义调色板 */}
          <div className="palette-wide" id="paletteSettings" style={{ marginTop: 16 }}>
            <p className="palette-title" style={{ fontSize: 13 }}>
              自定义调色 · 实时生效{" "}
              <button type="button" onClick={handleReset} data-reset-palette>
                恢复默认
              </button>
            </p>
            <div className="grid2" style={{ gap: 16, marginTop: 10 }}>
              <div>
                <label style={{ fontSize: 12, fontWeight: 500, color: "var(--text2)" }}>
                  主色
                </label>
                <div
                  className="swatch-row"
                  data-swatches="accent"
                  style={{ marginTop: 8 }}
                >
                  {accentSwatches.map((col) => (
                    <button
                      key={col}
                      className={`swatch ${accentColor.toLowerCase() === col.toLowerCase() ? "active" : ""}`}
                      data-color={col}
                      type="button"
                      style={{ background: col }}
                      onClick={() => handleAccentChange(col)}
                    />
                  ))}
                  <input
                    type="color"
                    className="color-pick"
                    data-pick="accent"
                    value={accentColor}
                    onChange={(e) => handleAccentChange(e.target.value)}
                  />
                </div>
              </div>
              <div>
                <label style={{ fontSize: 12, fontWeight: 500, color: "var(--text2)" }}>
                  背景色
                </label>
                <div className="swatch-row" data-swatches="bg" style={{ marginTop: 8 }}>
                  {bgSwatches.map((col) => (
                    <button
                      key={col}
                      className={`swatch ${bgColor.toLowerCase() === col.toLowerCase() ? "active" : ""}`}
                      data-color={col}
                      type="button"
                      style={{ background: col }}
                      onClick={() => handleBgChange(col)}
                    />
                  ))}
                  <input
                    type="color"
                    className="color-pick"
                    data-pick="bg"
                    value={bgColor}
                    onChange={(e) => handleBgChange(e.target.value)}
                  />
                </div>
              </div>
            </div>

            {/* 玻璃专属滑块 */}
            {theme === "glass" ? (
              <div className="glass-only" style={{ marginTop: 14 }}>
                <div className="grid2" style={{ gap: 16 }}>
                  <div>
                    <label style={{ fontSize: 12, fontWeight: 500, color: "var(--text2)" }}>
                      磨砂强度 <span data-blur-val>{blur}px</span>
                    </label>
                    <input
                      type="range"
                      min="0"
                      max="32"
                      value={blur}
                      onChange={(e) => handleBlurChange(Number(e.target.value))}
                      data-range="blur"
                      style={{ width: "100%", accentColor: "var(--blue)", marginTop: 6 }}
                    />
                  </div>
                  <div>
                    <label style={{ fontSize: 12, fontWeight: 500, color: "var(--text2)" }}>
                      卡片透明度 <span data-opacity-val>{cardOpacity}%</span>
                    </label>
                    <input
                      type="range"
                      min="20"
                      max="90"
                      value={cardOpacity}
                      onChange={(e) => handleOpacityChange(Number(e.target.value))}
                      data-range="opacity"
                      style={{ width: "100%", accentColor: "var(--blue)", marginTop: 6 }}
                    />
                  </div>
                </div>
              </div>
            ) : null}

            <p className="sub" style={{ marginTop: 10 }}>
              当前背景预览即整站背景 · 玻璃主题下背景色驱动 aurora 渐变
            </p>
          </div>
        </div>

        {/* 2. 通用设置 */}
        <div className="card">
          <h3>通用</h3>
          <div className="listrow">
            <div style={{ flex: 1 }}>
              <b style={{ fontSize: 14.5 }}>开机自启</b>
              <p className="sub" style={{ marginTop: 4, lineHeight: 1.5 }}>
                系统级自启状态未知；后端暂未提供自启动读取与设置 IPC
              </p>
            </div>
            <button
              type="button"
              className="pill pill-gray"
              disabled={true}
              style={{ cursor: "not-allowed", border: "none" }}
              title="后端暂未提供 autostart 读取与修改 IPC"
            >
              未实现
            </button>
          </div>
          <div className="listrow">
            <div style={{ flex: 1 }}>
              <b style={{ fontSize: 14.5 }}>隐藏到托盘</b>
              <p className="sub" style={{ marginTop: 4, lineHeight: 1.5 }}>
                全局关闭到托盘偏好由桌面主窗口外壳 (AppShell/Phase0) 管理，当前面板未直连托盘偏好持久化接口
              </p>
            </div>
            <button
              type="button"
              className="pill pill-gray"
              disabled={true}
              style={{ cursor: "not-allowed", border: "none" }}
              title="当前面板未直连托盘配置持久化 IPC，请参考主窗口关闭与系统托盘默认行为"
            >
              面板未直连
            </button>
          </div>
          <div className="listrow">
            <div style={{ flex: 1 }}>
              <b style={{ fontSize: 14.5 }}>界面语言</b>
              <p className="sub" style={{ marginTop: 4, lineHeight: 1.5 }}>
                多语言国际化运行时尚未接入，当前固定为简体中文界面
              </p>
            </div>
            <select
              className="input"
              value="zh-CN"
              disabled={true}
              style={{ minWidth: 120, cursor: "not-allowed" }}
              title="多语言国际化运行时尚未实现"
            >
              <option value="zh-CN">简体中文 (固定)</option>
            </select>
          </div>
        </div>

        {/* 3. 凭据 · Keyring */}
        <div className="card">
          <h3>凭据 · Keyring</h3>
          <p className="sub" style={{ lineHeight: 1.6 }}>
            系统级安全保管箱（操作系统凭据库支持，敏感插件运行时代理由受限工作区投影提供）。未做主动探测时不声明就绪或已校验。
          </p>
          <div className="keybox" style={{ marginTop: 10 }}>
            <code>
              keyring://proxyweave/master-key (凭据状态未知 · 未探测) · keyring://proxyweave/wb2api/upstream (凭据状态未知 · 未探测)
            </code>
          </div>
          <p className="sub" style={{ marginTop: 8, color: "var(--muted)" }}>
            说明：当前设置页未主动读取或校验系统凭据库，真实网关密钥请在概览/网关页查看受控预览，插件上游密钥在插件管理页安全更新。
          </p>
        </div>

        {/* 4. 数据路径 */}
        <div className="card">
          <h3>数据</h3>
          <div className="listrow">
            <div style={{ flex: 1 }}>
              <b style={{ fontSize: 14.5 }}>配置文件存储</b>
              <p className="sub mono" style={{ marginTop: 4 }}>
                ~/.proxyweave/desktop_config.json（后端默认存储路径）
              </p>
              <p className="sub" style={{ marginTop: 2, color: "var(--muted)" }}>
                系统原生文件管理器打开目录接口尚未暴露至前端
              </p>
            </div>
            <button
              className="btn btn-ghost btn-sm"
              type="button"
              disabled={true}
              title="后端暂未提供 open_path / reveal_in_file_manager 接口"
            >
              打开目录 (未实现)
            </button>
          </div>
          <div className="listrow">
            <div style={{ flex: 1 }}>
              <b style={{ fontSize: 14.5 }}>配置文件备份</b>
              <p className="sub" style={{ marginTop: 4 }}>
                后端原子写入机制具备写入前容灾，但独立备份健康度巡检接口尚未实现
              </p>
            </div>
            <span className="pill pill-gray">未巡检</span>
          </div>
          <div className="actions" style={{ marginTop: 12 }}>
            <button
              className="btn btn-soft btn-sm copy-btn"
              type="button"
              onClick={handleExportConfig}
            >
              导出主题配置
            </button>
            <button
              className="btn btn-ghost btn-sm"
              type="button"
              disabled={true}
              title="外部配置文件导入与解析接口尚未实现"
            >
              导入… (未实现)
            </button>
            <button
              className="btn btn-ghost btn-sm"
              type="button"
              style={{ color: "var(--red)" }}
              onClick={() => {
                handleReset();
                showNotification("主题与调色板内存设置已恢复默认");
              }}
            >
              重置调色板
            </button>
          </div>
        </div>

        {/* 5. 关于 */}
        <div className="card">
          <h3>关于</h3>
          <p className="sub" style={{ lineHeight: 1.8 }}>
            <b style={{ color: "var(--text)" }}>ProxyWeave</b> · v0.1.0-dev · Tauri v2 +
            Rust ·{" "}
            <a
              href="https://github.com/Cary222/ProxyWeave"
              target="_blank"
              rel="noopener noreferrer"
              style={{ color: "var(--blue)", textDecoration: "underline" }}
            >
              GitHub
            </a>{" "}
            · 插件应用平台
          </p>
        </div>
      </div>
    </section>
  );
};
