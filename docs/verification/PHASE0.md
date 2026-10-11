# Phase 0 Windows 最小桌面基座验证

## 结论与范围

Phase 0 已通过本轮限定的 Windows 最小桌面基座门禁：原基线与目标构建/测试、窗口、托盘、关闭及单实例均有真实证据；格式/锁文件清理后的最终回归退出 0。不是完整网关、插件或跨平台发布验收。Phase 1 尚未启动。

## 真实基线与保护措施

- 主仓库起始 HEAD：`ffe4f656c1d1999eec7fd3c491c3bf632de7e223`，起始工作区干净；保留主工程既有文档，不替换 Git 历史。
- 参考仓库 `D:/temp-audit/quotio-desktop`：Git 实读完整 HEAD `21d75d08b38a23f97fbd534a1768a829cb147f2c`，标签 `v0.7.7`，原参考源码始终未修改，最后复查工作区干净。
- MIT：`Copyright (c) 2025 Trong Nguyen`；完整文本保留于 `docs/references/licenses/Quotio-MIT.txt`。
- 原工程验证副本 `D:/temp-audit/proxyweave-phase0-upstream` 来自 452 个 tracked 文件。不复制原 Git、用户数据/缓存。
- 原 AppCore 启动存在配置/Keyring/数据库/Codex 恢复副作用。所有原工程 tests/runtime 使用隔离 HOME/USERPROFILE/APPDATA/LOCALAPPDATA：`D:/temp-audit/proxyweave-phase0-user/{home,roaming,local}`；仅副本的 Keyring service 改为 `proxyweave-phase0-upstream-validation`。这是明确记录的隔离适配，不宣称字节不变的原运行。
- Rust/Cargo 使用 `D:/build-tools`，Windows 构建设 `CARGO_BUILD_JOBS=1`。本轮缓存产物使用 `CARGO_TARGET_DIR=D:/temp-audit/proxyweave-phase0-upstream/target`；默认用户构建产物仍位于仓库 `target/`。

## 分步引入与模块溯源

先验证原工程，再原样引入可构建闭包，然后单独做标识调整、桌面业务裁剪、平台裁剪和回归，未同时大规模重命名。原样引入 238 文件的历史 SHA-256 清单：[phase0-introduction.json](phase0-introduction.json)。它不是裁剪后文件哈希清单。

| 原路径 → 目标路径 | 直接复用/必要适配 |
|---|---|
| Quotio `apps/desktop/src-tauri/src/{lib,main}.rs` → 同路径 | 机械提取原窗口命令、首注册 single-instance、托盘菜单/图标/面板定位/Mica；去掉 AppCore 与业务命令/后台任务/更新器；新增 native CloseRequested 隐藏胶水；托盘菜单恢复调用原 show_main_window（包含 unminimize）。 |
| `apps/desktop/src/main.tsx`、`components/AppShell.tsx`、相关 CSS → 同路径 | 保留原启动揭示计时器、窗口按钮、关闭/最小化函数与对话框；偏好改为 ProxyWeave 命名空间；基座说明页和无业务小面板是薄 UI 胶水。 |
| `crates/quotio-platform/src/lib.rs` → 同路径 | 保留通用路径/命令/原子写入/备份/Keyring 与适用原测试；配置/凭据命名空间独立；依调用闭包裁去 CPA/Kiro/Dream-Skin 资源发现及专用常量。Windows sensitive 权限原逻辑仍是 no-op，不能当作 ACL 隔离。 |
| `crates/quotio-types` → 同路径 | 暂保留原类型和 7 个测试，不凭名称删枚举；部分类型尚属后续裁剪候选。 |
| Cargo/npm/Tauri/Vite/TS 配置及图标 → 同路径 | 保留内部 quotio crate/exe 名称；展示 ProxyWeave、identifier `com.proxyweave.desktop`；去掉业务资源包/业务依赖与更新配置；保留上游图标资产、提交 Cargo.lock 和 npm lock。 |

机械提取过程记录为 `extract-shell.cjs` / `extract-frontend.cjs`，属于迁移记录，不是日常构建前置步骤。没有重写 AppCore 或新建整套核心框架。最终 workspace 无 quotio-core、vendor、SQLite/SQLx。`agent_config.rs` 与 `proxy_download.rs` 未引入运行时，登记为后续 IDE Handoff/下载验证候选，而非全盘否定其实现。

## 构建与测试证据

原始后台日志保留于本机 `D:/ProxyWeave/.pi/tasks/`（不提交临时任务日志）。表中 task ID 可定位日志；核心结果保留在此，不把 metadata 成功当验收。

| 阶段/命令 | 结果 | 归因/证据 |
|---|---|---|
| 原副本 `npm --prefix apps/desktop ci`、`run build`、`test` | PASS：5 test files / 30 tests | `b6700db74` |
| 原副本默认并行 `cargo build --workspace --locked` | FAIL | `error: failed to create encoded metadata from file: 页面文件太小，无法完成操作。 (os error 1455)`、rustc `0xc000012d`；资源不足级联，Layer 5 环境问题，改串行，不重写依赖。 |
| 原副本隔离串行 build + test | PASS：194 passed、1 ignored（178 core + 3 desktop + 6 platform + 7 types） | `bb8f88736`；上游有 dead_code warnings，未声称原工程零警告。 |
| 主工程原样引入 frontend build + workspace build | 首轮部分通过、Rust 内存失败；串行 retry PASS | `be5c87b49`：frontend build 成功，Rust `memory allocation of 4194304 bytes failed`，未计 check 通过；`b4e268a54` build retry 退出 0、上游 dead_code warning。构建闭包成立后再裁剪。 |
| 早期机械提取 shell check | FAIL：退出 101 | `b7a1e29a0`：``unexpected closing delimiter: `}```，`apps/desktop/src-tauri/src/lib.rs:36:1`；提取边界造成残留括号，属 Layer 2 移植遗漏，已针对性修正；不是上游缺陷或重写理由。后续冻结 check/build/test 均通过。 |
| 各轮标识/桌面/平台裁剪检查 | PASS（过程日志） | 先保留上游内部名，按调用关系删除；最终证据以冻结回归为准。 |
| 最小 frontend Vite/TS build + tests | PASS，退出 0：2 tests | `b1d6ffaad` / `b3e93ca7b`；修正 npm install 调用：原 `npm --prefix apps/desktop install` 被 npm 解释为安装根包，改用 desktop cwd 执行后清理自引用依赖，残留 extraneous lock 条目删除后由 `bffce2867` 再验收。 |
| 最小 workspace check/test + Tauri embedded debug build | PASS：13 Rust tests（6 platform + 7 types），0 failed；无 Rust warning/error | `bdf334c3c`；`tauri build --debug --no-bundle`，非 release installer。 |
| 早期裁剪闭包回归启动 | FAIL，退出 1；后续 check/test/build 未执行 | `b8999c17e` 停在 `cargo fmt --all`：`error: 'cargo-fmt.exe' is not installed for the toolchain 'stable-x86_64-pc-windows-msvc'.` 属 Layer 5 工具链组件/解析问题，不是源码编译失败；后续 `bd2607020` 的 fmt check 与完整回归已退出 0，不能将此次失败计为通过。 |
| 最终 `cargo fmt --all -- --check` / check / test / Tauri build | PASS，退出 0；13 Rust tests，0 failed；无 Rust warning/error | `bd2607020`；格式整理后冻结回归。npm 自引用随后单独清理并复验。 |
| 清洁 npm lock `npm ci` / frontend build/test / embedded Tauri build | PASS，退出 0：2 frontend tests；独立 debug exe 构建成功 | `bffce2867`；无根包自引用或 extraneous lock 条目，无 warning/error；CLI 在 desktop cwd 执行。 |
| 3 个 `manifests/*.yaml` PyYAML 6.0.3 safe_load + regex compile | 3/3 PASS | 修正 double-quoted `\.` 非法 YAML 转义；不等于 Schema 语义或插件运行通过。 |
| 主动 LSP probe（4 changed paths） | 0 findings，4 inconclusive | 服务 silent-on-clean，不能据此宣称 LSP confirmed clean；使用实际 TS/Rust CLI 编译验收。 |

## Windows 桌面实测

原 Quotio 独立副本：实际窗口/托盘可见，二次启动退出 0 并恢复主窗；自定义关闭弹出对话框，最小化到托盘后显示面板，面板 Quit Quotio 可退出。早期 Alt+F4 测得主窗消失而进程残留，因此没有把它写为正常退出。该隐患在目标端明确增加 native-close-to-tray 语义。只清理本轮拥有的验证进程，不对 External 服务发送 Kill。

目标：停止原 Vite 后用内嵌前端产物独立启动。普通 `cargo build` debug 会读取 devUrl，曾导致加载到旧 Quotio 前端；已定位为验证方法错误，改用 Tauri CLI 的内嵌构建，不把开发页误认作目标产物。

实际 UIA/Win32 观测详见 [phase0-runtime.json](phase0-runtime.json)，目标 PID `58948`，主 HWND `4528264`：

| 项目 | 实际结果 |
|---|---|
| 独立窗口 | PASS：ProxyWeave 标题，Phase 0 baseline 文案；无 Vite 服务，无 provider 业务启动。 |
| 托盘与操作 | PASS：Windows 11 overflow 发现 ProxyWeave 图标；Invoke 打开/切换原托盘面板，面板 Open ProxyWeave 恢复主窗并隐藏面板。**左击不是直接恢复主窗**。 |
| 自定义关闭 | PASS：取消/最小化到托盘/退出程序选项；托盘选择隐藏主窗、进程存活，显示小面板。 |
| 原生关闭 | PASS：对已观测主 HWND 发送 WM_CLOSE，主窗隐藏、进程仍活。未单独声称目标 Alt+F4 按键验收。 |
| 单实例 | PASS：主窗隐藏后二次启动 exit 0，原 PID/主 HWND 恢复，不创建第二个主实例。 |
| 显式退出 | PASS：关闭 → 退出程序，进程消失，所属窗口为空。未单独测试托盘右键菜单 Quit。 |

探针 `desktop-observe.ps1`、`desktop-tray.ps1` 只绑定本轮 PID/HWND/明确的托盘图标。Windows PowerShell 5.1 加载 UTF-8 中文脚本可用 `[scriptblock]::Create([IO.File]::ReadAllText(...))`，避免无 BOM 的 `-File` 编码误读；overflow 必须枚举独立 XAML 窗口而非只找 Shell_TrayWnd。

## 未解决/未验证与下一阶段

- macOS/Linux、release 优化构建、MSI/NSIS 安装器、自动更新与发布 CI 未验证。
- Keyring CRUD、Windows ACL、Unix 权限跨平台实测、Managed/External runtime、Job Object、端口恢复、网关/协议/市场/IDE 均未验收；Manifest syntax PASS 不代表这些功能可用。
- 尚保留原完整 types、部分 CSS 及上游图标/内部 exe 名；避免品牌重命名扩大范围，后续按真实引用继续裁剪。
- 上游默认并行构建受页面文件限制；本机采用 `-j 1`。不宣称修复 OS 内存配置。
- Phase 1 首个 Feature：Manifest Schema 1.0 与 Managed/External 进程所有权、凭据传递契约；先核验成熟实现，Unix mode / Windows ACL 分开设计并验收，再启动首个 Managed 插件。External 永不 Kill。
