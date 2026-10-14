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
| DEF-015 | P1 | Verified (Containment) / 止损修复独立复验通过（只读预检仍待实现） | 非法仓库地址、零检查调用仍展示固定 Manifest 与 SHA256 校验通过 | Developer（UI 真值修复） |
| DEF-016 | P2 | Verified (Containment) / 止损修复独立复验通过（真实请求追踪仍待实现） | 空事件输入仍显示并导出四条虚构 HTTP 请求链路 | Developer（UI 真值修复） |
| DEF-017 | P2 | Verified (Containment) / 止损修复独立复验通过（系统自启/托盘设置仍待实现） | 设置仅本地状态却显示系统功能已开启，未探测凭据仍声称已校验/就绪 | Developer（UI 真值修复） |
| DEF-018 | P2 | Verified / 独立复验通过（前端与 IPC 缝合已验证，原生 HWND 回归于全量集成阶段核验） | 最小化/最大化为无操作 Promise；关闭 IPC 失败被吞掉 | Developer（Wave 3 Shell 真值修复） |
| DEF-019 | P2 | Verified / 独立复验通过 | 宿主读取失败被显示为零登记/空事件与网关 OFF，成功缓存被失败覆盖 | Developer（Wave 3 Shell 真值修复） |
| DEF-020 | P1 | Verified / 独立复验通过 | 转码请求将本地 x-api-key 与 Connection 指定头传至上游；已实现严格敏感头与提名头剥离并验证 | Developer & Feature Lead |
| DEF-021 | P1 | Verified / 独立复验通过 | 转码 SSE 缺少协议完成证据时在 HTTP EOF 补发 message_stop；已移除异常截断补帧并验证保持真实 EOF | Developer & Feature Lead |
| DEF-022 | P2 | Verified / 独立复验通过 | 转码流对每个字节分片独立 lossy UTF-8 解码改变文本；已引入成熟跨 chunk 增量解码并验证跨分片中文字符完好 | Developer & Feature Lead |
| DEF-023 | P2 | Verified / 独立复验通过 | 转码路径未应用 header_timeout 导致慢上游绕过网关超时限制；已实现 504 Gateway Timeout 超时与租约释放并验证 | Developer & Feature Lead |
| DEF-024 | P1 | Verified / 独立复验通过 | 转码路径未调用 check_lossy_conversion 静默丢弃能力；已接入协议层有损能力损失校验并验证返回 400 | Developer & Feature Lead |

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
