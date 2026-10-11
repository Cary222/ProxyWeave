# Feature 4：极简桌面宿主、IPC 状态管理与 UI 闭环

状态：开发验证已通过，冻结送审（ReadyForReview），等待 Inspector 独立验收。F1/F3 已独立 Verified，F2 六个具体 DEF 已 Verified。真实插件端到端模型推理与原生桌面 MCP 操作验收仍开放。

## 实施范围

- `apps/desktop/src-tauri/src/host.rs`：Tauri v2 IPC 状态管理与命令胶水层。
  - 管理 `DesktopState`：持有 `Registry`、`RuntimeManager`、`gateway_handle`（`tokio::sync::Mutex<Option<GatewayHandle>>`）、`gateway_master_key`（`tokio::sync::RwLock<String>`）、`gateway_port`（`tokio::sync::RwLock<u16>`）、`events`（`tokio::sync::Mutex<VecDeque<HostEvent>>`，上限 500 条）与 `desktop_config`（原子写入持久化）。
  - 细粒度异步锁：废除跨 await 持有的独占锁，采用 CAS (`gateway_starting: Arc<AtomicBool>`) 防并发启动，短临界区读写锁保证命令响应及时。
  - 本地 Master Key 与上游 Plugin API Key 物理分离：Master Key 仅用于客户端访问本地网关（格式 `pw-sk-...`），插件凭据由各插件独立配置。
  - 密码框零回读与安全清空：`set_plugin_credential` 直接写入系统安全 Keyring；IPC 不提供任何密码读取命令；前端在保存成功后立即清空内存与输入框。
  - 宿主生命周期事件白名单：记录网关启动/停止、插件启停、配置与凭据操作等结构化事件，绝不声明原始 stdout/stderr 安全。
  - 退出清理：`shutdown_desktop` 优雅关闭网关（取消活跃流）并停止所有 Managed 插件。
  - 测试调用：`test_gateway_models` 命令通过回环网关带本地 Master Key 执行 GET `/v1/models`，提供 UI 测试调用验证能力。
- `apps/desktop/src/`：极简暗色工程仪表盘（React + TypeScript + CSS）。
  - `components/AppShell.tsx`：工业风暗色导航，包含“网关”、“插件”、“事件”、“关于”四大功能面板。
  - `components/GatewayPanel.tsx`：网关状态卡片、端口配置、Master Key 复制、一键启停、测试调用控制台（展示响应状态与模型列表）。
  - `components/PluginPanel.tsx`：本地已登记插件列表、状态标签（Stopped/Starting/Ready/Busy/Failed）、PID 与端口显示、一键启停、密码凭据配置弹窗（密码保存后立即清空）。
  - `components/EventsPanel.tsx`：宿主生命周期事件流实时滚动查看。
  - `components/dashboard.css`：暗色主题样式。
- 自动化测试：
  - Rust 集成测试：`apps/desktop/src-tauri/tests/desktop_host.rs`（配置持久化与回滚恢复、密码写入 Keyring、事件队列上限与裁剪）。
  - 前端单元测试：`apps/desktop/src/shell.test.tsx`（主窗口渲染与导航切换验证）。

来源比较与设计决策见 [ADR-005](../architecture/ADR-005-minimal-desktop-host.md)。

## 开发验证证据

- 全量构建与测试日志：`docs/verification/feature4-full-pass.txt`
  - SHA256：`9bcdf13212d924cc56d50f65982b8114a1085e33779816b879b576a875730ad7`
- 交付可执行文件：`D:/temp-audit/proxyweave-feature4-build/quotio-desktop.exe`
  - 文件大小：24,676,352 字节
  - SHA256：`c2c99d1beba40564a5c19a1eb39f718d0c79502f59f0abde91fc6705ad7b61a7`
- 验证门禁执行清单：
  1. `cargo fmt --all -- --check`：通过（exit 0，代码格式完全一致）。
  2. `cargo clippy -p quotio-desktop -p proxyweave-gateway -p proxyweave-plugin -p quotio-platform --all-targets --no-deps --offline -- -D warnings`：通过（exit 0，0 警告，0 错误）。
  3. `cargo test --workspace --offline`：通过（exit 0，所有 workspace 测试全部通过）。
  4. `npm test`（`apps/desktop`）：通过（exit 0，Vitest 2 项测试通过）。
  5. `npm run build`（`apps/desktop`）：通过（exit 0，TypeScript 检查及 Vite 生产包构建完成）。
  6. `npx @tauri-apps/cli build --debug --no-bundle`（`apps/desktop`）：通过（exit 0，生成嵌入式桌面二进制）。

## 诚实边界与限制

1. **真实凭据与模型推理门禁**：本地测试使用的是模拟测试与回环调用；真实的 2API 插件（如 `wb2api`）端到端真实凭据运行及模型推理，必须在后续由审查员在原生桌面环境中实机验收。
2. **原生桌面 MCP 操作门禁**：最终验收必须通过原生桌面 MCP 操作桌面 UI 确认“导入包 -> 配置凭据 -> 启动插件 -> 启动网关 -> 测试调用模型 -> 停止插件 -> 重启验证”完整闭环，脚本/API 调用不能充作替代证据。
3. **平台范围**：本轮仅完成 Windows x86_64 平台实现与验证；macOS 及 Linux 平台实现待后续阶段开展。
