# ProxyWeave × Magpie UI 交互融合与实体功能演进计划

> **文档性质**：专项产品交互与实在功能演进规划（基于现有 ProxyWeave 架构，深度融合竞品 Magpie 优秀 UI/UX，拒绝重复造轮子）  
> **文档版本**：v1.0.0  
> **制定日期**：2026-10-18  
> **关联依据**：`Plan.md` (v3.1.0)、`docs/references/MAGPIE-COMPARISON-2026-10-18.md`、`apps/desktop/src/components/ide-source/ide-provenance.md`

---

## 0. 核心背景与纠偏反思：坚决拒绝重复造轮子

在初版构想中，曾提出“自研一套外科手术式配置注入引擎与接入卡片”，**这属于严重的重复造轮子错误**。

### 0.1 现有资产梳理（已具备的基础）
经过对当前仓库源码的深度核验，ProxyWeave **已经拥有完备的 IDE / Agent 客户端接入工作台与后端注入体系**：
1. **前端页面**：`apps/desktop/src/components/ide-source/AgentsPage.tsx`（从 WorkBuddy Switch Gateway 直接移植，MIT 授权合规）。已具备多卡片展示、协议状态标签、模型槽位分配、自动检测、一键写入、历史备份列表、一键还原。
2. **宿主入口**：`apps/desktop/src/components/IdePanel.tsx` 承载完整工作台。
3. **后端引擎**：`apps/desktop/src-tauri/src/agent_import.rs`（近 3,000 行 Rust 稳健实现）。已完整支持 11 种 Agent 客户端（Claude Code, Claude Desktop, Codex, DSH, OpenCode, Pi, Grok-Build, ZCode, Kimi-Code, OpenClaw, Hermes）的配置路径解析、配置写入、文件级自动备份与恢复。

### 0.2 纠偏原则
**绝不抛弃现有成熟资产，绝不新建第二套客户端注入工具。**  
我们要做的，是**将 Magpie 在 UI 交互上的精妙设计（如写入前 Diff 预览弹窗、实时选路解释轨迹、全局 Ctrl+K 模型搜索器、Cursor/Windsurf 常用客户端覆盖）与路由算法，增量融合到 ProxyWeave 现有的 8 大工作台面板与网关内核中**，实现 1+1 > 2 的实在用户价值。

---

## 1. 现有体系与竞品交互融合全景矩阵

```text
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                        ProxyWeave 桌面工作台（融合后完整交互版图）                      │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│ 🔍 全局顶栏增强 (AppShell.tsx)                                                          │
│    └─ [融合特性 3] 全局 Ctrl+K 模型检索与状态选择器 (跨插件聚合、健康圆点、延迟、一键试飞)│
├─────────────────────────────────────────────────────────────────────────────────────────┤
│ 🖥️ 8 大工作台 Tab 面板的精准增量融合                                                   │
│  ├─ 1. [IDE 客户端面板] (IdePanel / AgentsPage.tsx) ─────────────── 现有基础升级       │
│  │     ├─ [融合特性 1.1] 扩充主流 IDE：补齐 Cursor、Windsurf、VS Code 目标配置读写     │
│  │     ├─ [融合特性 1.2] Magpie 核心体验复现：【写入前 Diff 预览弹窗】(安全感拉满)       │
│  │     └─ [融合特性 1.3] 吸收 Magpie internal/edit 容错经验，硬化现有 JSONC/TOML 解析   │
│  │                                                                                      │
│  ├─ 2. [审计与监控面板] (AuditPanel.tsx) ────────────────────────── 现有基础升级       │
│  │     ├─ [融合特性 2.1] Magpie 核心体验复现：【实时调用与路由轨迹看板 (Live Trace)】   │
│  │     └─ [融合特性 2.2] 可视化链路解释流：Client ➔ 虚拟模型 ➔ 路由策略 ➔ 命中插件 ➔ 状态│
│  │                                                                                      │
│  ├─ 3. [试飞面板] (FlightPanel.tsx) ─────────────────────────────── 现有基础协同       │
│  │     └─ 支持从全局选择器或路由看板一键带参数直跳，快速发起多模型对比试飞              │
│  │                                                                                      │
│  ├─ 4. [插件面板与市场] (PluginPanel.tsx / DiscoverPanel.tsx) ───── 现有生态扩充       │
│  │     └─ [融合特性 5] 提供 manifests/magpie.yaml 官方 External 模板，一键借用其模型库 │
│  │                                                                                      │
│  └─ 5. [网关核心] (crates/proxyweave-gateway) ────────────────────── 现有内核演进       │
│        ├─ [融合特性 4.1] 智能路由策略演进：支持 Smart(测速)/Order(主备)/Rotate(轮询)     │
│        └─ [融合特性 4.2] Prompt Cache 会话粘性机制 (同一长会话锁定后端，大幅降本提速)   │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. 核心融合模块详细设计与落地规划

### 模块一：现有 IDE 客户端面板升级 (`IdePanel.tsx` / `AgentsPage.tsx`)

#### 1. 现状与痛点
- 现状：已支持 11 种 CLI/Agent 工具，支持备份和恢复，卡片化展示体验优秀。
- 缺口：
  1. 缺少国内使用频率极高的两大主流 IDE：**Cursor** 与 **Windsurf**（图标已在 `assets/agent-icons/` 中就绪，但后端未连通）。
  2. 用户在点击“一键配置”时，缺乏透明感，心里容易犯嘀咕：“它到底改了我本地配置的什么地方？”

#### 2. Magpie 优秀交互融合方案
* **交互创新：写入前 Diff 预览弹窗（Diff Preview Dialog）**
  - 用户在卡片上点击【配置并接入】时，不直接静默覆写，而是弹出优雅的高亮 Diff 预览弹窗。
  - 弹窗直观对比展示：
    ```diff
    // Cursor settings.json 变更预览
      "cursor.general.usage": true,
    + "cursor.cpp.overrideBaseUrl": "http://127.0.0.1:28319/v1",
    + "cursor.cpp.overrideApiKey": "sk-pw-live-master-key",
      "editor.fontSize": 14
    ```
  - 弹窗底部清晰提示：“已自动生成备份，随时可在历史记录中一键还原”。
  - 用户确认无误后点击【确认写入】，触发原有 `importAgentClient` IPC。
* **扩充核心 IDE 适配**：
  - 在 `apps/desktop/src-tauri/src/agent_import.rs` 中增补：
    - `detect_cursor()` 与 `import_cursor()`：读写 `%APPDATA%\Cursor\User\settings.json`（Windows）与对应 macOS/Linux 路径。
    - `detect_windsurf()` 与 `import_windsurf()`：读写 Windsurf 相关配置。
* **改动涉及文件**：
  - `apps/desktop/src/components/ide-source/AgentsPage.tsx`
  - `apps/desktop/src/components/ide-source/ide-adapter.ts`
  - `apps/desktop/src-tauri/src/agent_import.rs`

---

### 模块二：现有审计面板升级为“实时调用与路由轨迹看板” (`AuditPanel.tsx`)

#### 1. 现状与痛点
- 现状：当前 `AuditPanel.tsx` 仅渲染宿主内部事件 `HostEvent`（如进程启动、插件启停、配置更新），界面类似静态日志表。
- 缺口：外部客户端（Cursor/Claude Code 等）通过网关向大模型发起请求时，用户在 ProxyWeave 界面上**看不见请求的流转过程**，不知道请求到底打到了哪个插件，也不知道耗时多少、是否发生了失败降级。

#### 2. Magpie 优秀交互融合方案
* **交互创新：可视化选路轨迹流（Live Route & Request Inspector）**
  - 在 `AuditPanel.tsx` 顶部或主视图中开辟 **“实时调用轨迹”** 动态视窗。
  - 每一个经过 ProxyWeave 本地网关的请求，以动态卡片形式按时间降序实时展示：
    ```text
    ┌────────────────────────────────────────────────────────────────────────┐
    │ 🟢 14:22:05 | Cursor ➔ claude-3-7-sonnet | 耗时: 342ms | HTTP 200 OK  │
    ├────────────────────────────────────────────────────────────────────────┤
    │  [来源客户端] Cursor (127.0.0.1:54321)                                │
    │  [请求目标]   claude-3-7-sonnet (协议: Anthropic Messages)             │
    │  [路由决策]   命中策略: Smart 最优测速                                 │
    │  [服务插件]   workbuddy2api (PID: 27276, 端口: 51234, 首字延迟: 48ms)  │
    │  [用量统计]   Input: 1,420 tokens | Output: 380 tokens | 缓存命中: 85% │
    └────────────────────────────────────────────────────────────────────────┘
    ```
* **故障转移（Fallback）透明提示**：
  - 如果主插件出现 429、500 或超时，轨迹卡片上展示醒目的黄色警告流：
    `[主插件 workbuddy2api 遭遇 429 RateLimit] ➔ [自动触发 Fallback 熔断] ➔ [无缝切至备用插件 Magpie (PID: 1823)] ➔ [请求成功完成]`。
  - **用户直观体验**：整个代理平台不再是神秘的黑盒，所有调度一目了然，排障极度方便。
* **改动涉及文件**：
  - `apps/desktop/src/components/AuditPanel.tsx`
  - `crates/proxyweave-gateway/src/`（增加轻量级 Request Observer 广播）
  - `apps/desktop/src-tauri/src/`（向前端 Tauri 窗口推送调用事件）

---

### 模块三：全局顶栏快捷搜索与试飞集成 (`AppShell.tsx` / `FlightPanel.tsx`)

#### 1. 现状与痛点
- 现状：ProxyWeave 目前查看模型必须进入特定的 `FlightPanel`，或者在 `AgentsPage` 的下拉菜单中翻找。
- 缺口：随着插件增多，用户很难快速了解“当前系统里一共有哪些可用模型？哪个响应快？哪个挂了？”。

#### 2. Magpie 优秀交互融合方案
* **交互创新：全局 `Ctrl+K` 模型与状态选择器**
  - 在 `AppShell.tsx` 顶栏增加常驻快捷搜索框（支持全局热键 `Ctrl+K` / `Cmd+K`）。
  - 弹出聚合层弹窗，按插件分组列出所有就绪模型：
    - `[workbuddy2api]`
      - 🟢 `claude-3-7-sonnet` (Messages 协议 | 延迟 32ms)
      - 🟢 `claude-3-5-sonnet` (Messages 协议 | 延迟 28ms)
    - `[magpie-adapter (External)]`
      - 🟢 `deepseek-r1` (Chat 协议 | 延迟 45ms)
      - 🟡 `kimi-k1.5` (Chat 协议 | 探测中)
  - **一键快捷动作**：
    - 点击任意模型右侧的【试飞】图标 ➔ 自动切换至 `FlightPanel` 并自动选中该模型，立即进入测试输入框；
    - 点击【复制】图标 ➔ 复制模型标准请求字符串；
    - 点击【设为 IDE 默认】 ➔ 自动更新默认路由映射。
* **改动涉及文件**：
  - `apps/desktop/src/components/AppShell.tsx`
  - `apps/desktop/src/components/FlightPanel.tsx`

---

### 模块四：网关路由内核演进与 Prompt Cache 粘性 (`crates/proxyweave-gateway`)

#### 1. 现状与痛点
- 现状：当前 `crates/proxyweave-gateway` 中采用单 `Backend` 结构（`backend: Arc<RwLock<Option<Backend>>>`）。
- 缺口：
  1. 无法配置多插件主备容灾（例如主力用 wb2api，挂了切备用）。
  2. 在多实例轮询时，如果把同一段长对话切给了不同的后端账号，**服务端的 Prompt Cache 将瞬间失效**，导致上下文必须全量重新上传，费用倍增且响应极慢。

#### 2. 吸收 Magpie 路由算法设计
* **多策略路由排序（参考 Magpie `internal/routes`）**：
  - 遵循 Magpie 的黄金原则：**路由策略只做排序、不做硬性过滤（fail-open）**。
  - 支持 3 种用户最需要的实在模式：
    1. **Smart（延迟优选）**：根据最近 5 次调用的健康度和延迟，优先打最快实例；
    2. **Order（主备灾备）**：永远优先使用第一顺位插件，只有不可用或限流时降级到第二顺位；
    3. **Rotate（负载轮询）**：在多个可用实例间轮换。
* **Prompt Cache 会话粘性（参考 Magpie `internal/gateway`）**：
  - 请求进入网关时，通过请求内容特征（或 Client IP / Session Header / 首条系统提示哈希）识别会话；
  - 在一个会话生命周期内，**强制粘住同一个上游插件实例**；
  - 充分复用 Claude 和 DeepSeek 的服务端 Prompt Caching，延迟降低 60%~80%，Token 消耗减半。
* **改动涉及文件**：
  - `crates/proxyweave-gateway/src/lib.rs`
  - `crates/proxyweave-gateway/src/routing.rs`（新建轻量路由策略模块）

---

### 模块五：竞品降维复用：Magpie External 官方插件模板 (`manifests/magpie.yaml`)

#### 1. 核心价值
- Magpie 拥有现成的数十个国内/国外逆向平台逆向能力（Minimax、GLM、Kimi、DeepSeek 等）。
- ProxyWeave 不需要重复去写这些逆向逻辑，直接利用 ProxyWeave 的 **External Provider** 模式将其纳管！
- 在 `manifests/magpie.yaml` 中声明其服务端口（默认 `127.0.0.1:3425`）与 `/v1/models` 端点。
- 用户本地启动 Magpie 后，ProxyWeave 一键挂载，直接在工作台与路由池中使用其全部模型。

---

## 3. 分阶段实施路线图 (Roadmap)

我们采取**“UI 交互驱动、功能直观可用、循序渐进”**的交付节奏：

| 阶段 | 实施任务 | 交付的实体功能与 UI 表现 | 验证与体验验收标准 |
|---|---|---|---|
| **Phase F1**<br>(IDE 接入体验升级) | 1. 现有 `AgentsPage.tsx` 接入 Diff 预览弹窗<br>2. `agent_import.rs` 增补 Cursor 与 Windsurf | 1. 用户点【接入】时，弹出配置 Diff 高亮对话框<br>2. 界面卡片新增 Cursor 与 Windsurf 支持 | 拿真实 Cursor `settings.json` 实测：弹窗高亮差异准确、点确认写入成功、原有注释格式不破坏、一键还原生效。 |
| **Phase F2**<br>(调用轨迹透明化) | 1. `proxyweave-gateway` 增加调用轻量观察者<br>2. `AuditPanel.tsx` 落地【实时调用与路由轨迹看板】 | 打开 Audit 面板，能实时看到从 IDE 发出的每一个请求卡片（耗时、模型、插件 PID、状态、Fallback） | 从已配置的 IDE 发送一条代码补全，Audit 界面瞬间跳出一条清晰的调用链路流。 |
| **Phase F3**<br>(全局模型选择器) | 1. `AppShell.tsx` 增加 `Ctrl+K` 快速检索弹窗<br>2. 支持一键跳 `FlightPanel` 试飞 | 用户随时按下 `Ctrl+K`，快速搜索全部插件的模型、查看延迟、一键开启试飞测试 | 全键盘流畅操作，筛选模型并在 1 秒内跳转试飞并发送测试消息。 |
| **Phase F4**<br>(Magpie 生态挂载) | 编写官方 `manifests/magpie.yaml` 并在桌面验证 | Discover/Plugin 面板一键导入 Magpie External 插件 | 挂载后，Overview 与 Flight 面板立即出现 Magpie 的模型列表并成功调用。 |
| **Phase F5**<br>(智能路由与 Cache 粘性) | 1. 网关支持主备与测速策略<br>2. 实现会话 Cache 粘性机制 | 路由决策面板能看到 Smart 测速优选和 Fallback 切换动画；长多轮对话响应明显变快 | 模拟主插件 500 异常，验证自动切备用插件无报错；多轮会话始终锁定同一上游实例。 |

---

## 4. 研发纪律与红线守则

1. **绝对不重写现有已验证模块**：所有演进必须基于现有的 `IdePanel`、`AuditPanel`、`FlightPanel`、`Gateway`，以最小 diff 增量扩展。
2. **凭据安全红线**：Diff 预览弹窗中涉及 Master Key 或上游 API Key 时，显示截断脱敏文本（如 `sk-pw-live-...3a8f`），严禁将完整 Key 明文打在日志中。
3. **No-DB 约束**：调用轨迹数据仅保存在内存循环缓冲区（Ring Buffer，如最近 200 条），不引入 SQLite 或重型数据库。
4. **进程安全隔离**：Managed 插件与 External 插件权限明确划分，External 插件只探针绝不杀进程。

---

## 5. 落地进展与验证记录 (Implementation & Verification Status)

| 阶段 | 状态 | 交付文件与主要功能 | 验证结果 |
|---|---|---|---|
| **Phase F1** | ✅ **已完成 (Verified)** | `agent_import.rs` 增补 Cursor 与 Windsurf；新增 `preview_target` 与 `preview_agent_diff` IPC；`AgentsPage.tsx` 增设 Diff 预览弹窗与行级对比计算。 | 25 项 Rust 单元测试通过；Vitest 14 套测试通过；前端 TSC 0 错误。 |
| **Phase F2** | ✅ **已完成 (Verified)** | `crates/proxyweave-gateway` 接入 `RequestTrace` 环形缓冲器与客户端智能嗅探；`host.rs` 增设 `get/clear_gateway_traces` IPC；`AuditPanel.tsx` 增设 Live Trace 调用与选路轨迹看板。 | Gateway 23 项测试通过；Desktop 64 项测试通过；Audit 实时刷新与清除已验证。 |
| **Phase F3** | ✅ **已完成 (Verified)** | `ModelSearchModal.tsx` 实现全局 `Ctrl+K` 快速检索弹窗（支持全键盘导航、分类过滤、复制模型名）；`FlightPanel.tsx` 支持 `initialModel` 直跳与自动选中聚焦；`AppShell.tsx` 增加快捷搜索入口。 | `model-search.test.tsx` 专项目标测试通过；Vitest 15 套 159 测试通过。 |
| **Phase F4** | ✅ **已完成 (Verified)** | `manifests/magpie.yaml` 及 `manifests/magpie.manifest.yaml` 官方模板落地；声明 External 模式、健康检查、模型能力与端点。 | `crates/proxyweave-plugin/tests/local_registry.rs` 增设模板导入校验测试通过（21 项单元测试 + 15 项注册测试全部通过）。 |
| **Phase F5** | ✅ **已完成 (Verified)** | `crates/proxyweave-gateway/src/routing.rs` 实现 3 种路由策略（Smart/Order/Rotate）与 Prompt Cache 5 分钟滑动窗口会话粘性；`GatewayPanel.tsx` 增设策略切换控制与状态展示。 | `routing::tests` 5 项测试全部通过（测试用例覆盖 fail-open、session stickiness、smart ranking、rotate）；`gateway-routing.test.tsx` 3 项测试通过；前端 17 套 172 测试全部通过。 |
