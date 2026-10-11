# Feature 3：统一 HTTP 网关与 Owned 请求生命周期

状态：开发验证已收口，当前进入冻结送审（ReadyForReview），未 Inspector Verified；原包测试 108 passed / 1 failed 如实保留，不记全绿。F1 已独立 Verified；F2 六个具体 DEF 的关闭不等于 F2 整体验收。真实 wb2api 凭据、实际推理与最终 Windows 桌面仍未完成，fixture 不替代最终目标。

## 实施范围

- `crates/proxyweave-gateway/`：loopback HTTP/1.1，仅 GET `/v1/models` 与 POST `/v1/chat/completions`；本地 key 与 upstream key 分离。
- 原代理直接依赖 `axum-reverse-proxy = 2.2.0`，不是重新生成代理。采用官方 `ReverseProxy` / `ProxyPolicy` / Hyper 原 body 转发；不引入 TiyGate SQLx、IR 或计费流水线。
- `crates/proxyweave-plugin/src/runtime.rs`：在原 FIFO 生命周期锁上增加 request lease。Managed 显式 stop 在请求存活时拒绝；External 的生命周期控制始终拒绝。
- 从等待上游响应头到 response body EOF / error / Drop 持有 lease；listener 以官方 Axum 连接循环薄适配的 JoinSet 拥有 connection tasks。网关 shutdown 取消并 drain，而不是无限等待 SSE。
- 不解析/重编码 SSE，不生成任何 `[DONE]`。有效 HTTP EOF 缺少协议终止标记仍只是 EOF，不宣称所有 SDK 都会报错或业务成功。

来源比较、必要薄适配与受限能力见 [ADR-004](../architecture/ADR-004-unified-gateway-and-sse-passthrough.md)。许可证见 [Axum-Reverse-Proxy-MIT.txt](../references/licenses/Axum-Reverse-Proxy-MIT.txt)。

## 原实现与包真实性

本地原包：`D:/temp-audit/proxyweave-feature3-proxy-audit/axum-reverse-proxy-2.2.0`。
发布源码 commit：`7350d118a69b6d98ae19a767b9d71630b2a3bc80`，MIT，Copyright 2024 Tom Lubenow。
与 Cargo registry 缓存逐文件比较：63 文件一致（忽略 Cargo 自生成 `.cargo-ok`），未修改原包或断言。
原包锁定依赖与宿主不完全相同：Axum 0.8.8→0.8.9、Hyper 1.8.1→1.10.1、Tokio 1.49.0→1.52.3、Tower HTTP 0.6.8→0.6.11；Hyper-util 0.1.20 / Tower 0.5.3 相同；原 tests 的 Reqwest 0.12.28 与宿主 tests 的 0.13.4 不同。原包测试按原 lock 运行，不将其通过结果当作宿主组合的验证；宿主组合由本工作区 `--locked` regression 单独验收。

| 原文件 | SHA256 |
|---|---|
| LICENSE | `b1c9d01a82c6f4fe98807f9eb176bc4876176760cf6607af78018ced7c967d71` |
| Cargo.toml | `1918d8f6e9c7c7a6a3993d635f01e0832afcfa40f284afd4a148d2f6174ddfab` |
| Cargo.lock | `9e37e9f56f4e105c861507f2d1c725c9dbf7be17e4b2642ca1d523b1289c3430` |
| .cargo_vcs_info.json | `1904f64b5cb08f690857cf5b3d653055598912fad17fea8e042a0066e05cbb62` |
| src/forward.rs | `485176d600d9866c7153be1a01c37f1e6a87b280061a4be4e309ef95880fbdd5` |
| src/proxy.rs | `038302dfbf32121993ddd9af25b46ea1b8305e46e629d74ee07a248ed3db5215` |
| tests/headers.rs | `3acbd0939a0fc8c3a63c33ce312d7ed45bd02e3c5029b0260d1c27b717f61f45` |
| tests/rfc9110.rs | `43d19c4aefe2516df430142add8c3144970e40e4bed90cb82f7a1b21d6c98287` |
| tests/streaming.rs | `aeae9cb5c1120196699830649272f38456e392d09b7ee05492b747f73a42c637` |
| tests/policy.rs | `74816c1b76b61815beabaa0b75dc424a55328635a57ff353eb58d70f1a1c5442` |

## 开发验证（不得替代独立审查）

- `b9f78713e`：首次 `cargo check -p proxyweave-gateway --locked -j 1` exit 0。只证明编译，不证明 TCP 取消或完整验收。
- `b8a59dcac`：exit 101，回归未执行；`error: cannot update the lock file D:\ProxyWeave\Cargo.lock because --locked was passed to prevent this`。首次 check 后新加的测试依赖 sha2/windows-sys 及 tracing 尚未反映在 gateway 自身 lock 节点；不是代理逻辑失败。
- 随后以 `cargo metadata --offline --format-version 1` 重新求解：仅 gateway 自身 dependency 列表改变，没有新增/移除版本。全平台 metadata 因未缓存 `android_system_properties v0.1.5` 退出 101，**不记 metadata 通过**。当前 Windows 回归仍保持 `--locked` 重新启动；没有移除该保护或升级依赖来掩盖失败。
- 原始 receipts：`feature3-first-check.txt` SHA256 `6665affe277f53c2c57e8a311bd494a59053b472e817171a848b46e4186c8f91`；`feature3-first-lock-failure.txt` SHA256 `f2e3bf32a933970a61f6dadea24921d42bfe7bdcd50a3a61a151f5152239288b`。
- 原代理 local lib/header/rfc9110/streaming/policy/routing/service/error tests：最终干净重编 108 passed / 1 failed，详细版本/失败与产物归属见下。
  - 首轮 `bf8323de5` exit 101：原 lib 53/53 通过；`tests/errors.rs` 1 通过、2 失败，后续 test targets 因 Cargo fail-fast 未执行。`test_proxy_error_handling` 在 `tests/errors.rs:75` 遇 `reqwest::Error { kind: Request, ... source: TimedOut }`（原客户 deadline 100ms）；`test_proxy_upstream_errors` 在 `tests/errors.rs:146` 断言 `Expected status code 400 but got 502 Bad Gateway`（left 502/right 400）。目前根因未证实，不修改原源码或削弱断言。
  - 原失败 receipt `feature3-original-first-failure.txt` SHA256 `1f3ab6c892ed8c0b12419f3cb5bee69e4747af5a51883b4b1063a0f9ace1129a`；单线程 + 仅该测试命令 `NO_PROXY=*` 的隔离诊断已启动，不能把诊断重试提前算通过，也不能把初始失败归为环境问题。
  - `b178aed40` 隔离复测 exit 101：同样 53 lib 通过、errors 1 通过 / 2 同因失败；因此单线程 / `NO_PROXY=*` 不消除问题。该次误用了默认 TLS feature 而非首轮 native-tls；只能记录额外诊断，不能当 native-tls 精确复验。receipt `feature3-original-serial-failure.txt` SHA256 `c70980adeba4fa69cf000da95279e23853ce16101e9aff0cc498cc6fd5fe4da8`。
  - 为避免 errors fail-fast 遮蔽其他原 tests，另启动精确 `--locked --no-default-features --features native-tls` 的剩余六 target；原包保持不改。故障 trace 在单独目录 `D:/temp-audit/proxyweave-feature3-proxy-diagnostic` 仅加 subscriber（不改原断言/期限）；这个修改副本只用于根因诊断，不充作原测试通过证据。
  - 原 native-tls 剩余 suites `bcea085f5` exit 101：headers 0 passed / 6 failed；预期 200 实际 502，或空 body JSON `EOF while parsing a value`；之后 policy/rfc9110/routing/service/streaming 仍被 Cargo fail-fast 遮蔽，**未运行，不计通过**。receipt `feature3-original-headers-failure.txt` SHA256 `003493d19ca2a6f7ea491d28aecb9b233749f6e59fd969f4cf3f237096c2d3af`。重复出现原默认 client 到 loopback 的问题，具体原因仍待 trace，对原默认 client 与宿主自定义 connector 必须分别归因。
  - 隔离 trace `b647e8907` 编译成功、tests 仍 exit 101：观察到 **Reqwest 测试客户端的 loopback 请求被系统代理 intercept**，在进入被测 ReverseProxy 前便走另一端口；100ms 时产生客户端 timeout，预期 400 则收到代理 502。未观察到该两次请求进入被测原代理的数据面。Windows NT 10.0.26200.0，OS proxy enabled/server/override 均为 present；初始 HTTP_PROXY/HTTPS_PROXY/ALL_PROXY/NO_PROXY（含小写）环境变量均 absent。系统配置不修改，地址和值不公开。
  - Reqwest 0.12.28 `src/proxy.rs:514-517` 默认调用 Hyper-util `Matcher::from_system()`，Hyper-util 0.1.20 `src/client/proxy/matcher.rs:238-250,659-693` 读取 Windows Internet Settings。前次声明 `NO_PROXY=*` 并未消除 trace interception，正在独立确认子进程实际 env 生效，不能武断称 bypass 成功。完整 trace 私有保留在 `.pi/development/feature3-original-trace-private.txt` SHA256 `55dca23a2b7ec224ad47b349045558b2414b5e5dde68ad8b438917f814d21a16`（含系统 proxy 值，不纳入公开仓库）。宿主 connector 与回归客户已显式 no-proxy，不依赖该原 tests 默认环境。
  - 进一步 `ba2d3f9e1` 的真实 Rust 测试进程布尔为 `NO_PROXY present=true exact_wildcard=true trimmed_wildcard=true`，仍系统代理 intercept / 400→502。源码根因：Hyper-util 0.1.20 `matcher.rs:445-466` 将 `*` 归入 domains；`:471-483` 对可 parse IP 的 host 仅调用 `self.ips.contains(ip)`，域名 wildcard 不参与 IP。不是 cmd 尾空格或 env 未传入；前述“未生效”仅指功能效果、不代表变量未设置。
  - `bb83a8f77` wildcard 与 `bd7cb0b7d` 显式 IP 对照均 exit 101，但发现诊断副本误共用 `original-target`，实际 exe 出现诊断 subscriber / 改动后的行号；**这两轮均不能作为“未改原包”的通过或失败证据**。源码 63 文件仍相等不证明实际运行 exe 归属。私有 raw SHA256 分别为 `3b69fb466b06c4b579d1c1c181903c259287e4c1d36efc6588abcd0eca4031db` / `ef4e5608f33ce68b69003adc577636468d4f9b56f539887b4c1556e1f19b730f`，不公开含系统代理值的 trace。后续 `bb8028193` 在原目录清理并从真实原源码以原 lock/native-tls/显式 IP bypass 重编；原 100ms 断言保持，干净结果见下。诊断副本今后必须独立 target，不得复用原包 test artifacts。
  - 干净重编 `bb8028193` exit 101：先确认此前 target 消费任务均已终止，`cargo clean -p axum-reverse-proxy` 实际删除 4,851 files / 4.4 GiB（不假称仅删除一个 test exe）；随后 receipt 明确 `Compiling axum-reverse-proxy v2.2.0 (D:\temp-audit\proxyweave-feature3-proxy-audit\axum-reverse-proxy-2.2.0)`，八个实际 test exe 路径/SHA256 已记录。原 lock/native-tls/原断言/100ms 保持，环境仅显式 IP bypass、`CARGO_INCREMENTAL=0`。本轮没有诊断 subscriber；重编后原目录 63 文件仍与发布缓存逐字节相等。
  - 干净原 tests：lib 53、headers 6、policy 7、RFC 9110 25、routing 12、service 2、streaming 1、errors 2 通过，共 **108 passed / 1 failed**。唯一失败 `test_proxy_error_handling`：原 `tests/errors.rs:75:10` 的 `unwrap` 收到 `reqwest::Error { kind: Request, url: "http://127.0.0.1:55164/test", source: TimedOut }`。该原测试同时采用客户总超时 100ms 与“likely not in use”的固定 upstream 59999；干净复现只证明期限内未收到预期 502，不足以断言产品缺陷、代理环境或固定端口是最终根因。不会加大超时、跳过或削弱断言，不宣称原 tests 全绿。
  - 原干净重编 receipt `feature3-original-clean-replay.txt` SHA256 `f8fe25e6c71c39684e56c410f3b0b76d091bc9938ad507bb3e5486407f730f07`；产物归属/版本范围见 `feature3-original-clean-artifacts.json`。此前系统代理的 trace 解释进入代理之前的 400→502，但不能用其解释此干净轮剩余 100ms 失败。其结果与宿主 `--locked` 自定义 connector 的 63 项 Rust regression 分开记录。
- Owned 原生 TCP regression / scoped Clippy：修正 fixture 后 `b216899b8` exit 0：7 passed / 1 ignored（子进程入口实际由 Owned runtime 调用），默认并行；两类原生 TCP 取消、lease 回收与其他断言均通过。fmt check / 三个受影响 crate 的 all-targets Clippy `--no-deps -D warnings` 无警告。原 receipt `feature3-targeted-pass.txt` SHA256 `6b12b7787f5f28b0d01e8d11cc1061c9c5c907fedc8f58f60fc6aaae3e0180da`。
  - `bc91db47f` exit 101：1 个 External regression 通过，6 个 Owned regression 在 fixture Manifest 解析时失败（未启动子进程），1 个子进程入口 ignored；Clippy 未执行。错误均为 `managed configuration must be a JSON object with bounded projections`，定位 `crates/proxyweave-gateway/tests/passthrough.rs:189`。根因是测试 fixture 漏声明严格 Managed 契约必需的 JSON object `config_template`；只为 fixture 增加空 object，没有放宽生产 Manifest 验证或断言。
  - 原始 receipt `feature3-first-fixture-failure.txt` SHA256 `3e6ba95446ec302026af79c062ba8de169aab64afd9cd26ae6e601ba50c0af5d`；修正后默认并行测试与 Clippy 在 `b216899b8` 通过，详见上一条；原失败未删除。
- 工作区默认并行 regression、fmt、frontend、embedded Tauri：`b611aa94d` exit 0：fmt、`cargo test --workspace --locked -j 1`（63 passed / 2 ignored，默认并行测试；两个 ignored 子进程入口实际启动验证）、三个受影响 crate 的 all-targets Clippy `--no-deps -D warnings` 无警告、npm ci、2 Vitest、tsc/Vite、embedded Tauri debug `--no-bundle` 均通过。完整 receipt `feature3-full-pass.txt` SHA256 `c73a76cdf7b0aa5714f49ade218dd6e5e11ab13d23f0ad3698869a6ff769adac`。
  - `b2bd794bd` exit 101：Tauri 编译阶段 `failed to build archive ... libtauri-7214d5b36c34d379.rlib: 磁盘空间不足。 (os error 112)`，完整 tests/frontend 未执行。D 盘仅剩 13,926,400 bytes；workspace target 10.89 GiB、原包 target 5.31 GiB。保留 receipt `feature3-full-disk-failure.txt` SHA256 `fca6a7c7a669170ac396c4e004306dd9792460122382abe7561670bb70fb62fe`。
  - 仅删除本轮 workspace `D:/ProxyWeave/target/debug/incremental` 可重建缓存（3.75 GiB 逻辑占用），未触碰产品数据、原包、审查目录、冻结 exe 或 logs；清理后可用 3,314,511,872 bytes。重跑总闸门 `b611aa94d` 只加命令级 `CARGO_INCREMENTAL=0`，不改 lock/source/生产配置/断言。
  - 隔离 trace 首次编译同样因 `syn` archive `os error 112` 退出，尚无根因 trace；`feature3-original-trace-build-failure.txt` SHA256 `e9ab0db5e9d0b95463f60eb1096771bc8991b42503aa017aea8478bbbf4291b1`。不能把诊断构建失败算原包 tests 结果。
- 主动 LSP 四路径检查：3 个 inconclusive，0 个确认 clean；1 个辅助 ast-grep `avoid-duplicate-export` 指向已有 public module + 根 re-export 约定，session-only defer，不为样式建议破坏原公共 API。以真实 Cargo test / scoped Clippy 为实际验证，不把 LSP 静默当通过。
- 冻结候选 embedded 桌面产物：`D:/temp-audit/proxyweave-feature3-build/quotio-desktop.exe` SHA256 `0b6d457bbf011a44a2b232c91ca1f2fa22d1392e2dbc690ae899260b6087fe32`。该产物仅当前桌面基线构建；F3 crate 尚未由桌面 UI 接入，不作为最终网关 UI/真实插件可用证据。

## 回归证据目标

`crates/proxyweave-gateway/tests/passthrough.rs` 使用真实受 Job 管理的 Rust 测试子进程，不以 External no-op 充作 Owned 证据：

1. missing/wrong/duplicate local Authorization 拒绝且不访问 upstream；真实 fixture 模型 JSON、未知请求字段与原 bytes、query/status/Content-Type/端到端 header 保留；去除 hop-by-hop 与客户 Cookie/Proxy-Authorization，本地 key 不泄漏到 upstream。
2. SSE UTF-8/CRLF/event 边界切分、正常上游 DONE、Content-Length 截断保持 transport error、合法 HTTP EOF 不补 DONE。
3. 客户原生 TCP 在 header wait 断连：upstream socket EOF、lease 归零；streaming 断连：upstream 写失败与 lease 归零，Managed stop 不得中断活跃请求。
4. header timeout、网关端口占用、backend unavailable、多 lease、停止后重新启动的实际端口更新、网关取消/drain。
5. External provider 在网关 shutdown 后仍响应健康请求，无生命周期控制。

## 重放命令与冻结边界

Windows PowerShell（工具链变量仅本命令环境；不改变 Windows 系统代理）：

```powershell
$env:RUSTUP_HOME = 'D:/build-tools/rustup'
$env:CARGO_HOME = 'D:/build-tools/cargo'
$env:PATH = 'D:/build-tools/cargo/bin;' + $env:PATH
$env:CARGO_INCREMENTAL = '0' # 磁盘恢复后的完整验证设置
Set-Location D:/ProxyWeave
cargo fmt --all -- --check
cargo test --workspace --locked -j 1
cargo clippy -p proxyweave-gateway -p proxyweave-plugin -p quotio-platform --all-targets --no-deps --locked -j 1 -- -D warnings
Set-Location D:/ProxyWeave/apps/desktop
npm ci
npm test
npm run build
npm run tauri -- build --debug --no-bundle
```

原包精确隔离命令（原发布目录不可改源码或断言；产物目录与 workspace 分开）：

```powershell
$env:CARGO_TARGET_DIR = 'D:/temp-audit/proxyweave-feature3-proxy-audit/original-target'
$env:NO_PROXY = '127.0.0.1,::1,localhost'
$env:no_proxy = '127.0.0.1,::1,localhost'
Set-Location D:/temp-audit/proxyweave-feature3-proxy-audit/axum-reverse-proxy-2.2.0
# 先确认此 target 无在途任务；诊断副本须使用不同 CARGO_TARGET_DIR。
cargo clean --manifest-path ./Cargo.toml -p axum-reverse-proxy
cargo test --manifest-path ./Cargo.toml --locked --no-default-features --features native-tls -j 1 --no-fail-fast --lib --test headers --test rfc9110 --test streaming --test policy --test routing --test service --test errors -- --test-threads=1
```

源码冻结仅 F3 crate / 现有 runtime lease 胶水 / workspace 配置及溯源 docs，绝不包含 `docs/DEFECTS_LEDGER.md`、`.pi/inspection/**` 或凭据。父 commit `771015be8c5cef291626bf38aa7bcc13c38372ca`；最终 commit/hash 尚待发布。原包检查与诊断对照不表示自测能替代 Inspector 独立复验。

## 明确未通过及未覆盖

真实 wb2api 凭据/model/nonstream/SSE、最终 native Windows 桌面 MCP 点击操作与持久化重开仍待 F4/最终整合；本报告不将 fixture 叫作真实插件 E2E。没有 HTTP2、远程监听、WebSocket/upgrades、跨平台权限、release installer、完整日志安全或自动空闲回收验收。已有 Job assigned-tree 证据不证明 spawn→assign 窗口原子安全。
