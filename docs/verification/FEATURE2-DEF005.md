# Feature 2 DEF-005 脱敏启动诊断修复

状态：开发门禁通过，ReadyForReview；未 Inspector Verified。旧审查快照 `b8f06e67ae998afcc8612a69d2eb54967e91da15` 不变。Inspector 已独立确认该快照的 DEF-001～004 Verified；不扩大为 F2 整体、真实 wb2api 调用、凭据安全细边界或原生桌面验收。

## 根因与最小修复

- `crates/proxyweave-plugin/src/runtime.rs` 的启动健康循环丢弃 `ProbeStatus::Failed.error`，达到 startup deadline 后用通用超时覆盖所有错误。
- 直接复用现有 `ProbeReport` / `ProbeStatus`：仅在当前启动尝试保留最近一次明确 Failed 的脱敏 error；就绪期限耗尽时追加 `last probe failure`。Starting / Timeout 不虚构身份错误；后续 Ready / Busy 正常胜出；进程退出仍按既有退出路径报告。
- `crates/proxyweave-plugin/src/probe.rs` 在诊断源头移除 reqwest 原始错误格式化：该字符串可能包含 URL。保留固定类别（HTTP client build / connection / HTTP request failure）、安全 HTTP 状态码和超时，不回显实际身份值、头名称/内容、凭据、URL 或正文。精确身份校验、无重定向、无代理及外部服务零 Kill 不变。
- 无新依赖/锁文件/Manifest 字段；无守护器重搭，无插件 ID 特判。

## 复用溯源

- 沿用 DEF-001～004 已审计的 reqwest 0.13.4 与 Tokio 1.52.3；使用现有 reqwest `is_timeout` / `is_connect` 分类和现有 RuntimeManager 启动循环，不重新实现 HTTP 或进程控制。
- 已核对本地实际发布源码 `D:/build-tools/cargo/registry/src/rsproxy.cn-e3de039b2554c837/reqwest-0.13.4/src/error.rs:17-19`：官方明确 Error 可能含完整 URL（包括 query API key），并提供 without_url。此处采用更严格的固定类别，不格式化错误 source 链，避免仅去 URL 仍回显其他非可信细节。
- 回归复用 `crates/proxyweave-plugin/tests/runtime_defects.rs` 的真实 Rust harness Managed 子进程、TCP listener、临时目录与 Windows OS PID 检查。只扩展响应模式，不读取/运行 Inspector 的探针。

## 真实 HTTP 回归

| 测试 | 断言 |
| --- | --- |
| `def005_missing_and_wrong_identity_preserve_safe_failure` | 缺失身份200、错误身份200/503均拒绝 Ready/Busy，最终报告安全身份错误；缓存 get_status 一致，进程仍受控，stop 后 OS 确认退出 |
| `def005_later_ready_supersedes_previous_identity_failure` | 第一次错误身份、第二次正确身份恢复 Ready，不让缓存失败阻止恢复，同一 PID 单次 spawn |
| `def005_pure_http_timeout_remains_timeout_without_stale_failure` | listener 接收真实 HTTP 但不及时响应，仅报告通用超时，不附不存在的失败原因 |
| `def005_correct_busy_is_not_replaced_by_failure_or_timeout` | 正确身份503维持 Manifest 声明 Busy、PID 存活与状态一致，不重启、不超时覆盖 |
| `def005_malformed_http_preserves_safe_error_without_url_or_body` | 畸形 HTTP 响应保留固定 HTTP request failure，不暴露 query/header/body 的私密标记 |

私密标记是专用测试字符串，不是真实凭据。身份失败/纯超时/Busy/畸形 HTTP 的公开 RuntimeStatus 检查不能包含私密 URL、响应头名称/身份值或正文标记。

## 门禁与原始收据

任务 `b8296d06f` terminal exit 0：cargo fmt；`cargo test -p proxyweave-plugin --test runtime_defects --locked -j 1 -- --test-threads=1` 12 passed / 0 failed / 1 fixture ignored（5 个新增 DEF-005 测试含 3 个身份错误场景；子进程仍通过 --ignored 独立调用）；plugin/platform all-targets Clippy `--no-deps --locked -j 1 -- -D warnings` 通过，无警告。原始收据 `feature2-def005-targeted.txt`。


首轮全量任务 `b8e7cd862` terminal exit 101：workspace 的 runtime_defects 9 passed / 3 failed / 1 ignored；fixture 在 `runtime_defects.rs:27:67` bind 失败 `Os { code: 10048, kind: AddrInUse }`，3 个断言分别为 Busy 期望、纯超时（实际 `Managed process exited during startup: exit code: 101`）及身份原因缺失。新增诊断测试传 allocated_port=None 导致各自选择现有固定8080，并行争用；定向串行通过掩盖测试隔离遗漏。只修正新增5处调用，显式传既有 `pick_unused_port()` 分配端口，不修改业务端口契约、不串行化全量测试、不放宽断言。保留原始失败 `feature2-def005-first-full-failure.txt`。该轮未运行后续Clippy/前端/Tauri，不算全量通过。
完整重跑 `b94e71de0` terminal exit 0：fmt check、workspace locked测试保持默认并行共52 passed / 0 failed / 1 fixture ignored（plugin37=lib8+registry13+runtime_defects12+lifecycle4；platform8；types7）、Clippy零警告、npm ci、Vitest2、tsc/Vite、embedded Tauri debug --no-bundle 均通过。原始收据 `feature2-def005-full-pass.txt`；不将构建等同于原生桌面功能操作验收。
LSP 主动检查 3 个修改文件均 inconclusive（零诊断不构成 clean）；以以上真实编译/测试门禁为准。

复验命令（Windows cmd，显式 Rust 环境）：

```cmd
set "RUSTUP_HOME=D:\build-tools\rustup"
set "CARGO_HOME=D:\build-tools\cargo"
set "PATH=D:\build-tools\cargo\bin;%PATH%"
cargo fmt --all -- --check
cargo test --workspace --locked -j 1
cargo clippy -p proxyweave-plugin -p quotio-platform --all-targets --no-deps --locked -j 1 -- -D warnings
cd apps\desktop
npm ci
npm test
npm run build
npm run tauri -- build --debug --no-bundle
```

## 边界与交付

- 最后一次明确失败可能早于最后一次网络超时：报告同时保留最终期限与最近明确失败，不伪称最后一次响应仍是同一错误。
- 缺失/错误身份共享安全错误类别，不回显实际头和值；诊断更精确不等于暴露不可信内容。
- 本修改不改变生命周期所有权或补足 spawn→Job assign 原子窗口、跨用户 ACL、真实凭据调用及原生桌面 UI 等尚开放验收。
- 单独 commit / SHA256 交付；不纳入 F3 gateway / ADR-004 / index / registry 草案，不修改 `docs/DEFECTS_LEDGER.md` 或 `.pi/inspection/**`。
- 本冻结提交限定：`crates/proxyweave-plugin/src/{probe,runtime}.rs`、`crates/proxyweave-plugin/tests/runtime_defects.rs`、`docs/verification/FEATURE2-DEF005.md` 与 `docs/verification/feature2-def005-{targeted,first-full-failure,full-pass}.txt`。DEF-006 单独处理，不修改 projection.rs、不混入本次证据。
- 冻结产物仅为已编译的桌面基座（F2 尚未接入桌面操作）；真实凭据、模型调用和最终 Inspector 原生客户端操作仍开放。报告/收据/产物 SHA256 与完整 commit 在冻结后 intercom 交付，不在报告内写自身 hash。
