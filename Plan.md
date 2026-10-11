# ProxyWeave 产品与技术架构规划书 (Plan)

> **版本**：v3.1.0 (Direct Reuse First & Production Engineering Edition)  
> **Slogan**：Plug In. Route Anywhere.（即插即用，随心路由）  
> **核心标语**：Turn AI Proxy Projects into Plug-and-Play Apps.（让每一个 AI 代理项目，都能成为即插即用的应用。）  
> **产品定位**：Local-first AI Proxy Application Platform & Desktop Workbench（本地优先的 AI 代理应用集成平台与统一工作台）  
> **技术定位**：Local-first AI API Plugin Host（本地优先的 AI API 插件宿主与统一工作台）  
> **核心哲学**：上游易逝，平台长存 · Everything is a Plugin（万物皆插件） · **成熟实现直接复用优先 (Direct Reuse First)**  
> **主开发规划路径**：`Plan.md`（根目录权威文档）与 `docs/PLAN-ProxyWeave.md`（架构设计归档）  
> **当前阶段**：**Phase 0 已通过（限定 Windows 基座）；Phase 1 Feature 1 实施中，未独立 Verified**。首个本地 Managed 初版按 Feature 1→4 独立冻结送审；网关/Managed 尚未运行，最终真实 Windows 客户端由 Inspector 桌面 MCP 全项复验后才完成目标。见 `docs/verification/MANAGED_ACCEPTANCE.md`；不将必要 Phase 2/3 局部能力标为整阶段完成。

---

## 目录 (Table of Contents)

- [0. 启动红线与开发前置流程 (Prerequisites & Launch Gate)](#0-启动红线与开发前置流程-prerequisites--launch-gate)
- [1. 核心宗旨与顶层定位 (Product Manifesto)](#1-核心宗旨与顶层定位-product-manifesto)
- [2. 开源脚手架采用策略与 Quotio Desktop 深度审计 (Scaffold Adoption Audit)](#2-开源脚手架采用策略与-quotio-desktop-深度审计-scaffold-adoption-audit)
- [3. 协议与网关核心参考源：TiyGate 深度分析 (TiyGate Protocol Reference)](#3-协议与网关核心参考源tiygate-深度分析-tiygate-protocol-reference)
- [4. 多引擎与微内核参考源：Tunnel Agent 与 DeepSeek Harness](#4-多引擎与微内核参考源tunnel-agent-与-deepseek-harness)
- [5. “成熟实现直接复用优先”强制开发政策与 SOP (Mandatory Reuse Policy)](#5-成熟实现直接复用优先强制开发政策与-sop-mandatory-reuse-policy)
- [6. ProxyWeave 目标系统架构与模块设计 (Target Architecture)](#6-proxyweave-目标系统架构与模块设计-target-architecture)
- [7. 插件规范（Manifest Schema 1.0）与生态发现 (Plugin Specification & Ecosystem)](#7-插件规范manifest-schema-10与生态发现-plugin-specification--ecosystem)
- [8. 交互体验与四大产品差异化功能 (UX & Key Differentiators)](#8-交互体验与四大产品差异化功能-ux--key-differentiators)
- [9. 安全模型与数据持久化治理 (Security & Persistence)](#9-安全模型与数据持久化治理-security--persistence)
- [10. 路线图与门禁体系 (Milestones & Release Gates)](#10-路线图与门禁体系-milestones--release-gates)
- [11. 威胁模型与安全边界界定 (Threat Model & Mitigations)](#11-威胁模型与安全边界界定-threat-model--mitigations)
- [12. 附录与实施核验索引 (Appendix & Reference Matrix)](#12-附录与实施核验索引-appendix--reference-matrix)

---

## 0. 启动红线与开发前置流程 (Prerequisites & Launch Gate)

### 0.1 研发纪律铁律：开发前置流程
在 ProxyWeave 项目中，**绝对禁止未立项先写代码、未定规范先搞实现、未建仓库先堆功能**。必须严格遵循以下前置阶段：
1. **产品命名确立**：确立正式独立品牌 `ProxyWeave`，杜绝通用化、临时性代号。
2. **独立仓库与纯净历史**：
   - 建立全新的独立代码仓库：`https://github.com/Cary222/ProxyWeave`；
   - 统一 Git 提交作者身份与签名，确保每次提交精确关联至官方所有者；
   - 杜绝直接从任何历史脏项目全盘 Fork 继承垃圾提交。
3. **顶层宗旨与规划先行**：
   - 将《ProxyWeave 产品宗旨》作为正式的第一版 `README.md` 推送至主干，完成 Initial Commit。
   - 编制全景架构规划书 `Plan.md` 与参考源登记表 `docs/references/REFERENCE_REGISTRY.md`。
4. **门禁核验通过后方可编码**：
   - 只有当 Phase -1 所有门禁全部验证通过后，方可进入 Phase 0 脚手架审计与代码引入。

### 0.2 当前仓库状态核查与代号归档
- **GitHub 仓库状态**：已正式就位，远程地址为 `https://github.com/Cary222/ProxyWeave`。
- **历史代号归档**：历史探索期使用的 `GlueHarness` / `APIDock` / `RelayCraft` 等代号已正式归档，旧规划草案已移至 `docs/PLAN-GlueHarness.md`，不得再作为当前实现的依据。

### 0.3 Phase -1 Launch Gate 检查清单 (当前统一状态)

| 门禁代号 | 检查项 | 准入/准出条件 | 核验依据 | 状态 |
|---|---|---|---|---|
| **GATE-01** | 独立品牌与 GitHub 仓库 | 确认仓库创建且与历史污点隔离 | `github.com/Cary222/ProxyWeave` 创立 | 🟢 **Done (已通过)** |
| **GATE-02** | 宗旨文档与开源协议 | `README.md` 与 `LICENSE` (MIT) 就位 | Commit `e323089` 归档 | 🟢 **Done (已通过)** |
| **GATE-03** | 权威规划与架构规范冻结 | 根目录 `Plan.md` 与镜像完成编制 | 本规划书编制核验 | 🟢 **Done (已通过)** |
| **GATE-04** | 脚手架与复用政策确立 | 确立直接复用优先与严禁 AI 重写条文 | `ADR-001` 就位 | 🟢 **Done (已通过)** |
| **GATE-05** | 参考源档案与按 Feature 前置审计 | 登记候选源；仅对本阶段实际引入闭包完成审计，其他模块开发前再核验 | 本阶段实际引入范围及来源已登记；后续候选仍待开发前核验 | 🟢 **本轮前置审计完成** |
| **GATE-06** | 阶段范围守护 | Phase -1 零产品代码；Phase 0 仅桌面基座，不引入网关、协议或插件功能 | 本轮仅桌面胶水，不含网关/插件 runtime | 🟢 **范围符合** |

---

## 1. 核心宗旨与顶层定位 (Product Manifesto)

### 1.1 核心问题与痛点定义
当前开源 AI 反代与 2API 生态空前繁荣，但呈现严重的碎片化与一次性脚本化问题：
- **上游易逝，生命周期短**：逆向接口极易失效，单一反代项目平均存活周期仅 3~6 个月；
- **部署维护成本畸高**：各类项目语言杂乱（Go、Node.js、Python、Rust），往往绑定重量级数据库（MySQL、Redis、Docker），桌面用户难以部署使用；
- **进程管理缺失**：后台常驻失控、内存泄漏、端口冲突、退出后残留僵尸进程；
- **接口不统一**：各类下游工具（Cursor、Claude Code、Windsurf、Continue 等）需要分别配置不同的端口与密钥，迁移成本极高。

### 1.2 ProxyWeave 的三大设计原则
1. **上游易逝，平台长存 (Upstream is Transient, Host is Eternal)**：
   ProxyWeave 绝不绑定任何特定的上游反代实现，自身不做任何脆弱的模型逆向逻辑。核心定位为**跨平台的极简插件宿主与本地 API 网关**，上游项目仅作为即插即用的无状态黑盒插件接入。
2. **万物皆插件 (Everything is a Plugin)**：
   无论是 Go 编译的单二进制（如 wb2api）、Node.js 编写的会话代理、Python 运行的推理转接，还是本地已有运行的外部端口服务，统一通过声明式 `manifest.yaml` 纳入管理。
3. **成熟实现直接复用优先 (Direct Reuse First)**：
   底层工程基础设施（跨平台进程组回收、本地动态端口无碰撞分配、异步 HTTP 流式转发、SSE 状态机解码、操作系统凭据托管）绝不盲目从零手写。在满足需求、许可证允许且依赖可控的前提下，必须直接复用或模块级移植成熟开源代码，再做最小必要适配。**严禁阅读参考代码后让 AI 重写一份相同逻辑并谎称复用！**

### 1.3 本地黑盒与验证项目的分类接入原则
针对本地开发与联调涉及的历史项目与现存服务，确立严谨的技术接入原则：
- **存在公开源码的项目 (如 `D:\workbuddy2api-panel`)**：在全面核验其 Go 源码（v1.13.0, commit `8461419`）、构建流程与 MIT 许可证后，允许作为首发 Managed 插件进行模块级复用或二进制内嵌打包。
- **已有运行服务 (如 `D:\CLIProxyAPI_...`)**：本阶段未实测其 28317 端口；本地未核验内部源码，不能推断项目没有公开源码。**Managed / External 取决于进程所有权，而不是源码可得性；已有服务仅按 External Provider 挂载，禁发 Kill，禁止假称复用其源码。**
- **存在许可证冲突的历史工程 (如方案一参考库 `D:\workbuddy-switch-gateway\...`)**：因其包含历史 GPL-3.0 代码，**严禁直接复制代码文件，仅作为避坑经验与算法逻辑参考**（如时间戳毫秒与秒换算、`0o600` 临时配置原子写入机制）。


## 2. 开源脚手架采用策略与 Quotio Desktop 深度审计 (Scaffold Adoption Audit)

### 2.1 脚手架采用政策：主脚手架采用 vs 功能模块复用的清晰界限

在工程实践中，必须严格区分“主脚手架采用”与“功能模块复用”，杜绝混淆概念：

| 维度 | 主脚手架采用 (Main Scaffold Adoption) | 功能模块复用 (Feature Module Reuse) |
|---|---|---|
| **适用范围** | 仅限单一首选工程骨架（已锁定 Quotio Desktop）。 | 除主脚手架外的所有功能点（TiyGate, Tunnel Agent, DSH 等）。 |
| **引入方式** | 允许完整获取固定版本仓库，用于审计并建立初始开发基线。 | **严禁整体合并其仓库或业务系统**；必须按需提取单一模块。 |
| **保留范畴** | 直接保留合适的跨平台外壳、平台抽象、构建流水线和基础通信。 | 提取该模块及最小必要局部依赖、类型定义与单元测试。 |
| **定制边界** | 在保留版权与许可证前提下，大刀阔斧剥离无关业务，开展二次开发。 | 严禁只复制代码片段而抛弃可靠性机制；保持薄适配层隔离。 |
| **核心纪律** | **不要为了品牌统一而盲目重写已适用的桌面脚手架！** | **严禁让 AI 重新生成一套相同功能的逻辑并宣称复用！** |

---

### 2.2 Quotio Desktop 权威深度审计

#### 1. 基础元数据与核验状态
- **仓库地址**：`https://github.com/xiaocoss/quotio-desktop`
- **锁定 Commit**：`21d75d08b38a23f97fbd534a1768a829cb147f2c`
- **开源许可证**：MIT License（完全商用友好，无传染性）
- **技术栈（真实配置）**：Tauri v2 + Rust 2021 Workspace + React 19.1 + Vite 7 + npm（`apps/desktop/package-lock.json`）
- **核验状态**：Git HEAD、v0.7.7 标签、干净工作区及 MIT 版权已核验；独立副本与裁剪后基座的构建/测试、当前 Windows 桌面行为已验证通过。详见 `docs/verification/PHASE0.md`，其他平台及插件功能未验收。
#### 2. 源码模块逐项审计与取舍决策

| 实际上游路径 | Phase 0 处理与边界 |
|---|---|
| `apps/desktop/src-tauri/`、`apps/desktop/` | 先验证原工程，再直接保留窗口、托盘、单实例及适用构建配置；逐步移除业务 IPC 与视图。 |
| `crates/quotio-platform/src/lib.rs` | 单文件含系统路径、Keyring、命令执行、原子写入与业务资源定位；不是独立 keyring/runner/autostart 文件。按真实调用闭包裁剪，保留对应测试。Windows sensitive 权限分支是空操作，不能视为 ACL 隔离。 |
| `crates/quotio-types/` | 根据保留代码调用关系渐进裁剪，内部 crate 名可保留。 |
| `crates/quotio-core/src/lib.rs` (`AppCore`) | 优先裁剪现有实现；若桌面生命周期无必要核心依赖，仅保留明确胶水职责，不重新构建核心框架。 |
| `crates/quotio-core/src/agent_config.rs` | 后续 Phase 3 IDE Handoff 候选；本轮除实际依赖外不引入。 |
| `crates/quotio-core/src/proxy_download.rs` | 后续安装器候选，下载、校验、解压实现仍有复用价值；不因 CPA 常量整体否定。 |
| `bridge.rs`、`usage_store.rs`、vendor 与专有业务 | 按依赖逐轮剥离并验证，不批量删除后重写替代实现；同步桥不作为目标网关。 |
#### 3. 为什么坚决剥离其核心业务逻辑？
1. **网络架构代差**：Quotio 采用同步阻塞的 `tiny_http` 搭建网关，每个连接独占线程，在流式 SSE 长连接下极易耗尽系统线程池；ProxyWeave 必须采用全异步的 `Axum 0.7 + Hyper 1.0` 架构。
2. **硬编码耦合严重**：Quotio 将 CLIProxyAPI 视为唯一特化下游，业务代码与 CPA 配置深度绑定，不具备通用插件扩展能力。

---

## 3. 协议与网关核心参考源：TiyGate 深度分析 (TiyGate Protocol Reference)

### 3.1 基础元数据与核验状态
- **仓库地址**：`https://github.com/tiylabs/tiygate`
- **锁定 Commit**：`6db40e54663455fb99d675bd8985e4dc50f68a14`
- **开源许可证**：Apache-2.0
- **技术栈**：Rust 1.88+, Axum 0.7, Hyper 1.0, Tokio
- **核验状态**：Git 基线已核验；下列模块路径/版本为候选规划，Phase 2 开发前按真实源码、许可证和测试复核，当前未引入或验收

### 3.2 核心模块审计与复用规划

```text
tiygate (6db40e54)
├── crates/core/src/ir/          -> 🟢 极高价值！内部统一中间表示 (Intermediate Representation)
│   ├── request.rs               -> 统一 LLM 请求结构抽象
│   └── response.rs              -> 统一 LLM 响应结构抽象
├── crates/protocols/src/        -> 🟢 模块级复用！成熟的多协议编解码器
│   ├── chat_completions.rs      -> OpenAI Chat Completions 协议编解码与流式解析
│   ├── messages.rs              -> Anthropic Claude Messages 协议编解码与流式解析
│   └── responses.rs             -> OpenAI 最新 Responses API 协议支持
├── crates/server/src/app.rs     -> 🟡 架构参考：Axum 0.7 路由编排与中间件设计
└── crates/store/                -> ❌ 坚决排除！SQLx 与数据库持久化依赖，ProxyWeave 保持纯内存无 DB
```

### 3.3 协议编解码复用规范
- **协议转换核心思想**：不写 `N × M` 个网状双向转换器，统一通过 TiyGate 的 IR（中间表示）进行汇聚转换：
  `Source Protocol -> IR -> Target Protocol`。
- **直接提取与隔离**：提取 `crates/protocols/` 与 `crates/core/src/ir/` 放入 `crates/proxyweave-protocol/`，保留其单元测试与用例；在外部提供面向 ProxyWeave 上下文的最小适配层。


## 4. 多引擎与微内核参考源：Tunnel Agent 与 DeepSeek Harness

### 4.1 Tunnel Agent：多引擎管理与桌面交互参考
- **仓库地址**：`https://github.com/beyondhumane/tunnel-agent`
- **锁定 Commit**：`9bfb7779b80de5e205a388e2e02d5840a3ed8b47`
- **开源许可证**：MIT License
- **核验状态**：仅 Git 基线核验；未完成本轮 Feature 源码审计或行为验收
- **复用决策与技术边界**：
  - **排除项**：其自身基于 C# .NET Avalonia 开发，技术栈与 Rust/Tauri 异构，**绝对不引入 .NET 运行时或 C# 源码**。
  - **借鉴项**：核心借鉴其“多反代引擎统一生命周期管控”状态机架构、多引擎并发调度逻辑、极简桌面系统托盘联动与多引擎状态面板排版布局。

### 4.2 DeepSeek Harness：微内核与插件插槽参考
- **仓库地址**：`https://github.com/deepseek-ai/deepseek-harness`
- **锁定 Commit**：`d743267388641bc76f17c45ce8b4c231aed1d32c`
- **开源许可证**：MIT License
- **核验状态**：仅 Git 基线核验；未完成本轮 Feature 源码审计或行为验收
- **复用决策与技术边界**：
  - **排除项**：其自身基于 Node.js/TypeScript 与 Cordis 框架，偏向于 Agent 任务与浏览器自动化；**绝对不引入 Node.js 运行时作为宿主核心**。
  - **借鉴项**：深度借鉴其“微内核 + 插件插槽 (Plugin Slots)”设计哲学、“可逆副作用 (Reversible Effects)”声明周期管理模型（插件卸载时自动注销路由、清理临时沙盒文件、归还分配端口）。

---

## 5. “成熟实现直接复用优先”强制开发政策与 SOP (Mandatory Reuse Policy)

### 5.1 核心开发政策终极条款 (Core Policy Clauses)

> #### 强制性核心原则：成熟实现直接复用优先 (Direct Reuse First)
> 1. **严禁以参考之名行重新生成之实**：
>    **在开源世界中找到满足功能需求、许可证允许且依赖可控的成熟代码后，必须直接使用依赖或移植提取对应功能模块，再进行必要适配。严禁在阅读其源码上下文后，要求 AI 重新生成一套功能相同的实现，并将这种 AI 重写称为“复用”！**
> 2. **AI 在开发流程中的法定职责**：
>    AI 主要承担**源码检索、方案比较、代码提取、接口适配、集成测试与真实缺陷定位修复**，严禁让 AI 默认重写已有成熟算法或底层系统机制。
> 3. **五级实现决策阶梯 (Decision Hierarchy)**：
>    - **第一级：直接依赖成熟库**：成熟标准库或 crates.io 成熟 SDK 满足需求时，优先 `Cargo.toml` 依赖引入；
>    - **第二级：模块级直接提取**：成熟项目中存在合适独立模块时，提取该模块及最小必要局部依赖，**完整保留其异常处理、跨平台保护、边界条件与必要单元测试**；
>    - **第三级：薄适配层隔离**：在复用模块外部构建胶水适配层（Glue Adapter），适配 ProxyWeave 的上下文与类型；
>    - **第四级：记录原因后必要修改**：仅在发现明确的编译不兼容、安全漏洞或协议冲突时允许修改原实现，且必须在代码注释与 PR 中记录具体修改原因；
>    - **第五级：自研例外与准入机制**：仅在完成充分检索与横向比较后，证明业界确实无可用成熟实现或存在致命不可控依赖时，方允许自研。必须在 `docs/architecture/ADR-xxx.md` 中完整记录：检索结果、无法复用的技术依据、替代方案评估、实现范围与测试计划（不机械要求凑齐三个无关项目）。

---

### 5.2 每个 Feature 的标准开发 SOP (Step-by-Step Workflow)

每一个具体 Feature 的开发必须严格执行以下 10 步标准作业程序：

```text
1. 需求与验收确认 (明确输入输出契约、性能 SLA 与可测试目标)
       │
       ▼
2. 检索现有工程和本地参考源码 (核查 Quotio, TiyGate, wb2api 等已登记参考源)
       │
       ▼
3. 检索官方实现及外部成熟开源项目 (检索 GitHub, crates.io 高星活跃实现)
       │
       ▼
4. 阅读真实源码与单元测试 (查阅具体分支/Commit 的代码实现与测试用例)
       │
       ▼
5. 比较评估 (契约契合度、开源许可证、依赖树体积、跨平台安全性、适配成本)
       │
       ▼
6. 登记采用决策 (在 REFERENCE_REGISTRY.md 中更新决策、锁定 Commit、记录依据)
       │
       ▼
7. 直接依赖引入 或 模块级代码提取 (保留原文件版权声明、局部依赖与完整测试)
       │
       ▼
8. 编写最小薄适配层 (面向 ProxyWeave 内部数据总线进行最小胶水封装)
       │
       ▼
9. 构建、测试与多端联调 (运行原模块自带测试与 ProxyWeave 集成测试)
       │
       ▼
10. 根据真实错误针对性修复并提交 (排查原因，禁止遇到报错直接推倒让 AI 重写！)
```

---

### 5.3 复用后修改与报错修复的严格约束

1. **先验证原模块，后做修改**：提取外部模块后，首先在隔离目录下执行 `cargo test` 验证原测试用例通过，确认基线完好后，再进行针对 ProxyWeave 的最小适配。
2. **完整保留工程可靠性资产**：提取代码时严禁贪图省事仅复制单个函数，必须一并保留原有的异常分支处理、边界校验、操作系统兼容分支、资源清理（Drop / Cleanup）及配套测试。
3. **针对性根因排查体系**：当构建或集成出现报错时，必须按以下 5 层排查链路精准归因：
   - `Layer 1: 原实现自身缺陷` -> 查验原项目 Issue 追踪是否有类似已知问题；
   - `Layer 2: 移植代码遗漏` -> 核对是否漏掉了特定的宏定义、辅助函数、特征标志（Cargo Features）或类型约束；
   - `Layer 3: 依赖版本冲突` -> 检查宿主与原模块引入的第三方 crate 版本（如 tokio 1.x, serde）是否存在破坏性语义差异；
   - `Layer 4: 接口适配问题` -> 检查胶水层类型映射、异步上下文生命周期是否正确；
   - `Layer 5: 本地编译环境问题` -> 检查构建工具链、C 运行时依赖或操作系统平台差异。
4. **绝对禁止的违规行为**：
   - ❌ **严禁因为集成报错就直接命令 AI “重写整个模块”！**
   - ❌ **严禁在未做任何检索的情况下直接要求 AI “实现一个 XX 模块”！**
   - ❌ **严禁为了代码“看起来整洁”而随意删除原作者经过实战打磨的防御性代码与边界判断！**
5. **前置安全处理**：不必等待报错才处理已经确认的安全问题、已知内存泄露或明显接口冲突，应在适配阶段前置解决并注明原因。


## 6. ProxyWeave 目标系统架构与模块设计 (Target Architecture)

### 6.1 目标系统架构拓扑 (Desktop -> Core -> Runtime)

```text
┌────────────────────────────────────────────────────────────────────────┐
│                   ProxyWeave Desktop (Tauri v2 + React)                │
│   [Tray / Window]    [Plugin Market]    [API Doctor]    [Flight Deck]  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ IPC Commands / Events
┌───────────────────────────────────▼────────────────────────────────────┐
│                   ProxyWeave Core (Microkernel Engine)                 │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ HTTP Gateway (Axum 0.7 + Hyper 1.0)                             │  │
│  │ 统一对外端口 (127.0.0.1:8080 可配置) | 本地 Master Key 鉴权认证        │  │
│  └──────────────────┬─────────────────────────────┬─────────────────┘  │
│                     │                             │                    │
│  ┌──────────────────▼───────────────┐ ┌───────────▼─────────────────┐  │
│  │ Protocol Adapters (TiyGate Codec)│ │ Model Router & Fallback     │  │
│  │ (Chat/Messages/Responses IR 转换)│ │ (虚拟模型映射 / 健康状态分流)  │  │
│  └──────────────────┬───────────────┘ └───────────┬─────────────────┘  │
│                     │                             │                    │
│  ┌──────────────────▼─────────────────────────────▼─────────────────┐  │
│  │ Credential Broker (Keyring + Unix mode / Windows ACL)          │  │
│  └──────────────────┬───────────────────────────────────────────────┘  │
│                     │                                                  │
│  ┌──────────────────▼───────────────────────────────────────────────┐  │
│  │ Plugin Supervisor & Allocator (Singleflight + Port Allocator)    │  │
│  │ (四象限生命周期调度 / 进程组 Job Object / 活跃引用计数器)           │  │
│  └──────────────────┬───────────────────────────────────────────────┘  │
└─────────────────────┼──────────────────────────────────────────────────┘
                      │
┌─────────────────────▼──────────────────────────────────────────────────┐
│                   Plugin Runtime (Managed & External)                  │
│                                                                        │
│   [Managed Plugin A]            [Managed Plugin B]   [External Provider]│
│   (如: wb2api, Go 单二进制)      (如: wild-work)      (如: CPA, 28317)  │
│   - Loopback 动态分配端口        - 按需冷启动/回收     - 仅挂载，禁杀禁管 │
│   - 绑定 Win32 Job Object       - 隔离沙盒目录       - 探针保活检测     │
└────────────────────────────────────────────────────────────────────────┘
```

---

### 6.2 建议目标 Rust Workspace 规划 (渐进式演进策略)

采取三阶段渐进式 Workspace 演进策略，平滑迭代：
以上为目标命名规划，不要求 Phase 0 重命名上游内部 crate；实际桌面闭包和来源以登记表及验证记录为准。

```text
proxyweave/
├── apps/
│   └── desktop/                  # Tauri v2 桌面外壳与 React 前端 UI
├── crates/
│   ├── proxyweave-core/          # [Phase 1] 目标宿主与生命周期；Phase 0 不重建核心框架
│   ├── proxyweave-platform/      # [Phase 0 复用 Quotio] 跨平台系统抽象与 Keyring
│   ├── proxyweave-gateway/       # [Phase 1 渐进演进] Axum 异步 HTTP 网关与 SSE 管道
│   ├── proxyweave-protocol/      # [Phase 2 提取 TiyGate] 协议编解码与 IR 转换
│   ├── proxyweave-plugin/        # [Phase 1 核心契约] Manifest Schema 解析与验证引擎
│   └── proxyweave-registry/      # [Phase 3] 插件市场索引、GitHub Release 下载器
├── plugins/
│   └── examples/                 # 内置参考插件 Manifest (wb2api, CPA)
├── docs/
│   ├── architecture/             # 架构与 ADR 归档
│   └── references/               # 参考源档案与审计记录
└── Cargo.toml                    # 顶层虚拟 Workspace
```

---

### 6.3 统一对外入口与端口分配机制 (Port Allocator)

#### 1. 统一对外入口
- 默认监听 `127.0.0.1:8080`，支持在系统设置中自定义更改。
- 启动时执行绑定预检，若端口被占用，弹出友好交互指引用户更换端口，杜绝无提示静默崩溃。

#### 2. 端口竞争预防与冲突恢复机制 (严谨区分预防与恢复)
在分配子插件动态端口时，必须清醒区分“预防措施”与“恢复兜底”，不能宣称探测后释放端口就完全消除了竞态：
- **A. 端口竞争预防 (Heuristic Collision Prevention)**：
  - 借鉴 `portpicker-rs`，采用带随机退避的动态端口初筛算法，尝试在 `[20000, 60000]` 范围内试探可用端口；
  - **客观认知**：试探成功后关闭临时 Socket 到子进程正式启动绑定之间，必然存在微秒级的 **TOCTOU (Time-of-Check to Time-of-Use) 竞态窗口**，外部其它进程可能在此瞬间抢占该端口。
- **B. 端口冲突重试与恢复循环 (Conflict Retry & Recovery Loop)**：
  - 真正的系统健壮性建立在启动异常捕获与恢复之上；
  - 启动 Managed 子进程后，以进程归属、声明身份和业务探针区分存活/HTTP 可达/Ready；只有明确绑定冲突证据（如 `EADDRINUSE` / `bind: address already in use` 或经核验的 Windows 对应错误）才执行换端口重试。健康探针超时、无可用凭据的 503 或 Busy 饱和 503 不能直接归因端口冲突、崩溃或触发重启；
  - 自动触发重试循环：将原端口加入黑名单，重新初筛新端口，更新子进程环境变量与命令行参数后重新拉起，最多重试 3 次；若重试 3 次仍失败，将插件标记为 `Failed` 并向 API Doctor 抛出结构化诊断报告。

---

### 6.4 异步流式网关与协议流水线 (Async Gateway & Pipeline)
- 基于 Axum 0.7 + Hyper 1.0 搭建全异步网关，彻底淘汰 Quotio 同步阻塞的 `bridge.rs`。
- **背压与取消传播**：当客户端（如 IDE）主动取消生成（关闭 HTTP 连接）时，网关通过 `tokio_util::sync::CancellationToken` 立即断开与上游插件的连接，停止无谓 Token 消耗与上游额度扣减。

---

### 6.5 插件双模运行架构：Managed vs External Provider

#### 模式 A：托管插件 (Managed Plugin)
- 二进制由 ProxyWeave 负责下载、完整性验证、释放、启动与监控；
- 分配动态本地回环端口（如 `127.0.0.1:AUTO_PORT`）；
- **能力边界与可测试目标**：在支持 Job Object 的 Windows 系统和支持 prctl 的 Linux 系统上实现子进程及子进程树的主动清理与宿主退出回收机制，并提供异常断电后的残留容灾探针与测试套件。

#### 模式 B：外部提供商 (External Provider)
- 用户接入已在本地或局域网运行的服务（如已有 CLIProxyAPI 监听 28317，或局域网 vLLM / Ollama）；
- ProxyWeave 仅建立反向代理路由与探针保活；
- **核心铁律：不视外部进程为宿主所有，绝对禁止向外部提供商发送 Kill 信号或强行接管其生命周期！**

---

### 6.6 插件生命周期状态机与四象限策略 (Lifecycle State Machine)

#### 1. 四象限生命周期策略
```text
┌──────────────────┬────────────────────────────────────────────────────────┐
│ 策略类别         │ 行为与调度规则                                         │
├──────────────────┼────────────────────────────────────────────────────────┤
│ 1. persistent    │ 核心常驻插件（如最常用的高频 2API），随宿主启动常驻，不休眠。 │
│ 2. idle_timeout  │ 空闲回收插件：无流量超过 TTL（默认 5 分钟）后优雅退出，释放内存。│
│ 3. on_demand     │ 极冷门或重型服务：冷启动拉起，请求处理完毕且无长会话后迅速释放。│
│ 4. external      │ 外部独立服务：宿主不拥有进程，永不执行启停杀死。           │
└──────────────────┴────────────────────────────────────────────────────────┘
```

#### 2. 完整状态转换表 (State Transition Table)

| 当前状态 | 触发事件 / 条件 | 目标状态 | 附带行为与同步保障 |
|---|---|---|---|
| **Stopped** | 收到调用流量 / 手动启动 | **Starting** | 申请端口初筛，创建 Singleflight 互斥锁，拉起子进程。 |
| **Starting** | 就绪探针成功通过 (HTTP 200) | **Ready** | 释放 Singleflight 锁，放行排队请求，`active_requests` 递增。 |
| **Starting** | 探针超时 / 启动报错 (端口冲突等) | **Starting (重试)** 或 **Failed** | 若冲突且重试 < 3，换端口重试；若超限，转为 `Failed` 并释放排队请求（返回 502）。 |
| **Ready** | 接收到业务请求 (`active_requests > 0`) | **Busy** | 活跃请求原子计数器加 1，重置空闲定时器。 |
| **Busy** | 业务请求完成 (`active_requests == 0`) | **Ready** | 活跃请求原子计数器减 1，启动空闲回收 TTL 倒计时。 |
| **Ready** | 空闲时间达到 TTL (仅 `idle_timeout` / `on_demand`) | **Stopping** | 发送优雅退出信号 (`SIGTERM`)，启动 5 秒强制退出计时器。 |
| **Stopping** | 子进程正常退出 / 超时 SIGKILL 退出 | **Stopped** / **Evicted** | 清理临时沙盒凭据与端口绑定，进入休眠（内存物理释放为 0MB）。 |
| **Failed** | 用户手动重启 / 配置更新 | **Starting** | 重置重试计数器，重新进入启动流程。 |

#### 3. 并发条件与请求同步规则
- **Singleflight 冷启动互斥**：当突发高并发流量（如 10 个请求同时到达）命中处于 `Stopped` 或 `Evicted` 状态的插件时，通过 `tokio::sync::OnceCell` 或 Singleflight 锁**仅触发一次拉起流程**；其余 9 个请求在 Tokio 异步队列中安全挂起等待，直至状态变为 `Ready` 后批量放行，杜绝重复启动导致的端口碰撞与资源争抢。
- **停止中与新请求进入的同步**：若插件处于 `Stopping` 阶段，新到达的请求将被阻塞排队（等待最多 2 秒）；若进程顺利退出，则紧接着由 Singleflight 锁重新安全拉起；若处于 `Busy` 状态（`active_requests > 0`），**严禁触发空闲回收或主动退出**。

---

### 6.7 跨平台系统级进程管控机制
- **Windows**：通过 Win32 `CreateJobObjectW` 创建作业对象，配置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，将每个 Managed 进程句柄绑定到该 Job。宿主退出或被任务管理器结束时，Windows 内核自动清理子进程。同时在启动时扫描残留孤儿端口提供容灾探针。
- **Linux**：在子进程 `CommandExt::pre_exec` 中注入系统调用 `libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL)`，当父进程死亡时内核自动发出 SIGKILL。
- **macOS**：创建父子继承的匿名管道（Inherited Pipe Watchdog），子进程在后台线程阻塞读取该管道 EOF，父进程退出导致管道读破裂时自动触发子进程退出。

---

### 6.8 SSE 流式协议与终止符规整 (Streaming & Termination)

#### 1. 补发终止符的前提条件
必须具有**协议层正常完成证据**：
- HTTP 响应状态码必须为 200 OK；
- 上游流已传输完整的数据块，且最后一帧明确包含合法的完成标识（如 `finish_reason: "stop"` 或 `"end_turn"`）；或者上游依照 SSE 规范自然完整关闭连接且无未完结的 JSON 片段；
- 此时若上游未发送末尾结束标记，网关才允许规范补发终止符。

#### 2. 异常截断坚决不补发
- 若上游遭遇网络异常断开、超时截断、HTTP 4xx/5xx 错误、或中间件抛错中断，**绝对禁止补发 `data: [DONE]`**！
- 网关必须向下游推送符合规范的结构化错误事件帧（Error Frame）或直接断开 TCP 连接；
- **事故防御考量**：若在异常截断时盲目追加 `[DONE]`，下游代码编辑器（如 Cursor、Claude Code）会误判代码生成已完整闭合，直接将残缺截断的代码应用到用户工程中，引发不可逆的代码损坏事故。

#### 3. 区分各协议独立终止规则
- **OpenAI Chat Completions**：正常终止帧为 `data: [DONE]\n\n`；
- **Anthropic Messages**：正常终止帧为 `event: message_stop` 事件块，伴随 `event: message_delta` 中的 `stop_reason`；
- **OpenAI Responses API**：正常终止事件为 `event: response.completed`；
- 协议转换器必须遵循目标协议的终止规范，严禁混用。

---

### 6.9 协议兼容矩阵 (Protocol Compatibility Matrix)

ProxyWeave 将协议适配能力严格划分为 5 级标准，明确能力边界：

| 源客户端协议 | 目标插件协议 | 支持等级 | 转换与降级处理规范 |
|---|---|---|---|
| **OpenAI Chat** | **OpenAI Chat** | **透传 (Passthrough)** | Headers/Body 原样透传，零额外序列化与流式延迟损耗。 |
| **OpenAI Chat** | **Anthropic Messages** | **支持转换 (Supported)** | 消息角色映射（system/user/assistant）、多轮对话拼接、标准 Tool Calls 参数片段规整。Anthropic 独有的 `thinking` 块降级为 Markdown 折叠文本或 System Prompt 引导。 |
| **OpenAI Responses** | **OpenAI Chat** | **支持转换 (Supported)** | Responses API 状态机与 Chat Completions 双向映射，流式事件转换。 |
| **Anthropic Messages** | **OpenAI Chat** | **支持转换 (Supported)** | `tools` 结构转换为 OpenAI Functions/Tools 规范，流式 `content_block` 映射为 `delta`。 |
| **多模态图像 (Image)** | **多模态图像 (Image)** | **降级适配 (Degraded)** | 统一将 Base64 / URL 图像数据块转换为目标协议支持的对象格式；超出分辨率限制时按比例降采样。 |
| **音频流 / 实时 WebRTC** | **文本 API 插件** | **明确拒绝 (Rejected)** | 存在不可逾越的协议与介质鸿沟，网关直接返回 `400 Bad Request` 伴随明确错误码，拒绝伪造转换。 |
| **厂商自定义专属扩展** | **任意目标协议** | **未验证 (Unverified)** | 同协议透传保持请求原始内容及字段，不静默剥离或改写。需要转换而能力未验证时返回明确错误/降级说明，不伪造支持。 |

---

### 6.10 客观可度量的性能 SLA 目标与压测规范
- **内存开销**：
  - `proxyweave-core` 守护进程核心常驻内存目标 < 25MB；
  - Tauri WebView 界面打开时总内存目标 < 85MB；
  - 插件休眠（进程退出）后，其自身系统内存占用物理降为 0MB。
- **网关附加延迟**：
  - 纯透传（OpenAI 兼容模式）网关附加延迟 P95 < 1.5ms；
  - 跨协议转换（Messages -> IR -> Chat）附加延迟 P95 < 4.0ms。
- **冷启动响应时间**：
  - Go 单二进制插件（如 wb2api）冷启动挂起等待时间约为 150 ~ 300ms；
  - 依赖运行时或较重插件依据其二进制特性测量，在 UI 显示真实冷启动耗时。


## 7. 插件规范（Manifest Schema 1.0）与生态发现 (Plugin Specification & Ecosystem)

### 7.1 Manifest 契约优先原则与前置研发要求
> **核心架构决策**：  
> **必须将最小 Manifest Schema 1.0 契约、凭据传递规范与安全配置提前到首个 Managed 插件开发之前完成定义与固化！**  
> 绝对禁止在未确定规范的情况下仓促手写特定插件（如 wb2api）的硬编码适配代码。宿主对所有插件一视同仁，统一按契约驱动。

### 7.2 Manifest Schema 1.0 规范定义 (14 个核心维度)

每个插件由根目录下的 `manifest.yaml` 统一定义，包含 14 个严谨字段：

```yaml
schema_version: "1.0"
metadata:
  id: "wb2api"
  name: "WorkBuddy 2API Engine"
  version: "1.13.0"
  description: "WorkBuddy 账号转接代理网关，内嵌多模型 Web 面板"
  author: "linguo2625469"
  homepage: "https://github.com/linguo2625469/workbuddy2api-panel"
  license: "MIT"

compatibility:
  min_host_version: "0.1.0"
  supported_os: ["windows", "darwin", "linux"]
  supported_arch: ["x86_64", "aarch64"]

runtime:
  type: "binary" # binary | external
  entrypoint:
    windows: "bin/wb2api.exe"
    darwin: "bin/wb2api"
    linux: "bin/wb2api"
  args: ["-config", "{CONFIG_PATH}", "-port", "{PORT}"]

lifecycle:
  policy: "idle_timeout" # persistent | idle_timeout | on_demand | external
  idle_timeout_seconds: 300
  startup_timeout_seconds: 15

network:
  listen_host: "127.0.0.1"
  port_allocation: "dynamic"
  health_check:
    type: "http"
    path: "/healthz"
    interval_seconds: 30

capabilities:
  protocols: ["openai_chat", "streaming", "tool_calling"]
  exported_models:
    - id: "claude-3-7-sonnet"
      upstream_model: "claude-3-7-sonnet-20250219"
    - id: "gpt-4o"
      upstream_model: "gpt-4o"

security:
  requires_secrets: ["accounts_token"]
  permissions:
    allow_internet_access: true
    allow_local_storage: true
```

---

### 7.3 安全边界界定与三大概念透明声明 (Security Concept Boundaries)

为保障技术诚实性与用户知情权，ProxyWeave 在规范中严谨厘清三大概念的差异：

1. **权限声明 vs 实际操作系统权限**：
   Manifest 中的 `security.permissions` 属于**声明式契约与用户告知机制**，用于在安装时向用户展示插件声称的能力并在宿主内部作为参数限制；在未运行于内核沙箱的前提下，外部启动的可执行文件具备当前操作系统用户的完整执行权限。**权限声明绝对不等于操作系统底层安全沙箱！**
2. **SHA256 完整性校验 vs 密码学数字签名**：
   宿主对 GitHub Release 资产执行 SHA256 校验仅用于验证安装包在网络下载过程中未损坏且与清单哈希一致（完整性验证）；在发布者未提供公私钥证书（GPG / Authenticode）进行签名验证前，**该校验不代表密码学身份认证或代码签名！**
3. **独立子进程 vs 安全沙箱**：
   Managed 插件运行于独立子进程中，具备内存隔离与崩溃隔离（插件崩溃不影响宿主运行）；但这属于多进程操作系统的天然特性，**独立子进程不等于安全沙箱**，用户必须谨慎评估安装来源。

---

### 7.4 真实 Manifest 样例验证状态说明
当前 `manifests/` 目录下的所有样例文件（`wb2api.manifest.yaml`, `cpa.manifest.yaml`, `wild-work.manifest.yaml`）均已明确标注：
- **验证状态**：🟡/⚪ **待验证 (Pending Live Verification)**；
- **说明**：当前作为 Schema 1.0 语法样例，尚未在 ProxyWeave 运行态完成端到端加载与联调测试。将在 Phase 1 核心引擎跑通后逐一实测闭环。

---

## 8. 交互体验与四大产品差异化功能 (UX & Key Differentiators)

### 8.1 极简暗色工程看板交互设计
- 借鉴 DeepSeek Harness 与 Tailscale 的工业风设计美学，采用 Tailwind CSS 构建高对比度、低干扰的暗色系仪表盘；
- 状态栏与卡片以最直观的方式展示：统一端口状态、插件四象限运行状态（内存物理占用、端口、活跃连接数）、冷启动延迟指标。

### 8.2 四大差异化产品功能
1. **API Doctor 全链路脱敏追踪与诊断**：
   - 捕获下游请求到上游响应的全生命周期耗时；
   - 自动过滤脱敏敏感 Headers（Authorization Bearer、Cookie、Key）；
   - 对 401 凭证失效、429 频率超限、502 网关无响应等常见错误提供智能归因与排错建议。
2. **Compatibility Flight 质检工作台**：
   - 内置一键自动化协议质检测试套件，提供 6 项端到端质检：OpenAI Chat 文本生成、SSE 流式首字延迟、多轮对话角色映射、Tool Calling 参数片段规整、长文本截断压力测试、多模态输入兼容性。
3. **IDE Handoff 自动配置引擎**：
   - 复用 `quotio-core/src/agent_config.rs` 的 54KB 精密注入算法；
   - 一键将 ProxyWeave 统一端点与 Master Key 写入 Cursor (`~/.cursor/extensions`)、Claude Code、Windsurf 等开发工具，杜绝手工复制配置的繁琐。
4. **Git-Native 零维护去中心化生态发现**：
   - 粘贴任意标准 GitHub 仓库 URL 即可解析安装；
   - 基于 GitHub Topic (`#proxyweave-plugin`) 实现社区插件聚合发现，无需维护昂贵的中心化元数据服务器。

---

## 9. 安全模型与数据持久化治理 (Security & Persistence)

### 9.1 系统安全模型与凭据隔离
- **凭据物理分离**：普通配置（端口、路由映射）保存在本地 JSON；API Token 与密钥通过 `keyring-rs` 托管至操作系统底层安全凭据库。
- **凭据临时投影（Phase 1 待实现）**：Unix 采用私有目录与 `0o600` 文件权限；Windows 必须单独设计与验证 ACL，不能以 `0o600` 宣称隔离完成。原子写入及退出清理不等于安全擦除；本轮不传递插件凭据。
- **本地 API Key 鉴权**：统一对外入口默认生成高强度本地随机密钥（`pw-sk-...`），客户端访问必须携带 Bearer Token，防御同机恶意脚本未授权调用。

### 9.2 数据持久化单文件原子治理
- **彻底杜绝外置数据库依赖**：不引入 SQLite、MySQL、PostgreSQL 或 Redis。
- **配置持久化容灾机制**：
  - 单文件 JSON 序列化；
  - 写入流程：内存序列化 -> 写入 `.tmp` 临时文件 -> `fsync` 刷盘 -> 原生原子 `rename` 覆盖目标文件；
  - 伴随生成 `.bak` 双副本，若解析出现格式损坏，自动读取 `.bak` 执行无损回滚。


## 10. 路线图与门禁体系 (Milestones & Release Gates)

### 10.1 研发门禁与状态基线 (统一权威状态)

| 门禁代号 | 门禁名称 | 状态 | 准入/准出条件与核验依据 |
|---|---|---|---|
| **Phase -1 Gate** | **项目命名与权威规范门禁** | 🟢 **已通过 (PASSED)** | 1. 确定正式产品名 `ProxyWeave`；<br>2. 建立独立权威仓库 `https://github.com/Cary222/ProxyWeave`；<br>3. 编写权威开发政策与参考源登记表；<br>4. 统一团队提交作者身份与签名规范。 |
| **Phase 0 Gate** | **脚手架审计与工程初始化门禁** | 🟢 **本轮 Windows 范围通过 (PASSED)** | Git 实读固定 Commit `21d75d08b38a23f97fbd534a1768a829cb147f2c` / v0.7.7 / MIT；原副本 build/test 后原样引入并分轮裁剪；目标 fmt/check/test 与 embedded Tauri build 通过，窗口/托盘/关闭/单实例实测通过。证据与未验证边界见 `docs/verification/PHASE0.md`；非全平台或完整产品验收。 |
| **Phase 1 Gate** | **核心微内核与 Manifest 规范门禁** | 🟡 **Feature 1 实施中，未 Verified** | 1. 固化 Manifest Schema 1.0 契约；<br>2. 跑通 Axum 0.7 异步网关；<br>3. 跑通端口初筛与冲突重试恢复；<br>4. 实现首个 Managed 插件 (wb2api) 端到端调用。 |
| **Phase 2 Gate** | **协议编解码与高级流式管道门禁** | ⚪ **待启动 (PENDING)** | 1. 移植 TiyGate IR 协议转换模块；<br>2. 跑通流式终止符规范补发与异常防御；<br>3. 完成 Compatibility Flight 质检套件自测。 |
| **Phase 3 Gate** | **桌面控制台与插件市场门禁** | ⚪ **待启动 (PENDING)** | 1. React + Tailwind CSS 极简暗色控制台；<br>2. Git-Native 插件发现与下载校验；<br>3. IDE Handoff 自动配置写入测试通过。 |
| **Phase 4 Gate** | **发布与全平台流水线门禁** | ⚪ **待启动 (PENDING)** | 1. GitHub Actions 多平台自动化构建；<br>2. 跨平台可执行资产打包与安装包发布；<br>3. 完整性校验与安全通告审计。 |

---

### 10.2 六阶段详细实施路线图 (Milestones)

#### Phase -1: 项目命名与权威规范初始化 (Week 0) - 【已完成】
- 建立独立仓库，建立规范体系，登记 15 个参考源与三级验证状态。

#### Phase 0: 脚手架审计、代码提取与基座搭建 (Week 1) - 【本轮 Windows 范围通过】
- 检出并核验 Quotio Desktop `21d75d08` 源码；
- 剥离不必要业务，保留 `quotio-platform` 与 Tauri 外壳；
- 原工程与最小基座分别构建/测试；目标 `cargo build` 零警告，并实测当前 Windows 窗口、托盘、关闭与单实例，证据见 `docs/verification/PHASE0.md`。

#### Phase 1: 异步网关微内核、Manifest 契约与首个插件验证 (Week 2)
- 实现 Manifest Schema 1.0 引擎与沙盒目录凭据投影；
- 搭建 Axum 异步网关、Singleflight 冷启动互斥与端口恢复循环；
- 接入并端到端联调首个 Managed 插件 (`wb2api`)。

#### Phase 2: 协议适配、SSE 管道与稳定性加固 (Week 3)
- 提取 TiyGate 协议编解码模块；
- 实现 OpenAI / Anthropic 协议双向映射与流式终止符规整；
- 加固 Phase 1 Managed 所有权与退出回收基线（Windows Job Object 需在首个 Managed 启动前验证）；运行 Compatibility Flight 功能质检。

#### Phase 3: 桌面控制台、API Doctor 与生态发现 (Week 4)
- 交付极简暗色工作区 UI 与系统托盘联动；
- 接入 API Doctor 全链路脱敏追踪；
- 实现 Git-Native 插件发现与 GitHub Release 资产下载器。
- 提取并验证 IDE Handoff 配置写入与恢复；未实现前不宣称可用。

#### Phase 4: Compatibility Flight 质检、打包与全平台发布 (Week 5)
- 运行 Compatibility Flight 全量自动化质检用例；
- 搭建 GitHub Actions 跨平台发布工作流；
- 发布 v0.1.0 社区预览版。

---

## 11. 威胁模型与安全边界界定 (Threat Model & Mitigations)

| 威胁场景 | 风险等级 | 潜在危害 | ProxyWeave 防御与缓释措施 |
|---|---|---|---|
| **恶意插件代码注入** | 高危 | 插件可能包含恶意逻辑窃取宿主权限 | 1. 严格声明：Manifest 权限声明为告知契约，非内核沙箱；<br>2. 独立子进程运行并做最小权限凭据投影（Unix 0o600；Windows ACL 待独立验证）；<br>3. 用户安装第三方插件时强制弹出风险与权限二次确认。 |
| **同机恶意脚本扫描** | 中危 | 恶意本地脚本尝试探测并调用 ProxyWeave 端口盗用配额 | 1. 默认仅监听本地回环 `127.0.0.1`，绝对禁止对外监听；<br>2. 强制启用本地随机 Master Key 鉴权，未携带有效 Bearer 拒绝访问。 |
| **子进程僵尸残留** | 中危 | 宿主异常退出或崩溃后，插件进程遗留占用 CPU 与端口 | 1. Windows 绑定 `CreateJobObjectW` 强制退出回收；<br>2. Linux 注入 `PDEATHSIG`，macOS 启用管道看门狗；<br>3. 宿主下次启动时执行孤儿进程清理与端口残留探针。 |
| **API 凭据泄漏** | 高危 | 用户模型密钥或平台 Token 明文暴露在磁盘配置文件中 | 1. 静态主凭据必须托管至操作系统底层 Keyring；<br>2. 临时子进程配置写在沙盒私有目录，Unix 权限 `0o600`、Windows 私有 ACL 分别验证，使用后删除（不保证安全擦除）。 |
| **流式截断代码损坏** | 高危 | 异常中断时下游编辑器误将残缺代码合入生产代码库 | 1. 严禁无条件补发 `[DONE]`；必须满足正常完成证据；<br>2. 异常时主动断开连接或推送结构化错误事件帧。 |

---

## 12. 附录与实施核验索引 (Appendix & Reference Matrix)

### 12.1 权威参考源快速索引
- 参考源全景登记表：`docs/references/REFERENCE_REGISTRY.md`（含 M01–M15 固定 Commit、路径、许可证及三级验证状态）。
- 架构决策记录：`docs/architecture/ADR-001-scaffold-and-reuse-policy.md`（脚手架采用与复用优先开发政策）。
- 历史文档标记：`docs/PLAN-GlueHarness.md`（已标记为归档历史参考，不作为当前执行依据）。

### 12.2 本文档权威声明
本文档为 ProxyWeave 项目的唯一权威架构与开发实施指南。所有后续代码开发、功能实现与测试集成必须严格遵守本文档所规定的开发政策与质量门禁。
