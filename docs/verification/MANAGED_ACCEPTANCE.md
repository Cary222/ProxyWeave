# 首个本地 Managed 插件目标：实施与独立验收契约

历史目标标识：`mv44jirh-kepwhn`（regular；原始 objective 历史由 goal 系统保存）。当前完整验收契约见本文件及 [唯一状态汇总](PHASE1_REVIEW_STATUS.md)；本次不恢复/聚焦/修改任何 Goal。

## 完成门禁

- 按 Feature 1 → 2 → 3 → 4 实施，每个 Feature 提供独立 Git commit 或完整 SHA256 冻结快照；来源版本、原/目标路径、必要适配、原测试与本地回归命令、未通过项及独立复验步骤齐全后送 Inspector。
- 自测通过不等于 Verified。送审后冻结相关文件；审查通过前不启动依赖该 Feature 的运行行为。Job Object 先独立 fixture 验证并获得门禁，再首次启动 wb2api。
- 最终复验对象是实际 ProxyWeave Windows 桌面客户端，不是 Pi Web、浏览器壳、仅 API 脚本或 MCP 工具可用性。Inspector 必须经对应桌面 MCP 亲自点击/输入/导入/配置/启动/停止/重启/重开完成全部用户流程并明确最终全项 Verified，才可申请 goal complete。
- 最终交付冻结源码、对应内嵌 Windows exe 指纹、可导入真实插件包指纹、非敏感操作说明、隔离测试 profile 启动方式、真实 E2E 复验步骤。凭据经安全配置入口或 Keyring 注入，不在报告/intercom中明文传送，不读取/搬运原插件 auths/data。
- 有效测试凭据/上游缺失时真实 Ready/推理保持未通过，继续其他独立工作并如实报告阻塞，不以 Mock 完成真实请求。后续 Goal 生命周期操作只遵循当次明确授权，本文件不触发恢复/结束目标。
- 开发会话不写 `docs/DEFECTS_LEDGER.md`、`.pi/inspection/**` 或审查员复验沙箱，不使用共享 dev server 作为对方复验来源。

## 进度与证据层级入口

唯一当前进度为 [PHASE1_REVIEW_STATUS](PHASE1_REVIEW_STATUS.md)：含 Phase 0 限定 Windows PASS、F1/F3 历史独立冻结、F2 具体 DEF 与整体门禁的区别、F4/后续 UI 原生局部证据，以及完整 8 项的 IN_REVIEW/原矩阵 pending 和待验层级表。本契约不再维持另一份“F1 实施中未送审、F2–4 待开发”旧进度表。

源码基线 `3471d077d657adc7e64826bac0a9d09f91d35347` 的整体 Phase 1 **IN_REVIEW**；代码落地与历史局部成功保留，完整 8 项统一原生冻结证据不足，不能关闭。旧 `.pi/inspection/managed-goal-reviewer/acceptance-matrix.json` 为历史 review 资产，1…8 仍 pending，不由 Developer 修改；它也不是当前失败重跑。

此目标包含 Phase 1、必要 Phase 2 局部行为以及最小 Phase 3 UI；Phase 2/3 局部已实现不等于整阶段通过。不扩展在线市场/自动编译/下载升级/完整 API Doctor/IDE/跨平台发布。本次仅文档纠偏，不恢复或修改 Goal，不授权运行插件/凭据访问/提交推送。

## 有界目录导入送审补充（不替换原八项门禁）

此补充仅交付既有原生“本地目录”入口的有界导入机制：显式完整 Manifest 优先；否则固定审计资产与 root/bin 映射；声明文件和 LICENSE 必需验 hash；拒绝未知产物、路径/links/reparse、歧义及冲突。开发证据限离线合成 fixture 与源码检查，状态 ReadyForReview，不是 Verified。已核验候选源码 `vcs.modified=true`，hash 仅完整性；此交付未读取真实 `D:/workbuddy2api-panel`、原凭据/config/auths/data，未执行插件。

Inspector 集成前须核对冻结七文件及 before hash，并从独立快照复验；原生使用新隔离 profile，不覆盖旧记录/凭据。同 ID 的旧 fingerprint 冲突维持拒绝，不因目录功能自动迁移。真实 exe 与 LICENSE 仍须当场重新核验，不允许未知新 hash 自动准入。

完整八项验收仍全部保留，目录 fixture 不抵消任何一项待验：
1. 真实 Windows 客户端输入本地克隆目录、导入、秘密安全配置、重开持久化；不能用 API/浏览器替身代替。
2. 真实 Ready/PID/当前端点与身份；缺账号/503 不算 Ready，不误判端口冲突或崩溃。
3. 统一入口 models、非流式与 SSE，真实可用模型/有效凭据；Mock 不代替上游请求。
4. 错误、取消、端口冲突、崩溃分类与恢复；不伪造成功终止。
5. 并发启动及活跃请求/长流保护，实际网络观察取消/断连。
6. Owned 停止/重启、正常与异常宿主退出回收，External 完全无损。
7. 凭据/文件权限/日志保密；首次真实启动前确认 host 及子进程 stdout/stderr 重定向 NUL，不能将原始插件输出写入任务日志；此 lane 未改 runtime。
8. 桌面行为与构建/回归测试。`/panel/` browser 是既有兜底入口，不是安全嵌入路线已完成证据；最终仍依产品首选嵌入政策及 Inspector 原生验证。

以上是原契约的实用核对索引，不修改唯一状态汇总、历史审查矩阵或 Goal。缺有效凭据/上游、原生证据或安全前置条件时，对应项继续未通过并明确阻塞。
