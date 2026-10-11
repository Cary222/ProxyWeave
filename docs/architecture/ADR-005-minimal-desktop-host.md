# ADR-005 — 最小 Windows 桌面接入

状态：实施中，未独立验收。F3 冻结 `60bc51566d2a7d6ec5fe4b1f323f86135c80f68a` 不修改。

## 直接复用与边界

- 保留 Quotio `21d75d08b38a23f97fbd534a1768a829cb147f2c` MIT 窗口/托盘/单实例壳，不恢复 AppCore、bridge 或 provider-specific 业务。读取真实 `apps/desktop/src-tauri/src/lib.rs` 的 credential_status、异步命令/阻塞隔离及 SettingsScreen 密码输入；其 AppCore 依赖不适合模块提取，本轮使用已提取的 platform Keyring 和官方 Tauri State/async command/spawn_blocking，而非重新实现存储/进程/反代。
- Registry、RuntimeManager、Gateway 原样调用，生命周期和流式行为的独立验收仍属于 F2/F3。官方 Tauri 2 state-management/calling-rust 文档支持 async Mutex 与 State，文件/Keyring 操作放 blocking worker。
- 系统随机数直接依赖已有锁版本 getrandom 0.3.4（MIT OR Apache-2.0），读取其 `fill` 及 Windows BCryptGenRandom backend；不自制 PRNG。
- 本地路径输入选择 Manifest，重用 F1 校验/复制，不自动扫描或编译源码。导入前明确第三方插件具有当前用户权限、不是安全沙箱，需确认。

## 持久化与生命周期

- 宿主只持久化选中插件 ID 和本地网关端口；Keyring 存 master key 与每插件独立 upstream key。插件必需字段只接受已登记 required_secrets，密码逐字段保存、删除，不回填原值。记录包含存在性而非 secret。
- 所有桌面写操作通过一个 async 操作互斥锁，防止 start/stop/restart/config/gateway-switch 交错；状态查询不占该锁。继续复用 runtime 的每插件 FIFO 锁与 RequestLease。密码修改要求 Stopped；Failed 先停止，避免更改仍在运行的投影实例。
- 默认不自动启动任何插件或网关。重开后登记与非敏感网关设置仍存在；用户显式开启。Gateway 仅 loopback，端口可设 0 分配，实际地址显示；绑定失败保持错误，不宣称开启成功。
- 一个统一入口本轮仅选一个 backend，不增加 router/market。切换需无 active request；显式停止/重启保留 F3 的 active-stop 拒绝行为。
- 退出先关闭并 drain gateway 请求，再停止宿主拥有的 Managed 实例；External 不调用 stop，绝无 kill。托盘退出、主界面退出和原生退出统一走该序列。异常退出仍靠已分配 Win32 Job，不夸大 spawn→assign 窗口保证。

## 日志与调用入口

- 日志为有界白名单宿主事件（操作名称、已登记 ID、成功/失败、时间），不接收原始错误、凭据、argv/env、响应、UID、插件 stdout/stderr。它不是完整 API Doctor，也不宣称原插件原始日志已经脱敏。
- 提供实际统一入口、显式查看本地 API Key（只临时内存显示、不自动读取、不写 localStorage/剪贴板），以及明确的 PowerShell 模型发现/非流式/SSE 命令模板。用户请求内容与模型选择在客户端，不合并 wb2api 业务代码。
- 本轮无需 WebView connect-src 放宽，无浏览器直连 provider，无命令注入/任意 shell API。

## 验证

新增 host 单元/真实 Managed 集成测试及 UI 静态/状态测试；保留既有壳行为测试并更新过期“没有 provider”文案。完整 fmt、作用域 Clippy、workspace tests、前端 tests/build 和 embedded Tauri 构建。Desktop native MCP 最终点击导入/配置/启停/重启/重开以及真实模型凭据调用必须另验；fixture 不替代。
