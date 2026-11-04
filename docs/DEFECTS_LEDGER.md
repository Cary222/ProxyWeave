# ProxyWeave 缺陷台账

本台账由 Inspector & Dispatcher 独占维护。只有独立冻结复验的数据支持才建立 DEF；开发自测、静态疑点或功能未开发不是 Verified。修复须交付冻结 commit、可复现步骤及证据，审查端独立复验后才更改状态。

## 审查基线及证据

- F2 冻结提交：`506980f370b23214192d7e912002fde61051fbdd`。
- 独立快照：`.pi/inspection/f2-lifecycle-reviewer/snapshot/`，由 `git archive` 解包。归档 SHA256：`8965e029726114600da65be7d80ffe0b91c161206ae50a228652d7be19089e3f`。
- 探针：`.pi/inspection/f2-lifecycle-reviewer/probe/src/main.rs`，直接依赖冻结 crate，不修改业务代码；全部子进程、数据、随机回环端口均归审查测试所有。
- 可复现命令及限制：`.pi/inspection/f2-lifecycle-reviewer/README.md`。
- 主要结果：`.pi/inspection/f2-lifecycle-reviewer/evidence-r2/results.json`，构建及执行 task `b60b9913d`。
- Job 异常宿主退出证据：`.pi/inspection/f2-lifecycle-reviewer/evidence/job-abnormal-exit.json`，task `b4ce00af5`。
- 第一轮 `evidence/results.json` 的 crash fixture 未实际退出，**不**作为崩溃缺陷证据；第二轮明确 TerminateProcess 审查自建 PID，并查询 OS 确认退出，才用于 DEF-001。
- 探针人工构造预校验 PluginRecord，哈希占位值仅用于 Runtime 测试，不覆盖 Registry 完整性，也不替代真实 workbuddy2api 凭据/模型或原生桌面 MCP 验收。

## 缺陷索引

| ID | 严重度 | 状态 | 摘要 | 负责人 |
|---|---|---|---|---|
| DEF-001 | P1 | Verified / 独立复验通过 | 已退出的 Managed 子进程仍 Ready，后续 start 复用死 PID | Developer |
| DEF-002 | P1 | Verified / 独立复验通过 | 健康检查忽略 Manifest 身份契约，错误服务被判 Ready | Developer |
| DEF-003 | P1 | Verified / 独立复验通过 | Starting 不可见且 stop 无法协调在途启动 | Developer |
| DEF-004 | P2 | Verified / 独立复验通过 | max_port_retries 被忽略，明确冲突后无恢复且状态仍 Stopped | Developer |
| DEF-005 | P2 | Verified / 独立复验通过 | Managed 就绪探针丢失身份失败原因，最终仅报告通用超时 | Developer |
| DEF-006 | P2 | Verified / 独立复验通过 | 凭据投影递归解释替换值，改变原始凭据；Debug 含敏感字段 | Developer |
| DEF-007 | P2 | Verified / 独立复验通过 | 前端 PluginPanel 访问 `record.manifest.ownership` 与真实嵌套结构 `record.manifest.runtime.ownership` 不符，Managed 插件被误渲染为 External | Developer |
| DEF-008 | P1 | Verified / 独立复验通过 | Keyring Windows 密码存储对长凭据抛出 TooLong 错误，阻止真实 OAuth JWT 存储 | Developer |
| DEF-009 | P1 | Verified / 独立复验通过 | 新隔离目录内真实 wb2api 包发布登记因 Windows 目录重命名瞬时权限拒绝失败；成熟重试机制修复后原生验证通过 | Developer 01a12b6d |
| DEF-010 | P1 | Verified / 独立复验通过 | 重新生成 Master Key 后旧 Key 仍获鉴权、新 Key 被拒绝；已实现 LocalAuth 动态更新并验证 | Developer & Bugfix Lead |
| DEF-011 | P2 | Verified / 独立复验通过 | Keyring 写入失败被忽略，仍返回成功并记录已持久化；已实现原子失败与显式错误传播并验证 | Developer & Bugfix Lead |
| DEF-012 | P2 | Verified / 独立复验通过 | 历史 set_password 的 UTF-16LE 条目被按 UTF-8 回读为含 NUL 的字符串，网关拒绝绑定；已实现 legacy ASCII 检测与官方双读兼容并验证 | Developer 01a12b6d |
| DEF-013 | P1 | Verified / 真实端到端推理与原生桌面 MCP 独立复验通过 | 网关停止、未发推理请求仍生成假模型回复、随机 TPS 和四项兼容通过；已实现原生双向 Channel 流式与非流式推理并实机验证 | Developer & Feature Lead |
| DEF-014 | P2 | Verified (Containment) / 止损修复独立复验通过（真实 IDE 写入仍待实现） | IDE 未写配置却显示成功；剪贴板拒绝仍显示已复制，缺失 Key 时替用虚构值 | Developer（UI 真值修复） |
| DEF-015 | P1 | Verified / 独立复验通过 | GitHub 只读安全沙箱预检与清单解析（`inspect_project_from_github`）及前端 `#inspectResult` 结构化呈现已完整实现并验证通过 | Developer 01a18152 / Inspector 复验 |
| DEF-016 | P2 | Verified (Containment) / 止损修复独立复验通过（真实请求追踪仍待实现） | 空事件输入仍显示并导出四条虚构 HTTP 请求链路 | Developer（UI 真值修复） |
| DEF-017 | P2 | Verified (Containment) / 止损修复独立复验通过（系统自启/托盘设置仍待实现） | 设置仅本地状态却显示系统功能已开启，未探测凭据仍声称已校验/就绪 | Developer（UI 真值修复） |
| DEF-018 | P2 | Verified / 独立复验通过（前端与 IPC 缝合已验证，原生 HWND 回归于全量集成阶段核验） | 最小化/最大化为无操作 Promise；关闭 IPC 失败被吞掉 | Developer（Wave 3 Shell 真值修复） |
| DEF-019 | P2 | Verified / 独立复验通过 | 宿主读取失败被显示为零登记/空事件与网关 OFF，成功缓存被失败覆盖 | Developer（Wave 3 Shell 真值修复） |
| DEF-020 | P1 | Verified / 独立复验通过 | 转码请求将本地 x-api-key 与 Connection 指定头传至上游；已实现严格敏感头与提名头剥离并验证 | Developer & Feature Lead |
| DEF-021 | P1 | Verified / 独立复验通过 | 转码 SSE 缺少协议完成证据时在 HTTP EOF 补发 message_stop；已移除异常截断补帧并验证保持真实 EOF | Developer & Feature Lead |
| DEF-022 | P2 | Verified / 独立复验通过 | 转码流对每个字节分片独立 lossy UTF-8 解码改变文本；已引入成熟跨 chunk 增量解码并验证跨分片中文字符完好 | Developer & Feature Lead |
| DEF-023 | P2 | Verified / 独立复验通过 | 转码路径未应用 header_timeout 导致慢上游绕过网关超时限制；已实现 504 Gateway Timeout 超时与租约释放并验证 | Developer & Feature Lead |
| DEF-024 | P1 | Verified / 独立复验通过 | 转码路径未调用 check_lossy_conversion 静默丢弃能力；已接入协议层有损能力损失校验并验证返回 400 | Developer & Feature Lead |
| DEF-025 | P1 | Verified / 独立复验通过（块顺序维度；同源空 id 塌陷另行登记 DEF-028） | Messages 流编码器把 content_block_stop 延迟到流末尾，混合块(tool→text→tool)输出违反内容块顺序，块在未关闭时即 start；缓冲发射修复后 12 案顺序探针与 3×3 矩阵同时通过 | Bugfix 01a12e7e |
| DEF-026 | P2 | Verified / 独立复验通过（原生桌面仍待阶段门禁整合） | 插件卡片对 Managed 插件固定显示 `idle_timeout · 300s`，与 manifest 声明 `idle_timeout_ms: null` 及 runtime 无回收器均不符；UI 已适配三态 | Developer 01a12b6d |
| DEF-027 | P2 | Verified / 独立复验通过（原生桌面仍待阶段门禁整合） | Flight 取消失败或后端返回 `cancelled:false` 时仍显示「已取消试飞推理」；已实现状态保留与错误呈现 | Developer 01a12b6d |
| DEF-028 | P1 | Verified / 独立复验通过（124 项跨协议 HTTP 矩阵与顺序/锁死探针全通） | Messages 流编码器按 id 归并工具调用，空 id 的并行工具调用被塌成单块且 arguments 变成非法 JSON 拼接；已实现合成唯一 ID 与歧义匿名调用即刻 fail-closed 锁死 | Bugfix 01a12e7e / Inspector 矩阵复验 |
| DEF-029 | P1 | Verified / 冻结本地边界独立复验（非真实网络/原生市场验收） | Release `.exe` 资产名含相对上跳或绝对路径时可写出暂存目录 | Developer 01a15d05 / Inspector 冻结复验 |
| DEF-030 | P1 | Verified / 冻结本地边界独立复验（非真实网络/原生市场验收） | 无 Manifest 的任意文件被赋予虚构 MIT/1.0.0/Chat/health 能力并成功注册 | Developer 01a15d05 / Inspector 冻结复验 |
| DEF-031 | P2 | Verified / 冻结本地边界独立复验（非真实网络/原生市场验收） | Release 所选插件 ID 与包内 Manifest ID 不符仍注册另一插件 | Developer 01a15d05 / Inspector 冻结复验 |
| DEF-032 | P2 | IN_REVIEW / 修复存在，待隔离补验与集成 | 内置条目未执行认证却全部 verified，详情显示官方/社区认证；固定版本和星数未标快照；已彻底移除虚假认证并接入只读来源存证与更新检查 | Developer / Inspector 隔离补验中 |
| DEF-033 | P1 | Verified / 冻结本地边界独立复验通过 | CompatibilityUnknown 拒绝前置阻断，测试许可隔离于不可达生产边界 | Developer 01a180b6 / Inspector 冻结复验 |
| DEF-034 | P1 | Verified / 冻结本地边界独立复验通过 | MaintenanceToken 保护维护栅栏，普通启停阻断不清除栅栏，仅持有者可退出 | Developer 01a180b6 / Inspector 冻结复验 |
| DEF-035 | P1 | Verified / 冻结本地边界独立复验通过 | resolve_active_package 单代原子返回记录与版本目录，消除混代 | Developer 01a180b6 / Inspector 冻结复验 |
| DEF-036 | P2 | Verified / 冻结本地边界独立复验通过 | store_version_package 核验证书与指纹，同 key 歧义拒绝冲突包 | Developer 01a180b6 / Inspector 冻结复验 |

### DEF-020 — 转码将本地鉴权及 Connection 指定头泄漏给上游

- 冻结生产源：`D:/ProxyWeave/crates/proxyweave-gateway/src/lib.rs` SHA256 `d730882d11f435d48049103c43cf2dc0d337c27937875a036781511f29917843`，逐字副本 `D:/ProxyWeave/.pi/inspection/f3-transcoding-reviewer/snapshot/`；154 个 Windows source dependency 版本与产品锁匹配。当前 dirty 源快照，不冒充 Git commit 冻结。
- 定位：`D:/ProxyWeave/crates/proxyweave-gateway/src/lib.rs:480-497` 的转码头复制未过滤 `x-api-key`，也未解析并剔除 `Connection` nominated headers。同协议 forward 此前已有部分过滤，不能据此宣称转码路径同样安全。
- 独立复现：自建 HTTP External fixture，使用唯一 dummy 本地鉴权与不同的 dummy 上游鉴权。向真实 Gateway listener 发 POST `/v1/messages`，同时发送 `Connection: x-owned-canary`。转码上游收到本地 `x-api-key`，亦收到被 Connection 指定的 `x-owned-canary`；独立上游 Authorization 正确不改变前两者泄漏事实。HTTP 200。
- 证据：task `b7183af64` exit 0；`D:/ProxyWeave/.pi/inspection/f3-transcoding-reviewer/results.json`（SHA256 `f537155d1c65845dce9e09ecf4c39c180e83956c4ab03ddd9a0555fd18d8442d`）中 `local_key_reached_upstream=true`、`nominated_header_reached_upstream=true`、`upstream_auth_is_separate=true`。无真实密钥/插件/用户 Keyring 操作，不宣称已有实际凭据外泄。
- 修复验收：从成熟代理的完整头过滤机制直接复用/薄适配，所有数据面路径剥离本地鉴权、proxy credentials、Cookie、标准 hop-by-hop 与 Connection 指定头，再加入独立上游凭据；重复或歧义本地鉴权准确拒绝，安全的普通扩展头保留。冻结后正向 dummy 转码/透传对照、两种本地鉴权、Connection 指定鉴权、错误/取消路径均复验。不以移除测试 canary 通过。

### DEF-021 — HTTP EOF 被转码补成成功结束

- 冻结与依赖、任务及结果文件同 DEF-020。定位 `D:/ProxyWeave/crates/proxyweave-gateway/src/lib.rs:706-719`：`if saw_terminal { ... } else { let done = encoder.encode_done(); ... }`。`D:/ProxyWeave/crates/proxyweave-protocols/src/messages.rs:1308-1311` 的 encode_done 产生 `event: message_stop`。
- 独立复现：上游 HTTP 200 `text/event-stream` 仅发送 Chat 内容增量 `partial-owned`，无 finish_reason、无 `[DONE]`，随后正常 HTTP EOF。客户端 Messages 接收到 partial 后仍出现 `event: message_stop`。HTTP transport 结束不是协议完成证据。
- 证据：上述 results.json `truncated.contains_partial=true`、`truncated.synthesized_success_stop=true`，完整 dummy wire body 留档。脚本 exit 0 表示缺陷数据采集成功，不是转换通过。
- 修复验收：直接复用已审计 SSE 状态机与终止判定，区分协议终止和 HTTP EOF；未确认正常完成时不能发 message_stop/response.completed/[DONE]，应准确暴露异常截断或协议失败。涵盖 clean EOF、缺长度中断、畸形事件、error 事件、内容后取消、finish_reason 与终止帧不同组合、三目标协议以及真实 lease 释放；保留正常流完成，不用补帧掩盖错误。

### DEF-022 — UTF-8 跨分片被替换字符破坏

- 冻结与依赖、任务及结果文件同 DEF-020。定位 `D:/ProxyWeave/crates/proxyweave-gateway/src/lib.rs:642`：`frame_buf.push_str(&String::from_utf8_lossy(&bytes));`，每个 HTTP body frame 独立解码。
- 独立复现：上游合法 JSON SSE 增量 `你好🧵`，在 `你` 三字节的第一字节后分两次发送，中间 100ms；完整上游字节有效 UTF-8。转码客户端增量成为 `���好🧵`，`utf8.intact=false`、`utf8.replacement_character=true`，HTTP 200。此分片用于测试数据抵达，不是状态等待轮询。
- 修复验收：成熟字节层 SSE/parser/增量 UTF-8 解码机制直接复用，不按 chunk lossy 解码或手写等价解析器；任意分片保留中文/emoji/工具 JSON 参数的准确语义，CRLF/注释/多行与边界缓冲有界。非法 UTF-8 明确失败而非静默改文；在三协议转码、正常/异常终止、取消与无终止 EOF 回归下复验。

### DEF-023 — 转码路径未应用 header_timeout 导致慢上游绕过网关超时

- 冻结源与版本环境同 DEF-020。定位 `D:/ProxyWeave/crates/proxyweave-gateway/src/lib.rs:509`：`let upstream_resp = match self.client.request(upstream_req).await` 未套用任何 `tokio::time::timeout(self.header_timeout, ...)`。
- 独立复现：自建 HTTP External fixture，故意将响应头延迟 1100ms；网关显式配置 `header_timeout: 400ms`。转码请求在耗时 1111ms 后仍返回 HTTP 200，而不是触发有界超时的 HTTP 504 Gateway Timeout。
- 证据：task `b32d05d2d` exit 0；`D:/ProxyWeave/.pi/inspection/f3-transcoding-reviewer/additional-results.json` 中 `header_timeout.status=200`、`header_timeout.elapsed_ms=1111`、`header_timeout.timeout_ignored=true`。
- 修复验收：直接复用有界异步超时机制对所有转码模式的上游头等待实施 `self.header_timeout` 限制；上游超时时向客户端返回协议规范的 504 Gateway Timeout（带格式化错误正文），断开连接并正确释放请求租约。

### DEF-024 — 转码路径未调用 check_lossy_conversion 静默丢弃不兼容能力

- 冻结源与版本环境同 DEF-020。定位 `D:/ProxyWeave/crates/proxyweave-gateway/src/lib.rs:400-440`：解码得到 `ir_request` 后直接进入 `egress_codec.encode_request(&ir_request)`，完全未调用协议层已提取的 `check_lossy_conversion` 校验。
- 独立复现：向 Responses 接口发送包含 HostedTools（`web_search_preview`）的请求，后端为不支持 hosted tools 的 Chat 协议；上游收到的请求体完全缺少 `tools` 字段，网关未报错，静默丢弃能力返回 200。
- 证据：task `b32d05d2d` exit 0；上述 `additional-results.json` 中 `hosted_tools.status=200`、`hosted_tools.contains_hosted_tool=false`。
- 修复验收：在转码请求编码前完整调用 `check_lossy_conversion(&ir_request, egress_protocol, egress_caps)`；对于不兼容的能力（HostedTools、CustomTools、不支持的媒体类型等）按契约返回 400 Bad Request（带具体有损维度说明），禁止静默丢弃关键字段。保留同协议透传的不受限路径。

### DEF-019 — 宿主读取失败被转换为成功空态和网关关闭

- 冻结：`apps/desktop/src/components/AppShell.tsx` SHA256 `5d705eec80354e4fd7f71ce2ba7035f0aafba7fbab3ff63a17d7182ca6b93914`；`apps/desktop/src/components/Overview.tsx` SHA256 `6b61925e212098d3de711533e1d18733529098a25fa07e1bf3b2fe1b875104b4`。副本 `.pi/inspection/ui-honesty-reviewer/shell-refresh-baseline/`。
- 定位：`apps/desktop/src/components/AppShell.tsx:61-70` 的 refreshData 将 getGatewayStatus 错误变 null、listPlugins/getHostEvents 错误变空数组并写入确认状态；`apps/desktop/src/components/AppShell.tsx:99` 无论局部读取失败仍 setDataReady(true)；`apps/desktop/src/components/AppShell.tsx:343-349` 未传 Overview 的 dataReady。`apps/desktop/src/components/Overview.tsx:32` 默认 dataReady=true；`apps/desktop/src/components/Overview.tsx:55-58` 将 null gateway 渲染“网关空闲”；`apps/desktop/src/components/AppShell.tsx:169-170` null gateway 的侧栏状态为 OFF。
- 独立复现：所有读取 IPC 受控拒绝时，概览显示 **0 个**登记、网关空闲、暂无事件；先受控读取成功显示 **1 个**审查 fixture（并非真实插件），再令周期刷新 IPC 全部拒绝，登记被清成 **0 个**，侧栏为 **OFF**。结果并非宿主确认无插件或网关停止。
- 证据：task `be3043498` terminal exit 0；`.pi/inspection/ui-honesty-reviewer/shell-refresh-baseline/results.json`、`reproduce-shell-refresh.cjs`。AppShell/Overview/api 原字节冻结；其它页面为 render-only stubs，invoke 是审查 dummy seam。没有真实 IPC/用户数据/凭据/原生窗口操作，不能用此证明宿主实际丢失登记。
- 修复契约：首次读取和后续刷新明确区分 loading/unknown/error/confirmed-empty；失败不覆盖最后确认插件/事件/端点为成功空值，保留缓存时须准确标过期/读取失败，不假称运行中或已停止。概览、侧栏、详情共享准确读取状态；部分错误仅影响相应数据，成功数据仍可用。防止旧轮刷新覆盖新轮及组件卸载回写；关闭所有插件后的已确认空结果要清掉旧 statuses，不能因缓存保护永不更新。通过 owned 延迟/拒绝/恢复/空数据检查，并从冻结嵌入产物原生 MCP 验证可理解错误提示。与 DEF-018 在同一 Shell 独占 writer 阶段，不并发修改。
- 修复独立复验：冻结副本 `.pi/inspection/ui-truth-shell-reviewer/snapshot/`（`AppShell.tsx` SHA256 `648ac0ed9ed7c843f03067466444767ca295cc2ac3a3d2fac670b40f357fb220`、`Overview.tsx` SHA256 `3780bd41f560a6198d70a17ff7e838868699b3992a049adcf05e891bb748de7a`）。task `b28b4fb19` exit 0（Vitest 7 suites / 80 tests 全绿，含 `ui-truth-shell.test.tsx` 8 项与 `ui-truth-shell.browser.test.mjs` 9 项真实 Chromium 回归，Vite 生产构建 0 错误）；task `b6677d9aa` exit 0 生成三主题概览与关闭弹窗 4 张截图并经亲自审阅（`.pi/inspection/ui-truth-shell-reviewer/review-receipt.json`）。验证首次拒绝展示“状态未知/读取网关状态失败”、轮询失败保留最后确认数据并展示 `(过期)` / `(刷新失败)` 横幅、成功空数组如实展示 `0 个` 并清理旧状态、局部资源失败隔离、`generationRef` 乱序世代守卫、以及 `Overview` 路线图 100% 权威对齐 `Plan.md`（移除虚构 `95%` 进度）。DEF-019 据此标记 Verified。

### DEF-018 — 窗口控制 API 无实际操作或吞掉失败

- 冻结：`apps/desktop/src/api.ts` SHA256 `74969f44ea5a4965e18b551829cd05be4933bd7b144512541c510c72bf48a57c`；副本 `.pi/inspection/ui-honesty-reviewer/window-api-baseline/api.ts`。
- 定位：`apps/desktop/src/api.ts:61-63` 的 `closeWindow: () => invoke<void>("quit_app").catch(() => {})`、`minimizeWindow: () => Promise.resolve()`、`maximizeWindow: () => Promise.resolve()`。`apps/desktop/src/components/AppShell.tsx` 的窗口点按钮直接调用这些 API。
- 独立复现：将冻结 API 打包，唯一 invoke 替换为 owned 计数且拒绝的 seam；minimize/maximize await 完成时调用 **0 次**；close 发 `quit_app` 被拒绝后仍 resolve。没有引入任何官方窗口 API，这两个接口实际未实施窗口控制。
- 证据：`.pi/inspection/ui-honesty-reviewer/reproduce-window-api.cjs`、`window-api-baseline/results.json`。此为未改 API 的受控执行，不是原生窗口截图；未关闭/最小化用户客户端。
- 修复契约：从保留的 Phase 0 成熟 Tauri Window API/原关闭偏好流程直接复用，不重写窗口壳。最小化/最大化有真实对应动作，关闭/托盘/退出语义遵循现有确认偏好及 Owned 清理边界，拒绝准确反馈、不吞掉。先独占 Shell/api 文件并冻结，自动化测试仅 mocks，再用独立 embedded binary 原生 MCP 操作证实 HWND 状态/窗口行为与 tray 回归；不可把按钮 resolve 当成功。
- 修复独立复验：冻结副本 `.pi/inspection/ui-truth-shell-reviewer/snapshot/`（`api.ts` SHA256 `464473c717b861442201363d5992a1580ab662a4afa8cdc06399fd5684b60524`）。task `b28b4fb19` exit 0 与 task `b6677d9aa` exit 0 验证：窗口控制点采用 `<button type="button">` 并具备 `aria-label`（最小化/最大化/关闭）与键盘激活支持；`api.ts` 直接复用 `@tauri-apps/api/window` 的 `minimize`/`toggleMaximize`/`hide` 及 `quit_app`/`show_menubar`，彻底移除空 `Promise.resolve()` 桩函数与 `.catch(() => {})` 吞错；自定义关闭确认弹窗支持取消、最小化到托盘、退出程序及记住偏好（`localStorage`），底层拒绝时在顶部与弹窗内展示 `role="alert"` 错误横幅。DEF-018 前端与 IPC 缝合标记 Verified（原生 Windows HWND 实机回归将在嵌入式构建阶段一并核验）。

### DEF-015 — 无实际检查却显示仓库清单与哈希校验通过

- 冻结：`apps/desktop/src/components/DiscoverPanel.tsx` SHA256 `a43ae31ed393f3b5dffdca8e50d196bfdaa032f6287736081dbf82fa531c29c5`；副本 `.pi/inspection/ui-honesty-reviewer/snapshot/components/DiscoverPanel.tsx`。
- 定位：`apps/desktop/src/components/DiscoverPanel.tsx:31-38` 的 handleParseRepo 只等待 400ms 再 `setShowParseResult(true)`；`apps/desktop/src/components/DiscoverPanel.tsx:131-148` 固定 ollama-bridge 清单及 `security: { sha256: 9f2c…a41b ✓ 校验通过 }`。页面后续“确认安装”有真实 import IPC，不改变此前假检查事实。
- 独立复现：输入 `not-a-github-url` 并点“解析清单”；IPC **0 次**、外部网络阻断，仍显示 ollama-bridge 0.3.1 与哈希校验通过。不得把“社区未核验”标签当作固定哈希已校验的免责。
- 证据：task `b203d16cf` terminal exit 0；`.pi/inspection/ui-honesty-reviewer/additional-results.json`、`reproduce-additional.cjs`、`additional-source-identity.json`。未真实克隆/安装，不是原生或 GitHub 功能验收。
- 修复契约：保留设计布局；非法 URL 不检查/安装，输入变化使旧结果失效。只有真实 inspect/import 返回的身份和完整校验语义才可展示，不可伪造 Manifest、来源、官方核验、stars 或哈希。无独立检查 IPC 则明确尚未实现，不把导入写操作偷换成只读检查；保留已实现的真实导入路径并据实际返回反馈，失败不显示成功/刷新登记。固定示例不可作为实际可安装仓库。真实安全检查未完成仍登记为 Goal 缺口。
- 止损修复独立复验：task `bc1c1c804` exit 0（4 项断言通过，9 张三主题截图亲自审阅无溢出/报错），`.pi/inspection/ui-truth-wave2-reviewer/results.json`。空 URL 禁用安装；非法 URL 触发校验反馈且不展示假 `9f2c...` 校验通过，解析清单按钮标明未实现。此复验仅关闭“假清单与校验止损”，真实仓库只读清单解析与沙箱预审功能尚未实现，不能视为 UI Goal 全部完成。
- 闭环修复独立复验：Developer 01a18152 实施并在本轮通过全量复验。后端在 `apps/desktop/src-tauri/src/host.rs` 引入 `inspect_project_from_github`，采用 `tempfile::Builder` 在系统临时沙箱执行 `git clone --depth 1` 并调用 `proxyweave_plugin::detect_project` 只读解析清单（零写入、零持久化、退出即销毁临时目录），严格校验空串、破折号与控制字符注入；前端 `DiscoverPanel.tsx` 激活 `parseRepo` 按钮，并在调用成功后渲染 `#inspectResult` 结构化预检卡片（包含模型、协议、权限声明、指纹与独立进程免责提示），支持「确认克隆并安装」与「清除预览」。`desktop_host.rs` 4 项测试通过，前端 Vitest 20 个测试套件（201 项测试）全部通过，生产构建零错误。DEF-015 止损与后续功能缺口已彻底闭环并关闭。

### DEF-016 — 审计页展示并导出不存在的请求记录

- 冻结：`apps/desktop/src/components/AuditPanel.tsx` SHA256 `62cb648209ce748fea8f5b166742bda384dce4ab5ab4fcaf8f4515a3539a7ce3`。
- 定位：`apps/desktop/src/components/AuditPanel.tsx:23-95` 固定 DEFAULT_TRACES；`apps/desktop/src/components/AuditPanel.tsx:98` 默认选中虚构 trace；`apps/desktop/src/components/AuditPanel.tsx:114-115` 导出固定 traces 和实际 host events；`apps/desktop/src/components/AuditPanel.tsx:168` 固定渲染。包含未执行的 Chat→IR→Messages、TTFT、HTTP 状态与 SSE 断开归因。
- 独立复现：提供 `events=[]`，页面仍 **4 条** HTTP 请求；点击导出，截获 Blob JSON 含 **4 条 traces、0 条 host events**。不是实际网关审计数据。
- 证据：同 task `b203d16cf`、上述 additional-results.json 的 audit 字段；截获下载与空 props，未保存任何用户请求正文/凭据。
- 修复契约：真实空输入必须空态，host event 不能冒充 HTTP request trace，事件类型/来源准确。导出仅实际数据，不导出 prototype traces；请求追踪尚未实现则明确，不合成时延、协议转换、归因或“已脱敏”证据。静态诊断建议须标为一般说明，不宣称实际已执行 Fallback/Doctor。导出操作成功反馈不超过真正下载可观察边界。保留表格/详情布局，真正请求审计接口另行实现验收。
- 止损修复独立复验：task `bc1c1c804` exit 0（`.pi/inspection/ui-truth-wave2-reviewer/results.json`）。空事件输入如实渲染 0 条事件记录与空态提示，无虚构 HTTP 请求与 100% 成功率，常见状态码明确标为“静态参考 · 未接入实时诊断”。此复验仅关闭“假链路与指标止损”，真实实时 HTTP 请求链路追踪尚未实现，不能视为 UI Goal 全部完成。

### DEF-017 — 设置显示未经读取或持久化的系统状态

- 冻结：`apps/desktop/src/components/SettingsPanel.tsx` SHA256 `e3373226a8b9a2462dc006d19c2253cd3f611c826c86eec33530459d4f589dd3`。
- 定位：`apps/desktop/src/components/SettingsPanel.tsx:67-68` autostart/trayHide 默认 true；`apps/desktop/src/components/SettingsPanel.tsx:409` 仅 setAutostart；`apps/desktop/src/components/SettingsPanel.tsx:422` 仅 setTrayHide；`apps/desktop/src/components/SettingsPanel.tsx:445-450` 未检测即固定“永不落盘明文”“已校验”“就绪”。配置路径/备份健康同为固定呈现，尚未做运行时独立诊断。
- 独立复现：开机自启初始“已开启”，点击为“已关闭”，重载恢复“已开启”；全程 IPC **0 次**。点击 Keyring“查看”，仍零 IPC 却出现“已校验/就绪”。此证据证明 UI 未检查，不证明真实 OS 自启状态或 Keyring 实际坏/好。
- 证据：task `b203d16cf`、additional-results.json 的 settings 字段。未访问系统启动项/用户凭据/真实配置文件。
- 修复契约：真实设置必须读取确认状态，写成功才更新、失败不污染持久化和后续草稿；不支持的自启/托盘/语言功能应标未实现/未知，不把主题本地偏好混称系统配置。Keyring 检测不存在就准确未知；禁止“永不落盘明文”绝对声明，敏感插件投影的受限文件不等于从不落盘。实际数据路径/备份需有真实接口才能声称健康或打开目录；不能用 toast 冒充 OS 操作。保留设计分区和可用的真实主题回调；禁用止损不等于设置功能全量交付。
- 止损修复独立复验：task `bc1c1c804` exit 0（`.pi/inspection/ui-truth-wave2-reviewer/results.json`）。开机自启标明“未实现”，隐藏到托盘标明“面板未直连”，移除了绝对的“永不落盘明文”声明，保留了主题配色实时调色与重置。此复验仅关闭“假系统配置止损”，系统级自启与完整设置尚未实现，不能视为 UI Goal 全部完成。

### DEF-013 — 试飞未请求模型却展示成功回复与虚构指标

- 冻结文件：`apps/desktop/src/components/FlightPanel.tsx` SHA256 `88d31c91e339f0ffa711a7af6dfb3c6dd45456ee96517dc3b67216b76ee85ffa`；副本 `.pi/inspection/ui-honesty-reviewer/snapshot/components/FlightPanel.tsx`。
- 定位：`apps/desktop/src/components/FlightPanel.tsx:27-29` 初始示例指标；`:58` 仅 models 探针且吞掉错误；`:63` `setTps((Math.random() * 20 + 40).toFixed(1));`；`:70` 固定“模型返回了验证测试流”；`:215-254` 四项固定通过。
- 独立复现：向未修改冻结组件提供 `is_running:false`，提交 dummy 输入；IPC 调用 **0**，仍显示上述成功回复、随机 TPS、四项兼容通过。真实模型/Chat/SSE 未执行。
- 证据：task `bb5b17a0c` terminal exit 0；`.pi/inspection/ui-honesty-reviewer/results.json`、`reproduce.cjs`、`source-findings.json`。这是已知缺陷复现成功，不是功能验收通过。初轮 `b15b84c2b` esbuild `Invalid alias name: "../lib/tauri"` 为审查 harness 配置错误，排除；修复只改变审查脚本，不改产品组件。
- 边界：owned Chromium 组件交互，不是原生 Windows 视觉/真实插件推理验收；使用虚构输入，未访问用户 Keyring、模型服务或原始 auth 文件。
- 修复契约：保留原型布局/三主题，移除虚构会话、别名、指标和未测通过。停止/未知/IPC 错误不允许假成功；只能由真实推理内容/时序/usage 支持回复与指标；models 探测不能冒充 Chat/SSE。若推理接口尚不存在，明确未实现并阻止发送，把真实推理功能列为 UI Goal 未完成项；禁用仅止损，不等于全量交互完成。冻结后独立错误路径复验，再以原生 MCP 与真实模型验收。
- 止损修复独立复验：task `bd16f560d` exit 0（13 项检查通过，6 张三主题截图亲自审阅无溢出/报错），`.pi/inspection/ui-truth-wave1-reviewer/results.json`。`unknown`/`stopped`/`running` 下 `#sendBtn` 均禁用，零会话、指标全为“未测试”、6 项兼容全为“未测试”，IPC 调用 **0**。
- 完整功能与原生桌面 MCP 终极独立复验：
  1. 后端实现：`apps/desktop/src-tauri/src/flight.rs` 引入 mature 开源库 `eventsource-stream 0.2.3`（MIT/Apache-2.0 经审计），实现 `flight_list_models`、`flight_chat_stream`、`flight_chat_nonstream` 及 `flight_cancel` 原生双向 Tauri Channel 接口。单元与集成测试 18/18 全绿（`cargo test -p quotio-desktop --lib -j 1 flight`），Clippy 0 warnings。
  2. 前端实现：`apps/desktop/src/components/FlightPanel.tsx` 完整对接 Channel 事件流，支持动态模型发现、流式与非流式切换、温度滑块、实时流式 Markdown/推理解析、TTFT 首字延迟、TPS 与总耗时统计、运行中主动取消（`#cancelBtn`）以及真实 Compatibility 状态同步更新。前端测试 8 个套件 92/92 全绿（`npm test`），生产打包 0 错误（`npm run build`）。
  3. 原生桌面实机验收（Nuphus MCP 操作）：在真实嵌入式客户端 `quotio-desktop.exe`（PID `62920`，HWND `7280404`）上，拉起托管插件 `wb2api`（PID `27276`，监听 `8080`）与统一网关（端口 `28319`）。通过原生 MCP 导航至“试飞工作台”，实测执行：
     - 模型发现：动态发现上游 20 个真实可用模型，成功选中 `cn:hy3`。
     - 轮次 1（流式 SSE）：提问 `1+1=?`，实时收到增量思考过程与终值 `1 + 1 = 2`，实测 TTFT 1112ms、TPS 29.0 tok/s、总耗时 1345ms。
     - 轮次 2（非流式推理）：关闭流式开关，提问 `2+2=?`，完整收到思考与终值 `2 + 2 = 4`，总耗时 1669ms。
     - 兼容性清单真实点亮：右侧 Compatibility Flight 依据真实试飞结果动态更新，成功通过 `✓ 1 · Chat`、`✓ 2 · SSE`、`✓ 3 · 多轮映射`。
  4. 证据凭据：截图证据已归档至 `.pi/inspection/flight-feature-reviewer/live-flight-stream-success.png` 与 `live-flight-stream-and-nonstream-success.png`；详细收口收据 `.pi/inspection/flight-feature-reviewer/review-flight-receipt.json`。DEF-013 缺陷彻底闭环并标记 Verified。

### DEF-014 — IDE 接入与复制失败显示假成功

- 冻结文件：`apps/desktop/src/components/IdePanel.tsx` SHA256 `b45ebd67d268f20502e7396f40f4f6d993740053a3c1136a0c2d5e67124eaed7`；副本 `.pi/inspection/ui-honesty-reviewer/snapshot/components/IdePanel.tsx`。
- 定位：`apps/desktop/src/components/IdePanel.tsx:15` Cursor 默认已接入；`:23` 缺失真实 Key 时替用 `pw-sk-proxyweave-master-dev`；`:31-34` 剪贴板异常仍调用 `setCopiedKey(id)`；`:38-46` 写入按钮只更新组件状态为“写入成功/已接入”，无配置写入。
- 独立复现：输入 null gateway/Key；初始 Cursor 已接入、配置含上述 dummy fallback。点击 Claude“一键写入”后显示写入成功，IPC 调用 **0**；owned clipboard `writeText` 拒绝 **1** 次，页面仍显示已复制。
- 证据：task `bb5b17a0c` exit 0；`.pi/inspection/ui-honesty-reviewer/results.json` 的 IDE 写入/剪贴板字段与上述冻结源码。只截获 dummy 数据，未实际修改任何 IDE 配置/系统剪贴板。
- 修复契约：未知安装/配置状态不能默认已接入；缺失密钥安全禁用或准确请求真实接口，禁止虚构 fallback。写入必须有实际受支持协议与安全备份/恢复契约、等待成功再更新；不存在 IPC 时准确未实现，禁止可点击假写入。当前 Chat 网关不能把 Messages 客户端标兼容。剪贴板仅 resolve 后显示成功，reject 可见错误、不更新成功，处理重试与卸载。保持原型布局；真实 IDE 模块复用另立实现阶段，止损不冒充 IDE 完成。
- 止损修复独立复验：task `bd16f560d` exit 0（13 项检查通过，6 张三主题截图亲自审阅无溢出/报错），`.pi/inspection/ui-truth-wave1-reviewer/results.json`。无 dummy key fallback、无写入假成功、Claude Code 标“协议不支持”、剪贴板拒绝显示 alert 且不标成功、重试/源变更/组件卸载/超时过期全部符合契约。此复验仅关闭“假成功止损”，真实 IDE 配置检测/备份/写入尚未实现，不能视为 UI Goal 全部完成。

### DEF-012 — 历史密码条目与新版 UTF-8 回读不兼容

- 冻结：`8cb7f2085c05bd67fb287de4922213fdbff640ff`。定位 `crates/quotio-platform/src/lib.rs:560-571` 的 `get_credential` 经 `get_secret()` 后直接 `String::from_utf8(bytes)`；`crates/proxyweave-gateway/src/lib.rs:101-114` 的 `set_backend` 调用 `sensitive_bearer(upstream_key)`，拒绝非 ASCII graphic 密钥。旧版 Windows `Entry::set_password` 存储 UTF-16LE；有效的旧 ASCII 密钥字节仍可能通过 UTF-8 解码，但含交错 NUL。
- 独立复现：`.pi/inspection/phase1-full-reviewer/host-probe/src/bin/legacy-key-boundary.rs` 直接依赖未修改冻结 platform/Gateway，使用真实 Windows Keyring 的唯一审查虚构账号；官方 `set_password` 写入 dummy 值后，冻结 `get_credential` 返回非原值、含 NUL、不是全 ASCII graphic，真实 `Gateway::set_backend` 返回固定错误 `Invalid API key`。未读取实际用户、插件或网关凭据。
- 正向对照：官方 `get_password` 返回原 dummy 值，读取前后原 blob 相同，`set_backend` 接受；官方 `set_secret` 写入 UTF-8 后，冻结 getter 字节语义正确且绑定接受；最终虚构账号删除确认成功。结果只记录布尔值与固定安全错误，无凭据内容。
- 证据：task `b8ebf03c6` terminal exit 0；`.pi/inspection/native-gateway-reviewer/legacy-boundary-result.json`、`.pi/inspection/native-gateway-reviewer/run-legacy-boundary.cmd`。`known_failure_reproduced=true`、`controls_pass=true`、`dummy_account_deleted=true`。
- 证据边界：这是历史条目兼容缺陷，不是当前用户条目编码或原生进程 `503 / no_backend` 根因的证明；不据此更改用户条目、自动迁移或轮换。当前默认 profile 的 dirty 构建也不是本次独立冻结验收对象。
- 修复验收：用官方 Keyring 接口读取可明确识别的历史格式，未知或歧义格式安全失败且不回显字节；不自动写回。旧 ASCII 条目、新 UTF-8、合法 Unicode/长 OAuth、未知 NUL/非 UTF-8、缺失/读取错误均有测试；确认原 blob 不变。绑定结果须准确反馈，选择不等于实际挂载；后续独立冻结副本及原生桌面复验，不以本次缺陷复现成功标为修复通过。DEF-010/011 独立保持 Open。
- 修复独立复验：冻结平台代码 `crates/quotio-platform/src/lib.rs` 经 `is_legacy_ascii_password` 实现受限历史格式识别，仅对符合且无歧义的旧 UTF-16LE 布局调用官方 `get_password`，其它全部采用 `get_secret` 严格按 UTF-8 读取。task `b0496eb81` exit 0（11 passed / 0 failed），其中 `test tests::legacy_windows_password_and_new_utf8_keep_original_blobs ... ok` 与 `test tests::long_credential_roundtrip_supports_tokens_up_to_platform_blob_limit ... ok` 验证了历史密码与新 UTF-8 长凭据互不干扰且原 blob 保持不变。DEF-012 据此标记 Verified。

### DEF-010 — 网关 Master Key 轮换未更新实际鉴权

- 冻结：`8cb7f2085c05bd67fb287de4922213fdbff640ff`。定位 `apps/desktop/src-tauri/src/host.rs:645-677`、`crates/proxyweave-gateway/src/lib.rs:59-97`；Gateway 在 new 时保存不可变 LocalAuth，轮换只改 DesktopState 字符串与持久化记录，没有更新 Gateway/router 使用的鉴权。
- 独立复现：`.pi/inspection/phase1-full-reviewer/host-probe/src/main.rs` 通过 include 引入逐字未改冻结 host.rs；仅将凭据 IO 换为审查自有内存适配器，Tauri mock State 调用真实 regenerate_gateway_master_key。停止状态轮换后启动实际回环 Gateway listener，旧 Key HTTP 503（已鉴权、无 backend），新 Key HTTP 401（拒绝）；期望恰好相反。新 Key 确实不同。未输出任何 Key；此证据是实际网关 HTTP+未改宿主命令，不是原生用户/真实插件验收。
- 证据：`.pi/inspection/phase1-full-reviewer/evidence-host/host-results.json`，task `ba77ee25f`；首次 `be671770e` 虽 exit0 但无结果、exe OS loader exit `3221225785`，不计验收；仅在 reviewer harness 嵌入官方 tauri-build 2.6.2 common-controls manifest 后才得实际 JSON。业务冻结未修改。
- 验收：停止状态轮换成功后，下一次实际 start_gateway 的 HTTP 请求旧 Key 拒绝、新 Key 接受；停止/重启/重开持续一致，保持 backend 与本地/上游鉴权隔离。并发开始/轮换必须有明确互斥，未落盘失败不污染已确认 Key。修复先独占 host/gateway 文件归属并冻结，原生 MCP+HTTP 独立复验。
- 修复独立复验：`crates/proxyweave-gateway/src/lib.rs` 将 `LocalAuth` 鉴权值设计为 `Arc<std::sync::RwLock<HeaderValue>>`，并在 `Gateway` 提供 `set_local_key` 进行热重载；`apps/desktop/src-tauri/src/host.rs` 的 `regenerate_gateway_master_key` 先落盘 Keyring、再调用 `gateway.set_local_key` 更新鉴权，全部成功后才更新内存状态并发布事件。task `bf46a3dda` exit 0，其中单元测试 `test tests::set_local_key_updates_gateway_auth_and_rejects_invalid_keys ... ok` 验证了轮换后旧 Key 即刻返回 401、新 Key 即刻通过鉴权返回 503（无后端），且格式非法 Key 被安全拒绝不破坏当前状态；集成测试 `test host::key_tests::master_key_rotation_updates_gateway_auth_and_persists_keyring ... ok` 验证了宿主轮换后网关鉴权与持久化同步生效。DEF-010 据此标记 Verified。

### DEF-011 — Keyring 写入失败被当作成功

- 定位 `apps/desktop/src-tauri/src/host.rs:111-119`、`apps/desktop/src-tauri/src/host.rs:655-674`：`let _ = quotio_platform::set_credential(...)` 丢弃 Result，轮换后写内存并记录 `Master Key regenerated and saved to OS Keyring`。
- 与 DEF-010 同一冻结未改宿主探针，owned 适配器令 set_credential 返回固定 `owned injected persistence failure`。rotate 返回 Ok、success event=true；清空 owned 凭据后在写入失败状态创建 DesktopState 仍 Ok。未修改用户真实 Keyring，无凭据输出。证据同 host-results.json。
- 影响：用户以为 Key 已保存；重开后凭据不一致或被重新生成，违背持久化成功反馈与安全失败语义。
- 验收：读/写错误必须准确安全失败，禁止静默覆盖已有凭据或假成功 event；写入成功后才发布新确认状态，失败保持原 Key/实际鉴权/成功记录不变；首次创建、轮换、重开与注入失败路径均验证。与 DEF-010 共享文件不得并发 writer；在 DEF009 后由开发独占组合修复或明确分离文件阶段。
- 修复独立复验：`apps/desktop/src-tauri/src/host.rs` 在 `DesktopState::new` 初始化以及 `regenerate_gateway_master_key` 轮换时，彻底移除了 `let _ = quotio_platform::set_credential` 静默丢弃模式，写入失败时显式返回 `Err` 并阻断状态更新；同时 `save_desktop_config` 写入校验前置，磁盘落盘失败不再静默更新内存状态。task `bf46a3dda` exit 0，其中集成测试 `test host::key_tests::master_key_rotation_fails_atomically_on_keyring_persistence_error ... ok` 与 `test host::key_tests::desktop_state_init_fails_if_master_key_cannot_be_persisted ... ok` 验证了 Keyring 写入失败时原子报错、不更新内存状态、不产生假成功事件。DEF-011 据此标记 Verified。

### DEF-009 — 真实包在独立桌面新目录导入发布失败

- 冻结提交：`8cb7f2085c05bd67fb287de4922213fdbff640ff`；独立产物 SHA256 `d8231f2072a71e0b6696f4a261c5af1e6a28460b9f0dc84651a798dcc719901f`。原生 PID `68748` 的 OS ExecutablePath 为 `.pi/inspection/phase1-full-reviewer/target/debug/quotio-desktop.exe`，不是新主题预览。
- 定位：`crates/proxyweave-plugin/src/registry.rs:120-121` 的 `fs::rename(staged.path(), &package_dir)` 将实际错误压成统一发布失败；根因尚未定位，不预设 MAX_PATH、ACL 或登记冲突。
- 亲自复现：正常退出预览后启动独立嵌入客户端；profile `D:/ProxyWeave/.pi/inspection/phase1-full-reviewer/native-profile-20261012`。使用原生桌面 MCP 点击插件管理，SetValue `D:/ProxyWeave/.pi/inspection/phase1-full-reviewer/test-package/manifest.yaml`，点击“导入并注册”；revision 11 返回 `导入插件失败：cannot publish package; existing registration was not overwritten`、已安装插件 0。OS 目录核验 `plugins/packages=[]`、`plugins/data=[]`，没有原已登记插件。
- 真实包：exe SHA256 `38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485`；Manifest `2d237912d2a396cd39143251d3985250308444bd50551a7073f4955fd9cf5a76`；LICENSE `49b9e27ddfc7cceab55e19eb7d405bda30a8229de3cc094e6b4d0397ab76a34b`，均亲自复核。未读取插件原始 auths/json，导入失败先于凭据投影，不是缺凭据阻塞。
- 证据：`.pi/inspection/phase1-full-reviewer/evidence-native/import-result.json`、`evidence-native/import-failed.png`、`native-launch.json`；MCP observation `obs:4b5ea9a26a1d45528febdd0a7870ecad`。Developer 独立冻结 CLI 在其长 owned 路径亦失败，短 owned 路径成功、重复/设置/重开通过；后者为第二方对照，不冒充本人原生成功。
- 影响与边界：当前本机新隔离目录内用户导入真实插件失败，不能进入启动/调用闭环；不宣称所有目录、所有 Windows 环境均失败。Main 同时使用 reviewer 根下较短目录做原生对照，Developer 独占 Registry 诊断/修复。
- 修复验收：定位底层 OS 错误并用成熟实现做必要最小修复；长/短目录真实包均能新导入、重复幂等、内容校验及重开持久化，失败不发布半包、不同内容不覆盖，保留并发/链接/路径安全；按冻结 SHA 独立原生复验。不手工拷登记记录绕过，不动 External 进程，不恢复暂停 Goal，不提交/推送。

### DEF-001 — 已退出的 Managed 子进程仍 Ready

- 定位：`crates/proxyweave-plugin/src/runtime.rs:132-140`、`crates/proxyweave-plugin/src/runtime.rs:157-163`（上述冻结提交）。
- 根因：`get_status` 只复制缓存状态，没有观察已持有 Child 的退出；`start_plugin` 看到缓存 Ready/Busy 直接返回，未验证进程还存活。
- 复现：启动审查自建 HTTP fixture 达到 Ready；仅强制终止其 PID；等待 OS 证实退出；查询状态并再次 start。
- 数据：PID `68480`，端口 `63009`；`pid_alive=false`，但 `get_status` 仍 `Ready { pid: 68480, port: 63009 }`；后续 start 仍返回相同 Ready，journal 只有一次 spawn。
- 影响：界面/网关可以接受死实例的 Ready 状态，无法通过再次启动恢复，违反准确崩溃反馈与可再次使用目标。
- 修复验收：实际退出后状态不得 Ready/Busy 或暴露死实例为有效端点；应记录可理解的退出/失败原因；后续显式启动/重启能生成新存活 PID，或准确报告新失败。保留 External 零 Kill；增加真实 Managed child 退出回归，不用 External fixture 替代。

### DEF-002 — 错误身份服务被接受为 Ready

- 定位：`crates/proxyweave-plugin/src/probe.rs:63-103`、`crates/proxyweave-plugin/src/runtime.rs:334-347`；契约定义 `crates/proxyweave-plugin/src/manifest.rs:68-75`。
- 根因：探针接受 URL/plugin_id/timeout 而不接收完整 HealthCheck；仅读取固定 `x-service` 并以 HTTP 200 判 Ready，未核对 Manifest 的 `identity_header`/`identity_value`。探针还有固定 wb2api 语义分支，需以已核验 Manifest/最小适配契约处理，不扩张平台业务到宿主。
- 复现：Manifest 声明 `X-Service: review-fixture`；审查自建子进程返回 HTTP 200 与 `X-Service: different-service`。
- 数据：PID `58604`，端口 `63012`；结果仍 `Ok(Ready { port: 63012, pid: Some(58604) })`。
- 影响：Ready 不再证明接入的是声明插件；端口竞态/错误服务可能被错误挂载。
- 修复验收：缺失或错误身份头不得 Ready；正确身份才可进入契约允许状态；验证自定义合法头名而非固定 x-service。保留真实 wb2api 503 的未配置/饱和语义，但不得把错误身份当该插件，也不得把健康超时推断成端口冲突。

### DEF-003 — Starting 不可见且 stop 无法协调在途启动

- 定位：`crates/proxyweave-plugin/src/runtime.rs:341-401`、`crates/proxyweave-plugin/src/runtime.rs:409-418`。
- 根因：runtime entry 与 active Child 在整个探针循环结束后才发布；stop 只查该 map，没有与 start 的在途状态/互斥协调。
- 复现：fixture 已 spawn 并写入 PID journal，但延迟 1800ms 才提供健康端点；在此期间查询状态并 stop；随后等待启动任务结束。
- 数据：PID `28240`；启动中返回 `Stopped`；stop 返回 `Err(PluginNotFound("Plugin review-starting not found"))`；随后启动完成为 Ready，PID 仍活着。
- 影响：用户看不到真实 Starting，点击停止不能停止进行中的启动，结束后反而留下运行实例。
- 修复验收：spawn 前/启动期间发布可信 Starting 状态；同插件 start/stop/restart 必须一致协调，不能丢失 stop。明确 stop 的取消/排队语义，并在其成功返回后保证没有该轮启动留下的实例；验证重复/并发启动、stop-during-start、失败重试以及任务取消，不影响 External。

### DEF-004 — 端口冲突恢复契约未实施

- 定位：`crates/proxyweave-plugin/src/runtime.rs:152-177`、`crates/proxyweave-plugin/src/runtime.rs:254-255`；契约字段 `crates/proxyweave-plugin/src/manifest.rs:80-84`。
- 根因：`check_port_available(port)?` 对明确占用立即返回；未使用 Manifest `max_port_retries`，错误路径也未发布启动失败状态。
- 复现：审查自身持有一个随机回环 listener；Manifest 配置 `max_port_retries: 3`，以该端口调用 start。
- 数据：端口 `63019`；立即返回 `AddrInUse` / Windows `os error 10048`，spawn 次数 0，查询 runtime 仍 `Stopped`；自建 listener 保持占用且未受影响。
- 影响：正确识别了冲突，但没有完成用户明确要求的冲突恢复；配置被接受但运行时忽略。
- 修复验收：在约定的运行时/上层协调入口实现有界重新分配和重试，实际成功端口、PID、失败原因可见；耗尽重试准确失败，零重试行为明确。健康超时不能触发无证据的冲突重试；不得终止占用端口的外部进程。若恢复由上层实现，须连同真实调用路径与回归一起冻结送审，不能仅声明会在以后接入。

### DEF-005 — Managed 就绪失败原因被通用超时覆盖

- 冻结基线：`b8f06e67ae998afcc8612a69d2eb54967e91da15`；独立归档 SHA256 `73cbf1e29b44a15c943a2e6f971cbf032b9ef63f9aa506b0690f3a641dee2224`，快照 `.pi/inspection/f2-repair-reviewer/snapshot/`。
- 定位：`crates/proxyweave-plugin/src/runtime.rs:445-447` 中 `ProbeStatus::Failed { .. }` 丢弃 error；`crates/proxyweave-plugin/src/runtime.rs:455-458` 以通用超时覆盖最终结果。
- 根因：Managed 启动轮询允许失败后继续等待，但不保存/发布探针提供的脱敏失败原因；因此已获得身份拒绝证据，仍报告与无响应相同的超时，阻碍准确诊断。
- 独立复现：审查自建 HTTP 子进程返回 200 和错误身份；Manifest 声明 `review-fixture`，实际 `different-service`。扩展轮结果 `Ok(Failed { error: "Readiness probe timed out after 1.5s" })`。缺失身份 PID `59232` 及错误身份的 503 PID `29184` 也仅返回 `Readiness probe timed out after 1.1s`；正确身份 503 PID `66812` 被正确判 Busy。
- 证据：`.pi/inspection/f2-repair-reviewer/evidence-expanded/results.json`、`.pi/inspection/f2-repair-reviewer/check-results.cjs`；task `bcd57ac74` exit 0，九项断言通过证明其他契约，不证明该诊断缺口不存在。探针当轮 Tokio `1.53.2` / reqwest `0.13.5`，源代码为冻结提交；后续精确 workspace 版本 Tokio `1.52.3` / reqwest `0.13.4` 独立复验 task `bbeb7f49c` 已完成、exit 0，证据 `.pi/inspection/f2-repair-reviewer/evidence-exact/results.json`；版本范围不再仅限扩展轮。九项断言通过不等于该诊断缺口通过。
- 修复验收：保持有界就绪等待，保留并发布可理解的脱敏探针失败原因，区分身份不匹配/缺失与无响应超时；不可把身份值、请求头、凭据或原始上游正文泄漏到错误或日志。通过真实 HTTP fixture 验证错误/缺失身份、正确身份后续就绪、纯超时、正确身份 Busy；不得把健康失败当端口冲突。新修复需另行冻结送审，不修改当前冻结快照。
- 此项不同于 DEF-002：错误服务已被拒绝为 Ready；本项仅涉及拒绝原因的准确反馈。F2 全体验收仍开放。
- 修复独立复验：冻结 `1dd2bcd7fc698e6ab4c394a746e0f737a6fc25f7`（parent 上述 b8f06e6），审查归档 SHA256 `cb06b57edca8e982ab12b0aba9483f8ce334929639dc5a701eb038a40d047641`。task `bf731ec21` terminal exit 0；精确依赖自建 HTTP fixture 的九项生命周期检查及三种身份失败诊断/脱敏检查通过，正确身份 Ready/Busy 保持通过。错误/缺失身份最终包含 `last probe failure: Health check identity mismatch or missing header`，不回显实际身份值。
- 同一冻结快照默认并行 Rust tests：52 passed / 1 fixture ignored；fmt 与 scoped Clippy `-D warnings` 通过；其中包括后续就绪恢复、纯 HTTP 超时和畸形响应安全诊断测试。该次未重建完整 Tauri、未执行前端或真实插件/原生桌面验收，不扩大为 F2 整体通过。
- 证据 `.pi/inspection/f2-def005-reviewer/review-receipt.json`、`evidence/results.json`、`verify.cmd`、`freeze-identity.json`；结果 SHA256 `dbf9d0d0302025a03880ef3409ecb20291135c5554d18f76e79a3b721a6586b3`，日志 SHA256 `dd73d910fd6578b75a494110a2feb2e00ee08e2a8fab7c5113e339f6be69d915`。独立诊断 checker 对旧版三身份场景确实失败，对本修复通过，DEF-005 据此标记 Verified；DEF-006 仍 Open。

### DEF-006 — 投影将凭据值再次解释为模板，改变原始凭据

- 冻结基线：`b8f06e67ae998afcc8612a69d2eb54967e91da15`；独立快照 `.pi/inspection/f2-repair-reviewer/snapshot/`，沿用上述归档及精确依赖版本。
- 定位：`crates/proxyweave-plugin/src/projection.rs:44-60`（`ProjectionContext::render_str`）；`crates/proxyweave-plugin/src/projection.rs:7-16`（敏感 Context 的派生 Debug）。
- 根因：先替换模板，再按 BTreeMap 顺序继续替换整个结果；来自某个凭据值的文本会被后续凭据占位符规则再次解析。末尾在整个输出上寻找未解决的 SECRET 也未区分原模板与替换数据。凭据值应为不透明数据，而不是额外模板。
- 独立复现：仅用 dummy 数据；原模板 `{SECRET:a}`，凭据 a 为 `review-a-{SECRET:b}`，凭据 b 为 `review-b-replacement`。冻结实现返回 `review-a-review-b-replacement`，不是用户提供的 a。该结果可造成投影配置中的凭据被改变，令原本正确的凭据无法使用。
- 证据：task `bb351d6db` terminal exit 0；`.pi/inspection/f2-repair-reviewer/probe-exact/src/bin/projection-boundary.rs`、`.pi/inspection/f2-repair-reviewer/evidence-security/projection-boundary.json`。这是缺陷复现成功，不是安全验收通过；探针只调用冻结生产库，未修改业务文件或使用真实凭据。
- 同一探针还证明 `format!("{context:?}")` 包含 dummy plugin_api_key 和 secret。这仅证明 Debug 暴露面，不证明生产日志已输出它，不登记“真实日志泄漏”结论。修复时去掉敏感值的 Debug 输出或实施可靠脱敏，并在实际错误/日志路径检查后再宣称日志安全。
- 修复验收：只识别原始模板中的占位符，插入的凭据/API Key/路径值不再次解释；原模板真正缺少的 secret 仍应安全失败，错误不得带出凭据值；验证不同键顺序、包含 SECRET/标准占位符的合法值、JSON 投影字节语义、参数/env 渲染与 Debug 脱敏。遵守成熟实现直接复用政策，若需新机制先说明参考/直接依赖或必要薄适配依据。独立冻结送审，保留既有敏感 atomic_write/ACL 与生命周期回归。
- 修复验收探针反向验证：task `bc69460fd` 构建成功后按预期 exit 1，旧冻结版 14 项检查中 5 pass / 9 fail（不同 secret 键序、含缺失 SECRET 的字面凭据及其 JSON 投影、API Key/路径不透明性、Debug 脱敏失败）；重复原模板 PORT、真正缺失 secret 拒绝等对照检查通过。证据 `.pi/inspection/f2-repair-reviewer/evidence-security/projection-acceptance-baseline.json`、`probe-exact/src/bin/projection-acceptance.rs`。该失败是验收 checker 能检出已知缺陷的反向验证，不是新修复验收失败，也不是新业务编译错误；该基线轮当时尚未收到修复；后续新冻结独立复验结果见下条。
- 修复独立复验：冻结 `771015be8c5cef291626bf38aa7bcc13c38372ca`，parent `1dd2bcd7fc698e6ab4c394a746e0f737a6fc25f7`；归档 SHA256 `4e8a7c608aa97703597c1a51d33ce5a384941f642344dc298845f4362a13e249`。task `b29492f90` terminal exit 0；同一独立验收探针从旧版 5 pass / 9 fail 变为 **14 pass / 0 fail**，涵盖凭据键序、SECRET 字面值、JSON、API Key/路径不透明性及 Debug 脱敏。
- 精确冻结依赖 regex `1.12.3` / regex-automata `0.4.14` / regex-syntax `0.8.10` / aho-corasick `1.1.4` / memchr `2.8.1` / Tokio `1.52.3` / reqwest `0.13.4` / Keyring `3.6.3`；九项真实 Owned 生命周期检查、三种身份诊断脱敏检查及正向 Ready/Busy 对照通过；默认并行 Rust **56 passed / 0 failed / 1 fixture ignored**，fmt 与 scoped Clippy `-D warnings` 通过。包含生产 RuntimeManager 的 args/env/config/credential/Keyring 投影与 missing-secret 无新增 spawn 回归；未重新执行前端或完整 Tauri 构建。
- 独立核对本地 regex 发布源码 `.cargo_vcs_info.json` commit `b028e4f40eac8959d05e82abf8404906b1c565c0`、官方 Fallibility 示例及 MIT 许可证；冻结锁仅新增 plugin→regex 依赖边，无版本变更。报告 LF SHA256 与开发交付一致；归档内 12 个变更文件在复验前后哈希一致。
- 证据 `.pi/inspection/f2-def006-reviewer/review-receipt.json`、`verify.cmd`、`reuse-identity.json`、`exact-lock-comparison.json`；投影结果 SHA256 `94050d22af41c62bc1cbff92947d8a61fbf9b41d39e930cb15326f9e4919e6cd`，生命周期结果 SHA256 `29bb083afe468b29e202deb84ab496d1f9f76254d228dab59e2ac8c439f8d0ab`，执行日志 SHA256 `b01f2a857e415b00672e2bc3cd8a11423989339afb1374a7d5454cdab35ce2c9`。DEF-006 据此 Verified；仅关闭该具体缺陷，不证明真实上游凭据、全部日志、F2 整体或原生桌面 MCP 最终验收。

## F2 修复冻结版回归门禁记录（DEF-001–004 独立复验通过，F2 整体仍开放）

- 修复冻结提交 `b8f06e67ae998afcc8612a69d2eb54967e91da15`；审查快照 `.pi/inspection/f2-repair-reviewer/snapshot/`，复验脚本 `.pi/inspection/f2-repair-reviewer/regression.cmd`。
- 独立 task `bd85d9404` 完成、exit 0：`cargo fmt --all -- --check`；锁文件 Rust 测试 47 passed / 1 ignored（插件 32、platform 8、types 7；ignored 项为供子进程启动的 fixture）；插件/platform `cargo clippy --all-targets --no-deps --locked -j 1 -- -D warnings`；`npm ci`、前端 2 项测试、`tsc && vite build` 均通过。
- workspace 实际编译 Tokio `1.52.3`、reqwest `0.13.4`，涵盖提交内 DEF-001–004 回归；审查自建扩展 probe 已以相同依赖版本独立运行：task `bbeb7f49c` exit 0 / 九项断言通过；证据 `.pi/inspection/f2-repair-reviewer/evidence-exact/results.json`，锁版本比对 `.pi/inspection/f2-repair-reviewer/exact-lock-comparison.json`。
- 扩展 task `bcd57ac74` exit 0 / 9 项断言通过：单实例、崩溃后重拉、身份拒绝、启动期间排队 stop、重选端口、取消进程回收、自定义头/Busy、零重试、取消状态。该轮 probe 的不同依赖版本限制见 DEF-005。
- 精确版本复验具体数据：DEF-001 原 PID `70436` OS 确认退出，状态 `Managed process exited: exit code: 91`，重拉 PID `26808` 存活；DEF-002 错误/缺失身份不得 Ready，自定义头 PID `51452` 正确 Ready，正确身份 503 PID `59452` Busy；DEF-003 Starting 可见，排队 stop 返回成功且 PID `59324` 已退出，abort PID `62380` 已回收并标记取消 Failed；DEF-004 明确冲突 `62771` 重选 `62772`，PID `73892` Ready，原 listener 未受影响；零重试 `62801` 无投影/无 spawn 且 Failed。并发 start 仅一个 PID `37140`。这些数据支持四项具体修复，不扩大为完整生命周期、安全或桌面验收。
- 这些门禁没有构建完整 Tauri 可执行文件，也没有验证实际插件凭据/模型、原生桌面 MCP、完整安全隔离或八项最终验收。DEF-001–004 已在该冻结修复版独立复验为 Verified；该轮之后 DEF-005 在独立新冻结复验中 Verified（见 DEF-005 小节），后续 DEF-006 亦在独立新冻结复验中 Verified（见 DEF-006 小节）；六项具体缺陷关闭不等于 F2 整体通过。目标 continuation 后 `get_goal` 已确认 regular / running，继续监督未完成验收。

## 已独立通过的具体检查（不等于 F2 全部通过）

- Managed singleflight：两个并发 start 均返回 PID `42152`/端口 `63006`，journal 只有一个实际 spawn。
- 启动任务取消：abort 后 PID `43252` 已回收。
- 成功分配 Job 后的异常宿主退出：host `69944` 被强制终止，parent `73488` 与 leaf `34692` 均退出；未分配 Job 的自建 sentinel `25260` 仍存活。25ms 为本次轮询观测粒度，**不是**一般性能上界，也不证明 spawn→assign 窗口无竞态。
- 此前 35 项自动化测试通过仅证明各测试覆盖场景；Windows Protected DACL 的实际授权不等同 Unix 0o600；不得宣称绝对回收或跨平台隔离。
- 修复冻结版 `b8f06e67ae998afcc8612a69d2eb54967e91da15` 独立安全 task `bbc61029e` exit 0：审查自建 Keyring 两个命名空间完成写入/读取/单项删除隔离/最终清理；敏感文件首次原子写入和替换后的同用户读取均通过。真实 Windows `Get-Acl` 确认 Protected DACL、恰好一个非继承 Allow ACE、主体为当前用户 SID 且具有 FullControl。证据 `.pi/inspection/f2-repair-reviewer/evidence-security/acl-result.json`；探针 `.pi/inspection/f2-repair-reviewer/probe-exact/src/bin/security-check.rs`、`.pi/inspection/f2-repair-reviewer/check-acl.ps1`；实际依赖 keyring `3.6.3` / Tokio `1.52.3` / reqwest `0.13.4`。
- 安全证据边界：本次没有以另一 Windows 用户执行拒绝访问测试，不证明同用户进程隔离，也不证明最终插件投影、进程参数或所有日志都已脱敏；测试未使用任何真实上游凭据。F2 安全整体验收仍需结合真实调用路径及日志/投影检查。

## 尚未验收

F2 全体验收维持未通过；修复复验、日志/错误可用性与相关安全边界仍需证据。F3、F4、真实凭据/模型调用及最终原生桌面 MCP 八项用户验收未通过，不能关闭 Managed 初版目标，也不能宣称整个 Phase 2/3 完成。

### DEF-007 — 前端 PluginPanel 访问 `record.manifest.ownership` 与真实契约不符导致 Managed 被误判为 External

- 定位：`apps/desktop/src/components/PluginPanel.tsx:205`，`apps/desktop/src/types.ts:31`。
- 根因：`crates/proxyweave-plugin/src/manifest.rs` 的 Rust 定义中，`ownership` 字段位于 `pub enum Runtime { Managed { .. }, External { .. } }` 标签内（`#[serde(tag = "ownership", rename_all = "snake_case")]`），序列化为 JSON 时结构为 `record.manifest.runtime.ownership`；但前端 `types.ts` 将其定义为顶级 `ownership: "managed" | "external"`，且 `PluginPanel.tsx` 在判断 `const isManaged = record.manifest.ownership === "managed";` 时因属性为 `undefined`，导致所有导入的 Managed 插件（包括 wb2api）均被误判为 `External (外部服务·零杀伤)`，在 UI 上不渲染“启动插件”按钮，仅显示外部服务探活标签。
- 复现步骤及证据：
  1. 启动嵌入式桌面应用 `quotio-desktop.exe`，配置独立数据目录 `PROXYWEAVE_PROFILE`。
  2. 在桌面 UI 中导入 `wb2api.manifest.yaml`。
  3. 通过 `nuphus-mcp` 的 `desktop_semantic_observe` 观测 UI 树，发现已导入的 `wb2api` 插件显示：
     `Other 'External (外部服务·零杀伤)'`
     `Other '（External 外部提供商由宿主透明探活，不执行进程杀死与启停）'`
  4. 检查已持久化的 `plugins/packages/wb2api/record.json`，发现 `record.manifest.runtime.ownership` 为 `"managed"`，而 `record.manifest.ownership` 为 `undefined`。
- 修复契约：
  1. 修正前端 `types.ts`：`Manifest` 中的 `runtime` 定义应包含 `ownership: "managed" | "external"` 或提供兼容性访问器。
  2. 修正前端 `PluginPanel.tsx`：通过 `record.manifest.runtime?.ownership === "managed"`（或兼容两者）准确识别 Managed 插件，正确渲染“启动插件”、“停止插件”、“重启插件”按钮。
  3. 执行 `npm test`、`npm run build` 并重新构建桌面客户端，在桌面实机上观测到 `Managed (托管进程)` 标签及“启动插件”按钮。
- 修复独立复验：提交 `ca0e3bbd5902c697d6e4bc3af7f9b51ae5408fe6`。前端 vitest 5 passed，tsc & vite build 通过，cargo test -p quotio-desktop 3 passed，重新编译 `quotio-desktop.exe`。
- 实机 MCP 验证：在独立测试 profile 中重新拉起 `quotio-desktop.exe`（PID 27604），通过 `nuphus-mcp` 的 `desktop_target_bind` / `desktop_semantic_observe` 检查 UI 树，确证已导入的 `wb2api` 插件显示为 `Managed (托管进程)`，且成功渲染 `启动插件`、`停止`、`重启` 按钮。DEF-007 据此标记 Verified。

### DEF-008 — Keyring Windows 密码存储对长凭据抛出 TooLong 错误，阻止真实 OAuth JWT 存储

- 定位：`crates/quotio-platform/src/lib.rs:560-575`。
- 根因：`quotio_platform::set_credential` 内部使用 `keyring::Entry::set_password(value)`。在 Windows 平台实现（`keyring::windows`）中，`set_password` 将字符串强制转码为 UTF-16 并检查字节大小不得超过 `CRED_MAX_CREDENTIAL_BLOB_SIZE`（2560 字节），因此字符串最大字符数被硬限制在 1280 字符（`password.encode_utf16().count() * 2 > 2560`）。实际场景中真实 OAuth JWT（例如 `workbuddy2api` 的 `access_token` 为 1524 字符，`refresh_token` 为 1526 字符）调用时直接抛出：
  `Attribute 'password encoded as UTF-16' is longer than platform limit of 2560 chars`，导致所有长凭据在 Windows 下无法保存。
- 独立复现：
  使用 `workbuddy2api-panel/auths` 真实 token（1524 字节）通过 `quotio_platform::set_plugin_credential` 存储，触发上述 platform limit 错误。
  经探针对比验证，`keyring::Entry::set_secret(value.as_bytes())` 使用原始 UTF-8 字节存储，直接支持 2560 字节以内的全部凭据，并在读取时通过 `get_secret()` 反序列化回字符串，成功存储并还原 1524 字节 token。
- 修复契约：
  1. `crates/quotio-platform/src/lib.rs` 中 `set_credential` 改用 `entry.set_secret(value.as_bytes())`，`get_credential` 改用 `entry.get_secret()` 并解码为 UTF-8 字符串；保留原有 KeyringError 处理与隔离命名空间。
  2. 在 `crates/quotio-platform` 增加长 token（如 1500~2000 字节 UTF-8 字符串）的单测与读写一致性测试。
  3. 执行 `cargo test -p quotio-platform` 验证通过。

### DEF-025 — Messages 流编码器延迟 content_block_stop 导致内容块顺序违例（混合块流）

- 状态：Open / 已独立复现，待修复（不是整个 Phase 2 的结论）。
- 发现来源：Inspector 在独立复验 B1（矩阵 3×3 转码）时，另行建立库级顺序探针，检查矩阵未覆盖的混合块组合。
- 冻结对象：`crates/proxyweave-protocols/src/messages.rs` SHA256 `682d9c045c957dcaaf339566db94efb2f86262af8ba0fff74c3ba0f7b94ef2cc`；源码树为 `git archive HEAD`（tar SHA256 `453232dac7c80466a288035c87c5b87c5121727d22d04de31f33a4a5242a01fd`）叠加该冻结文件；探针 `D:/ProxyWeave/.pi/inspection/phase2-b01-reviewer/probe-order/src/main.rs`（库级，无 HTTP、无真实凭据）。
- 根因：B1 修复为支持 Chat/Responses 上游**并行工具调用交错分片**，把 `tool_use` 块的 `content_block_stop` 从“开新块前立即关闭”改为延迟到流边界（`Finish` / `ResponseCompleted` / `encode_done`）由 `flush_tool_block_stops` 按索引升序统一 flush。该改动使两条及以上工具块可以同时在“打开”状态，此时若中间插入文本块（`TextDelta` 经 `ensure_block` 开启 text 块），Anthropic 流式契约要求的 `content_block_start → deltas → content_block_stop` 串行嵌套被打破。
- 独立复现（同一冻结，实测输出）：
  1. 输入 `tool a → text → tool b → text → Usage → Finish`：`starts=[(0,tool_use),(1,text),(2,tool_use),(3,text)]`，`stops=[1,3,0,2]`；违例：块 1 在 0 未关闭时 start；块 2 在 0 未关闭时 start；块 3 在 [0,2] 未关闭时 start。
  2. 输入 `tool a → tool b → text → Finish`：`starts=[(0,tool_use),(1,tool_use),(2,text)]`，`stops=[2,0,1]`；违例：块 1 在 0 未关闭时 start；块 2 在 [0,1] 未关闭时 start。
  3. 对照：修复前基线 `ea0ab693ef581092695508554582c805fecd686e14ffba01e99903c0bb05a27b` 每次 opener 都先 `close_block` 再 `open_block`，块严格串行——本缺陷是 B1 修复引入的新行为，不是既有问题。
- 与矩阵结果的关系：同一冻结下 3×3 矩阵 **124 passed / 0 failed**（闭合原 4 项工具/usage 失败）；矩阵只解析事件内容，不校验块嵌套顺序，因此两项证据互补，不矛盾也不互相抵消。
- 影响边界：已在编码器输出层确证；**真实 Anthropic SDK / Claude Code 客户端消费该流是否报错未验证**，不宣称一定失败，也不宣称兼容。受影响的组合需要同一流里既出现并行工具块、又出现文本/其他块。
- 修复契约：
  1. 直接复用成熟实现的状态机处理交错（TiyGate 源同样使用单一 `current_index`，需在真实上游交错语义与块顺序契约间取正确解），不得靠“为了通过矩阵”绕过块顺序。
  2. 并行工具块的参数分片仍必须按 `id` 归属到各自索引（B1 已建立的能力不得回退）。
  3. 块顺序必须保证：任一时刻至多一个块处于打开状态，或采用协议允许的显式多块模型并给出源码依据；终止帧顺序仍需 `content_block_stop` 全部先于 `message_delta`/`message_stop`。
  4. 必须补正向/负向测试：`tool→text→tool`、`tool→tool→text`、纯并行工具（无文本）三类，断言块嵌套与事件顺序；异常 EOF / 错误路径不得补成功终止。
  5. 修复后由 Inspector 用本探针（`probe-order`）与 3×3 矩阵**同时**复验，两者都通过才算关闭；仅矩阵通过不关闭本 DEF。
- 证据文件：`D:/ProxyWeave/.pi/inspection/phase2-b01-reviewer/order-evidence.json`（含探针哈希与逐案事件序列）。
- 复验进展（2026-10-18，Bugfix 交付冻结件 `8515b2b9c8675f8ce91783c8fe3324f20076be28a01e9a554c146c29d167e3da`，130875 B）：
  - 修复路线为「缓冲发射」：`ToolCallDelta` 不再流式发射，按 id 缓冲到 `buffered_tools`，在终止部件（`Finish`/`ResponseCompleted`/`encode_done`）由 `emit_tool_blocks()` 以 `content_block_start` → 单条 `input_json_delta(完整参数)` → `content_block_stop` 按 opener 顺序串行发射。
  - 库门禁独立通过（task `ba987dfe0`）：`cargo test -p proxyweave-protocols` lib **152 passed**（原 149 + 新增 3）、`cross_protocol` 56 passed、`streaming_transcode` 6 passed；`cargo test -p proxyweave-gateway` lib 1 + transcoding 7 passed/1 ignored + 5 passed + transcoding_robustness 10 passed；`cargo clippy --all-targets -- -D warnings` 0 error/0 warning；`cargo fmt --check` clean。
  - 顺序探针（`probe-order2`，A–L 共 12 案）独立复验：A/B（tool→text→tool 交错分片精确重组）、C（纯并行工具交错分片）、D（纯文本 end_turn）、E（异常 EOF 不伪造 terminal message_delta）、F（Error 不发 message_stop）、G（空 arguments 仍合法）、I（Finish→Usage→encode_done 正确延迟 stop_reason）、J（text→tool→Usage→Finish→ResponseCompleted）**全部通过**；H/L 为既有 deferral 行为（非本缺陷引入，见 DEF-028 段）。
  - 本缺陷所报的块顺序违例（`stops=[1,3,0,2]` / `stops=[2,0,1]` 两案）在冻结件上**不再复现**。
    - **复验结论（Verified，2026-10-18）**：`review-receipt.json`（SHA256 `51311074b9dc75fd7ee6982cd206f7d87cb052948f13af31c4fe61a76494fba8`）verdict = `INDEPENDENT_PASS_OF_DEF025_BLOCK_ORDER_DEFECT`。
    - 3×3 矩阵在冻结件上重建并运行（task `b3660e127`）：`{"passed":124,"failed":0,"failures":[],"stderr_empty":true}`，结果文件 `D:/ProxyWeave/.pi/inspection/phase2-def025-reviewer/matrix-results.json` SHA256 `ad560e19d3d676e8f34c75387647a3e2ad33923ff9cb24613c5e5b8f643682c7`；harness `probe/verify-matrix.cjs` SHA256 `a42400cb7d1283f1877f5eea666beb2280f816e9e658a9a4174813d429e482e6`。
    - 顺序探针 `probe-order2`（SHA256 `8dd299bacf93cdf05d1e2d834af6ab55edd8d692240fa43dfe4f7b5effdfbe0a`）A–L 12 案中**块顺序维度全部通过**：任一时刻无块在另一块未关闭时 start、stop 与当前打开块对应、终止帧位于所有 stop 之后。本缺陷原先的 `stops=[1,3,0,2]` 与 `stops=[2,0,1]` 两案不再复现。
    - 修复契约第 5 条因此满足（矩阵 + 顺序探针同时通过）。**限定**：同一顺序探针的 case K 属独立缺口（空 id 并行工具塌陷），已在 **DEF-028** 单独登记并保持 Open；DEF-025 的关闭仅针对「块顺序违例」这一原缺陷，不代表 Messages 编码器整体无缺陷。

### DEF-026 — 插件卡片对 Managed 插件固定宣称 `idle_timeout · 300s`，与 manifest 及 runtime 均不符

- 状态：Verified / 独立复验通过（2026-10-18，由 Inspector 在隔离快照与独立测试探针上复验通过；原生桌面端整合留待阶段门禁）。
- 复验进展（2026-10-18，Developer 冻结件 `81014e300ce4c16cc69f8c37d6e69315c2ba5c83984ca5360fcfe51b1473fa58`）：
  - 交付文件与快照 SHA256 完全一致（`PluginPanel.tsx` `9f96ccb9...`、`types.ts` `630327f3...` 等 6 文件）。
  - 前端回归与构建全量通过（task `b4d01fcb2` exit 0）：14 个测试文件、144 项测试全部通过；Vite 生产打包通过。
  - 独立失败路径探针（task `bdd9595d0` exit 0）：对 `idle_timeout_ms: null`、配置正数、缺省字段三态精确显示「不启用空闲回收 / 配置空闲超时 Ns（尚未生效）/ 空闲策略未声明」，不再无条件硬编码 `idle_timeout · 300s`。收据见 `D:/ProxyWeave/.pi/inspection/def026-027-reviewer/review-receipt.json`。
- 发现来源：Inspector 独立复验 D1/D2（`docs/verification/PHASE1_REVIEW_STATUS.md` 第 6 项「活跃流不被空闲回收」）时，核实该表述在当前代码上是空洞满足。
- 定位：`D:/ProxyWeave/apps/desktop/src/components/PluginPanel.tsx:413` `{isManaged ? "idle_timeout · 300s" : "永不 Kill"}`；`idle_timeout` 与 `300s` 均为**硬编码字面量**，不读取 manifest 的 `lifecycle.idle_timeout_ms`；前端 `D:/ProxyWeave/apps/desktop/src/types.ts` 的 `Manifest` 接口**没有 `lifecycle` 字段**，因此 UI 也无从读取真实值。
- 根因（两层）：
  1. 生产 runtime 根本没有空闲回收器：`D:/ProxyWeave/crates/proxyweave-plugin/src/runtime.rs`（653 行）中 `idle_timeout` 出现 **0 次**，无 `tokio::spawn` 后台 reaper、无 `interval`、无 `evict`/`last_activity`；`apps/desktop/src-tauri/src/host.rs` 中 `idle|reap|evict` 命中 **0**。`manifest.rs:101` 的 `pub idle_timeout_ms: Option<u64>` 与 `:198` 的下界校验仅为 schema 字段与校验，未被任何策略消费。
  2. UI 不读 manifest，无条件对每个 Managed 插件渲染 300s。
- 对照：真实样例 manifest `D:/ProxyWeave/manifests/wb2api.manifest.yaml` 与 `D:/ProxyWeave/.pi/inspection/phase1-full-reviewer/test-package/manifest.yaml` 都声明 `idle_timeout_ms: null`（即「不回收」）。
- 独立复现：`D:/ProxyWeave/.pi/inspection/phase2-d1d2-reviewer/probe-truth/`——逐字节复制冻结 `PluginPanel.tsx`（SHA256 `c50a300b82c75a1393eef0cbd95691b6c98ddf0e2c255492f9f7bc37478f35ea`）与 `PluginDetails.tsx`（`915f636cd4353fec6fb627535d756861c01dd3bd4d476881d21aca508270a4c7`），只替换 `src/lib/tauri.ts` 这一 IPC 出口；传入 Managed fixture，其 manifest 显式声明 `lifecycle:{idle_timeout_ms:null}`。渲染实测 `renderedIdleClaims=["idle_timeout · 300s"]`，`.pill` 文本含 `idle_timeout · 300s`。结果文件 `probe-truth/probe-results.json`（SHA256 `c084f97e493d1bf7b3516363f6991dfd08a9089bc39eac02108c1967f8e5432d`），截图 `shot-plugins-idle.png`。同探针确认 External 卡片的「永不 Kill」文案正确（不误伤）。
- 影响边界：这是**用户可见的错误能力宣称**，不是运行崩溃。它同时使 `docs/verification/PHASE1_REVIEW_STATUS.md` 第 6 项里的「活跃流不被空闲回收」无法作为已验证机制陈述——因为不存在可被触发的回收器。**不宣称**真实用户因此遭受进程被杀（当前无该行为）。
- 修复契约：
  1. UI 必须依据 manifest 实际声明渲染：`lifecycle.idle_timeout_ms` 为 `null` 时显示「不回收 / 常驻」，有值时才显示该秒数；不得对 Managed 一类插件无条件硬编码。前端 `Manifest` 类型需补 `lifecycle`（只读展示，不新增后端能力）。
  2. 若后续实现真实 idle 回收，须同时满足「活跃请求 > 0 时严禁回收」并有 Owned 真实子进程证据；在此之前文档与 UI 一律不得宣称已实现空闲回收。
  3. 修复后须由 Inspector 用同一 owned 探针（manifest 声明 null / 声明 300000 两例）与原生桌面 MCP 复验，并经独立截图确认。
- 关联：`D:/ProxyWeave/.pi/inspection/phase2-d1d2-reviewer/findings.json`（待补写）。

### DEF-027 — Flight 取消失败仍显示「已取消试飞推理」

- 状态：Verified / 独立复验通过（2026-10-18，由 Inspector 在隔离快照与独立测试探针上复验通过；原生桌面端整合留待阶段门禁）。
- 复验进展（2026-10-18，Developer 冻结件 `81014e300ce4c16cc69f8c37d6e69315c2ba5c83984ca5360fcfe51b1473fa58`）：
  - 交付文件与快照 SHA256 完全一致（`FlightPanel.tsx` `df18faef...` 等 6 文件）。
  - 前端回归全量通过：新增的取消语义与卸载清理用例（`flight-cancel.browser.test.mjs`）14 项测试全部通过。
  - 独立失败路径探针（task `bdd9595d0` exit 0）：覆盖取消返回 false（未找到会话）、取消拒绝/抛错、重试、迟到完成不被旧取消覆盖、组件卸载重挂载拦截未决取消等 7 项关键路径全部通过，错误状态如实呈现且在飞会话保留，不再虚假提示「已取消试飞推理」。收据见 `D:/ProxyWeave/.pi/inspection/def026-027-reviewer/review-receipt.json`。
- 发现来源：Inspector 独立复验 D1（Flight 试飞）时，审查 `handleCancel` 的错误路径。
- 定位：`D:/ProxyWeave/apps/desktop/src/components/FlightPanel.tsx:120-133`
  ```ts
  const handleCancel = useCallback(async () => {
    if (activeRequestIdRef.current) {
      const reqId = activeRequestIdRef.current;
      try { await api.flightCancel(reqId); }
      catch (err) { console.warn("Cancel request failed:", err); }
      setIsStreaming(false);
      activeRequestIdRef.current = null;
      setError("已取消试飞推理");
    }
  }, []);
  ```
- 根因：无论 `flight_cancel` IPC 成功、返回 `{cancelled:false, message:...}`、还是抛错，UI 一律 `setIsStreaming(false)` + `setError("已取消试飞推理")`；返回体中的 `.cancelled` **从未被读取**。后端 `D:/ProxyWeave/apps/desktop/src-tauri/src/flight.rs:238-271` `pub async fn cancel_session(&self, window_label: &str, request_id: &str) -> Result<FlightCancelResult, String>` 明确可返回 `cancelled:false` 与三种原因（「请求 ID 不匹配或该会话已结束」/「会话已在结束流程中」/「未找到活跃的飞行会话」）。同文件 `:111-118`（unmount cleanup）与 `:142-144`（新请求前取消）也用 `.catch(() => {})` 吞错。
- 独立复现：`D:/ProxyWeave/.pi/inspection/phase2-d1d2-reviewer/probe-cancel/`——逐字节复制冻结 `FlightPanel.tsx`（SHA256 `3f5ed1a235defafea3b2342dac000bb9618c3e75b3f3600086bfa8cc8e5a2c62`）、`types.ts`、`api.ts`，仅替换 `src/lib/tauri.ts` IPC 出口（`flight_cancel` 按场景返回/抛错）。Chromium 实测 6 项断言 4 PASS / 2 FAIL：
  - PASS：`cancel IPC invoked with the active request id` → `{"calls":["flight_chat_stream","flight_cancel"],"cancelArg":{"requestId":"owned-req-1"}}`
  - PASS：`backend answered cancelled=false (session not found)` → `{"cancelled":false,"message":"未找到活跃的飞行会话"}`
  - **FAIL**：`DEF-CANDIDATE: UI must not claim '已取消试飞推理' when backend said cancelled=false` → `{"alertText":"已取消试飞推理"}`
  - **FAIL**：`DEF-CANDIDATE: cancel IPC rejection must not be presented as successful cancel` → `{"alertText":"已取消试飞推理"}`
  - PASS（对照）：`successful cancel still reports success`
  - PASS：`no page errors`
  结果文件 `probe-cancel/probe-results.json`（SHA256 `7f5053c396e8358c9a219a5c013f908fa187c4bc471b8f45d951abdb30b802f0`），截图 `shot-cancel-notfound.png`、`shot-cancel-throw.png`。
- 影响边界：与 DEF-013/014 同类「假成功」模式。**不宣称**真实模型请求一定未被取消（当 `cancelled:true` 时后端确实中止）；缺陷在于失败被呈现为成功。真实 Tauri IPC 取消语义与真实 Anthropic/OpenAI 客户端行为未在本探针覆盖范围。
- 修复契约：
  1. 必须读取 `FlightCancelResult.cancelled`：`true` 才呈现已取消并清空 `activeRequestIdRef`；`false` 保留在飞状态并展示后端 `message`（脱敏后）；IPC 抛错时呈现失败且**不得**清空在飞状态或声称已取消。
  2. unmount cleanup 与「新请求前取消」需区分「已无活跃会话」与「取消失败」，后者不得静默改变用户可见状态。
  3. 补 Vitest/浏览器用例覆盖三条路径（成功 / `cancelled:false` / 拒绝），断言 UI 文案与流状态一致；修复后由 Inspector 用同一 owned 探针复验。
- 关联：本项与 D1 交付的 Flight 验收直接相关；缺陷修复后才可把「取消语义」列入 Flight 兼容项。

### DEF-028 — Messages 流编码器按 id 归并工具调用，空 id 并行调用塌成单块并产生非法 JSON

- 状态：Verified / 独立复验通过。由 Inspector 在 `.pi/inspection/protocol-capability-reviewer/` 进行严格隔离快照构建与 124 项跨协议 HTTP 矩阵端到端网络复验，全部通过（124/124，0 失败，stderr 为空），收据见 `matrix-receipt.json`。DEF-028 宣告彻底关闭。
- 发现来源：Inspector 独立复验 DEF-025 修复时，自行扩展顺序探针得到的 case K；随后用 `ChatCompletionsStreamDecoder → MessagesStreamEncoder` 真实 wire 路径确认可达性。
- 定位：`D:/ProxyWeave/crates/proxyweave-protocols/src/messages.rs`（冻结件 SHA256 `8515b2b9c8675f8ce91783c8fe3324f20076be28a01e9a554c146c29d167e3da`）
  ```rust
  fn buffer_tool_call(&mut self, id: &str, name: Option<&str>, arguments: &str) {
      if let Some(call) = self.buffered_tools.iter_mut().find(|c| c.id == id) { ... }
  }
  ```
  只按 `id` 归并；`id` 为空串时所有调用塌进同一条，`emit_tool_blocks` 再把拼接后的 arguments 作为**单条** `input_json_delta` 发出。
- 上游是否会产生空 id：`D:/ProxyWeave/crates/proxyweave-protocols/src/chat_completions.rs` 的 `ChatCompletionsStreamDecoder` **只在上游给出非空 `tool_calls[].id` 时绑定 id**，否则 `slot(index).0` 保持 `""`，继续发出 `StreamPart::ToolCallDelta{id:""}`。`D:/workbuddy2api-panel/internal/upstream/dsml.go:457,480` 会主动合成互不相同的 `call_<ns>_<idx>`，故 **wb2api→Chat 路径不触发**；任意省略 id 的第三方 OpenAI 兼容上游可达。
- 三版归因（同一 case K 输入：两个 `ToolCallDelta{id:"", name:Some(...), arguments:"{\"a\":1}" / "{\"b\":2}"}` + `Finish{ToolCalls}` + `encode_done`）：
  - HEAD 基线 `ea0ab693ef581092695508554582c805fecd686e14ffba01e99903c0bb05a27b`：`blocks=[(0,"","first_tool"),(1,"","second_tool")]`、`args=[(0,"{\"a\":1}"),(1,"{\"b\":2}")]`，`block_count=2` → **正确**。
  - B1 版 `682d9c045c957dcaaf339566db94efb2f86262af8ba0fff74c3ba0f7b94ef2cc`：`blocks=[(0,"","first_tool")]`、`args=[(0,"{\"a\":1}"),(0,"{\"b\":2}")]`，`block_count=1` → **已塌陷**（B1 引入 `tool_index_by_id`，空 id 撞同一 key）。
  - DEF-025 冻结版：`blocks=[("","first_tool")]`、`args=["{\"a\":1}{\"b\":2}"]` → 塌陷 + **arguments 变成非法 JSON 拼接**。
  → 本缺陷由 B1 引入，DEF-025 未修复；DEF-025 因改为单条 `input_json_delta` 使损坏载荷更明确，但逻辑后果同类。
- wire 路径可达性（不是仅手工构造 IR）：`D:/ProxyWeave/.pi/inspection/phase2-def025-reviewer/probe-decode/` 直接串 `ChatCompletionsStreamDecoder` → `MessagesStreamEncoder` → `encode_done`：
  - 上游带 id：`blocks=[(0,"call_a","first_tool"),(1,"call_b","second_tool")]`、`args=[(0,"{\"a\":1}"),(1,"{\"b\":2}")]` → PASS。
  - 上游省略 id：`blocks=[(0,"","first_tool")]`、`args=[(0,"{\"a\":1}{\"b\":2}")]` → **FAIL：两个不同调用塌成 1 块，且 arguments 非法 JSON**。
  运行 `verify-decode.cmd`，rc 1，输出见 `probe-decode/run-output.txt`。
- 影响边界：Anthropic 客户端会收到一个 `tool_use` 块，其 `input` 解析失败、且第二个工具调用完全丢失。**不宣称**真实 Claude Code 在 wb2api 场景下必然触发（wb2api 自带 id）；针对第三方上游需按本缺陷处理。
- 修复契约：
  1. 归并键不得只依赖 `id`：需支持「id 缺失时按上游 index/调用位置」区分调用，或由 Chat 解码器在 id 缺失时合成稳定唯一 id（并保留 `call_id` 语义区分），保证 N 个并行调用产生 N 个 `tool_use` 块。
  2. 每个 `tool_use` 块的 `input` 必须是合法 JSON（不得跨调用拼接）。
  3. 不得回退 B1 的「按 id 归属交错分片」能力，也不得回退 DEF-025 的块顺序保证。
  4. 补正负测试：`Chat SSE 省略 id 的两路并行工具`、`省略 id 的交错分片`、`带 id 的既有用例` 三类，断言块数、id 唯一性、arguments 每个块内可 JSON 解析。
  5. 关闭条件：3×3 矩阵 124 项、DEF-025 顺序探针 A–L、本缺陷的 probe-decode 三者同时通过。
- 历史复验进展（2026-10-18，身份与计数更正）：原记录 `1004a44f9f74a0058b8849b28f72c05763b0645600c7329dfae498ff0efb7ea5` / 131802 B 错误，不是换行差异；真实 def028-v2 `messages.rs` 为 139275 B，SHA256 `18d36d31cf7350c309b21df32fa2a2de93dc115249ddec9b802152de6bc002a8`。原错误收据保留并由 `D:/ProxyWeave/.pi/inspection/phase2-def028-reviewer/receipt-correction.json` 更正。
  - 交付修复机制：
    1. ChatCompletionsStreamDecoder 在上游缺省 id 时自动合成 `call_synth_<index>` 保证 IR 语义完整；
    2. MessagesStreamEncoder 对匿名工具调用，若只有一个且无歧义则兼容保留，若存在多个或歧义调用立即 fail-closed 并锁死 `self.failed = true`，清空缓存并拦截后续一切完成事件，彻底防止非法 JSON 拼接或截断泄露；
  - 库门禁独立复验全通：
    - `cargo fmt -p proxyweave-protocols -- --check`: PASS
    - `cargo test -p proxyweave-protocols`: PASS（224 passed（160 lib + 56 cross_protocol + 8 streaming_transcode；原记 212 错误）, 0 failed, 0 ignored）
    - `cargo test -p proxyweave-gateway`: PASS（23 passed, 1 ignored fixture, 0 failed）
    - `cargo clippy -p proxyweave-protocols -p proxyweave-gateway --all-targets --no-deps -- -D warnings`: PASS（0 warnings）
  - 独立探针复验全通：
    - `probe-decode`（端到端 wire 解码）：PASS（`blocks=[(0, "call_synth_0", "first_tool"), (1, "call_synth_1", "second_tool")]`，零塌陷，两路合法 JSON）
    - `probe-order`（块顺序探针 A~L）：PASS（7 checks, 0 failures）
    - `probe-latch`（失活锁死与安全防护）：PASS（歧义分片拒收，fail-closed 锁死且不误发 message_stop）
    - `verify-matrix.cjs`（3×3 跨协议网络矩阵）：PASS（124/124 全部通过，0 failed，stderr 为空）
  - 复验结论（Verified，2026-10-18）：收据见 `D:/ProxyWeave/.pi/inspection/phase2-def028-reviewer/review-receipt.json`。DEF-028 宣告关闭。


### DEF-029 — Release 资产名在 `.exe` 分支逃逸暂存目录

- 状态：Verified / Inspector 独立冻结本地边界复验通过；P1。仅关闭本缺陷的限定修复，不宣布整个市场安装可安全使用。
- 冻结：`D:/ProxyWeave/.pi/inspection/market-install-reviewer/snapshot/crates/proxyweave-plugin/src/downloader.rs`，SHA256 `e0ebf33ef3e963ae2cc11306b59289ba4bc516674679b3477a46de0d722fd083`。对应产品 `D:/ProxyWeave/crates/proxyweave-plugin/src/downloader.rs:413-427`；`:424` 为 `let dest = target_dir.join(asset_name);`。下载路径 `D:/ProxyWeave/crates/proxyweave-plugin/src/downloader.rs:690` 同样直接拼接远端 `asset.name`，该网络入口尚未动态复现。
- 独立复现：任务 `bfdfde11b` 编译成功，随后安全断言失败退出 1。传入 `../escaped.exe` 和审查自有绝对路径，两者均返回 Ok 且在 stage 外写入。仅使用自有 TempDir 内的惰性字节，不执行、不联网、不访问凭据或用户配置。不宣称 GitHub 当前允许返回此类名称，也不把 helper 测试说成网络端到端攻击证明。
- 证据：`D:/ProxyWeave/.pi/inspection/market-install-reviewer/boundary-results.json` SHA256 `c3281644af53de246d99973219741cbdc8e09e8e33b079db2b724d5b5fe17e00`；原日志 SHA256 `24ee1cc62cd3fcaaf56668b875b8465662f8be855d7b8129ea06c6a5ba8457ed`。完整来源及清理更正在同目录 `review-receipt.json`：失败退出绕过 TempDir Drop，原 stdout 的 cleaned 字样不准确；之后只删除该审查目录残留并保留结果。
- 修复契约：所有资产名在任何 File::create/copy 前共用严格单文件名验证；拒绝绝对、盘符、UNC、上跳、任一斜杠、Windows ADS/设备名等跨平台危险形式，不以取 basename 静默改名。保留已有 zip/tar 条目验证、哈希检查与原测试。普通合法 exe/zip/tar 正向通过；恶意名称失败且 stage/下载目录之外无新增写入。网络流程入口必须复用同一验证。

### DEF-030 — 无真实契约的任意文件被编造 Manifest 并成功注册

- 状态：Verified / Inspector 独立冻结本地边界复验通过；P1。无契约拒绝是安全收敛，不是任意项目适配或真实市场安装已完成。
- 定位：`D:/ProxyWeave/crates/proxyweave-plugin/src/downloader.rs:531-592` 的 `build_minimal_manifest`，以及 `D:/ProxyWeave/crates/proxyweave-plugin/src/downloader.rs:596-637` 的 `ensure_staging_manifest`。同 DEF-029 冻结源。
- 独立复现：自有目录只有惰性 `arbitrary.exe`；调用 `ensure_staging_manifest(..., "owned-arbitrary", "not-semver")` 返回成功，随后实际 `Registry::import` 成功。生成声明为 MIT、版本 1.0.0、openai_chat、streaming=true、/health、x-plugin-id 及空 required_secrets；这些均无实际插件契约依据。不是只审代码注释，也未证明真实运行 Ready。
- 证据：同 `boundary-results.json` 的 `no_manifest`。原测试仅证明生成结构可通过 schema，不证明这些能力为真。
- 修复契约：删除/禁用猜测型注册 fallback；只有包内显式有效 Manifest 或已审查、与产物身份绑定的现有完整适配契约可以进入 Registry。未知契约须明确报不支持，不能默认许可证/版本/协议/探针/配置。显式无效 Manifest 不得降级猜测。缺契约/错哈希失败无注册；真实显式 Manifest/现有已审查适配器正向保持。若暂仅拒绝无 Manifest，这是安全收敛，不代表真实市场安装全闭环完成。

### DEF-031 — 市场选择的插件身份未与实际包内 Manifest 绑定

- 状态：Verified / Inspector 独立冻结本地边界复验通过；P2。实际 GitHub 网络和原生安装集成验收仍待后续门禁。
- 定位：`D:/ProxyWeave/crates/proxyweave-plugin/src/downloader.rs:603-615` 对已有 Manifest 直接返回，`D:/ProxyWeave/crates/proxyweave-plugin/src/downloader.rs:807-810` 随后注册；未在发布前比对请求 plugin_id 与包内 metadata.id。
- 独立复现：已有有效 Manifest ID=owned-other；以 requested-plugin 再调用 ensure_staging_manifest 获得同一路径，随后 Registry 实际注册 owned-other。证据同 `boundary-results.json` 的 `mismatched_requested_id`。这是 staging helper→Registry 路径，不是已完成 GitHub 网络集成验收。
- 修复契约：包验证完成、任何注册发布发生前校验所选 ID 与 Manifest ID；不符必须失败且两 ID 均不产生新增包/记录，已有包无损。根目录或单层包装 Manifest 行为一致；匹配 ID 正向保留。不得仅在 import 成功后回看 ID 再返回错误（已产生副作用）。版本/仓库来源持久化属于后续明确来源契约，不在本最小修复中擅加字段。
- DEF-029/030/031 共同独立复验收据：`D:/ProxyWeave/.pi/inspection/market-boundary-repair-reviewer/review-receipt.json`。两文件冻结 SHA256：`downloader.rs` 为 `10e82c6faf4eda8a7b0e05d49f690897460a92b58dd3e3f7e3d6edc7b1a940bc`，`tests/downloader.rs` 为 `b9f8e128eaed059ebc2adc550856accb675e7ca8943d57cdb1857c787c84714c`。任务 `bb246af40` exit0：29 项自有惰性字节/Registry 边界断言、15 项下载器测试、两文件 rustfmt 及 scoped all-targets Clippy `-D warnings` 通过；原始日志 SHA256 `3895b6f6669e4c597ab71edc4b2d95d227b6a8adf1c7cbc93e47e6964ed6086c`。保留的 reduced workspace 353 项 registry 锁身份与参考产品锁匹配，非完整桌面 feature 闭包；此前非 locked metadata 尝试及后续锁更正如实保留。无联网、进程执行、凭据、真实插件/原生桌面测试；后续来源/版本开发改动需另冻复验，不能沿用本结论覆盖新版本。

### DEF-032 — 市场条目无认证依据却展示官方/社区认证及未标明的固定元数据

- 状态：IN_REVIEW / 虚假认证与星数的源码修复存在，隔离子集补验进行中；撤回旧收据的全量冻结通过表述。P2。原生市场与只读来源/更新交互尚待统一集成门禁。
- 冻结：`D:/ProxyWeave/.pi/inspection/market-install-reviewer/snapshot/crates/proxyweave-plugin/src/marketplace.rs`，13568 B，SHA256 `162762350251891b807b5743bc55a3dad938f6adc17065cfa098d4243a99b20f`。`D:/ProxyWeave/.pi/inspection/market-install-reviewer/catalog-ui/components/DiscoverPanel.tsx`，48461 B，SHA256 `48e7d70a839756b2b2be1ef3b626459ac4bfd0b3b6f722528e05a73957589e8a`。
- 定位：`D:/ProxyWeave/crates/proxyweave-plugin/src/marketplace.rs:55-185` 内置 catalog 六项全部固定 `verified: true`、版本、星数；`D:/ProxyWeave/apps/desktop/src/components/DiscoverPanel.tsx:389-390` 将该布尔值显示为 `✓ 官方/社区认证`。在线映射另在 `D:/ProxyWeave/crates/proxyweave-plugin/src/marketplace.rs:252-270` 根据作者赋 verified、版本写死0.1.0、default_branch赋release_tag；这部分本次仅源码核对，未执行网络复现。
- 复现：task `bfd7215fa` exit0，调用冻结公开 `get_builtin_catalog()`，`count=6 verified=6 fixed_stars=6 network_requests=0`。随后 `check-catalog-ui.cjs` 对原封不动组件注入该 catalog 标志作惰性 SSR，`official_or_community_certification_rendered=true`、`ipc_calls=0`。无认证API调用或证据绑定；未测试上游实时版本/星数值是否恰巧相同，也不据此声称所有固定数据均与上游不符。
- 证据：`D:/ProxyWeave/.pi/inspection/market-install-reviewer/catalog-review-receipt.json`；`catalog-truth-results.json` SHA256 `06f99cedee20a135be816cb7b2560f47dd32d9fe8520e1eacedddefe6f925ff7`；`catalog-ui-results.json` SHA256 `5469a09d47d102e24ee3800e121686344f37368d26d488e0a951ac6a52393026`；依赖身份152 Windows registry版本匹配、278 lock项完整身份匹配，helper非完整workspace TLS功能闭包，未执行HTTPS。
- 修复契约：发现信息不是认证或安装授权。删除无依据认证标识；若保留审核状态必须绑定具体对象/范围/证据而非作者/topic。固定数据明确标离线快照及其来源/观察时间，否则未知，不伪装实时。branch不作Release tag，未知版本/能力不猜；保留在线搜索、筛选和明确错误/离线状态。owned fixture/真实catalog负向均不再显示认证；实际可靠来源/版本查询正向不得被统一清空冒充完成。Task3-B 接续单writer实现，当前下载安全writer范围不扩展；尚未修复、未原生验收。
- 历史修复测试与范围纠正：`b136197ee` exit0 的 `verify.cmd` 实际切换到共享 `D:/ProxyWeave`，未使用隔离冻结源；只执行市场测试、plugin Clippy、宿主编译与前端测试/构建，**未执行此前误记的平台/下载器/协议全量测试**。历史 187 个前端测试成功不等于当前完整 Task3-B 冻结或原生市场验收，Vite chunk warning 也不能称零警告。原收据保留，由 `D:/ProxyWeave/.pi/inspection/task3b-reviewer/receipt-correction.json` 更正。Task4 已改变其中六个交付文件，具体 SHA 差异见 `post-start-identity.json`；仅未变的 market/Discover 子集可独立补验，其他门禁需原字节恢复或重新集成冻结。当前不据旧收据宣布 DEF-032 全项关闭。
- 隔离子集补验结果（任务 `bc9bfa343` exit0）：11 项惰性 catalog/详情 SSR 断言、4 项 marketplace 测试通过；六项内置 `verified` 全 false、固定星数全 None，真实传入星数仍显示。四个捕获文件 SHA 与清单完全匹配，278 registry 锁身份无偏差；收据 `D:/ProxyWeave/.pi/inspection/task3b-reviewer/catalog-subset-review-receipt.json`。本次零网络、零 IPC、非原生验收；六项固定版本/Release tag 仍未标来源或观察时间，故 DEF-032 保持 IN_REVIEW，不将虚假认证子集通过扩大成全项关闭。

### DEF-033 — CompatibilityUnknown 未阻断更新，测试许可暴露于正常库

- 状态：Verified；P1。
- 冻结：`D:/ProxyWeave/.pi/inspection/task4-reviewer/stage3-review/source-identity.json`，67 项原始业务文件；Host SHA256 `58bc7af7fbbe7fb54b594ba55d3bd1f6f5ebc089573c1d8dab861af0b08a6e5b`，activation SHA256 `3390a775426bcc36a793145e2cc349f24a53535270376314517498f1bab20a85`。执行与记录时主树对应原字节相同。
- 定位：`D:/ProxyWeave/apps/desktop/src-tauri/src/host.rs:766-770`：`if !permit.is_compatible() { /* comment only */ }` 未返回错误。`D:/ProxyWeave/crates/proxyweave-plugin/src/activation.rs:605-618` 的测试许可构造器在普通库中公开可调用，注释不是编译期隔离；不据此宣称远程可直接调用。
- 复现：逐字提取 Host 兼容分支，生产判定为 CompatibilityUnknown 却继续；惰性 Registry 包可推进至 switched version 2.0.0。普通库依赖亦能调用 public test_permit 并取得 Compatible。结果 `registry-results.json` SHA256 `4d5fe7491118777c6fd0f4b9451be93d5711ca86bad3c21c33ebb68ddaf89bc4`。
- 修复契约：在网络、版本发布、停止及 activation 修改之前明确拒绝 compatibility_unknown；Stopped 同样拒绝，保留旧 PID/文件/数据/网关。生产无可信证据来源时保持安全拒绝，不由远端 bool、调用方字符串或插件 ID 授权。测试许可隔离于不可达生产的可信测试边界；保留 owned 正向测试而不暴露生产开关。完整 Host 命令拒绝副作用须另测。
- 独立复验通过：`D:/ProxyWeave/.pi/inspection/task4-reviewer/stage3-verification/verification-receipt.json`，任务 `b58b1d669` exit 0。Developer 01a180b6 在 `apps/desktop/src-tauri/src/host.rs` 实施前置生产兼容判定，生产环境无来源时返回 `CompatibilityUnknown` 即刻 Err 拒绝更新，前置于网络、下载、暂存与生命周期副作用；在 `crates/proxyweave-plugin/src/activation.rs` 将 `test_permit` 与 `test_incompatible` 收拢至 `#[cfg(any(test, feature = "test-support"))]` 编译条件，正常生产库构建不暴露测试许可。复验 harness 确证兼容拦截且生产隔离完好。

### DEF-034 — 公共启停与无持有者 cancel 绕过事务维护栅栏

- 状态：Verified；P1。
- 冻结 runtime SHA256 `7e37db055c0ef8e0336c06980d40502aa64de0d8c97c13e64894518dcf9dac57`。定位 `D:/ProxyWeave/crates/proxyweave-plugin/src/runtime.rs:241-346` 维护/退出接口，以及 `D:/ProxyWeave/crates/proxyweave-plugin/src/runtime.rs:568`、`D:/ProxyWeave/crates/proxyweave-plugin/src/runtime.rs:778`、`D:/ProxyWeave/crates/proxyweave-plugin/src/runtime.rs:827-834` 的 maintenance=false。
- 复现：真实自有 HTTP fixture 旧 PID 17088，进入维护 generation 1 后 fence=true；公共 stop 后 fence=false，普通 start 生成 PID 32152 / generation 3，显式退出维护或提交前 acquire_request 已成功。另一次没有维护持有者凭证的公共 cancel 成功并重新准入。中止维护等待后 fence 仍保留是正向事实，但无凭证 cancel 能打开它，不能作为安全恢复。所有自有进程最终清理。
- 证据：`D:/ProxyWeave/.pi/inspection/task4-reviewer/stage3-review/runtime-results.json` SHA256 `f1439716d04918d5743cde6a85d15f449f525d51ddb959ed6afafc54ab983002`。
- 修复契约：复用同一每插件 FIFO/租约准入，维护所有权覆盖事务全程；普通 start/stop/restart/acquire 拒绝绕过，仅持有者内部生命周期操作可进行，旧 token/generation 不可撤销新维护。避免持锁递归调用公共启停死锁；Drop/取消/恢复失败保持 fenced/manual_recovery_required，只有确认恢复或提交可开放。确定性并发、真实 Gateway SSE 租约、取消与旧 token 回归均须冻结复验，External 无损。
- 独立复验通过：`D:/ProxyWeave/.pi/inspection/task4-reviewer/stage3-verification/verification-receipt.json`，任务 `b58b1d669` exit 0。Developer 01a180b6 在 `crates/proxyweave-plugin/src/runtime.rs` 实现 `MaintenanceToken` 凭证化所有权栅栏。进入维护后普通 start/stop/restart/acquire fail-closed 返回 `ServiceUnderMaintenance` 且保持 fence=true，普通操作拒绝不清除维护栅栏；仅持有效 token 可执行 `stop_plugin_under_maintenance`、`start_plugin_under_maintenance`、`exit_maintenance` 与 `cancel_maintenance_barrier`。复验 harness 确证真实子进程在维护期正常操作被拒绝、凭证操作成功执行、且退出维护后成功重新准入。

### DEF-035 — 激活新版记录与普通启动包目录不同代

- 状态：Verified；P1。
- 冻结 registry SHA256 `a83d08a8996eca84a0b054e67b2bf0e6c554bb6cccae8bb3c2c481def2789440`。定位 `D:/ProxyWeave/crates/proxyweave-plugin/src/registry.rs:409` 的 load 与 `D:/ProxyWeave/crates/proxyweave-plugin/src/registry.rs:552-557` 的 package_dir；Host 普通启动组合在 `D:/ProxyWeave/apps/desktop/src-tauri/src/host.rs:1103-1104`，重启组合在 `D:/ProxyWeave/apps/desktop/src-tauri/src/host.rs:1248-1249`。
- 复现：提交 owned 惰性 v2 后 load 返回 manifest 2.0.0；package_dir 却仍指向含 v1 文件的旧容器，按该路径取字节与 v2 Manifest SHA256 不符；真正 active version 目录字节与 v2 声明一致。因此仅将 UI 记录更新为 v2 不代表后续普通启动会用 v2。
- 证据：`D:/ProxyWeave/.pi/inspection/task4-reviewer/stage3-review/reopen-results.json` SHA256 `aaaeb31e5aa6d85a1436b4e0781c8eaabcd9ba6d9f605db6f5c0de6511e41d07`。
- 修复契约：以单次 coherent resolution 返回 record+active package path+activation stamp；容器/文件锁目录 API 与活跃包 API 明确分离。审查启动/重启/配置资源/Registry 读/Backend 调用链，防激活切换穿插后的混代，不能只替换 package_dir 实现导致版本目录递归。测试 legacy、重开、两版不同 exe/资源与真实 owned 身份响应、旧 Backend generation 拒绝及恢复失败；不得运行真实用户新版。
- 独立复验通过：`D:/ProxyWeave/.pi/inspection/task4-reviewer/stage3-verification/verification-receipt.json`，任务 `b58b1d669` exit 0。Developer 01a180b6 在 `crates/proxyweave-plugin/src/registry.rs` 提供 `resolve_active_package(id)` 单代一致解析接口，原子返回 `(PluginRecord, PathBuf, Option<ActivationStamp>)`；`host.rs` 的普通启动与重启接入该接口，彻底消除 load 与 package_dir 分离导致的记录与版本目录混代。复验 harness 确证 active version 目录读取字节与 v2 声明 SHA256 精确一致，旧容器文件隔离不被误读。

### DEF-036 — 不可变版本 key 冲突被静默复用旧包

- 状态：Verified；P2。
- 定位：`D:/ProxyWeave/crates/proxyweave-plugin/src/registry.rs:708-733` 的 store_version_package；target 已存在即读 record 返回，未核验请求候选包身份。冻结与共同证据同 DEF-035。
- 复现：version_key 已发布 2.0.0 后，再用同 key 请求 manifest 3.0.0 的不同包，collision_rejected=false，调用成功但返回 2.0.0。存在路径不等于请求候选已验证，也不等于实际安装了请求的新 release。
- 修复契约：复用 staging/Manifest/包摘要/source 绑定校验；只有精确相同 Manifest/fingerprint、实际包字节、仓库/release/asset/摘要来源身份才允许幂等复用，缺失或冲突准确拒绝且保留原包/激活。并发同 key 相同与不同候选、损坏既有包、source 不同及声明/实际 hash 不符均回归；不按 tag/版本文字相同就授权复用。
- 独立复验通过：`D:/ProxyWeave/.pi/inspection/task4-reviewer/stage3-verification/verification-receipt.json`，任务 `b58b1d669` exit 0。Developer 01a180b6 在 `crates/proxyweave-plugin/src/registry.rs` 的 `store_version_package` 增加候选包身份与完整性校验。当目标版本目录已存在时，必须验证候选包的 Manifest、文件指纹（sha256/fingerprint）与来源一致才允许幂等返回；不同候选包请求同 key 立即返回冲突错误（`immutable version key conflict`），拒绝静默复用旧包。复验 harness 确证相同包幂等复用成功，不同包碰撞准确拒绝。

#### DEF-033—036 共同复现收据与限制

- 初始复现收据：`D:/ProxyWeave/.pi/inspection/task4-reviewer/stage3-review/review-receipt.json`，任务 `b280b19c0` exit0：三个测试成功采集并断言旧缺陷存在，结论 **FAILED_STAGE3_SAFETY_REVIEW**，不是修复通过。原始日志 SHA256 `1ed0456bd440e2ef253d0ad758ec0c0d76060322b814267a37337e5ddd22cc44`。
- 独立复验通过收据：`D:/ProxyWeave/.pi/inspection/task4-reviewer/stage3-verification/verification-receipt.json`，任务 `b58b1d669` exit 0，结论 **VERIFIED_STAGE3_SAFETY_BOUNDARIES**。由独立隔离副本运行两项严格断言 `reviewer_verified_maintenance_fence_and_token_protection` 与 `reviewer_verified_registry_and_compatibility` 均绿，四项缺陷全数独立闭环。
- Reduced workspace 保持产品 registry 依赖版本/source/checksum 和解析依赖边子集，不声称完整桌面 feature 闭包等同；早期 harness 格式/锁问题与失败日志保留，不记产品缺陷。
- 观察到持锁时 load 有界拒绝、锁释放后未提交激活回旧版、真实 active 包 hash 正确及 Owned 清理，这些局部正向不消除四项缺口。Stage1/2 旧共享树测试收据不得外推安全事务通过。
- 本次无真实 GitHub 下载、完整 Host 更新 IPC、真实插件/Keyring/用户数据/IDE 写入、原生桌面更新验收、Goal 恢复或提交推送。修复 writer 在发现页 writer 冻结释放后顺序授权；Main 不写业务源码。
