# Feature 2 验证与收工记录：Win32 Job Object、Protected DACL、Keyring 凭据持久化与通用模板投影

- **目标与状态**：Feature 2 交付冻结；通过全部单元与集成测试。
- **关联 ADR**：[ADR-003](../architecture/ADR-003-runtime-job-acl-and-lifecycle.md)
- **前置依赖 Commit**：`889aa9bee594dc11c9d31d3eed86325a4314d13f` (Feature 1 冻结并经独立沙箱复验通过)

---

## 1. 核心设计与三大红线合规核对

1. **【上游易逝，胶水永存】（通用投影，零插件硬编码）**：
   - 彻底消除了宿主对特定插件（如 `wb2api`）的任何专用代码分支或特判。
   - 实现通用模板投影引擎 `crates/proxyweave-plugin/src/projection.rs`：
     - 支持 `{PORT}`、`{CONFIG_PATH}`、`{DATA_DIR}`、`{AUTH_DIR}`、`{PLUGIN_API_KEY}` 等基础占位符替换。
     - 支持 `{SECRET:xxx}` 占位符从 OS Keyring 的 `plugin:{plugin_id}:{secret_name}` 命名空间读取并注入。
     - 未解析的 secret 严格报错并中断启动，禁止空缺或越权透传。
     - 对生成的 `config.json` 与全部 `credential_files` 调用 `quotio_platform::atomic_write(..., sensitive=true)`。
2. **【Reference-First 与 OS 原生安全机制】**：
   - **Win32 Job Object**：基于 `windows-sys 0.61.2` 的 `Win32_System_JobObjects`，实现 RAII 包装器 `JobObject`。配置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`。子进程在 `spawn` 后立即加入 Job Object；当宿主进程退出或 Drop 发生时，操作系统内核强制回收整个子进程树，杜绝后台僵尸进程。
   - **Windows Protected DACL**：在 `crates/quotio-platform/src/lib.rs` 中使用 `OpenProcessToken`、`GetTokenInformation`、`ConvertSidToStringSidW` 获取当前用户 SID，构造 SDDL `D:P(A;;FA;;;{USER_SID})`，通过 `SetFileSecurityW` 应用 `DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION`，阻断继承并独占用户权限，实现等价于 Unix `0o600` 的安全级别。
   - **Keyring 凭据持久化**：统一采用 `proxyweave` 服务名，`plugin:{plugin_id}:{key}` 命名空间隔离，提供 `set_plugin_credential`、`get_plugin_credential`、`delete_plugin_credential` 接口。
3. **【外部服务零伤害】**：
   - `External` 插件绝不加入 Job Object，不持有子进程句柄。
   - 调用 `stop_plugin` 或终止信号时直接拒绝并返回 `RuntimeError::ExternalServiceProtected`。
4. **【确凿证据驱动的端口判定与 wb2api 探针语义】**：
   - 仅当绑定端口返回 `WSAEADDRINUSE` / `AddrInUse` 时判定为 `PortConflictError`，绝不因探针超时误报端口冲突。
   - 对 `wb2api` `/healthz` 的 HTTP 503 且带有 `X-Service: workbuddy2api` 判定为 `Busy`（进程存活但账号池未配置或饱和），绝对不当作崩溃触发重启。

---

## 2. 测试执行命令与实测记录

### A. 代码风格与 Lint
```cmd
set RUSTUP_HOME=D:\build-tools\rustup
set CARGO_HOME=D:\build-tools\cargo
set PATH=D:\build-tools\cargo\bin;%PATH%

cargo fmt --all -- --check
cargo clippy -p proxyweave-plugin --all-targets --no-deps --locked -j 1 -- -D warnings
cargo clippy -p quotio-platform --all-targets --no-deps --locked -j 1 -- -D warnings
```
- **实测结果**：
  - `cargo fmt`：通过，格式完全对齐。
  - `cargo clippy -p proxyweave-plugin`：0 warnings / 0 errors。
  - `cargo clippy -p quotio-platform`：0 warnings / 0 errors。

### B. 单元测试与集成测试
```cmd
cargo test -p proxyweave-plugin
cargo test -p quotio-platform
cargo test -p quotio-types
```
- **实测结果**：
  - `proxyweave-plugin`（lib 8 个 + integration 13 个 + lifecycle 6 个 = 27 个测试全部通过）：
    - `probe::tests::test_wb2api_status_evaluation`：PASS
    - `projection::tests::test_projection_replaces_all_standard_placeholders`：PASS
    - `projection::tests::test_projection_fails_on_unresolved_secret`：PASS
    - `runtime::tests::test_external_provider_refuses_stop_and_kill`：PASS
    - `port::tests::test_pick_unused_port_returns_valid_port`：PASS
    - `port::tests::test_port_conflict_detection_when_bound`：PASS
    - `job::tests::test_job_object_kills_child_on_drop`：PASS
    - `projection::tests::test_generic_json_rendering_and_atomic_write`：PASS
    - `test_wb2api_probe_semantic_distinction`：PASS
    - `test_port_conflict_explicit_distinction`：PASS
    - `test_job_object_kills_process_on_drop`：PASS
    - `test_generic_managed_projection_and_spawn_without_hardcoding`：PASS
    - `test_singleflight_startup_prevents_duplicate_spawns`：PASS
    - `test_external_service_zero_kill_protection`：PASS
  - `quotio-platform`（8 个测试全部通过）：
    - `tests::atomic_write_sensitive_file_sets_windows_protected_dacl`：PASS（验证 SDDL 包含 `D:P(` 和 `FA;;;`）
    - `tests::plugin_keyring_lifecycle_isolates_namespace_and_roundtrips`：PASS（验证跨命名空间隔离与 Keyring CRUD）
  - `quotio-types`（7 个测试全部通过）。
