# ADR-003：插件 ACL、Keyring 持久化与 Owned Runtime（Job Object、端口判定与生命周期）

状态：设计与实施中；Feature 2 开发阶段。

## 范围与核心决策

承接已完成的 Feature 1 插件包与 Manifest 严格契约（commit `889aa9bee594dc11c9d31d3eed86325a4314d13f`），本阶段实现 Feature 2 核心能力：
1. **静态凭据持久化与命名空间隔离**：托管至 OS 底层 Keyring（Windows Credential Manager / macOS Keychain / Secret Service），服务名统一为 `proxyweave`，通过 `plugin:{plugin_id}` 命名空间隔离。
2. **私有配置投影与真实 Windows ACL**：
   - 彻底废除 `crates/quotio-platform/src/lib.rs` 中 Windows sensitive 权限分支的空操作（`let _ = path; Ok(())`）；
   - 基于 Windows 官方安全 API（`windows-sys` 0.61），通过进程 Token 获取当前用户 SID，构造受保护的 DACL（Protected DACL，阻断父目录继承），仅授予当前用户 `FILE_ALL_ACCESS`，实现与 Unix `0o600` 等价的 OS 级私有文件 ACL 隔离。
3. **Managed 插件 Owned Runtime 与 Win32 Job Object**：
   - 为 Managed 插件创建 Windows Win32 Job Object 并配置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`；
   - 子进程启动后挂载入 Job Object，已成功分配 Job 的进程树在 Job 最后句柄关闭时由内核回收；spawn→assign 窗口不是原子事务，不声称此窗口的全树保证。
4. **External Provider 主动保护与零 Kill 铁律**：
   - 类型系统与运行时严格隔离 `ManagedRuntime` 与 `ExternalRuntime`；
   - `ExternalRuntime` 绝不加入 Job Object，绝不持有可发出终止信号的句柄，绝不执行任何 Kill / Terminate 操作；对 External 插件仅做反向代理挂载与健康探针，stop/restart 操作显式拒绝。
5. **端口分配、TOCTOU 防护与明确冲突证据**：
   - 支持回环端口分配与启动前占用预检；
   - 就绪超时绝不单独推定为端口冲突；必须收集确凿证据（端口已被监听、绑定返回 `WSAEADDRINUSE`/`AddrInUse`、子进程退出日志报错）才判定为端口冲突，超时但无冲突证据时分类为 `ProbeTimeout`。
6. **健康探针状态机与 wb2api 特殊判定**：
   - 探针状态：`Stopped`, `Starting`, `Ready`, `Busy`, `Failed`；
   - 对首发验证插件 `wb2api` 的 `/healthz` 响应，结合其源码（`handler.go:181-203`）进行语义识别：HTTP 503 伴随 `X-Service: workbuddy2api` 证明进程完全存活且 HTTP 监听就绪，仅因 Token 账号池为空或饱和，绝对不能当作崩溃重试重启，标记为未激活/Busy 并记录原因；仅当 HTTP 200 时转移为 `Ready`。
7. **Singleflight 并发启动互斥**：
   - 基于异步互斥锁，多请求突发命中未启动插件时仅触发一次拉起；后续请求等待就绪状态通知后放行，避免并发重复启动导致端口碰撞。

## Direct Reuse First 前置比较与来源考据

- **Win32 Job Object**：
  - 来源：Windows 官方 Win32 API (`kernel32.dll`)，通过项目现存的 `windows-sys` 0.61.2 的 `Win32_System_JobObjects` 特性直接调用。
  - 核心 API：`CreateJobObjectW`, `SetInformationJobObject`, `AssignProcessToJobObject`，标志位 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`。
  - 考据结论：直接依赖 `windows-sys`，不引入未知第三方封装；自行封装安全 RAII `JobObject` 句柄包装器。
- **Windows File DACL / ACL**：
  - 来源：Windows 官方安全 API，通过 `windows-sys` 0.61.2 的 `Win32_Security` 和 `Win32_Security_Authorization` 特性。
  - 核心 API：`OpenProcessToken`, `GetTokenInformation`, `ConvertSidToStringSidW`, `ConvertStringSecurityDescriptorToSecurityDescriptorW`, `SetFileSecurityW`, `LocalFree`。
  - 考据结论：结合 SDDL `D:P(A;;FA;;;{USER_SID})` 阻断继承并独占用户权限；直接完善 `quotio-platform::set_sensitive_permissions` 的 Windows 实现，不增加外部库。
- **Keyring**：
  - 来源：直接复用 `Cargo.toml` 既有依赖 `keyring 3.6.3`（已配置 `windows-native`）；
  - 完善：通过统一的账户前缀（如 `plugin:{plugin_id}`）避免全局账号名冲突，增加完整的 CRUD 验证。
- **网络探针与端口检测**：
  - 来源：标准库 `std::net::TcpListener` 与已在 `Cargo.lock` 中的 `reqwest` 0.13 或 `hyper`。
  - 考据结论：使用简单轻量的探针逻辑，准确解析 `X-Service` 头与状态码。

## 验证门禁与步骤

1. **ACL 验证**：在 Windows 上创建敏感文件并应用 `set_sensitive_permissions`，验证 DACL 属性中所有者包含当前用户、继承标志被阻止；
2. **Job Object 隔离测试**：编写独立测试拉起长期存活的子进程（如 ping 或测试 fixture），验证主进程退出/Job 关闭时子进程被操作系统内核可靠终止；
3. **External 零 Kill 验证**：测试 External 运行时拒绝 stop/restart 信号；
4. **wb2api 探针语义验证**：模拟/真实测试 200 -> Ready、503+X-Service -> Busy/Starting（不触发 Kill）、超时 -> Timeout（不误报 PortConflict）。

## DEF-001～004 根因修复补充（等待独立复验）

- 直接复用 Tokio 1.52.3 `Child::try_wait()`（`src/process/mod.rs` 非阻塞退出/回收实现）、`Command::kill_on_drop(true)` 与 Tokio FIFO `Mutex`。没有新依赖、进程轮询服务或第二套守护器。
- `get_status` 对 Managed 子进程进行 OS 退出核对，失活转 Failed 并关闭其 Owned Job；再次 start 能创建新的 PID。Ready/Busy 不再仅依赖缓存。
- start 在投影/spawn/probe 之前发布 Starting；stop 使用同一每插件 Mutex 排队，等待有界启动结束后清理该轮 Owned 实例，成功返回后为 Stopped。中途 task abort 会 drop 本地 Job/Child，锁释放后 get_status 将未发布 Starting 转 Failed。
- `probe_http` 读取 Manifest `HealthCheck` 的路径、身份头/精确身份值、预期状态与超时；`busy_statuses` 为默认空的可选扩展（最多 8 个唯一合法 HTTP 状态，不含 expected_status）。wb2api 在 Manifest 声明 `[503]`，宿主不再按插件名称特判。禁用探针重定向与环境代理，身份头不回显至错误信息。
- 端口预检只有 `AddrInUse` 能消耗 `max_port_retries` 重选，使用既有 std `pick_unused_port()` 并把最终端口传入全部模板。0 次重试冲突无 spawn/投影。探针超时不重选，不推断冲突。
- 诚实边界：bind/drop 预检不能消除子进程 bind 之前的 TOCTOU；spawn→AssignProcessToJobObject 尚非 suspended 创建原子事务，`kill_on_drop` 不构成此窗口的全树保证。只对已分配 Job 的树声明内核回收；DACL 测试不等于另一用户令牌的负向访问测试。External 不进入 Job、stop 保持拒绝，未扩大生命周期权限。
