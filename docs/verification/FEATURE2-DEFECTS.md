# Feature 2 DEF-001～004 修复交付

状态：开发门禁通过，冻结送审 ReadyForReview；不是 Inspector Verified。原冻结 `506980f370b23214192d7e912002fde61051fbdd` 独立审查失败。F3 暂停，未完成依赖/ADR 草案不纳入本修复提交。审查台账及 `.pi/inspection/**` 由 Inspector 独占，本次不修改。

## 根因与最小修复

| DEF | 原根因 | 修复路径 | 可执行回归 |
| --- | --- | --- | --- |
| 001 | get_status / start 信任缓存 Ready，未检查 OS 子进程退出 | `Child::try_wait()` 失活转 Failed，关闭 Owned Job，后续正常重拉 | `def001_exited_child_reconciles_and_restarts_with_live_new_pid`：/exit 后先 OS 确认退出，再核对 Failed、新 PID 存活与两次 spawn |
| 002 | HTTP200 绕过 Manifest 身份契约，宿主按插件名称特判 | `evaluate_health_status` 先精确比对身份，再按 expected_status / busy_statuses 分类；probe 使用 Manifest timeout，禁用代理/重定向 | `def002_http200_wrong_identity_is_not_ready`；单元测试同时覆盖缺失/大小写不匹配/尾空格身份及未声明 Busy |
| 003 | probe 结束才发布 entry；stop 不使用 start 锁 | 先发布 Starting；start/stop 共用每插件 FIFO Tokio Mutex；abort 由 Job/kill_on_drop 回收 | `def003_starting_visible_and_queued_stop_cleans_delayed_child`；`abort_during_start_reclaims_job_and_marks_failed` |
| 004 | max_port_retries 未执行 | 仅明确 AddrInUse 消耗重试，既有 pick_unused_port 重选并重新模板投影 | `def004_conflict_reselects_port_and_templates_reach_real_child`；`zero_retry_conflict_has_no_spawn_and_no_projection` |

## 复用溯源

- Tokio 1.52.3 MIT：已读取本地发布源码 `D:/build-tools/cargo/registry/src/rsproxy.cn-e3de039b2554c837/tokio-1.52.3/src/process/mod.rs` 中 `try_wait`、child id 和 kill_on_drop 语义；直接依赖现有 API，不自写进程守护。
- Tokio Mutex 的 FIFO 生命周期串行化沿用现有每插件锁；不引入第二套锁服务/监控线程。
- 端口：沿用 std TcpListener 与现有 `pick_unused_port`。probe：沿用 reqwest 0.13.4 原生 timeout/no_proxy/redirect policy。
- Manifest 扩展 `busy_statuses` 默认空并在为空时不序列化，保持旧未声明字段契约及历史 fingerprint；最多 8 个不重复合法 HTTP 状态，不含 expected_status。
- 兼容迁移：未声明 busy_statuses 的旧登记仍能解析/加载，空列表不改变其规范化序列化/fingerprint；它不会自动获得 503→Busy 语义。旧 wb2api 包保留历史 hash；新包 Manifest 明确声明 [503] 后 fingerprint 改变，相同 ID 的差异包按既有规则冲突拒绝，不静默覆盖。复验请用独立新 profile 导入 `D:/temp-audit/proxyweave-f2-defects-review-package`。此轮没有实现已登记差异包升级/迁移入口，不声称原 profile 自动升级。
- 删除旧 External 无 spawn 的 singleflight 测试，不再作为 Managed 并发证据。`runtime_defects.rs` 使用 Rust 测试 harness 的真实子进程、独立临时目录、真实 TCP 响应和 Windows WaitForSingleObject 核对 PID；没有使用 Inspector probe。

## 交付边界

- External stop 仍拒绝，无 Job/kill 句柄；不对既有本地/远程服务做生命周期控制。
- 正常 FIFO stop 不是立即取消 start：等待有界 startup 后收口，成功返回 Stopped 并验证该轮 PID 已退出。
- bind/drop 预检仍有 TOCTOU，probe timeout 不推断端口冲突，不能证明任意启动后冲突可恢复。
- spawn→Job assign 尚非 suspended 原子事务；kill_on_drop 降低 abort/assign 错误风险，不证明此窗口全树无泄漏。已分配 Job 的树才有内核回收语义。
- 本回归不构成另一个用户令牌 ACL 负向访问、真实凭据模型调用、最终原生桌面客户端验收。
- 真实 wb2api exe/LICENSE 未变；Manifest 新增 busy_statuses:[503] 后 SHA256 为 `2d237912d2a396cd39143251d3985250308444bd50551a7073f4955fd9cf5a76`。`prepare-wb2api-package.cjs` 的审计哈希同步更新；旧 Feature1 包/hash 作为历史证据保留，不就地修改旧包。

## 回归命令与结果

```cmd
set "RUSTUP_HOME=D:\build-tools\rustup"
set "CARGO_HOME=D:\build-tools\cargo"
set "PATH=D:\build-tools\cargo\bin;%PATH%"
cargo fmt --all -- --check
cargo test -p proxyweave-plugin --locked -j 1
cargo test -p quotio-platform -p quotio-types --locked -j 1
cargo clippy -p proxyweave-plugin -p quotio-platform --all-targets --no-deps --locked -j 1 -- -D warnings
```

- 首轮任务 `bfb395466` exit 101：新增测试中 5 处 `error[E0425]: cannot find function pick_unused_port in crate proxyweave_plugin`。仅修正测试调用为既有 `proxyweave_plugin::port::pick_unused_port`；该轮未执行测试、不算通过。重跑任务 `b3102e913`。
- 全量任务 `b261e054f` exit 101，在 workspace 测试前报 `error[E0107]: struct takes 3 generic arguments but 2 generic arguments were supplied`（schemars 0.8.22 的 `indexmap::IndexMap<K, V>`）。失败原始日志 `feature2-defects-desktop-cache-failure.txt` SHA256 `c40b92f45e8e820579309b2cfcc4b7066fe7ffcd44842e68eab91260ad3d80e2`。当时读取 indexmap build stderr 见 `error[E0462]: found staticlib std instead of rlib or dylib`，output 缺少 has_std；清缓存后诊断源已被覆盖，不把空文件冒作原始证据。执行 `cargo clean -p indexmap@1.9.3` 请求定向清缓存（实际范围见下方警告），同门禁重跑 `b6c0531ee`，不改依赖/锁/编译flags。恢复是否成立以重跑结果为准。
任务 `b3102e913` exit 0：plugin 32（lib 8 + registry 13 + real Managed 7 + lifecycle 4）、platform 8、types 7，共 47 passed / 0 failed；另 1 个 fixture 入口默认 ignored，但各真实 Managed 回归以 --exact --ignored 子进程调用。Clippy all-targets --no-deps -D warnings 通过零警告。完整回归日志 `feature2-defects-regression.txt` SHA256 `c9000d639a5bc081397ca45b5a3a1b19c7740a45ba81fe0a4250aacb993ebec5`；首轮失败日志 `feature2-defects-first-compile.txt` SHA256 `1fcd55b641aa7ba536fbde470d657a03b7e281334e4311f64ab0d10acf2e0db7`。workspace/前端/嵌入式桌面重跑任务 `b6c0531ee` exit 0：47 Rust passed / 1 fixture ignored、2 Vitest passed、tsc/Vite、embedded Tauri debug --no-bundle 全通过。清理阶段有一条工具范围警告：`warning: version qualifier in -p indexmap@1.9.3 is ignored, cleaning all versions of indexmap found`，实际清理了 indexmap 1/2 的 43 文件 11.2 MiB 缓存，并非仅 1.9.3；不是业务编译警告。重跑恢复已确认，依赖/锁未变。完整日志 `feature2-defects-desktop-pass.txt` SHA256 `a94a7ba519909f2db894e453160f7c27a2c61477d4a42e1808a0380d0494389e`。不以 LSP 空诊断当通过。LSP 探测 5 文件中 4 inconclusive，仅得到 3 条 Rust2024 let-chain 信息建议；本工作区 edition2021 不为建议变更语言版本。

## 冻结交付与复验入口

- 开发门禁通过，状态 ReadyForReview，四 DEF 由 Inspector 独立复验后才可关闭；F3/F4/真实模型与原生桌面操作目标继续开放。
- 冻结 exe：`D:/temp-audit/proxyweave-f2-defects-build/quotio-desktop.exe` SHA256 `08d5e9cb0da1ba76bb321459cca6f87722370b54a347604b4dad4fdc2abd2d49`。它证明现有嵌入式桌面基座可构建，不代表 F2 已接入桌面按钮；F4 操作界面尚未交付。
- 新三文件包：`D:/temp-audit/proxyweave-f2-defects-review-package`。exe SHA256 `38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485`、LICENSE SHA256 `49b9e27ddfc7cceab55e19eb7d405bda30a8229de3cc094e6b4d0397ab76a34b`、Manifest SHA256 `2d237912d2a396cd39143251d3985250308444bd50551a7073f4955fd9cf5a76`。只制作包，未启动 wb2api / 读取原凭据。
- 提交范围：`.gitattributes`；`crates/proxyweave-plugin/src/{manifest,probe,runtime}.rs`；`crates/proxyweave-plugin/tests/{runtime_defects,runtime_lifecycle}.rs`；`docs/architecture/ADR-003-runtime-job-acl-and-lifecycle.md`；`docs/verification/{FEATURE2.md,FEATURE2-DEFECTS.md,prepare-wb2api-package.cjs,feature2-defects-first-compile.txt,feature2-defects-regression.txt,feature2-defects-desktop-cache-failure.txt,feature2-defects-desktop-pass.txt}`；`manifests/wb2api.manifest.yaml`。
- 排除：F3 gateway/ADR-004/index/registry 草案、Inspector 台账/probe、私密 profile、目标元数据；Cargo.toml / Cargo.lock 无本修复修改。
- 完整 Git SHA 与本报告 SHA256 在冻结 commit 后通过 intercom 交付，避免将报告自身 hash 写入自身。
