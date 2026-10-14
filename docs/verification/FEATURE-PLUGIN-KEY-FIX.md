# Feature 交付报告：插件访问密钥配置、历史凭据兼容与管理入口安全止损

## 1. 背景与用户需求

- **用户明确要求**（m07779）：WorkBuddy2API 网页控制台登录需要 `api_key`，此前桌面客户端插件管理页仅展示 `required_secrets`，未提供 `api_key` 的查看与配置入口。用户要求在对应插件管理页直接提供凭据配置与说明，而不是让用户自行去编辑底层 `config.json`。
- **DEF-012 历史凭据存储格式兼容**（审查 seq 5, 8）：历史版本使用 `set_password` 写入 Windows 凭据管理器（UTF-16LE 字节流），新版本平台接口使用 `set_secret` 写入 UTF-8。若以 UTF-8 读取历史写入的 ASCII 密钥，会出现 `\0` 交叉字符或乱码。需做向后兼容识别，且不可向日志/异常泄漏凭据明文。
- **真实网关挂载状态**（审查 seq 4）：网关实际挂载状态必须通过 `backend_plugin_id()` 真实读取，挂载失败必须如实上报，不得显示虚假的“已成功挂载”或“★ 当前网关挂载后端”。
- **安全止损与嵌入边界**（审查 seq 11）：
  - **本次安全止损已实现**：撤下未经核验的 `<iframe>` 及“已在安全沙箱中内嵌”的未证实文案；不改 CSP、不绕过插件 frame-ancestors 策略、不向第三方页面暴露宿主 IPC。复用 `get_plugin_management` 描述符，在插件详情页提供分类明确的“插件访问设置与使用说明”，并在系统浏览器中以独立进程打开管理页面。
  - **原生隔离嵌入仍未解决**：系统浏览器打开仅为临时兜底方案，不标榜为“嵌入功能完成”。WebView2/原生安全隔离嵌入的技术路线核验与工程落地仍处于待决状态，后续在明确方案与权限边界后再行推进。

---

## 2. 核心修改清单

### 2.1 凭据存储与向后兼容 (`crates/quotio-platform/src/lib.rs`)

- `get_credential(key)` 增加向后兼容探测：
  - 若读取出的字节符合 Windows `set_password` 写入的 UTF-16LE ASCII 编码（奇数字节为 ASCII 打印字符，偶数字节为 0x00），则自动无缝通过官方 API 还原为标准 UTF-8 字符串；
  - 若为标准 UTF-8 编码，正常解码返回；
  - 若为无法安全识别的非 UTF-8 编码，安全返回固定的分类错误（`"Unsupported credential encoding; save the credential again through its settings page"`），绝不将原始字节或部分明文拼入错误消息。
- 增加 7 项针对历史格式、UTF-8、Unicode、长 OAuth Token、任意二进制 blob 及非标准编码安全拦截的单元测试。

### 2.2 宿主管理与挂载真实性 (`apps/desktop/src-tauri/src/host.rs`, `crates/proxyweave-gateway/src/lib.rs`)

- `proxyweave-gateway` 增加 `backend_plugin_id(&self) -> Option<String>`，供宿主无锁原子读取当前真实绑定的后端插件 ID。
- `host.rs`：
  - `get_gateway_status` 中的 `active_backend` 字段由网关实际运行时的 `backend_plugin_id()` 供给，杜绝假挂载；
  - 挂载命令 `select_plugin` 在网关启动失败或后端绑定失败时，立即清除内存及持久化偏好，向前端如实返回错误，绝不返回虚假成功；
  - `start_plugin` / `restart_plugin` 在启动 Managed 插件前，若其配置包含 `{PLUGIN_API_KEY}`，自动通过 `load_or_create_plugin_api_key` 从系统 Keyring 读取现有密钥；若未设置则安全生成 32 字符安全随机密钥写入 Keyring，绝不覆盖已有密钥；
  - 新增专用 IPC `get_plugin_api_key(id)`，仅供用户在前端界面明确点击“复制插件访问密钥”时按需获取，前端渲染时密码框永不回读明文；
  - 凭据修改与删除统一纳入 `plugin_operations` 串行互斥锁，并在执行前强制校验插件状态必须为 `Stopped`；在 `Busy` 或 `Failed` 状态下拒绝修改，防止竞争。

### 2.3 插件详情与管理界面 (`apps/desktop/src/components/PluginPanel.tsx`, `PluginDetails.tsx`)

- 插件详情页：
  - 移除原先硬编码的 `id === 'wb2api'` 路径分支及无隔离依据的 `<iframe>` 预览；
  - 增加“插件访问设置与使用说明”卡片，包含详细指引：
    - 明确解释插件自带控制台登录需要使用该插件独立的 `api_key`，而非统一网关的 Master Key；
    - 提供“前往插件访问设置”跳转按钮，点击后自动滚动并聚焦至对应凭据项；
    - 明确标注“原生隔离嵌入仍待核验”，不向用户误导功能完整性；
  - 访问控制台按钮：调用系统默认浏览器安全打开，不改变桌面应用 CSP，不开放宿主 IPC。
- 凭据管理列表：
  - 对于声明了 `{PLUGIN_API_KEY}` 的 Managed 插件，自动将其作为第一项凭据项与 `required_secrets` 合并去重展示；
  - 密码输入框保持空白，不预填明文，密码框禁用自动填充；
  - 针对 `api_key` 提供明确的“复制插件访问密钥”按钮，通过异步剪贴板写入并在成功后给出合规提示；失败时不显示虚假成功；
  - 保存失败时完整保留用户输入草稿，解锁输入框供用户修正重试；
  - 保存进行中时锁定输入框与生命周期控制，防止并发竞争；
  - `Failed` 状态下保持凭据输入锁定，但开放“停止插件”按钮，允许用户主动回收残留子进程后再行修改凭据。

---

## 3. 测试与验证结果

### 3.1 安全排除 3 项共享凭据测试的 Workspace 回归
因 `apps/desktop/src-tauri/tests/desktop_host.rs` 中的 3 个既有测试直接调用 `DesktopState::new(prof_dir, registry)`，其内部会直接访问并修改 OS Keyring 中的全局共享账户 `gateway:master_key`，经审查方书面同意（seq 8），在非完全隔离的本地运行中予以显式 `--skip` 排除：
- 排除项：
  1. `test_desktop_state_initializes_master_key_and_persists_config`
  2. `test_desktop_host_events_are_recorded_and_bounded`
  3. `test_desktop_state_start_and_stop_gateway`
- 命令：`cargo test --workspace -- --skip test_desktop_state_`
- 结果：通过（见后续自动化回归执行输出，除排除的 3 项及 2 项独立子进程 harness 外全部通过）。

### 3.2 独立 Owned 宿主边界测试 (`apps/desktop/src-tauri/src/host_key_tests.rs`)
通过内部构建隔离的 `DesktopState`（仅使用独占 dummy Keyring 命名空间 `plugin:owned-key-test-fixture:api_key`，绝不触碰 `gateway:master_key`），验证：
1. `load_or_create_plugin_api_key_creates_and_loads_owned_key`：通过。首次生成并保存，再次调用幂等加载，不覆盖已有值；
2. `save_and_delete_plugin_secret_isolated_and_cleared`：通过。独立凭据的保存、状态列出与删除闭环，删除后自动清理 Keyring；
3. `rejected_gateway_binding_preserves_actual_confirmed_backend`：通过。网关挂载错误时保留原始真实后端状态，不伪造成功。

### 3.3 浏览器端 UI 与交互回归 (Playwright Dummy IPC 24 项全部通过)
在隔离的浏览器测试环境中，注入完全受控的 dummy IPC（绝不触碰真实 OS 凭据），24 项边界验证全部通过：
- [x] guide navigates to this plugin api_key input
- [x] plugin-owned config guidance present
- [x] embedding limitation remains explicit
- [x] no unverified iframe or automatic remote page
- [x] password blank: no readback
- [x] deduplicated api_key and only plugin key copy
- [x] OAuth remains password/status only
- [x] no implicit api_key read
- [x] write rejection keeps draft
- [x] write rejection unlocks retry
- [x] pending save locks editor
- [x] pending save locks lifecycle
- [x] pending save does not clear or claim success
- [x] confirmed save clears draft
- [x] clipboard rejection unlocks retry
- [x] no optimistic clipboard success
- [x] explicit copy does not populate password input
- [x] busy: credential editing disabled
- [x] failed: credential editing disabled
- [x] Failed exposes explicit stop to reclaim child before editing
- [x] backend error not success
- [x] failed selection not attached
- [x] no document overflow 360
- [x] no document overflow 1366

### 3.4 前端单元测试 (Vitest)
- 15/15 项测试全部通过 (`shell.test.tsx` 涵盖所有关键组件渲染、无 iframe 约束、敏感输入脱敏、访问密钥展示与状态区分）。

---

## 4. 交付文件与构建产物 SHA-256 校验列表

| 文件路径 | 状态 | SHA-256 校验和 | 说明 |
| :--- | :--- | :--- | :--- |
| `crates/quotio-platform/src/lib.rs` | Modified | `bfb1b6d191198bd025f7509e5b2e21a18fa0a6eabf2007ba0a33dfda2031a189` | UTF-16LE 历史格式识别与安全报错 |
| `crates/proxyweave-gateway/src/lib.rs` | Modified | `7e2b6328317ac295395c0b9e5afa1ef89f67928979b190c028e15100f8fad538` | 增加 `backend_plugin_id` 真实状态读取 |
| `apps/desktop/src-tauri/src/host.rs` | Modified | `2d243b34c880f9e70ae54118780df99f49eb8714df924d07c91c07a5bfaa7c78` | 凭据管理、并发锁与真实网关挂载绑定 |
| `apps/desktop/src-tauri/src/lib.rs` | Modified | `36708ad13fdee5f7c9e3e606bc780c4a4515f764c1b99daabee9ede5bc4a313c` | 注册 `get_plugin_api_key` IPC |
| `apps/desktop/src-tauri/src/host_key_tests.rs` | Added | `a4acd70306c1443a5080dcf198cf83e816288d7cd348cfb3048399b1867fcca0` | 独立 Owned 宿主边界单元测试 |
| `apps/desktop/src-tauri/Cargo.toml` | Modified | `f67972cce167b9599c233033eb26185d3db3cf8c242e72ee96af6d6375ae29a3` | 仅为 dev-test 引入已锁定的 sha2 |
| `apps/desktop/src/components/PluginPanel.tsx` | Modified | `f0456fc4e0b339113d69ddd6948086a3ce82e7b82ed6ebdddf492c7fe35c817b` | 插件管理凭据输入、复制与交互锁定 |
| `apps/desktop/src/components/PluginDetails.tsx` | Modified | `92985be6ec29272868eebea26acf4fa271658f734975def6ed14a510d22f884e` | 撤下未核验 iframe，提供分类指导文案 |
| `apps/desktop/src/shell.test.tsx` | Modified | `96565a0dd45d8c154c8f13b1328e197db5b84becf4a1f7472b4a687728efdae5` | 前端 15 项单元测试全部通过 |
| `Cargo.lock` | Modified | `31eb236d8a637f00de3fc97dbac507d3c3ab5fcc0fd9d448fca64c7a3c73898f` | 仅同步 desktop dev-dependencies 锁定 |
| `target/debug/quotio-desktop.exe` | Artifact | `f1d7852e83f9cf476d60a28ebff037bd4a266e9bb97fb074557546db61c71163` | 原生 Tauri 调试构建二进制产物 |
