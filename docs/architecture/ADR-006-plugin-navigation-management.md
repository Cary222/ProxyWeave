# ADR-006：逐插件导航、能力声明与安全管理链接（UI-001）

状态：用户授权的独立 UI 修正；实施交付不等于 Inspector Verified。不恢复暂停的完整 Managed Goal，不授权真实凭据联调或最终全项验收。

## 直接复用与隔离决策

复用既有 Quotio 衍生 React sidebar/PluginPanel、原生 HTML button、现有 url 2.5.8。新增唯一直接依赖 `tauri-plugin-opener = 2.5.4`，MIT OR Apache-2.0，发布源码 commit `5c7668b6bb7c9a509f394d584568b3a922161e50`。只调用 Rust `open_url(url, None::<&str>)`；不注册 JS opener 插件、不授予 opener/远程/file/process capability，不构造 shell 命令。

官方 Tauri capabilities/security/Webview/opener 文档与实际锁定源码均核查：Tauri 2.11.2 将 initialization script 标为 main-frame-only；Wry 0.55.1 `src/lib.rs` 明确 Windows 子帧忽略该标志，`src/webview2/mod.rs` 使用 AddScriptToExecuteOnDocumentCreated。因此不能仅凭 iframe sandbox/无 remote capability 宣称第三方没有注入 invoke surface。本轮停止 iframe/childWebview 路线，仅用户明确点击后由系统浏览器打开插件原页。没有加载远程页面、postMessage 桥、HTML 代理或自动 fallback；不修改 CSP、frame-ancestors、X-Frame-Options 或 `capabilities/default.json`。这不是声称所有未来隔离设计均不可行。

原 opener `src/windows_shell_path.rs` 路径 tests 与 open 5.4.1 Windows tests 已审阅，不能替代本产品 URL 策略、真实 ShellExecute 与原生 UI 验收。来源详细路径、许可证、测试与网络/工具故障记录见 developer 交付 `.pi/tasks/ui-001-developer/`；权威来源登记见 REFERENCE_REGISTRY。

## Schema 1.0 可选扩展

```yaml
management:
  path: /admin
  presentation: browser
```

以上只是通用示例，不是任何 pluginID 的默认入口。只有 `browser`，必须严格对象；未声明不猜测 root/端口/业务路由，不改 `manifests/*.yaml` 或已导入记录。路径 1..256 ASCII 字节，必须 origin-relative `/...`；允许字母数字、`/ - _ .`，禁止 `//`、空内部段、`.`/`..` 段、绝对 URL、百分号、反斜线、控制/空白、userinfo、query/fragment、模板、HTML/引号等。只能公开静态路径，不能把 token 放在路径；宿主不读凭据、不做模板替换，不探测网页。

解析使用 `url`，最终只允许 HTTP(S)、无用户名/密码/query/fragment、与当前 origin 完全相同。禁止 Tauri 保留 `.localhost` 应用域及当前开发宿主端口 origin。Managed 必须 freshly reconciled Ready/Busy 运行时端口，停止/启动中/失败不提供 URL。External 只从登记 origin，链接不证明在线且不会启动/停止/Kill。系统浏览器后续导航/插件登录属于插件与浏览器，不声称限制其所有重定向。

`get_plugin_management(id)` 返回当前 descriptor；`open_plugin_management(id)` 不接受 JS URL/端口/程序名，点击时重新读登记、验证并取得当前 runtime，错误不回显输入。仅本地 main webview caller 允许；debug 精确 dev origin，production 精确应用 origin，不能把 native label guard 当远程权限。宿主此路径不加载第三方内容，因此无新远程 invoke 权限。随后进程退出或浏览器未加载是固有竞态，不宣称加载成功/自动重试。

## 旧契约与指纹

Manifest 末尾 `management: Option<Management>` 使用 `default, skip_serializing_if = "Option::is_none"`；原字段顺序/默认不变，不升 schema_version。旧 JSON 序列化及 SHA256 字节一致，旧 record 可直接 load，不自动 migration；显式 null 序列化亦省略。新增/改变字段改变 fingerprint，已有同 ID 不同内容按原策略拒绝，同内容幂等；本轮没有覆盖/升级机制。

## 导航与能力真实性

展开插件管理显示总览/导入与每个登记 pluginID 子按钮（名称/ID/状态）；不以 ID 硬编码业务。选中页过滤既有 PluginPanel，保留凭据配置、Managed 启停/重启、网关挂载。新增/重复导入不抢选中，选中项消失回总览并播报；折叠不改变选中，原生 Tab/Enter/Space 与 aria-expanded/aria-controls/aria-current、可见 focus。没有伪造 ARIA tab keyboard contract。

能力只取 Manifest protocol/models_path/inference_path/streaming，显式「声明（不是实测）」；Ready/Busy 不等于模型/推理/streaming Verified。现有宿主没有每插件持久能力实测记录，显示「能力尚未实测」，不生成模型列表、不为 UI 新发探测/推理。管理入口缺失/未就绪/校验失败如实提示；可用时始终提供手动系统浏览器按钮；不自动打开/嵌入。

## 质量门与仍开放边界

契约/path/origin/动态端口/旧指纹/导入冲突 Rust 回归，现有 SSR 与导航/details SSR/state 测试，TypeScript/Vite 与原生 embedded build 各自记录，不互相替代。禁止操作用户人工确认窗口或现有进程。Inspector 从冻结 SHA256/diff 副本独立验证原生键盘/focus/网页打开/端口重启；开发构建通过不代表上述行为 Verified。真实账号、模型调用、全 F2/最终八项仍开放且不在 UI-001 范围。
