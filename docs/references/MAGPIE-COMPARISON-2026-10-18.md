# 竞品代码级对比：Magpie × ProxyWeave（2026-10-18）

审查对象：yetone/magpie @ `d783df46cbac9021e26a6bd71ca0b55544552eb2`（MIT）
对比基线：ProxyWeave @ `3471d07`
方法：Magpie 仓库 README、`go.mod`、`docs/subsystems/` 文档与 `internal/` 重点包源码抽查（pluginrt 全读、plugin/gateway/agent/provider/edit 量化统计）；未做完整构建与全量安全审计。证据分三级标注：**已实查**（本轮读过源码/文档）、**已量化**（统计脚本实测）、**未复核**（仅文档或外部描述）。

## 1. 定位边界（结论）

- Magpie 的管理对象是 **Provider / 模型 / 订阅账号 / Agent 配置**：给每个 Agent 选模型、写回其配置、额度耗尽时在账号间故障转移。网关 `127.0.0.1:3425` 四协议互转（OpenAI Chat / Responses / Anthropic Messages / Gemini）。
- ProxyWeave 的管理对象是 **独立代理应用及其运行实例**：插件包 → Manifest 校验 → 凭据托管 → 进程/端口托管 → 健康探针 → 统一入口 → 停止/升级/回滚。
- 本轮实查未在 Magpie 中发现与上述链路等价的通用契约：其插件是宿主内执行的 **Provider 能力代码**（登录/刷新/模型发现/fetch），不是被托管的可执行服务实例；进程所有权、Job 回收、端口冲突恢复、External 保护无对应物。**结论：相邻竞品与重点参考源，不构成脚手架替代；ProxyWeave 差异化主线（代理应用管理）成立且必须守住。**

## 2. 代码规模与测试密度（已量化）

| 模块 | Magpie（Go） | ProxyWeave（Rust） |
|---|---|---|
| 网关/协议 | `internal/gateway` 832 文件，src 63,806 行 / test 120,929 行（比 ≈1.9） | gateway src 1,008 / test 1,869；protocols src 14,199 / test 1,993（比 ≈0.14） |
| 插件体系 | `internal/plugin` src 7,898 / test 11,751；`internal/pluginrt` src 1,012 / test 463 | plugin crate src 3,490 / test 1,790 |
| Provider/账号 | `internal/provider` src 65,323 / test 65,592 | 无对应层（按定位不做账号聚合） |
| Agent 接入 | `internal/agent` src 37,807 / test 49,130（每客户端一文件） | desktop `agent_import.rs` 2,641 行等，宿主 Rust 共 7,677 行 |
| 配置编辑 | `internal/edit` src 2,999 / test 3,973（保格式编辑引擎） | agent_import 内含写配置逻辑，未独立成引擎 |
| 持久化 | SQLite（`modernc.org/sqlite`，`internal/sqlitedb` 薄封装） | 无 DB（红线） |

差距解读：Magpie 网关测试语料约为其源码 2 倍，是多年回归沉淀；ProxyWeave protocols 的测试比明显偏薄——这是 Phase 2 Compatibility Flight 要补的真实缺口，不是功能缺口。

## 3. 逐模块对比与借鉴判定

### 3.1 插件运行时契约 — `internal/pluginrt`（已实查，重点借鉴：设计）

- 语义：`Runtime.Reconcile(on, config)` 先按**逆序释放**不再需要的插件及其依赖方，再按 Requires/Provides 依赖序反复 Apply 至不动点；状态机 `off / pending / active / failed / disposed`；**Apply 失败或 panic 时回滚其已登记部分，且不连累其他插件**。
- 契约：Manifest 声明 `API` 兼容范围（caret，如 `^1.2` 对宿主 `1.0`）、`Requires / Provides` 服务、Essential 预设、`InUse` 守卫（在用插件不可关）；核心从不 import 插件、插件互不 import，**由测试 `TestPluginsImportOnlyCore` 强制**；配置经 `SchemaOf(Settings)` 由结构体生成 JSON Schema 并校验，手改非法值在 Compose 时回落默认。
- 对 ProxyWeave 的判定：**设计参考（高价值）**。Manifest Schema 1.0 后续演进（版本兼容范围、服务依赖声明、启停回滚语义、在用守卫）应显式对标此模型，并考虑以同类「边界测试」锁定宿主/插件 import 边界。Go 代码不移植；Rust 侧如实现，须按 Direct Reuse First 如实登记为设计参考后自研并留 ADR。

### 3.2 插件宿主 — `internal/plugin`（已实查，借鉴：语义与校验）

- 形态：Bun 运行 JS 插件（社区包来自 npm / magpie-community），宿主 `host.js` 承担调用状态、流式事件、异常与取消；Provider 所有权可在内置实现与社区插件间**迁移**（movers 权威映射、每项带最低插件版本、旧实现 retired 后未迁移账号显式列 `unmoved` 并给原因）。
- 声明校验：插件 options 的 key 正则、标签/描述/数量上限、重复 key 丢弃、default 只认 true——一组现成的防御性校验清单。
- 对 ProxyWeave 的判定：**设计/校验清单参考**。迁移状态机与 options 校验可对标；Bun 宿主形态与「插件即独立进程」定位冲突，不采用。

### 3.3 网关与路由 — `internal/gateway`、`internal/routes`、`internal/gwmw`（已实查文档与结构，借鉴：策略与测试维度）

- 路由策略只做**排序不做筛选**：Strategy 返回池内序号的一个排列，panic 或返回非法排列时 **fail-open** 回落声明顺序且每条路由仍会尝试；JS 策略 20ms 预算。路由组五模式 smart/order/rotate/usage/pace 与会话粘性（粘住已答账号保 prompt cache）。
- 失败策略分级：普通中间件/路由 fail-open；**脱敏（masking）fail-closed**——无可用脱敏器时返回 503 且不向厂商发送任何内容。用量观察者只收白名单字段（无 body/prompt/id）、有界队列、满了丢弃并计数、永不阻塞请求路径。
- 对 ProxyWeave 的判定：**Phase 2 Model Router 的 ADR 必须显式比较此策略集与 M06 既有候选（TiyGate/LiteLLM）后定案**；「扩展点按风险分 fail-open / fail-closed」「观察者白名单+有界队列」两条原则直接写入网关/审计设计约束。网关回归四维度（工具调用、流式转换、错误、取消）与 deferred usage 五面覆盖（透传/转码/账本/跨度/定价）作为 Compatibility Flight 的缺口对照表。

### 3.4 Agent 配置接入 — `internal/agent` + `internal/edit`（已量化，借鉴：行为与用例）

- 每客户端一个 wiring 文件（364 文件），配置改写由独立 `edit` 引擎承担：容错解析 JSONC/TOML/YAML、只改目标键、保留注释/顺序/格式；Profiles 为命名快照，一键切换全部 Agent 配置。
- 对 ProxyWeave 的判定：**行为参考 + 回归用例来源**。`agent_import`（源自 wb-switch）后续硬化时，把「改写后 diff 最小化、用户手改不被覆盖、异常配置 fail-closed」立为验收用例；Profiles 交互形态列入 Phase 3 控制台参考（宿主仍只原生渲染契约内同构内容，业务 UI 边界不变）。

### 3.5 用量与签到 — `internal/usage`、checkin 体系（已实查文档，借鉴：计量条约）

- Deferred usage：插件事后获知的用量随后步上报，网关仅在计费/账本/定价层合并，**不污染回给客户端的当步用量**（客户端据此算上下文占用）。
- 签到状态机：outcome 枚举（claimed/done/ineligible/inactive/captcha/failed）、按北京日结算、失败 30 分钟重试、双通道防重复、**captcha 宿主与插件永不代解**。
- 对 ProxyWeave 的判定：**设计条约参考**。未来用量统计立「计费口径与客户端可见口径分离」条约；WorkBuddy 类插件的签到/设备令牌处理（Magpie 内置 workbuddy_checkin 专门逻辑）作为 wb2api 生态旁证与插件侧设计参考——注意其属于插件业务，不进宿主核心。

### 3.6 明确不借鉴（已实查，红线/定位冲突）

| 项 | 原因 |
|---|---|
| SQLite 持久化（modernc.org/sqlite） | 触碰 No-DB 红线 |
| 读取各 Agent 签到文件/Keychain 充当订阅来源 | 与凭据契约（不读取/搬运原 auths、Keyring 托管投影）正相反 |
| Bun/JS 插件宿主作为核心运行时 | 与「插件即独立进程、宿主核心不用 Node/Bun」冲突 |
| Go + Wails 整体脚手架 | 与已定 Rust + Tauri 路线冲突；功能面（账号/用量/同步）远超当前阶段 |
| Go 代码翻写为 Rust | Direct Reuse First 明令：不可直接复用者只登记为设计/测试参考，不得冒充复用 |
| WebDAV/S3/GitHub 同步 | 非本地优先当前范围，不引入 |

注：其社区中间件包存在与 New API（AGPL 项目）「同 JSON 语义」的兼容实现；未来若评估相关格式，只对齐**格式语义**，不复制 New API 代码，评估时单独做许可证审计。

## 4. 产品设计层借鉴（交互形态，不含业务逻辑）

1. 路由决策实时视图：每次选路的原因可见——对应 ProxyWeave 审计轨迹/API Doctor 的可解释性目标。
2. 模型选择器以 `provider/model` 全局可搜索列表收敛复杂配置；模型元数据由 models.dev 补全（数据源候选，单独评估许可后登记）。
3. 配额窗口提醒（重置前大量未用额度提示）与「每日预热」是订阅型产品的留存设计——ProxyWeave 不做订阅聚合，仅借「额度/状态变化主动提示」的交互原则。
4. 子系统文档纪律：`docs/subsystems/` 14 篇与代码一一对应的设计文档，可作为 ProxyWeave ADR/Feature 文档组织的参照（保持我方证据分级：已实现/已独立验证/未验证分开）。

## 5. 落地顺序（与阶段门禁对齐）

1. 本文件与登记表 M16/竞品节、Plan 附录补充同批落盘，仅文档，不改任何阶段状态与代码。
2. Phase 2 启动时：以 §3.3 的维度表给 protocols/gateway 列 Flight 缺口清单；Model Router ADR 显式比较三方策略集。
3. Manifest 契约下一次演进时：以 §3.1 为对标清单逐项裁决（兼容范围、服务依赖、回滚语义、在用守卫、边界测试），裁决结果留 ADR。
4. Phase 3 控制台设计时：参考 §4 的交互形态，仍守能力页边界与极简规范。
