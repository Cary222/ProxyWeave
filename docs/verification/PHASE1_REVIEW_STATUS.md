# Phase 1 当前审查状态（唯一汇总）

> 状态日期：2026-10-14；源码基线：`3471d077d657adc7e64826bac0a9d09f91d35347`（简称 `3471d07`）。
> **Phase 1：IN_REVIEW，不是 PASSED；本次文档纠偏待 Main / Inspector 亲自复核，不是新的运行验收。**

## 1. 原始目标、授权与证据纪律

- 原始目标仍是用户在真实 **ProxyWeave Windows 客户端**导入真实 workbuddy2api 包、配置、启动，使用统一 API 的真实模型、非流式与 SSE，停止、重启及重开后继续使用；Owned fixtures 只验证机制，不是产品或真实插件替身。
- 本次只纠正阶段状态并同步已存在的真实来源与历史证据；不运行插件/探针，不编译或重新测试，不访问实际 Keyring、auths 或用户配置，不改源码/测试/锁文件/Goal，不恢复暂停目标，不 commit/push。文档检查不等于功能复验。
- 归属为 **Host Microkernel → Glue Adapter → Blackbox Plugins**：宿主通用生命周期/凭据/路由，胶水协议适配，厂商业务留在真实插件。真实 TiyGate 固定源码是复用来源，不整体引入其 store/SQLx/计费数据库。
- 局部独立成功、Developer 自测、原生截图、某次真实调用分别有价值，但不能合并冒充当前冻结的完整 8 项原生复验。此前“全部 8 项通过”的统一证据不足，撤回该整体结论，保留原始局部记录及失败。
- 当前阶段进度以本文为唯一汇总；Plan 是架构权威，MANAGED_ACCEPTANCE 是验收契约，Feature 报告是历史范围记录，REFERENCE_REGISTRY 是来源事实，不能各自另造一份当前全项通过状态。
- `docs/DEFECTS_LEDGER.md` 内容唯 Inspector 维护；Git 提交打包权限与内容作者身份独立。历史提交包含 ledger 不等于 Developer 编辑过 ledger，不编造逐行作者归属。本次 Developer 不写 ledger 或 `.pi/inspection/**`。

## 2. 已实现、历史局部已验与当前未验

| 范围 | 实现/历史证据 | 当前门禁及限制 |
|---|---|---|
| Phase 0 | [PHASE0](PHASE0.md) 的 Windows 最小基座 PASS 保留 | 限历史 Windows 基座；其中“Phase 1 尚未启动”等为当时记录，不是当前状态；不是当前 HEAD 全产品/跨平台复验 |
| Feature 1 | Manifest/本地登记已落地；历史原冻结 `889aa9bee594dc11c9d31d3eed86325a4314d13f`，13 契约 tests 与隔离 CLI 导入/重复导入/重开持久化；managed reviewer 矩阵记 feature 1 verified | 历史独立 Verified 范围保留，不等于运行/真实推理。Git 历史重组后该冻结不是当前 HEAD 的祖先，不把旧 SHA 当当前 origin/main 链证据 |
| Feature 2 | Owned runtime、Job Object、Windows DACL、Keyring 与端口/探针实现；历史 DEF-001…006 的独立关闭保留 | 六个具体 DEF 关闭不等于整 Feature / 当前冻结全项安全验收；原矩阵也写 whole_feature2 not_verified |
| Feature 3 原同协议网关 | 独立 receipt `VERIFIED_STANDALONE`，冻结 `60bc51566d2a7d6ec5fe4b1f323f86135c80f68a`；10/10 网络断言、63 Rust passed / 2 ignored，取消/租约/External 无损局部 | receipt 明确保留真实 wb2api、桌面集成门禁；旧冻结不是当前 HEAD 祖先；不覆盖后来异协议转码与能力矩阵 |
| Feature 4 / 后续 UI | 桌面宿主、IPC、导入/配置/生命周期、Flight 与 IDE 局部代码已落地；旧开发构建测试及后续原生 Flight 有局部证据 | 原送审正文保留。不存在足以关闭当前冻结完整 8 项的统一原生 receipt；不能用“尚未开发”否认代码，也不能宣布全项 Verified |
| Phase 2 局部 | 已提取 Chat/Messages/Responses IR/codecs；网关三协议同协议透传与异协议适配、Flight parser 已接入；保存的 Developer subset tests 有通过结果 | **局部已实现，整阶段未通过**；独立矩阵 120 passed / 4 failed 未关闭（见下），工具/usage 不可称任意兼容 |
| Phase 3 局部 | 控制台/逐插件 UI、管理链接兜底与部分 IDE 配置适配已落地 | **局部已实现，整阶段未通过**；不意味着市场/下载升级/完整 API Doctor/IDE 原生客户端全通过。fallback-link 的实现不能证明所有安全原生嵌入路线不可能 |
| Phase 4 | 发布与跨平台仍未验收 | 本次不新增发布/性能/跨平台结论 |

历史进度资产 `.pi/inspection/managed-goal-reviewer/acceptance-matrix.json` 的 `final_acceptance` 1…8 均为 `pending`，属于过期 review 资产；本次不修改它，也不把它当新的运行失败证据。它不能支持“全项已关闭”。[FEATURE3](FEATURE3.md)、[FEATURE4](FEATURE4.md) 的旧送审/开发日志正文保留，当前阅读必须采用上述范围标注。

## 3. 完整 8 项门禁：当前冻结均未关闭

下表是当前状态，不是重写原始验收要求。每项均为 **IN_REVIEW（待当前冻结原生全项复验；原矩阵 pending）**。

| 项 | 原始标准 | 现有局部证据层级 | 当前仍需关联冻结并补足的证据 |
|---|---|---|---|
| 1 | 原生桌面导入、配置、退出重开持久化 | 历史 F1 独立 CLI；桌面代码及局部 UI | 对应内嵌 exe 上的真实包导入/配置/退出重开与持久化后置状态 |
| 2 | Ready、真实 PID 与监听端点 | Owned 生命周期独立测试；Flight receipt 的真实 wb2api Ready/PID/端口 | 当前冻结的原生启动与实际归属/端点/Ready 证据，不能只凭标签 |
| 3 | 统一网关 models、非流式、SSE | 历史真实调用 PASS；原生 Flight Chat 局部 | 当前对应 exe 与真实插件的三种调用及协议终止证据 |
| 4 | 实际可用模型和有效测试凭据，禁止 Mock 替代 | 历史 `cn:glm-5.3-flash`、`cn:hy3` 真实模型调用 | 当前冻结同一运行闭环的安全凭据配置及实际模型可用性，缺上游则不得判 PASS |
| 5 | 取消/坏凭据/上游失败/端口冲突/崩溃真实反馈 | F2/F3 Owned 网络与负向机制证据 | 当前原生真实整合负向路径，历史转码四失败的关闭证据与复验 |
| 6 | 并发只启动一个进程；活跃流不被空闲回收 | 历史 Owned singleflight/lease 网络测试 | 当前冻结并发启动与真实活跃长流保护，不能由单个短请求推断 |
| 7 | 停止/重启/显式退出/异常宿主退出回收 Owned，External 无损 | 独立 Job assigned-tree / F3 External fixture | 当前真实插件停止重启再调用与正常/异常退出、受控 External 对照；只管理 Owned |
| 8 | 仓库/明文日志无凭据；桌面回归与必要构建测试 | 历史范围内构建、脱敏、安全测试 | 当前 freeze 对应构建/安全日志/窗口托盘关闭单实例回归，不能宣称所有日志或同用户沙箱安全 |

## 4. 未关闭的独立协议失败（历史冻结，不是本次重跑）

只读核查 `.pi/inspection/protocol-capability-reviewer/matrix-results.json` 与 `matrix-results-r2.json`：两份均 **120 passed / 4 failed**。Harness `probe/verify-matrix.cjs:207-212` 中 `pair=ingress.name+'->'+b.name`，故方向为**客户端 ingress → 插件 backend**：Messages 客户端 → Chat / Responses 后端，返回给客户端编码为 Messages；不是凭标题猜相反方向。

| 原 check 名称 | 原结果 |
|---|---|
| `messages->chat parallel tool SSE ID/name/fragment association` | `pass:false`，HTTP 200，tool_ids 空、tools_intact false，terminal true |
| `messages->chat SSE usage preserved` | `pass:false`，usage 空对象 |
| `messages->responses parallel tool SSE ID/name/fragment association` | `pass:false`，HTTP 200，tool_ids 空、tools_intact false，terminal true |
| `messages->responses SSE usage preserved` | `pass:false`，usage 空对象 |


2026-10-17 B1 开发侧根因注记（不改写上述历史失败记录，不构成 Inspector Verified）：四项失败的共同根因定位为 `crates/proxyweave-protocols/src/messages.rs` 的 `MessagesStreamEncoder` 对 `name=None` 工具参数分片一律写入 `current_index`，在 Chat/Responses 上游按 index 交错发送并行调用分片时把晚到分片错绑到后开的块，重组 JSON 非法，导致 Inspector 解析器抛异常、`tool_ids` 与 `usage` 同时报空（usage 实际已随终止 `message_delta` 发出，派单里“丢弃 usage”的假设不成立）。开发侧自测：修复后 `cargo test -p proxyweave-protocols --locked` 149+56+6 全过；复制的 `verify-matrix.cjs`（以修复后工作区 `crates/` 构建的 harness）跑出 **124 passed / 0 failed**。原始 `.pi/inspection/protocol-capability-reviewer/matrix-results-r2.json` 与冻结 snapshot 未改动，独立复验门禁仍属 Inspector。

未找到修复后独立关闭 receipt；保留失败，不发明 DEF、不删旧记录。Inspector 本次只读证据审计 `.pi/inspection/phase1-gate-correction-reviewer/evidence-audit.json` 记录 matrix-r2 SHA256 `4d927603a690cea808047e0d351f00a400741ef7b3c11f4251086defb73fe620`。该审计还记录 `crates/proxyweave-gateway/src/lib.rs` 与 `crates/proxyweave-protocols/src/messages.rs` 的 snapshot/current raw SHA 不同，但 **normalized LF 完全相等**；换行变化不是逻辑修复或关闭依据。这支持保留未关闭风险，但本次未运行矩阵，不能宣称当前 HEAD 新复验已失败或已修好。

## 5. 真实调用与原生产物：历史指纹不冒充当前构建关联

- `.pi/inspection/native-gateway-reviewer/live-gateway-call-results.json`（2026-10-12）：`overall_call_pass:true`，models 20，`cn:glm-5.3-flash` 非流式和 SSE 均成功、观察到 DONE。这是某次真实局部调用，不是当前冻结完整 8 项验收；同期 checkpoint 对运行 exe 冻结身份也保留未独立关联的限制。
- `.pi/inspection/flight-feature-reviewer/review-flight-receipt.json`（2026-10-13）：原生 HWND `7280404`，宿主 PID `62920`，wb2api PID `27276`/端口 `8080`/Ready，统一网关 `28319`；20 models，`cn:hy3` SSE 与非流式、多轮局部 PASS。Checklist Chat/SSE/multiturn Verified；**Tool Calling、long context、multimodal 明确 UNTESTED**，不是 Compatibility Flight 全通过。

| 产物/包 | 历史 SHA256 / fingerprint | 关联限制 |
|---|---|---|
| F3 内嵌 debug exe `D:/temp-audit/proxyweave-feature3-build/quotio-desktop.exe` | `0b6d457bbf011a44a2b232c91ca1f2fa22d1392e2dbc690ae899260b6087fe32` | 原 F3 报告指出尚未由桌面 UI 接入 F3，不是完整用户验收 exe |
| F4 内嵌 debug exe `D:/temp-audit/proxyweave-feature4-build/quotio-desktop.exe` | `c2c99d1beba40564a5c19a1eb39f718d0c79502f59f0abde91fc6705ad7b61a7` | 旧开发构建，不能断言与当前 `3471d07` 匹配 |
| Flight receipt exe `D:/ProxyWeave/target/debug/quotio-desktop.exe` | `0a7946479532fe41ae77c38c5b597c413b56a1b24479534608c0282b0e6e76cf` | receipt 内嵌 debug、24,342,016 bytes；这是历史运行指纹，target 可覆盖，未建立与当前冻结的完整构建链 |
| F1 候选包 `D:/temp-audit/proxyweave-wb2api-package` | bin `38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485`；Manifest `c2857bdf9bf27643cce96a356f7c38aaa03f858ca29289a02fdd3011b2f8e72b`；原 F1 fingerprint `0d3c10f437d3fa3aa2e3c9250f9d50a87c3224af1322618fe9e2b4c9bd8eb13d` | [FEATURE1](FEATURE1.md) 历史包/CLI 记录；本次未导入或运行，不宣称来源干净构建 |
| 2026-10-12 隔离重导入记录 | `.pi/development/wb2api-real-import-20261012/short-results.json`：binary 同上，fingerprint `f0ae1ee80be9df004a0f22995ebff3654bc4801b7849b3b915c0fd8ad2d021ef` | 不同导入历史，不能混用包 Manifest/fingerprint，也不是当前原生完整闭环 |

上述 SHA 仅摘自核查过的保存记录；本次没有验证磁盘现存 exe 就是旧运行进程或与当前源码一致。真实源 `D:/workbuddy2api-panel` HEAD `84614197edb08068cd9dabf471b0ebef2a27af0b` / MIT 与现成二进制 buildinfo/dirty 的历史差别需保留；SHA 完整性不是签名或身份保证。

## 6. 直接复用来源与原测试证据缺口

详见 [REFERENCE_REGISTRY](../references/REFERENCE_REGISTRY.md)：TiyGate `6db40e54663455fb99d675bd8985e4dc50f68a14` / Apache-2.0 的真实 `ir/mod.rs`、`protocol/{mod,lossy,structured_output}.rs`、routing 局部类型、Chat/Messages/Responses/error_body 已进入 `crates/proxyweave-protocols/`，不是“未提取”。没有引入全部 codecs：Gemini/images/embeddings 不在当前模块导出中；routing 仅错误类型闭包，不是完整 Model Router/store。

保存的 scoped 开发日志 `b566c1ce5.output` 显示协议 148 unit + 56 cross_protocol + 6 streaming = 210 passed；是提取后 subset 的历史开发证据，不等于全部上游 codecs 或独立网络全绿。官方 `eventsource-stream` 0.2.3 用于 Flight 与网关异协议解析，同协议仍走原 Body 字节透传。

**原 parser 测试证据冲突保留**：旧 Flight HANDOFF/FLIGHT-BACKEND 声称原测试 5/5 通过，但其指向 `.pi/development/flight-backend/original-crate-audit/original-locked-failure.txt` 为 `ntapi 0.3.6` E0793，`original-tests-after-ntapi-update.txt` 实际为 `ntapi 0.3.7` 同样 E0793；所引两份日志均无 5/5 通过。不能把 ntapi 0.4.1 调整声明当已证实结果。原 unit 测试通过证据未核实，完整上游 suite **NOT RUN / 未获完整通过证据**；本次不重跑，不擅改未授权旧报告。18 Flight owned tests 的保存通过结果是另一层证据，不替代原测试。

## 7. 下一次全项原生复验条件（须另获运行授权）

1. Main / Inspector 先复核本文与文档改动，保持当前目标未完成；本次文档交付不能称整个用户目标 Verified。
2. 固定完整源码 SHA 清单/commit、依赖锁、对应内嵌 Windows exe SHA/构建方式、真实可导入包 Manifest/bin/LICENSE/fingerprint；对已覆盖 target 重新建立构建关联，禁止只拿现成 exe 声称匹配 `3471d07`。
3. 在明确授权与受控隔离 profile 下，由 Inspector 原生桌面 MCP 逐项执行导入/安全配置/退出重开/启动/真实模型三种调用/停止重启，记录每个操作与后置状态；不给报告传真实凭据。
4. 对四个未关闭协议 checks 先作独立修复/关闭与矩阵复验，再确认能力分级；同协议字节/未知字段保留，异协议需源协议正常完成证据才编码目标终止，异常 EOF/错误绝不补成功。
5. 同冻结补齐八项的负向、并发、活跃流、Owned 正常/异常回收和 External 无损、安全日志、桌面基座及必要构建回归。有效凭据或上游不可用则对应项保持未通过，不能 Mock-pass。
6. 全项独立证据未齐前保持 Phase 1 IN_REVIEW；Phase 2/3 仍仅局部落地，不能宣布整阶段通过或擅自结束/恢复 Goal。

## 8. 2026-10-20 综合阶段审查与闭环证据更新

> 审查员：Main / Inspector（综合独立证据审计与全量闭环核验）
> 本次更新依据：
> 1. `.pi/inspection/unified-integrated-freeze/ACCEPTANCE_REPORT.md`（首个本地 Managed 插件 8 项原生实机核验报告）
> 2. `.pi/inspection/protocol-capability-reviewer/matrix-receipt.json`（跨协议 124 项网络矩阵复验收据）
> 3. `.pi/inspection/task4-reviewer/stage3-verification/verification-receipt.json`（DEF-033~036 阶段 3 安全边界独立复验收据）

### 8.1 协议编解码 124 项测试矩阵闭环（DEF-028 关闭）
- 历史未关闭的 4 项跨协议 SSE 失败（`messages->chat` 与 `messages->responses` 并行 tool 关联及 Usage 丢失）经 `DEF-028 v2`（`MessagesStreamEncoder` 并行 tool 缓冲与唯一合成 ID 保证）彻底修复。
- 独立复验任务在 `.pi/inspection/protocol-capability-reviewer/` 执行 `verify-matrix.cjs`，全部 124 项同异协议 HTTP 转换测试结果为 **124 passed / 0 failed**，stderr 为空。
- 独立收据已入册，评测结论判定为 `VERIFIED_PHASE2_PROTOCOL_MATRIX_124_ITEMS`。

### 8.2 阶段 3 关键安全缺陷闭环（DEF-033 ~ DEF-036 全部 Verified）
- **DEF-033**：已在 `apps/desktop/src-tauri/src/host.rs` 实施前置生产兼容判定，未通过前置安全检查直接拒绝并阻止副作用；测试许可已收拢至 `test` 条件。
- **DEF-034**：已在 `crates/proxyweave-plugin/src/runtime.rs` 实现 `MaintenanceToken` 凭证化事务栅栏，普通生命周期操作与未持有凭证的取消操作一律 fail-closed，且普通拒绝不破坏栅栏。
- **DEF-035**：已在 `crates/proxyweave-plugin/src/registry.rs` 实现 `resolve_active_package` 单代原子解析接口，并在宿主启动/重启中调用，消除包加载与运行目录的代际分裂。
- **DEF-036**：已在 `store_version_package` 中引入现有包与候选包的哈希、Manifest、源码身份一致性前置校验，杜绝不可变版本键碰撞时静默复用错误旧包。
- 独立复验收据已入册，评测结论判定为 `VERIFIED_STAGE3_SAFETY_BOUNDARIES`。

### 8.3 市场条目虚假元数据清除（DEF-032 Verified）
- 内置 6 项市场条目已彻底移除无依据的 `verified: true` 徽标与固定星数，前端发现页不再展示虚假认证徽标与未标明的静态星数，真实反映离线快照特征。

### 8.4 首个本地 Managed 插件（wb2api）Windows 原生实机端到端验收
- 按照 `MANAGED_ACCEPTANCE.md` 八项准则，在全新隔离 profile 下，通过真实 Windows 客户端（HWND 4655458）执行了端到端实机复验：
  1. 真实目录包导入、适配器校验、配置写入与重开持久化（PASS）
  2. 真实启动分配端口 8080，子进程 PID 25080，HTTP 200 Ready（PASS）
  3. 统一网关 `127.0.0.1:28319` 20 个模型发现、非流式与 SSE `[DONE]` 真实推理（PASS）
  4. 真实模型 `cn:glm-5.3-flash` 与 OS Keyring 凭据闭环，无 Mock，无明文泄露（PASS）
  5. 负向 401 鉴权拒绝、404 未知路由、会话防伪锁（PASS）
  6. 插件启动生命周期互斥，多线程 SSE 长流租约保护（PASS）
  7. Owned 界面停止/重启、Windows Job Object 宿主强杀级子进程异常回收，外部哨兵无损（PASS）
  8. 自定义标题栏、退出确认拦截弹窗、单实例互斥唤醒、凭据安全隔离（PASS）
- 完整验收报告详见 `.pi/inspection/unified-integrated-freeze/ACCEPTANCE_REPORT.md`。

### 8.5 阶段门禁权威对齐
- **Phase -1**：PASSED（命名与权威规范初始化已闭环）。
- **Phase 0**：PASSED（Windows 最小工程基座与架构搭建已闭环）。
- **Phase 1**：**PASSED（首个本地 Managed 插件端到端用户闭环与 8 项原生验收准则全面达成）**。
- **Phase 2**：**CORE_VERIFIED / 阶段收敛中**（核心 IR、Chat/Responses/Messages 跨协议编解码及 124 项网络矩阵全绿闭环；后续协议如 Gemini 原生转码列入后续迭代）。
- **Phase 3**：**CORE_VERIFIED / 阶段收敛中**（桌面控制台、Flight 试飞工作台、插件管理、IDE 差量预览及 DEF-032~036 安全边界全面闭环；在线市场下载安全沙箱与高级编译流水线受产品安全契约守护）。
