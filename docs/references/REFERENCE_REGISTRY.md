# ProxyWeave Reference Source Registry（参考源档案）

版本：v2.4 — 2026-10-14 文档纠偏；当前进度/历史证据层级与未关闭项仅见 [PHASE1_REVIEW_STATUS](../verification/PHASE1_REVIEW_STATUS.md)（源码基线 `3471d077d657adc7e64826bac0a9d09f91d35347`），本表登记来源事实，不将局部 Verified 扩为全项验收。

## 状态纪律

- 来源 Git/许可证核验、源码审计、构建测试、运行行为是不同证据，不能相互替代。
- 已引入桌面/网关/协议等闭包按真实源码登记；未引入候选仍需开发前审计，不沿用“本阶段仅桌面”的历史概括。
- 历史 v2.0 的全绿状态、错误完整 Hash、假设源码路径及未留证据的行为结论不再作为验收依据。
- 直接依赖 → 模块提取并保留测试 → 薄适配 → 有依据修改 → 自研例外 ADR。

## 实际 Git 基线

以下为已固定的本地来源基线；历史 Git/标签/clean 记录与当前代码复用、测试结果分别判断。本次只读再次核对 TiyGate 与 WorkBuddy 源 HEAD/许可证，不将历史“均 clean”宣称为当前所有来源的全面新审计。

| 来源 | 本地路径 | 完整 Commit | 版本/标签 | 本阶段验证范围 |
|---|---|---|---|---|
| Quotio Desktop | `D:/temp-audit/quotio-desktop` | `21d75d08b38a23f97fbd534a1768a829cb147f2c` | `v0.7.7` | MIT LICENSE（版权 2025 Trong Nguyen）已读；实际桌面闭包已审计；独立副本与裁剪后 Windows 构建/测试/行为验证通过，详见 PHASE0.md |
| TiyGate | `D:/temp-audit/tiygate` | `6db40e54663455fb99d675bd8985e4dc50f68a14` | 无 HEAD 标签 | Apache-2.0；IR/protocol 与 Chat/Messages/Responses/error_body 实际闭包已提取，源/目标与原测试 subset 见下；非全部 codecs 或 SQLx业务系统，非全项验收 |
| Tunnel Agent | `D:/temp-audit/tunnel-agent` | `9bfb7779b80de5e205a388e2e02d5840a3ed8b47` | 无 HEAD 标签 | 仅 Git 基线核验；不引入 C#/.NET |
| DeepSeek Harness | `D:/temp-audit/deepseek-harness` | `d743267388641bc76f17c45ce8b4c231aed1d32c` | `dsh-v0.2.1-alpha.2` | 仅 Git 基线核验；不引入 Node 作为宿主核心 |

## M01–M15 候选矩阵

| 编号 | Feature | 来源候选与原路径 | 决策/当前状态 |
|---|---|---|---|
| M01 | Process Supervisor | Win32 Job Object 官方 API (`windows-sys` 0.61.2) | 代码已落地；历史局部与当前整体门禁见唯一状态汇总：基于 Win32 Job Object 实现 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 进程生命周期绑定；External 严禁 Kill。详见 ADR-003 |
| M02 | Port Allocator | 标准库 `std::net::TcpListener` 端口检测 | 代码已落地；历史局部与当前整体门禁见唯一状态汇总：回环端口分配与占用预检；收集明确冲突证据（`AddrInUse`），就绪超时绝不单独推定为端口冲突。详见 ADR-003 |
| M03 | Async Gateway | axum 0.8.9 / axum-reverse-proxy 2.2.0 / Hyper 1 / Tower HTTP | 直接依赖已接入；原包 `D:/temp-audit/proxyweave-feature3-proxy-audit/axum-reverse-proxy-2.2.0`，MIT commit `7350d118a69b6d98ae19a767b9d71630b2a3bc80`；连接所有权/凭据薄适配。同协议 Body 直透与后来异协议 IR 分层，版本变更见 ADR-004；门禁只见当前汇总 |
| M04 | SSE | 原代理 `src/forward.rs` 的 `Body::new(Incoming)`；官方 eventsource-stream 0.2.3 | 同协议原字节/Content-Type/未知字段直透、不合成 DONE；异协议与 Flight 真实接入官方跨分片 UTF-8/SSE parser，源协议完成证据才编码目标终止，异常 EOF/错误不得补成功；原测试证据缺口与 subset 见下 |
| M05 | Protocol IR | TiyGate `crates/core/src/{ir,protocol,routing}/`、`crates/protocols/src/` | 已按闭包提取到 `crates/proxyweave-protocols/`；当前只导出 Chat/Messages/Responses/error_body、IR/protocol 与 routing 局部错误类型，保留适用原测试/snapshots；Gemini/images/embeddings 未引入，禁止宣称全部源 codecs 已提取 |
| M06 | Model Router | TiyGate/LiteLLM/Magpie 设计候选 | 未审计，不预先授权重新实现同质算法；Magpie 路由组五模式与策略 fail-open 语义见竞品参考源补充节，ADR 定案前须显式比较三方 |
| M07 | Credential Broker | Quotio `crates/quotio-platform`；`keyring` 3.6.3；`windows-sys` 0.61.2 | 代码已落地；历史局部与当前整体门禁见唯一状态汇总：Keyring 命名空间隔离与持久化；实现真实 Windows 文件 DACL 限制（阻断继承、独占当前用户），废除空操作。详见 ADR-003 |
| M08 | Installer | Quotio `crates/quotio-core/src/proxy_download.rs` | 下载/校验/解压候选，未引入；后续按范围核验与泛化，不能整体否定 |
| M09 | Config Store | Quotio platform `atomic_write`/`replace_file` 与原测试 | 已读平台文件及对应 Windows/Unix 分支；原子写入不等于双副本容灾或 Windows 凭据隔离 |
| M10 | Manifest Engine | 直接依赖 serde_yaml_ng 0.10.0 / Serde；Quotio atomic_write | Feature 1 严格契约/本地包登记已 Inspector Verified；来源/测试见 ADR-002 与 FEATURE1.md；非运行支持声明 |
| M11 | Lifecycle | DSH 可逆副作用设计候选 | 未审计，后续按实际 Rust 依赖选型 |
| M12 | IDE Handoff | Quotio `agent_config.rs` 为早期候选；实际 WB Switch `crates/wb-switch-core/src/modules/agent_import.rs` / `src/pages/AgentsPage.tsx` | 已提取后端到 `apps/desktop/src-tauri/src/agent_import.rs`、前端页面及最小 UI 闭包；不是只待引入候选。真实源/当前 MIT 快照与历史许可链限制见本表 WB Switch 节，真实客户端调用门禁只见当前汇总 |
| M13 | Desktop Shell | Quotio `apps/desktop/src-tauri/`、React bootstrap | 已直接复用；本轮 Windows 构建/测试与窗口/托盘/关闭/单实例行为验证通过；Feature 4 极简桌面宿主、IPC 胶水、网关/插件/事件管理与测试调用 UI 已实现并通过构建与测试，详见 ADR-005 与 FEATURE4.md |
| M14 | API Doctor | tower-http trace 候选 | 未引入；版本、脱敏契约与测试开发前核验 |
| M15 | Compatibility Flight | 官方 eventsource-stream 0.2.3；已有 Tauri Channel/Flight 源码 | Flight 已落地并有真实 Chat/SSE/多轮局部原生证据，不再称未引入；Tool Calling/长上下文/多模态未测，完整质检/发布回归未通过，见唯一当前状态 |

## 竞品参考源补充（2026-10-18，用户引入，审查员实查登记）

| 编号 | 名称 | 固定 Commit | 定位与可用范围 | 许可证 | 验证状态/边界 |
|---|---|---|---|---|---|
| M16 | Magpie（yetone/magpie） | `d783df46cbac9021e26a6bd71ca0b55544552eb2` | 相邻竞品/重点参考源：管理对象为 Provider/模型/订阅/Agent 配置，与本项目「代理应用管理」定位相邻不重合。可用范围限设计/测试参考：pluginrt 生命周期与回滚语义、Manifest API 兼容范围与服务依赖、路由策略集与 fail-open/fail-closed 分级、网关回归维度、配置保格式编辑行为、deferred usage 计量条约 | MIT（仓库 LICENSE 已读；其依赖含 SQLite/Wails/Bun 等，未逐项审计） | 2026-10-18 已实查 README、go.mod、docs/subsystems 与 internal/pluginrt 等重点包源码并量化各包规模；未做完整构建与全量安全审计。Go 代码不直接复用；其 SQLite 持久化、读取 Agent 签到文件作订阅、Bun 插件宿主三项与本项目红线/凭据契约冲突，明确不采用。详细对比见 [MAGPIE-COMPARISON-2026-10-18](./MAGPIE-COMPARISON-2026-10-18.md) |

## M05 真实提取闭包与保留测试（本次只读核查）

源 `D:/temp-audit/tiygate` HEAD `6db40e54663455fb99d675bd8985e4dc50f68a14` / Apache-2.0。目标 `crates/proxyweave-protocols/LICENSE` 与源 LICENSE 的 LF 文本相等；代码头保留源 commit / SPDX / copyright。源没有 NOTICE，不编造 NOTICE。以下均核对真实文件，不沿用早期假设的 `ir/request.rs` / `response.rs` 或 `core/src/protocol.rs`：实际为目录内 `mod.rs`。

| 真实原路径 → 目标路径 | 保留/必要适配与边界 |
|---|---|
| `crates/core/src/ir/mod.rs` → `crates/proxyweave-protocols/src/ir/mod.rs` | IR 请求/响应/流式类型及适用原单元测试；import/闭包适配留证 |
| `crates/core/src/protocol/{mod,lossy,structured_output}.rs` → `crates/proxyweave-protocols/src/protocol/{mod,lossy,structured_output}.rs` | EndpointCodec/StreamDecoder/StreamEncoder、能力与有损/结构化输出检查，保留原测试；不是任意未知字段无损转换保证 |
| `crates/core/src/routing/mod.rs` → `crates/proxyweave-protocols/src/routing.rs` | 只提取 `ErrorClass` 等错误分类依赖闭包，源完整路由模块未搬入，不能叫 Model Router 全实现 |
| `crates/protocols/src/{chat_completions,messages,responses,error_body}.rs` → `crates/proxyweave-protocols/src/{chat_completions,messages,responses,error_body}.rs` | 三 codec 和协议错误体、原内部 tests/snapshots；必要别名/import/编译 pattern 等适配保留在 source diffs，非 AI 等价重写 |
| `crates/protocols/tests/cross_protocol.rs` → `crates/proxyweave-protocols/tests/cross_protocol.rs` | 适用原测试 subset 56 个可执行 `#[test]`；源 91 functions → 64 retained functions（含 helpers，不是 64 tests） |
| `crates/protocols/tests/streaming_transcode.rs` → `crates/proxyweave-protocols/tests/streaming_transcode.rs` | subset 6 个可执行 `#[test]`；源 26 functions → 18 retained functions（含 helpers，不是 18 tests） |

当前 `crates/proxyweave-protocols/src/lib.rs` 只导出 `ir` / `protocol` / `routing` / `chat_completions` / `messages` / `responses` / `error_body`；不导出源 `gemini.rs` / `images.rs` / `embeddings.rs`。保留测试的旧注释“5×5 / 4×4”是源注释，不代表当前导出全部 codecs。Gemini-only 等未移植函数逐项列于 `.pi/development/protocol-capability-alignment/source/test-scope.json`；不得声称全部源测试已保留或已运行。

- 源副本/适配 diff：`.pi/development/protocol-capability-alignment/source/`（`crates/`、各 codec/原测试 `.diff`、`test-scope.json`）；checkpoint A HANDOFF 与 dependency manifest 为开发来源记录，Inspector identity 为独立冻结关联，两者不混为验收结论。
- 已读保存日志 `.pi/development/protocol-capability-alignment/checkpoint-a/receipts/b566c1ce5.output`：提取后 148 unit + 56 cross_protocol + 6 streaming = **210 passed**；网关 1+7+5+10=23 passed / 1 ignored，插件 protocol_validation 2 passed，是 scoped Developer subset 历史自测。未运行完整 TiyGate 上游 workspace/all-codecs suite，记 **NOT RUN**；完整宿主全项也未由该日志关闭。
- 本次不运行上述命令、不复制日志结果为伪新测试。历史独立 HTTP matrix **120 passed / 4 failed**，Messages 客户端 → Chat/Responses 后端的 SSE tool/usage 四失败未获关闭 receipt；当前冻结风险与复验条件只见 [PHASE1_REVIEW_STATUS](../verification/PHASE1_REVIEW_STATUS.md)。
- 2026-10-17 B1 修复注记（不改写历史门禁结论）：4 项失败的唯一级联根因定位为 `crates/proxyweave-protocols/src/messages.rs` 的 `MessagesStreamEncoder`——Chat/Responses 上游并行工具调用按 index 交错发送参数分片，而编码器对 `name=None` 分片一律写入 `current_index`（“当前打开的块”），导致晚到分片被错绑到后开的工具块，重组 JSON 非法，进而使 Inspector 的解析器抛异常、`tool_ids` 与 `usage` 同时报空（usage 实际已随终止 `message_delta` 发出）。修复方式为按工具 `id` 惰性绑定块索引（`tool_index_by_id`）并延迟 `content_block_stop` 至终止帧按序 flush（`open_tool_blocks` + `flush_tool_block_stops`）。回归：`cargo test -p proxyweave-protocols --locked` 149+56+6 全过；复制的 `verify-matrix.cjs`（以修复后工作区 `crates/` 构建的 harness）跑出 **124 passed / 0 failed**，四项原失败转 PASS。原始 Inspector 冻结副本与 `matrix-results-r2.json` 未改动；本注记不构成 Inspector 独立 Verified，仅记录开发侧根因与自测结果。
- 未引入 `crates/store`、SQLx、计费 DB 或服务完整业务；当前 workspace/gateway/protocol Cargo 闭包无该依赖。Host/Glue/插件业务职责不因提取而混合。

## M04 / M15 官方 SSE parser 真实直接依赖与测试出处

- `eventsource-stream` **0.2.3**，`MIT OR Apache-2.0`，本地发布源码 `D:/build-tools/cargo/registry/src/rsproxy.cn-e3de039b2554c837/eventsource-stream-0.2.3`，开发保存副本 `.pi/development/flight-backend/eventsource-stream-0.2.3/`。原 `src/utf8_stream.rs`、`src/parser.rs`、`src/event_stream.rs` 负责跨 HTTP chunk UTF-8 缓冲和 SSE 事件解析，不重写算法冒称复用。
- 当前网关 Cargo 精确 `eventsource-stream = "=0.2.3"`；桌面 Cargo 为 `"0.2"`，实际锁 `0.2.3`，registry checksum `74fef4569247a5f429d9156b9d0a2599914385dd189c539334c625d8099d90ab`。`crates/proxyweave-gateway/src/lib.rs` 与 `apps/desktop/src-tauri/src/flight.rs` 实际导入 `eventsource_stream::Eventsource` / 调用 `.eventsource()`；网关仅异协议解析，Flight 解析真实网关 Chat SSE，同协议网关依然原 Body 直透。旧自建解析历史不能冒充现在接入的官方实现。
- 官方原单元测试在 `src/event_stream.rs`（`valid_data_fields` / `spec_examples`）与 `src/utf8_stream.rs`（`valid_streams` / `invalid_streams`），`src/lib.rs` 另有文档示例；`src/parser.rs` 无 `#[test]`，不虚构 `populate_fields` 测试；原包保留。**证据冲突**：旧 HANDOFF/FLIGHT-BACKEND 声称修改 dev 依赖 ntapi 到 0.4.1 后 5/5 通过；所引 `.pi/development/flight-backend/original-crate-audit/original-locked-failure.txt` 实为 ntapi 0.3.6 的 `error[E0793]: reference to field of packed struct is unaligned`；`original-tests-after-ntapi-update.txt` 实为 ntapi 0.3.7 同错且编译未通过。未核到可支持 5/5 的保存成功日志；不把原测试标全绿，完整原 suite **NOT RUN / 未获完整通过证据**，本次不重跑或改其未授权历史报告。
- `.pi/development/flight-backend/owned-tests-output.txt` 保存 **18 Flight owned tests passed**（仅该 subset，不运行固定共享 Keyring 四 tests）；后续原生 Flight receipt 为 Chat/SSE/多轮局部，Tool Calling/长上下文/多模态 UNTESTED。来源测试、移植后自测与原生验收三层不可相互替代。
- 异协议仅源协议正常完成证据才编码目标终止；异常 EOF/错误绝不补成功。parser 正确切帧不保证 codec 工具/usage 语义全部兼容，同协议直透也不保证所有 SDK 对缺 terminal EOF 都报告失败。

## 当前 WB Switch 许可快照与 IDE 模块来源事实

只读本地真实源 `D:/workbuddy-switch-gateway/workbuddy-switch-gateway-latest` HEAD `ebd9427b350a866108a8787714e8cd54fd7ed33a` 的 LICENSE 为 MIT（Copyright 2026 wb-switch）。实际 `apps/desktop/src-tauri/src/agent_import.rs` 已有该来源的 Claude 配置生成器提取与 ProxyWeave 备份/原子写入/模型槽位适配；保存原源/diff 位于 `.pi/development/claude-protocol-integration/source/` 与 `.pi/development/protocol-capability-alignment/source/wb-agent_import.*`，开发质量日志见其 HANDOFF。**旧 GPL 禁复制判断与当前 MIT 快照不同**：本次同步已发生的模块来源事实，不追认历史许可链全安全、不宣称整个项目商业法务已通过、不更改产品安全嵌入政策或扩大新的提取授权。实际客户端配置写入能力不等于 Claude/Codex/Grok 原生调用已验收；具体 fail-closed/未验边界见当前状态汇总。

前端实际来源为同一固定源 `src/pages/AgentsPage.tsx` 及必要 CSS/UI primitives/assets 闭包，当前 `apps/desktop/src/components/ide-source/ide-provenance.md` 只读登记，源副本/bytes SHA 在 `.pi/development/ide-source-page/reference/` 与 `reference-sha256.json`。后端实际原文件为 `crates/wb-switch-core/src/modules/agent_import.rs`。本次仅核查来源和授权事实，不读实际用户 IDE 配置；快照 NOTICE 的历史授权主张不是独立法律证据。

## 竞品参考源补充（2026-10-18，用户引入，审查员实查登记）

| 编号 | 名称 | 固定 Commit | 定位与可用范围 | 许可证 | 验证状态/边界 |
|---|---|---|---|---|---|
| M16 | Magpie（yetone/magpie） | `d783df46cbac9021e26a6bd71ca0b55544552eb2` | 相邻竞品/重点参考源：管理对象为 Provider/模型/订阅/Agent 配置，与本项目「代理应用管理」定位相邻不重合。可用范围限设计/测试参考：pluginrt 生命周期与回滚语义、Manifest API 兼容范围与服务依赖、路由策略集与 fail-open/fail-closed 分级、网关回归维度、配置保格式编辑行为、deferred usage 计量条约 | MIT（仓库 LICENSE 已读；其依赖含 SQLite/Wails/Bun 等，未逐项审计） | 2026-10-18 已实查 README、go.mod、docs/subsystems 与 internal/pluginrt 等重点包源码并量化各包规模；未做完整构建与全量安全审计。Go 代码不直接复用；其 SQLite 持久化、读取 Agent 签到文件作订阅、Bun 插件宿主三项与本项目红线/凭据契约冲突，明确不采用。详细对比见 [MAGPIE-COMPARISON-2026-10-18](./MAGPIE-COMPARISON-2026-10-18.md) |

## 本阶段模块溯源与验证

已从 Quotio `21d75d08b38a23f97fbd534a1768a829cb147f2c` 分步引入与裁剪；原路径、目标路径及必要修改见下表。原样引入 238 文件的 SHA-256 历史快照为 `../verification/phase0-introduction.json`（不是当前文件校验清单）。

| 原路径 → 目标路径 | 复用范围与修改 | 验证 |
|---|---|---|
| `apps/desktop/src-tauri/src/{lib,main}.rs` → 同路径 | 直接提取窗口命令、单实例首注册、托盘菜单/图标/定位；去除 AppCore、业务 IPC、更新/后台采集；新增 native-close 隐藏胶水、托盘恢复复用 unminimize | 原样与提取后 Rust check/build/test 已通过；最终独立前端嵌入与桌面行为见报告 |
| `apps/desktop/src/main.tsx`、`components/AppShell.tsx`、CSS → 同路径 | 保留启动揭示计时器、原窗口按钮和关闭/最小化函数/对话框；本地偏好命名空间 ProxyWeave；状态页及小面板为薄胶水 | 裁剪后 2 frontend tests + Vite build 通过 |
| `crates/quotio-platform/src/lib.rs` → 同路径 | 通用路径/原子写入/备份/命令/Keyring 封装和原测试；命名空间调整，按调用闭包删除 CPA/Kiro/Dream-Skin 资源及专用账号常量 | Windows 原 6 测试保留；最终裁剪回归见报告；ACL/Keyring 未验收 |
| `crates/quotio-types` → 同路径 | 暂保留原类型及 7 测试，不凭类型名盲删 | 7/7 原测试通过；后续按依赖渐进裁剪 |
| Cargo/npm/Tauri/Vite/TS 配置与图标 → 同路径 | 内部 crate 名保留；展示名/独立 ID 调整；移除资源包/更新器/业务依赖并提交锁文件；图标暂保留上游资产 | 各步骤构建及最终冻结证据见报告 |
详细过程与日志证据见 `../verification/PHASE0.md`；准备成功或 metadata 成功不能登记为构建通过。

## Feature 1 直接依赖与登记胶水（已独立 Verified）

| 成熟来源/版本 | 发布源码 Commit | 原路径 → 目标使用 | 许可证/必要适配 |
|---|---|---|---|
| serde_yaml_ng 0.10.0 | 45bce7bd7efb754ffc9dc910f3d1bae7edd0adf4 | 发布 `src/de.rs`/`src/mapping.rs`、`tests/test_error.rs` → 直接 crate 依赖 | MIT；不改源码，严格类型/映射预检胶水；原测试Rust1.99兼容lint仅独立审计使用 |
| sha2 0.10.9 | 82c36a428f8d6f05f3bfccdedb243e9d1f85359d | RustCrypto `sha2` → 直接依赖 SHA256 | MIT OR Apache-2.0；无散列算法重写 |
| semver 1.0.28 | 7625c7aa3f0e8ba21e099d1765bcebcb72aa8816 | Version::parse → 直接依赖 | MIT OR Apache-2.0 |
| url 2.5.8 | d6ea13c5f8e7e6e627f6390161b3e185bda5e5ce | URL origin/parser → 直接依赖 | MIT OR Apache-2.0 |
| tempfile 3.27.0 | 5c8fa12eb584931b4f1bccfde87eb72fbfa7dc61 | 原 TempDir/Builder及原测试 → staging目录 | MIT OR Apache-2.0 |
| 已采用 Quotio platform | 21d75d08b38a23f97fbd534a1768a829cb147f2c | `atomic_write`/`replace_file` → 当前同路径直接调用 | MIT；登记非敏感，原6测试保持，非Windows ACL验收 |

以上 Commit 来自各发布包 `.cargo_vcs_info.json`，非补写短Hash。原发布代码/测试在独立副本实跑，结果及失败归因见 `../verification/FEATURE1.md`；测试执行中不作提前通过声明。产品契约/边界胶水位于 `crates/proxyweave-plugin/`；对照 Hermes固定源因BSL许可证及IDE业务契约不匹配未提取，见 ADR-002。wb2api候选exe/buildinfo/dirty差异见同ADR；无真实凭据/Ready/推理验收。

## 本地插件与隐私边界

| 资产 | 本地路径 | 本阶段状态/约束 |
|---|---|---|
| wb2api | `D:/workbuddy2api-panel` | 真源 HEAD `84614197edb08068cd9dabf471b0ebef2a27af0b` / MIT；历史隔离导入与真实局部调用存在，不再称仅未探测候选。本次未编译/启动/读 auths；现成 exe 旧 buildinfo/dirty 与 freeze 关联限制见当前状态与 ADR-002 |
| 已有 CPA | `D:/CLIProxyAPI_7.2.115_windows_amd64` | 本地已有二进制，28317 未于此轮探测；不能推断无公开源码或专有许可证。作为 External 接入时禁启停禁 Kill |
| 历史 REF / 当前 WB Switch 源 | `D:/workbuddy-switch-gateway/workbuddy-switch-gateway-latest` | 旧 GPL 禁复制指令是历史边界；当前固定 HEAD `ebd9427b350a866108a8787714e8cd54fd7ed33a` LICENSE 为 MIT，实际 IDE 来源提取事实见上。当前快照 MIT 不证明历史许可链全安全，不恢复旧禁令为新代码事实，也不作全面法律结论 |
| 用户真实账号/配置 | 用户私有目录 | 本阶段无需读取，不读取/复制/提交真实凭据 |

Managed/External 由进程所有权决定，不由源码可得性决定。仓库 Manifest 语法/契约、真实包登记与真实运行是不同层级；不能再笼统称全部只是草案，也不能由导入成功推出完整运行。Unix `0o600` 与 Windows ACL 分开验证，具体状态仅见汇总。

## UI-001 单独修正：直接 opener 依赖（待独立 Inspector UI 验收）

| 成熟来源/版本 | 发布源码 Commit | 实际直接使用 | 许可证/验证边界 |
|---|---|---|---|
| tauri-plugin-opener 2.5.4 | 5c7668b6bb7c9a509f394d584568b3a922161e50 | Rust `open_url(url, None::<&str>)` → backend 校验当前端点；不注册 JS plugin | MIT OR Apache-2.0；原路径单元测试已读，构建/本产品 URL 策略测试分别留证；不代表真实浏览器/原生 UI 通过 |
| open 5.4.1（官方 opener 原有依赖） | 发布包源码已读；未推测 Commit | Windows `ShellExecuteExW`，不调用 cmd 拼 URL | MIT；原 Windows tests 与许可证已读；系统浏览器加载与后续导航不由宿主保证 |
| Tauri 2.11.2 / tauri-runtime-wry 2.11.2 / Wry 0.55.1（已有锁定依赖） | 实际 Cargo registry 源码核查 | main-frame initialization / WebView2 子帧注入边界 | 不引入 iframe/child WebView、不扩 CSP/capabilities；详见 ADR-006 |

既有 Quotio React shell/PluginPanel 和原生 HTML button 直接复用，无新浏览器框架/JS opener 依赖。可选静态 management 字段无 pluginID 特例；原包与导入记录不改变；声明能力不冒充实测。此 UI-001 不更改上列历史 Feature 状态，不恢复完整 Goal；详细安全依据、旧 fingerprint 与质量门见 [ADR-006](../architecture/ADR-006-plugin-navigation-management.md) 及 developer 冻结交付。
