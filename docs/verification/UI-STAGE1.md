# 阶段 1 UI：原型映射与前端交付

## 授权与状态

- **开发交付候选，不是 Inspector Verified，也不是原生桌面验收通过。**
- 当前工作基线：`8cb7f2085c05bd67fb287de4922213fdbff640ff`，另有既存 UI-001 dirty 资产；先保存基线文件及 SHA256，未 pull/rebase、commit/push。
- 原型：`d92e6a0d672f586594dcae401ba03b5246bbb530:docs/design/proxyweave.html`，原始 SHA256 `60640d62f5600ae8561c5077c1fcd8c4b4a9c9b5214de45c3ba3b250e4b6ee7a`。
- 原型 CSS tokens、布局、线性 SVG 与调色计算直接提取并薄适配既有 React 窗口；沿用现有面板与 IPC，不重搭 Tauri 壳、不修改宿主运行时/网关/Manifest/Keyring/CSP/能力权限。
- 旧 Managed Goal `mv44jirh-kepwhn` 保持暂停；当前 UI 工作不恢复、不完成它。IDE 前置源码审计只读完成，不授权 IDE backend 实施。

## 用户闭环与实际范围

| 区域 | 当前真实功能 / 显示边界 |
| --- | --- |
| 主题 | 晴空 / 深空 / 玻璃；各主题独立会话配色；主色、背景色；玻璃模糊 0..32、不透明度 20..90；恢复当前主题默认，不删除其他主题草稿；不承诺跨重启持久化 |
| 系统概览 | 使用真实登记与已成功读取的运行状态；没有数据时显示未知，不生成 PID、流量、模型数量、38ms 等演示指标 |
| 统一网关 | 保留既有端口、启动/停止、Master Key 与真实 `/v1/models` 自测入口；未知状态禁用启动和端口操作；移除“永不落盘”的无证据绝对文案 |
| 插件 | 保留本地 Manifest 导入、生命周期、网关挂载与凭据操作；任意 ID 逐插件导航；卡片与详情；声明能力不冒充实测；未知状态不显示 STOPPED，未知凭据状态不显示未配置 |
| 管理界面 | 沿用 UI-001 的明确管理入口；不添加 iframe、远程 IPC/CSP 特权，不将浏览器入口等同安全隔离嵌入；安全嵌入仍为长期首选 |
| 审计 | 现有宿主事件；未读取与已读取空列表分别显示；不宣称覆盖原始插件日志脱敏 |
| IDE | 明确待实施，仅说明当前 OpenAI Chat Completions 协议及候选；不读写用户客户端配置，不假称支持 Messages/Responses |
| 发现 / 试飞 | 市场、自动下载/构建、完整对话试飞、路由映射、Doctor 未实现；仅跳转现有本地导入 / 网关真实模型测试，不提供假成功按钮 |
| 窗口 | 沿用已有关闭/最小化/托盘命令与偏好；关闭对话框补充键盘焦点圈定、Escape 关闭及焦点归还；没有改后端退出/单实例实现 |

## 文件与隔离

本次 UI 增量位于 `apps/desktop/src/`：`components/AppShell.tsx`、`Appearance.tsx`、`Icon.tsx`、`Overview.tsx`、`prototype.css`、`legacy-shell.css`、既有 `GatewayPanel.tsx` / `PluginPanel.tsx` / `PluginDetails.tsx` / `PluginNavigation.tsx` / `EventsPanel.tsx`、`MenuBarPanel.tsx` 与 `shell.test.tsx`。保留 UI-001 的既有测试与业务资产。`types.ts` 未继续修改；没有新增依赖。

私有交付目录：`D:/ProxyWeave/.pi/development/ui-redesign/`。

- `baseline/manifest.json`、`baseline/tracked.patch`、`baseline/files/`：实施前已存在的 21 个 dirty 资产。
- `interactions.cjs`：明确标注浏览器 IPC dummy fixture 的可重放交互，不读取真实账号或凭据。
- `detail-{360,768,1366,1920}.png`、`settings-{sky,dark,glass}.png`：显式 UI 夹具截图，不能用作真实插件证据。
- `frontend-verification.txt`：最终前端命令的 stdout / stderr 和退出码。
- `freeze/`：冻结内容、相对 HEAD 的 patch、逐文件 SHA256 与基线差分分类；仅开发资产，排除 Inspector 台账和 `.pi/inspection/`。

## 开发验证与失败记录

浏览器开发层最终通过 **30 项**（含三主题选中导航 hover 配色）检查：无 IPC 时状态未知且禁用启动、各主题独立自定义与重置、玻璃边界、IDE/审计诚实占位、任意 ID 详情、没有 iframe、360/768/1366/1920 详情不横向溢出、窄窗凭据可读与输入宽度、主题 aria-pressed、关闭对话框/Escape/焦点归还、导入拒绝显示失败、单插件状态失败禁用启动、窄窗导航可达与选择后收起。

测试进程中的失败均不隐藏：

1. MCP unsafe 代码环境无 `require`，改用该工具支持的 `filename`；旧 5194 服务已退出，重新启动仅本次拥有的 Vite 服务。
2. 导入脚本使用泛化 textbox / 模糊导航名称产生 strict-mode 歧义；为 Manifest 路径和凭据输入增加语义 label，并将测试导航定位限定为真实主导航。
3. 导入拒绝实际文案包含 `Error:`，初始脚本遗漏此前缀且未等待异步完成；修正脚本，不修改生产错误为假成功。
4. `getByText('未知').first()` 命中隐藏逐插件导航项；删除此无效测试步骤，仍保留可见详情 UNKNOWN 和启动禁用断言。
5. 初始窄窗截图发现凭据名称逐字竖排。将窄窗凭据行改为纵向块布局、名称横排、输入独占行；复验名称高度 <30px、输入宽度 >120px。最初名称宽度断言 >40px 不符合 5 字符实际宽度 37.57px，调整为 >30px 且同时检查单行高度；不以文档未溢出代替可用性。
6. 选中导航的 hover 规则曾覆盖主题高亮；限定 hover 为未选中导航。精确 RGB 复验最初撞上 CSS transition 中间值，改为等待最终 computed color，再对三主题各断言原型 token 对应颜色。

最终 `npm test`（Vitest）和 `npm run build`（tsc / Vite）的结果以冻结命令 receipt 为准。没有独立前端 lint 脚本；不能把 tsc 当作未运行的 lint 或 Rust 验收。

## 尚未验证 / 交接门禁

- 原生 Windows ProxyWeave：冻结副本重建、MCP 三主题/导航/导入详情与实际窗口/托盘/单实例回归仍待 Inspector。
- 当前浏览器测试没有真实插件执行、真实凭据、真实模型调用；不是旧 Managed 全闭环通过。
- 该 UI lane 未重新构建 Tauri executable，也没有修改后端安全边界；已有 native 行为“保留源码”不等于“独立原生回归已通过”。
- 所有内容 SHA256 冻结后再交付 Main，由独立冻结副本复验。没有 commit/push 或旧 Goal 状态变更。
