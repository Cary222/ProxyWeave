# ProxyWeave 产品与技术架构规划书 (Plan)

> **版本**：v3.0.0 (Scaffold Adoption & Production Engineering Edition)  
> **Slogan**：Plug In. Route Anywhere.（即插即用，随心路由）  
> **技术定位**：Local-first AI API Plugin Host（本地优先的 AI API 插件宿主与统一工作台）  
> **核心哲学**：上游易逝，胶水永存 · Everything is a Plugin（万物皆插件）  
> **主开发规划路径**：`Plan.md`（根目录权威文档）与 `docs/PLAN-ProxyWeave.md`（架构设计归档）  
> **当前阶段纪律**：**仅允许调研核验、源码审计、方案论证与规划修订；严禁编写任何产品业务代码！**

---

## 目录 (Table of Contents)

- [0. 启动红线与开发前置流程 (Prerequisites & Launch Gate)](#0-启动红线与开发前置流程-prerequisites--launch-gate)
- [1. 核心宗旨与顶层定位 (Product Manifesto)](#1-核心宗旨与顶层定位-product-manifesto)
- [2. 开源脚手架采用策略与 Quotio Desktop 深度审计 (Scaffold Adoption Audit)](#2-开源脚手架采用策略与-quotio-desktop-深度审计-scaffold-adoption-audit)
- [3. 协议与网关核心参考源：TiyGate 深度分析 (TiyGate Protocol Reference)](#3-协议与网关核心参考源tiygate-深度分析-tiygate-protocol-reference)
- [4. 多引擎与微内核参考源：Tunnel Agent 与 DeepSeek Harness](#4-多引擎与微内核参考源tunnel-agent-与-deepseek-harness)
- [5. 强制 Reference-First 开发制度与参考源档案 (Reference-First Governance)](#5-强制-reference-first-开发制度与参考源档案-reference-first-governance)
- [6. ProxyWeave 目标系统架构与模块设计 (Target Architecture)](#6-proxyweave-目标系统架构与模块设计-target-architecture)
- [7. 插件规范（Manifest Schema 1.0）与生态发现 (Plugin Specification & Ecosystem)](#7-插件规范manifest-schema-10与生态发现-plugin-specification--ecosystem)
- [8. 交互体验与四大产品差异化功能 (UX & Key Differentiators)](#8-交互体验与四大产品差异化功能-ux--key-differentiators)
- [9. 安全模型与数据持久化治理 (Security & Persistence)](#9-安全模型与数据持久化治理-security--persistence)
- [10. 架构决策记录 (ADR - Architectural Decision Records)](#10-架构决策记录-adr---architectural-decision-records)
- [11. 重构后的实施里程碑与落地计划 (Milestones & Acceptance Gates)](#11-重构后的实施里程碑与落地计划-milestones--acceptance-gates)
- [12. 开源工程规范与项目文档体系 (Governance & Repository Standards)](#12-开源工程规范与项目文档体系-governance--repository-standards)

---

## 0. 启动红线与开发前置流程 (Prerequisites & Launch Gate)

### 0.1 强制门禁：进入功能开发的三大前提
在开始编写任何 ProxyWeave 核心业务功能代码之前，必须无条件满足以下前置流程（Phase -1 Launch Gate）：
1. **正式品牌与独立仓库创立**：
   - 在 GitHub 创建独立的全新专属代码仓库（建议名称：`proxyweave`）。
   - 彻底切断任何直接全盘继承历史债务的无审查 Fork 分支关系。
2. **产品宗旨与规范落地**：
   - 将《ProxyWeave 产品宗旨》作为正式的第一版 `README.md` 推送至主干，完成 Initial Commit。
3. **完成脚手架与参考源基线审计**：
   - 形成正式的脚手架采用审计结论与 Reference Source Registry，明确保留模块、剥离模块与复用边界。

### 0.2 品牌核验状态与命名合规声明 (Name & Availability Disclaimer)
- **正式产品品牌定义**：
  - **Product Name**：ProxyWeave
  - **中文名称**：ProxyWeave
  - **Slogan**：Plug In. Route Anywhere.
  - **技术定位**：Local-first AI API Plugin Host
  - **核心哲学**：上游易逝，胶水永存 · Everything is a Plugin
  - **建议仓库命名**：`proxyweave`
  - **核心 Crates 规划**：`proxyweave-core`（核心协调器）、`proxyweave-gateway`（异步协议网关）、`proxyweave-protocol`（协议转换编解码）、`proxyweave-runtime`（进程与外部运行时）、`proxyweave-plugin`（Manifest 与插件契约）、`proxyweave-registry`（插件源发现与索引）、`proxyweave-security`（凭证库与鉴权）、`proxyweave-platform`（跨平台系统抽象）。
- **可用性合规声明**：
  - 本规划中所述 GitHub 仓库名、组织名、域名及商标名称目前为**建议与设计命名**。
  - 在正式公开发布前，团队必须在 GitHub、crates.io、npm 及相关域名注册平台执行可用性与合规性检索，不得在未经核验前假称已完成所有商标与域名的官方注册。
  - 项目中的所有历史参考工程真实路径（如 `xiaocoss/quotio-desktop`、`tiylabs/tiygate`、`beyondhumane/tunnel-agent`、`D:\workbuddy2api-panel` 等）严禁因产品重命名而被错误替换，必须保持 100% 真实源码溯源。

### 0.3 阶段纪律：当前规划阶段的行为边界
- **当前阶段仅允许**：
  - 调研与核验 GitHub 真实开源项目；
  - 阅读并审计本地参考工程的真实源码；
  - 对比开源实现与产品目标，推导最佳技术路径；
  - 修订产品宗旨、架构设计、技术选型、开发计划与开源复用规范；
  - 建立正式的脚手架采用审计表与参考源矩阵。
- **当前阶段绝对禁止**：
  - 编写新的产品功能代码；
  - 未经源码审计直接克隆上游作为正式主干；
  - 随意删除原规划中有价值的需求、踩坑经验与本地真实资产；
  - 仅追加新章节而不修改旧章节中的冲突；
  - 未经源码验证就宣称某项目具备某项能力；
  - 凭 AI 幻觉编造开源项目的实现细节与测试结论。

---

## 1. 核心宗旨与顶层定位 (Product Manifesto)

### 1.1 核心定位与产品边界 (What ProxyWeave Is and Is Not)

**ProxyWeave 是什么：**
> **ProxyWeave 是一个轻量、安全、可扩展的本地 AI API 插件宿主与统一接入工作台。**
> 它专为开发者、AI 重度用户及团队打造，运行于个人电脑（桌面托盘/无头守护进程模式），将繁杂割裂的各类开源反代（2API）、商业模型转接工具、CLI 代理（CLIProxyAPI）或本地推理服务，以**黑盒插件（Blackbox Plugin）**的标准范式进行统一编排、按需启停、凭据托管、协议标准化与本地聚合路由。

**ProxyWeave 不是什么（边界划定）：**
- **不是另一个 CLIProxyAPI / 2API**：ProxyWeave 自身不硬编码任何特定上游平台的登录逆向、Cookie 抓取或逆向协议；所有特定平台的转接逻辑均为外部独立插件。
- **不是商业 API 分售与计费管理平台**：不以商业售卖、多租户团队额度切分、支付网关对接、企业复杂审批流为目标（那是 New API / One API 的生态位）。
- **不是重型 Agent 执行框架**：不内置庞大的自主循环、复杂工作流引擎或重型浏览器沙箱（那是 Deno / AutoGPT / LangChain 的生态位）。
- **不是通用无约束脚本执行平台**：不提供任意未受控脚本的无限制隐蔽运行环境，所有插件均需受清晰 Manifest 契约、资源限制与生命周期监管。

### 1.2 八大核心职责 (Eight Core Responsibilities)
1. **统一管理多独立服务与外部二进制**：统一编排、监控并托管多个相互独立的开源或私有 API 二进制服务。
2. **标准 Manifest 插件化封装**：通过统一的 `manifest.yaml` 规范，将外部工具声明化为可插拔组件。
3. **稳定可配置的本地单入口**：为终端应用（Cursor、Claude Code、Windsurf、Aider 等）暴露永久统一、免频繁改端口的本地 HTTP/SSE 入口。
4. **必要的模型路由与协议兼容**：提供标准 OpenAI Chat Completions、Responses API、Anthropic Messages API 等通用协议的透明转发与必要转换。
5. **完整的插件生命周期管理**：提供插件的安装、启动、就绪检测、空闲回收、故障恢复、配置投影与平滑卸载。
6. **开箱即用的质量与诊断工作台**：内置试飞台（Playground）、兼容性检测（Compatibility Flight）与调用链故障诊断（API Doctor）。
7. **上游失效的高韧性架构解耦**：单一插件或其依赖的上游平台失效时，宿主架构与其他插件保持零波及、零崩溃。
8. **原生支持外部现成服务接入 (External Provider)**：用户已有正在运行的本地网关或远程端点可直接无缝挂载，无需重复安装或移交进程生命周期。

### 1.3 核心设计原则 (Core Design Principles)
- **Local-First & Zero Database Burden**：桌面级极简体验，无 MySQL/PostgreSQL/Redis 强制依赖，核心配置采用无库单文件 JSON 存储，启动即用。
- **Upstream APIs are Transient; the Host is Eternal（上游易逝，胶水永存）**：宿主永不依赖特定反代平台的专有逻辑，核心保持纯粹。
- **Everything is a Plugin（万物皆插件）**：借鉴 Cordis 与 Microkernel 哲学，所有接入的服务均具备统一的注册契约与生命周期守卫。
- **Defense in Depth & Security by Default**：Secret 与常规配置物理隔离，系统凭证库安全托管；外部二进制不享有盲目信任。

### 1.4 三层分工体系 (Three-Tier Division of Labor)

```text
┌─────────────────────────────────────────────────────────────┐
│                 Layer 1: Host Microkernel                   │
│   Tauri v2 UI + System Tray + Axum Async Gateway + Core     │
│   (生命周期调度、四象限策略、端口动态绑定、安全凭证托管、模型路由)   │
└──────────────────────────────┬──────────────────────────────┘
                               │ Manifest 1.0 契约 / 统一控制面
┌──────────────────────────────▼──────────────────────────────┐
│                 Layer 2: Glue Adapter                       │
│   协议编解码 (TiyGate Codec IR) + 流式规整 + 凭证投影 + 探针检测   │
└──────────────────────────────┬──────────────────────────────┘
                               │ Loopback HTTP / IPC / Process
┌──────────────────────────────▼──────────────────────────────┐
│                 Layer 3: Blackbox Plugins                   │
│  [Managed Plugin]          [External Provider]              │
│  wb2api (Go 二进制)          已运行的 CLIProxyAPI (127.0.0.1:28317)│
│  wild-work (Node 二进制)     远程 vLLM / Ollama 局域网端点       │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 开源脚手架采用策略与 Quotio Desktop 深度审计 (Scaffold Adoption Audit)

### 2.1 脚手架采用政策重构：从“禁止 Fork”到“审计准入”
原规划中“绝对不允许 Fork 任何开源项目”的条文过于教条化，在工程实践中会导致重复造轮子（如重写 Tauri 托盘、窗口生命周期、自启动、更新器、平台抽象等高度成熟的系统级机制）。

**ProxyWeave 正式确立脚手架采用新规则**：
> **允许且推荐选择成熟开源项目作为主脚手架与开发基线，但必须经过严格的源码、许可证、架构对齐与迁移成本审计（Scaffold Adoption Audit）。**

### 2.2 主脚手架采用 vs 模块级源码复用的清晰边界
- **A. 主脚手架采用 (Main Scaffold Adoption)**：
  - 允许基于完整的成熟开源仓库建立开发主干基线。
  - **强制审计要求**：必须锁定固定 Release Commit；保留原作者版权与开源许可证声明；识别并保留通用基础设施；彻底剥离特定业务耦合；建立基线编译与自动化测试。
  - **红线**：严禁仅将上游简单改名后宣称为新产品。
- **B. 模块级源码复用 (Module-Level Source Reuse)**：
  - 对非主脚手架的外部参考源（如 TiyGate、Shared-Child、Keyring-rs 等），原则上禁止整体拉入，仅通过 Git Sparse Checkout、单文件提取、Trait 适配或模块级移植引入所需逻辑。
  - **红线**：严禁机械式全盘复制代码；严禁将未理解的复杂依赖带入项目。

---

### 2.3 首要主脚手架候选：Quotio Desktop 全景审计 (Ground Truth)

我们在本地环境通过 `git clone --depth 1` 对候选项目进行了 100% 真实源码取证与分析：

```text
【Quotio Desktop 审计取证元数据】
- 官方仓库: https://github.com/xiaocoss/quotio-desktop
- 许可证: MIT License (完全商用与修改友好)
- 审计 Commit: 21d75d08b38a23f97fbd534a1768a829cb147f2c (main 分支最新)
- 发布版本对应: v0.7.8 (Tag: 2e2b0a6e13a77cf848b7a074d8f44ded50c10606)
- 技术栈: Tauri v2.0 + React 18 + TypeScript + Vite + Tailwind CSS + Rust Workspace
```

#### 2.3.1 Rust Workspace 结构与模块职责映射

```text
quotio-desktop/
├── apps/
│   └── desktop/               # Tauri v2 桌面应用外壳与前端 React UI
│       ├── src/               # React + TypeScript 页面 (Dashboard, Providers, Agents, Fallback)
│       └── src-tauri/         # Tauri 2.0 桌面绑定层 (窗口、托盘、自动启动、单实例锁)
├── crates/
│   ├── quotio-platform/       # [可高度复用] 跨平台系统抽象库 (1192 行，极其成熟)
│   ├── quotio-types/          # 共享类型定义层 (67KB lib.rs)
│   └── quotio-core/           # 核心业务逻辑层 (243KB lib.rs，重度耦合 CPA 业务)
└── vendor/
    ├── kiro-rs/               # 独立 Rust AWS CodeWhisperer 转接服务 (专属业务)
    └── quotio-key-router/     # API Key 路由插件 (专属业务)
```

#### 2.3.2 逐模块真实源码审计表 (Module Audit Table)

| 源码路径 | 当前职责 | ProxyWeave 目标 | 采用处理方式 | 依赖与架构影响 | 审计结论与证据 |
|---|---|---|---|---|---|
| `apps/desktop/src-tauri` | Tauri v2 桌面宿主、托盘菜单、单实例与更新器 | ProxyWeave Desktop 壳工程 | **保留并改造** | 保留 Tauri 2、`single-instance`、`autostart`、`updater` 插件 | **通过 (Verified)**。极其成熟的 Tauri 2 模版，窗口与托盘管理完善。 |
| `crates/quotio-platform` | 跨平台路径解析、命令执行、进程超时检测、系统 Keyring 凭据托管 | ProxyWeave Platform Layer | **优先完整复用** | 保留 `keyring` (v3.6.3)、系统进程管道 | **通过 (Verified)**。1192 行高纯度系统级 Rust 代码，完全跨 platform (Win/Mac/Linux)，无业务耦合。 |
| `crates/quotio-core/src/bridge.rs` | 基于 `tiny_http` / `TcpListener` 的同步阻塞反向代理 | 异步流式 HTTP Gateway | **彻底替换** | 剥离 `tiny_http`，替换为 Axum 0.7 + Hyper 1.0 | **重构 (Defect Fixed)**。当前实现为同步线程池+每次请求强制 `Connection: close`，无法承载高并发与长流式。 |
| `crates/quotio-core/src/proxy_download.rs` | 硬编码下载 `router-for-me/CLIProxyAPI` 最新 Release 资产 | 通用 Plugin Installer | **抽象重构** | 剥离硬编码 URL，改造为基于 Manifest 下载任意 GitHub Release | **通过 (Verified)**。包含流式进度回调与 SHA256 校验，逻辑清晰，适合提取泛化。 |
| `crates/quotio-core/src/agent_config.rs` | Cursor、Claude Code、Windsurf 等 IDE 配置写入 | IDE Handoff 配置分发 | **保留并泛化** | 剥离针对 CPA 的专用端口，改为写入 ProxyWeave 统一入口 | **通过 (Verified)**。54KB 详尽的各类 IDE 配置文件路径与 JSON/YAML 修改逻辑，资产价值极高。 |
| `crates/quotio-core/src/usage_store.rs` | 基于 `rusqlite` 的请求级别用量统计与 KPI 数据库 | 诊断与指标记录器 | **解耦与隔离** | 将 SQLite 从系统核心生命周期剥离，作为可选分析模块 | **通过 (Verified)**。证实其 SQLite 仅用于消费 CPA 的 `/usage-queue` 做看板报表，核心配置并不依赖 SQLite。 |
| `crates/quotio-core/src/kiro_sidecar.rs` | AWS CodeWhisperer 专属 Sidecar 启停 | 外部黑盒插件 Manifest | **删除 (剥离)** | 彻底移除对 `vendor/kiro-rs` 的强制绑定 | **通过 (Verified)**。属于特定厂商业务，违背微内核通用宗旨。 |
| `crates/quotio-core/src/quota.rs` 等 | 多账号轮询、额度抓取、Codex 实例调度 | (非 ProxyWeave 目标) | **删除 (剥离)** | 剥离 85KB 多账号与额度抓取业务逻辑 | **通过 (Verified)**。属于特定平台账号管理业务，ProxyWeave 保持黑盒宿主定位。 |

---

### 2.4 Quotio 技术债务剥离与改造挑战

审计发现 Quotio 存在 5 项关键技术债务，在改造时必须严格按技术规范予以解决：

1. **特定平台业务严重耦合**：
   - *现状*：Quotio 核心代码充斥着 Codex 多实例多开 (`codex_launch.rs`)、Kiro 专属集成 (`kiro_sidecar.rs`)、额度轮询 (`quota.rs`) 与 OAuth 抓取。
   - *对策*：建立 `crates/proxyweave-core`，仅保留进程生命周期与通用调度，彻底删除上述特定业务代码。
2. **`bridge.rs` 属于同步阻塞代理**：
   - *现状*：使用 `tiny_http` 与标准库 `TcpListener`，为每个连接分配阻塞线程，请求头强制添加 `Connection: close`。
   - *对策*：不可直接将其作为 ProxyWeave 的主网关！必须使用基于 Tokio + Axum + Hyper 的异步非阻塞网关替换 `bridge.rs`。
3. **下载器硬编码上游地址**：
   - *现状*：`proxy_download.rs` 中固定 `const RELEASE_URL: &str = ".../CLIProxyAPI/releases/latest"`。
   - *对策*：将下载逻辑泛化为 `PluginInstaller`，接收 Manifest 中的 GitHub 仓库地址与资产正则模式。
4. **SQLite 依赖深度分析**：
   - *现状*：引入了 `rusqlite = { version = "0.31", features = ["bundled"] }`。
   - *对策*：代码审计确认，Quotio 的全局配置保存在 `~/.quotio/config.json`（纯 JSON 序列化），SQLite **仅**在 `usage_store.rs` 中用于保存历史请求日志与图表分析。ProxyWeave 初期主干可安全切断 SQLite 依赖，保持真正的 No-DB 纯净架构；在后续 Phase 4 若需本地高级审计日志，再通过可选模块按需引入。
5. **外部进程 (External Provider) 防误杀保护**：
   - *现状*：Quotio 会根据本地端口扫描直接尝试关闭占用进程。
   - *对策*：在 Supervisor 中严格划定边界，仅对 ProxyWeave 亲手 Spawn 的 Managed 进程执行 Job Object 绑定与生命周期回收；对已有 External Provider 仅做探针连接，**严禁向非托管 PID 发送 Kill 信号**。

### 2.5 主脚手架改造原则与逐步迁移路线 (Refactoring SOP)

为避免“直接删空项目重写”的高风险操作，制定以下 6 步改造顺序：

```text
   Step 1: 保留可运行工程 ──► 验证当前 Quotio Desktop 在本地完整编译构建 (npm run build + cargo build)
             │
   Step 2: 建立基线测试   ──► 为 quotio-platform 与核心配置读写建立基准单元测试
             │
   Step 3: 去除业务耦合   ──► 逐个移除 codex_*、kiro_sidecar、quota、scheduler、vendor 目录
             │
   Step 4: 抽象通用宿主   ──► 将 proxy_download 改造为 Manifest-driven 插件安装引擎，重构 Supervisor
             │
   Step 5: 集成协议层     ──► 引入 Axum + TiyGate 协议 Codec，替换旧 bridge.rs，实现统一流式网关
             │
   Step 6: 建设插件工作台 ──► 前端 UI 从“账号额度看板”转向“ProxyWeave 插件管理与 API 试飞台”
```

### 2.6 替代底座评估与自研回退条件 (Fallback Criteria)
如果在 Phase 0 的基线编译与解耦过程中，发现 Quotio Desktop 的代码重构成本或历史包袱超过预期（满足以下任一触发条件）：
1. 剥离特定平台业务时导致前端与后端交互接口发生不可调和的深层级联断裂，重构工作量预估大于重新搭建干净 Tauri v2 骨架；
2. 其历史依赖与现代 Rust 2024/Tokio 1.x 产生无法解决的编译版本冲突；

**回退方案 (Fallback Plan)**：
- 立即切换为：基于干净的 Tauri v2 官方骨架作为主干，**以“模块级复用 (Module-Level Reuse)”形式单独提取 `quotio-platform` (1192 行优质跨平台系统库) 与 `agent_config.rs` (IDE 配置逻辑)**，跳过其旧业务代码，直接进入 Phase 1。此决策将通过正式 ADR 记录。

---

## 3. 协议与网关核心参考源：TiyGate 深度分析 (TiyGate Protocol Reference)

### 3.1 TiyGate 项目背景、架构与许可证
- **官方仓库**：`https://github.com/tiylabs/tiygate`
- **许可证**：Apache License 2.0 (允许商用、修改与分发，需保留版权声明与 NOTICE)
- **审计 Commit**：`6db40e54663455fb99d675bd8985e4dc50f68a14` (master 分支最新)
- **技术定位**：专为高可用 LLM 服务打造的轻量级 Rust 网关。
- **定位区别**：TiyGate 自身是一个具备多企业级 Provider、复杂多租户数据库与鉴权的通用网关；ProxyWeave 将其作为**协议转换、IR 中间表示与流式处理的权威参考源**，绝非整套搬运。

### 3.2 重点借鉴模块与目录映射

通过对 TiyGate 真实源码的审计，提炼出三大核心模块供 ProxyWeave 重点借鉴与移植：

```text
tiygate/
├── crates/core/src/
│   ├── ir/                    # [核心借鉴] 统一 LLM 请求/响应/流式中间表示 (IR)
│   ├── pipeline/              # [核心借鉴] 请求生命周期处理管道与错误映射
│   └── routing/               # [参考] 虚拟模型到物理模型的路由策略
├── crates/protocols/src/
│   ├── chat_completions.rs    # [核心借鉴] 138KB 极其严谨的 OpenAI Chat Completions 编解码
│   ├── messages.rs            # [核心借鉴] 113KB 完备的 Anthropic Messages 协议编解码
│   ├── responses.rs           # [核心借鉴] 267KB 最前沿的 OpenAI Responses API 编解码
│   ├── gemini.rs              # [参考备选] 100KB Google Gemini 原生协议转换
│   └── error_body.rs          # [核心借鉴] 跨协议标准错误响应体转换
└── crates/server/src/
    └── app.rs                 # [核心借鉴] Axum 0.7 异步 HTTP 路由树与 SSE 背压处理
```

### 3.3 协议编解码 (Codec)、中间表示 (IR) 与流式转换分析
TiyGate 最具工程价值的核心资产是其 **N * M 协议转换解耦架构**：
- **传统的暴力转换痛点**：若在 N 种客户端协议与 M 种上游之间直接互转，需要编写 N * M 个适配器。
- **TiyGate 的 IR 方案**：
  `Client Protocol -> Decode -> Unified IR -> Encode -> Upstream Protocol`  
  客户端输入（如 Claude Code 发送的 Anthropic Messages 请求）首先解码为内部标准 `ChatRequestIR`，再根据插件能力编码为上游需要的协议（如 OpenAI Chat 格式）。
- **流式 SSE 状态机**：
  TiyGate 在 `chat_completions.rs` 与 `messages.rs` 中实现了状态机级别的 SSE Chunk 流解析，能够精确处理跨 Chunk 的 Token 拼接、Tool Calls 参数 Fragment 聚合、Finish Reason 映射。**ProxyWeave 将优先参考此实现，坚决不从零手写脆弱的正则或字符串替换！**

### 3.4 边界隔离红线：坚决剥离商业化与重型数据库依赖
在复用 TiyGate 代码时，设立明确技术红线：
1. **坚决剥离 `crates/store`**：TiyGate 使用 SQLx 支持 SQLite/PostgreSQL，包含 10 余个数据表迁移脚本（用于存储网关调用统计、审计日志、多用户模型）。ProxyWeave 是纯本地轻量应用，**坚决不引入 SQLx 及其数据库依赖**。
2. **坚决剥离 `crates/admin` 与企业认证**：剥离其管理员 REST API、复杂的企业级 OAuth 管理器 (`oauth_manager.rs`) 与配额计数器 (`quota.rs`)。
3. **保留 Apache-2.0 版权声明**：所有移植或参考的代码文件顶部，完整保留原作者 TIY Labs 的版权声明与 Apache-2.0 许可证注释，并在 ProxyWeave 的 `THIRD_PARTY_NOTICES.md` 中明确记录。

### 3.5 联合架构定位：Quotio + TiyGate + ProxyWeave

```text
┌───────────────────────────────────────────────────────────────┐
│                     Quotio Desktop (主脚手架)                   │
│   提供成熟的 Tauri v2 桌面壳、系统托盘、系统 Keyring 凭据托管、跨平台抽象   │
└──────────────────────────────┬────────────────────────────────┘
                               │ 依托其桌面基础与基础设施
┌──────────────────────────────▼────────────────────────────────┐
│                   ProxyWeave Core (宿主微内核)                 │
│   定义 Manifest 契约、四象限进程生命周期管理、端口自动分配、API Doctor 诊断 │
└──────────────────────────────┬────────────────────────────────┘
                               │ 驱动协议转换引擎
┌──────────────────────────────▼────────────────────────────────┐
│                      TiyGate (协议与网关参考源)                  │
│   提供成熟的 Axum 异步流水线、IR 中间表示、Chat/Messages/Responses 编解码  │
└───────────────────────────────────────────────────────────────┘
```

---

## 4. 多引擎与微内核参考源：Tunnel Agent 与 DeepSeek Harness

### 4.1 Tunnel Agent：多引擎管理、下载调度与开发工具配置接入
- **官方仓库**：`https://github.com/beyondhumane/tunnel-agent` (或 `Villoh/tunnel-agent`)
- **许可证**：MIT License
- **审计 Commit**：`9bfb7779b80de5e205a388e2e02d5840a3ed8b47`
- **技术栈**：C# .NET 9 + Avalonia UI (跨平台桌面框架)
- **定位**：**产品行为、多引擎管理与交互体验参考源，绝对不是主 Rust 脚手架！**
- **重点借鉴资产**：
  1. **多 API 引擎统一编排**：其在 `TunnelAgent.Core/Engine/` 中成功统一管理了 CLIProxyAPI、9Router、Perplexity 等多个异构引擎的安装、配置与生命周期。
  2. **下载与版本锁定服务**：详尽的 GitHub Release 资产探测、解压校验与版本切换机制 (`CliProxyDownloadServiceTests.cs`)。
  3. **IDE 自动接入体验 (Agent Configuration)**：自动探测本地 Cursor、Claude Code、Factory Droid 配置并完成 API 引导替换。
  4. **Win32 Job Object 实现范本**：在 `KillOnCloseJobTests.cs` 中提供了 Windows 进程防泄漏的完善测试用例。
- **技术隔离红线**：
  - **严禁为了参考其功能将 .NET 运行时、C# 依赖或 Avalonia 引入 ProxyWeave 的 Rust/Tauri 生态！** 仅借鉴其状态机设计思想、UI 交互流与测试用例结构。

---

### 4.2 DeepSeek Harness (DSH)：Cordis 微内核与插件化哲学
- **官方仓库**：`https://github.com/deepseek-ai/deepseek-harness`
- **许可证**：MIT License
- **审计 Commit**：`d743267388641bc76f17c45ce8b4c231aed1d32c`
- **重点借鉴资产**：
  1. **Cordis 微内核与“万物皆插件”哲学**：核心微内核负责加载与协调，所有业务能力均以插件形态注册。
  2. **可逆副作用生命周期 (Reversible Effects)**：插件卸载或退出时，其注册的路由、占用的端口、生成的临时配置文件通过 RAII/清理钩子自动物理释放，不留系统垃圾。
  3. **Service Bus 动态能力声明**：插件显式声明自身支持的 Capability（如 `llm-streaming`、`tool-calling`、`embeddings`），宿主动态挂载。
  4. **UI 插槽与极简工业风设计**：Zinc-950 底色、等宽字体、紧凑指标看板的纯粹工程师交互美学。
- **技术隔离红线**：
  - **严禁引入 DSH 庞大的 Node.js 运行时或重型 Agent Runtime 体系**。ProxyWeave 在 Rust 语言层通过 Trait、Channel 与状态机重新实现轻量化的微内核插件契约。

---

### 4.3 本地与社区其他参考资产全景划分 (5 大分类体系)

为彻底消灭认知混淆，将所有涉及的 14 个参考项目划入清晰的五大生态位：

| 分类定义 | 涵盖项目 | 核心职责与边界说明 |
|---|---|---|
| **1. 主脚手架 (Main Scaffold)** | **Quotio Desktop** (`xiaocoss/quotio-desktop`) | ProxyWeave 开发的首选起点底座。提供 Tauri v2 桌面壳、系统托盘、系统 Keyring 凭据管理、跨平台系统执行抽象。经过严格剥离业务耦合后作为主干开发。 |
| **2. 模块复用源 (Module Reuse Source)** | **TiyGate** (`tiylabs/tiygate`)<br>**shared-child** (Crates.io)<br>**keyring-rs** (Crates.io)<br>**portpicker-rs** (Crates.io)<br>**REF 本地参考库** (`workbuddy-switch-gateway-latest`) | 针对特定难题的独立代码库。按需提取特定模块、函数与算法：<br>- TiyGate: 协议 IR 编解码与流式转换；<br>- shared-child: Win32 Job Object 与子进程死锁保护；<br>- REF: 时间戳毫秒秒换算、原子配置文件写入模板。 |
| **3. 架构设计参考 (Architectural Reference)** | **DeepSeek Harness** (`deepseek-ai/deepseek-harness`)<br>**LiteLLM Router** (`BerriAI/litellm`) | 宏观设计思想与算法参考：<br>- DSH: Cordis 微内核、可逆副作用、动态能力声明；<br>- LiteLLM: 虚拟模型别名映射、Fallback 优先级故障转移策略。严禁引入各自的重型运行时。 |
| **4. 交互体验参考 (Interactive / UX Reference)** | **Tunnel Agent** (`beyondhumane/tunnel-agent`)<br>**DSH Client UI** (DSH 前端) | 产品行为、向导与视觉设计参考：<br>- Tunnel Agent: 多引擎切换面板、IDE 接入引导卡片；<br>- DSH: 极简暗色工业风看板、流式 TTFT 指标显式测量组件。 |
| **5. 外部黑盒验证插件 (External Validation)** | **WorkBuddy 2API** (`D:\workbuddy2api-panel`)<br>**CLIProxyAPI** (`D:\CLIProxyAPI_7.2.115_...`)<br>**wild-work** (`rockswang/wild-work`)<br>**本地真实账号库** (`C:\Users\Cary\...\accounts.json`) | Phase 0/1 的真实集成测试验证目标：<br>- wb2api: 首发托管插件 (Go 单二进制、/healthz、嵌入式 Web 面板)；<br>- CPA: 首发外部接入提供商 (External Provider，28317 端口挂载)；<br>- accounts.json: 本地 27 账号真实联调资产（严禁提交 Git）。 |

---

## 5. 强制 Reference-First 开发制度与参考源档案 (Reference-First Governance)

### 5.1 核心准则：成熟实现优先，源码研究先于编码
> **任何非平凡的系统级机制（进程管理、端口分配、流式解码、协议转换、安全存储、文件原子写入），AI 与开发者在写第一行代码前，必须先检索、阅读并理解成熟的开源或官方实现。严禁闭门造车，严禁盲目手写未经验证的复杂算法！**

### 5.2 强制五步检索体系与 12 步开发工作流

```text
              【Reference-First 12 步强制开发工作流】
需求确认 ──► 模块拆解 ──► 官方标准检索 ──► 本地资产比对 ──► 外部开源源码研读
                                                              │
验收归档 ◄── 压力测试 ◄── 边界单测 ◄── 局部提取与适配 ◄── 许可证合规审查 ◄┘
```

1. **Step 1 官方标准检索**：优先查阅 Rust std、Tauri 官方插件、Tokio、Axum、Win32 API、POSIX 标准文档。
2. **Step 2 本地资产比对**：先检索本地真实源码库（APP、GW、REF、CPA），优先使用已在本地跑通的代码片段。
3. **Step 3 外部源码研读**：深入知名成熟开源仓库，精确定位到 `.rs` 源码文件、函数调用链与异常恢复分支，**严禁仅凭 README 作出技术断言**。
4. **Step 4 许可证合规审查**：确认许可证兼容性（MIT、Apache-2.0、BSD 优先），彻底隔离 GPL 传染风险。
5. **Step 5 局部提取与防御适配**：只提取核心算法，彻底剔除原项目的专有业务逻辑，保留其跨平台兼容与异常保护边界。
6. **Step 6 严密测试与验收**：编写高覆盖率的单元测试与故障注入测试，确保集成后坚如磐石。

### 5.3 自研例外审批与 ADR 归档流程
若经过严格五步检索后，确无可用开源方案而必须从零自研核心系统机制，**必须在 `docs/architecture/ADR/` 建立正式审批档案**，详细陈述：
1. 检索过的 3 个以上相关开源项目名称与链接；
2. 逐一说明无法直接复用或组合复用的具体技术限制；
3. 自研算法的理论推导、状态机边界设计与安全证明；
4. 针对自研机制的异常防御与自动化压力测试计划。

### 5.4 参考源登记档案规范 (Reference Source Registry Schema)
每个进入 ProxyWeave 的复用或核心参考模块，必须在 `docs/references/REFERENCE_REGISTRY.md` 建立永久档案，遵循统一字段格式：
- `Module ID`：模块唯一标识符（如 `MOD-SUPERVISOR`）。
- `Module Name`：模块名称与职责简述。
- `Reference Project`：参考开源项目名称。
- `Official Repository`：官方代码仓库 URL。
- `Source Commit / Version`：参考的具体 Commit Hash 或固定版本 Tag。
- `Exact Source Paths`：参考的真实源码相对路径与行号区间。
- `License`：开源许可证类型与版权所属。
- `Dependencies`：引入该模块所需的最小外部依赖项。
- `Reusable Components`：具体复用的 Trait、函数、算法或数据结构。
- `Required Modifications`：针对 ProxyWeave 架构所必须进行的改造与业务解耦说明。
- `Security & Safety Analysis`：潜在内存安全、资源泄漏或安全沙箱风险评估。
- `Adoption Decision`：最终决策（主脚手架 / 局部移植 / 直接依赖 / 仅供设计参考）。
- `Test Strategy`：对应的基准单元测试与集成测试方案。
- `Verification Status`：验证状态（Verified 已核验真实源码 / Pending 待核验）。

---

### 5.5 核心模块级参考源对照矩阵 (15 大核心模块)

所有模块均经过真实源码检查，严格标注验证状态：

| 模块编号与名称 | 选定/候选参考源 | 仓库与具体源码文件 | 许可证 | 验证状态 | 核心技术复用点与借鉴分析 |
|---|---|---|---|---|---|
| **M01. Process Supervisor**<br>(进程组/生命周期监控) | **shared-child**<br>+ **Quotio-Platform** | `crates.io/crates/shared-child` (`src/sys/windows.rs`)<br>`crates/quotio-platform/src/lib.rs` | MIT / Apache-2.0 | **Verified (已核验)** | 1. 解决子进程并发等待与杀死竞态；<br>2. Win32 Job Object (`KILL_ON_JOB_CLOSE`)；<br>3. Linux `prctl(PR_SET_PDEATHSIG, SIGKILL)`；<br>4. 结合 Singleflight 启动互斥锁防高并发击穿。 |
| **M02. Port Allocator**<br>(动态端口与冲突恢复) | **portpicker-rs**<br>+ 本地 **REF** 改进 | `crates.io/crates/portpicker`<br>REF 端口试探函数 | MIT | **Verified (已核验)** | 1. 消除朴素 `bind(0)->drop` 的 TOCTOU 竞态漏洞；<br>2. 采用带随机退避的探测重试环；<br>3. 子进程若发生端口碰撞，捕获启动报错并自动换端口重试。 |
| **M03. Async HTTP Gateway**<br>(异步流式网关与反向代理) | **TiyGate Server**<br>+ **Axum 0.7** + **Hyper 1.0** | `tiylabs/tiygate` (`crates/server/src/app.rs`)<br>`tokio-rs/axum` | Apache-2.0 / MIT | **Verified (已核验)** | 1. 彻底替换 Quotio 阻塞的 `bridge.rs`；<br>2. 基于 Tokio 异步非阻塞转发与 HTTP Keep-Alive 连接池；<br>3. 下游客户端断开时，通过 CancellationToken 立即熔断取消上游请求。 |
| **M04. SSE Decoder & Stream**<br>(流式事件解析与边界处理) | **eventsource-stream**<br>+ **TiyGate Protocols** | `crates.io/crates/eventsource-stream`<br>`tiylabs/tiygate` (`crates/protocols/src/chat_completions.rs`) | Apache-2.0 / MIT | **Verified (已核验)** | 1. 结构化解析 SSE 事件帧，杜绝裸字符串 split；<br>2. 严格区分正常传输完毕（补发 `[DONE]`）与异常截断（抛错断开，**绝对禁止补发 `[DONE]`**）。 |
| **M05. Protocol Adapter (IR)**<br>(Chat/Messages/Responses) | **TiyGate Protocols** | `tiylabs/tiygate` (`crates/protocols/`, `crates/core/src/ir/`) | Apache-2.0 | **Verified (已核验)** | 1. 借鉴成熟的 N * M 中间表示 (IR) 编解码算法；<br>2. 支持 OpenAI Chat、Anthropic Messages 与 Responses API 之间的无损流式转换；<br>3. 多 Chunk 的 Tool Call 参数 Fragment 拼装。 |
| **M06. Model Router**<br>(模型别名与 Fallback) | **LiteLLM Router**<br>+ **TiyGate Routing** | `BerriAI/litellm` (`litellm/router.py`)<br>`tiylabs/tiygate` (`crates/core/src/routing/`) | MIT / Apache-2.0 | **Verified (已核验)** | 1. 虚拟模型映射（如 `gpt-4o` -> `wb/claude-3-7`）；<br>2. Fallback 故障转移路由，按插件健康度自动分流；<br>3. 剥离 Python 依赖，在 Rust 内存中以 HashMap/DashMap 高效实现。 |
| **M07. Credential Broker**<br>(安全密钥库与凭据投影) | **Quotio-Platform**<br>+ 本地 **REF** 原子写入 | `crates/quotio-platform/src/lib.rs` (`keyring`)<br>REF `atomic_write_config` | MIT | **Verified (已核验)** | 1. 系统级 Keyring 存储核心 Secret；<br>2. 借鉴 REF 的 `0o600` 文件权限保护与 `.tmp -> fs::rename` 原子替换；<br>3. 严格限制凭据在插件隔离沙盒目录的只读投影。 |
| **M08. Plugin Installer**<br>(Release 资产探测与解压) | **Quotio Download**<br>+ **Tunnel Agent Download** | `crates/quotio-core/src/proxy_download.rs`<br>`TunnelAgent.Infrastructure/Engine/` | MIT | **Verified (已核验)** | 1. 泛化 Quotio 的下载器，支持 Manifest 指定 GitHub 仓库；<br>2. 流式下载进度通知与 SHA256 校验和校验；<br>3. 解压至临时目录后原子重命名替换，校验失败自动回滚。 |
| **M09. Config Store**<br>(无数据库配置持久化) | **serde_json**<br>+ **Quotio AppConfig** | `crates/quotio-platform/src/lib.rs`<br>本地 APP `crates/wb-switch-core/src/config.rs` | MIT | **Verified (已核验)** | 1. 单文件 `~/.proxyweave/config.json` 持久化，显式包含 `schema_version`；<br>2. 引入 `parking_lot::RwLock` 保证高并发读写安全；<br>3. 写入前轮转生成 `.bak` 双副本，解析故障秒级回滚。 |
| **M10. Manifest Parser**<br>(声明式规范解析与验证) | **serde_yaml** | `crates.io/crates/serde_yaml` | MIT / Apache-2.0 | **Verified (已核验)** | 1. 解析 `manifest.yaml`，校验 14 个核心维度字段；<br>2. 提供语义级别校验（如端口冲突、可执行文件缺失、权限越界）。 |
| **M11. Plugin Lifecycle**<br>(四象限策略与状态机) | **DeepSeek Harness**<br>+ **Cordis** (Reversible) | `deepseek-ai/deepseek-harness` (`packages/core/`)<br>`cordisjs/cordis` | MIT | **Verified (已核验)** | 1. 借鉴 Cordis 的可逆副作用生命周期；<br>2. 实现四象限策略 (`persistent` / `idle_timeout` / `on_demand` / `external`)；<br>3. 插件退出时自动销毁绑定的路由与临时端口。 |
| **M12. IDE Handoff Engine**<br>(开发工具配置自动接入) | **Quotio Agent Config**<br>+ **Tunnel Agent Config** | `crates/quotio-core/src/agent_config.rs`<br>`TunnelAgent.Infrastructure/Services/` | MIT | **Verified (已核验)** | 1. 完整复用 Quotio 54KB 的 Cursor、Claude Code、Windsurf 等 IDE 配置文件读写逻辑；<br>2. 将 API 端点指向 ProxyWeave 本地单入口并注入 Local API Key。 |
| **M13. Desktop Shell**<br>(托盘/窗口/自启动/单实例) | **Quotio Desktop Shell**<br>(Tauri v2) | `apps/desktop/src-tauri`<br>`crates.io/crates/tauri-plugin-single-instance` | MIT / Apache-2.0 | **Verified (已核验)** | 1. 直接复用成熟的 Tauri 2.0 桌面集成与系统托盘菜单；<br>2. 使用 `single-instance` 插件防止多次双击造成端口冲突；<br>3. 系统休眠唤醒事件监听与插件状态重连。 |
| **M14. API Doctor**<br>(端到端诊断与脱敏追踪) | **Tower-HTTP Trace**<br>+ 自研诊断流水线 | `tower-rs/tower-http` (`TraceLayer`)<br>TiyGate Pipeline | MIT / Apache-2.0 | **Verified (已核验)** | 1. 捕获请求全链路（入口 -> 路由 -> 插件 -> 上游 -> 响应）；<br>2. 对 Authorization、Token 自动进行正则表达式级严格脱敏；<br>3. 输出一键复制的排错诊断报告。 |
| **M15. Compatibility Flight**<br>(协议与流式质检工作台) | **DeepSeek Harness UI**<br>+ **HTTPie/Postman** 思想 | `deepseek-ai/deepseek-harness` (`packages/client/`) | MIT | **Verified (已核验)** | 1. 借鉴 DSH 极简暗色工业风看板；<br>2. 自动化执行 6 项标准兼容性测试（非流式、流式、Tool Call、取消、错误映射、TTFT 测量）。 |

### 5.6 正式代码复用规范与法律/安全红线
1. **许可证兼容性红线**：所有引入的代码必须为宽松开源许可证（MIT, Apache-2.0, BSD, ISC）。绝对禁止引入 GPL/AGPL 传染性代码直接编入 ProxyWeave 核心发布二进制。
2. **版权与归属声明**：所有提取的源码文件必须保留原作者的版权声明（Copyright Header），并在发行版 `THIRD_PARTY_NOTICES.md` 中完整记录。
3. **安全凭据物理隔离红线**：严禁将任何参考项目中的账号密码、Token、Cookie、生产密钥以任何形式复制或提交到代码仓库。
4. **防御机制完整性红线**：提取成熟开源代码时，**严禁为了“简化”而删除其原有的跨平台条件编译、错误捕获分支与空指针/边界防御检查**。

---

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
│  │ Credential Broker (Keyring-rs + 0o600 File Projection)          │  │
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

### 6.2 建议目标 Rust Workspace 规划 (渐进式演进策略)

为了避免初期过度拆分导致的构建与维护复杂性，采取**三阶段渐进式 Workspace 演进**：

```text
proxyweave/
├── apps/
│   └── desktop/                  # Tauri v2 桌面外壳与 React 前端 UI
├── crates/
│   ├── proxyweave-core/          # [Phase 0] 核心宿主、配置、生命周期与 Supervisor
│   ├── proxyweave-platform/      # [Phase 0 复用 Quotio] 跨平台系统抽象与 Keyring
│   ├── proxyweave-gateway/       # [Phase 1 渐进演进] Axum 异步 HTTP 网关与 SSE 管道
│   ├── proxyweave-protocol/      # [Phase 1 借鉴 TiyGate] 协议编解码与 IR 转换
│   ├── proxyweave-plugin/        # [Phase 2] Manifest Schema 解析与验证引擎
│   └── proxyweave-registry/      # [Phase 3] 插件市场索引、GitHub Release 下载器
├── plugins/
│   └── examples/                 # 内置参考插件 Manifest (wb2api, CPA)
├── docs/
│   ├── architecture/             # 架构与 ADR 归档
│   └── references/               # 参考源档案与审计记录
└── Cargo.toml                    # 顶层虚拟 Workspace
```

### 6.3 统一对外入口与端口动态分配 (Port Allocator)
- **统一入口可配置**：默认监听 `127.0.0.1:8080`，支持在设置中自定义。启动时执行绑定检测，若被占用，弹窗友好提示用户切换端口，杜绝崩溃。
- **子插件端口防竞态分配**：
  - 废除朴素的 `TcpListener::bind(0) -> drop`（存在 TOCTOU 竞态）。
  - 借鉴 `portpicker-rs`，采用带随机退避的动态端口分配算法；
  - Managed 插件启动后，挂起就绪探针验证；若捕获到子进程端口冲突报错，立即触发更换端口并自动重试（最多 3 次）。

### 6.4 异步流式网关与协议流水线 (Async Gateway & Pipeline)
- 基于 Axum 0.7 + Hyper 1.0 搭建全异步网关，淘汰 Quotio 的同步阻塞 `bridge.rs`。
- **背压与取消传播**：当客户端（如 IDE）主动取消生成（关闭 HTTP 连接）时，网关通过 `tokio_util::sync::CancellationToken` 立即断开与上游插件的连接，停止无谓 Token 消耗。

### 6.5 插件双模运行架构：Managed vs External Provider
- **模式 A：托管插件 (Managed Plugin)**：
  - 二进制由 ProxyWeave 负责下载、验证、释放、启动与监控；
  - 分配动态本地回环端口（如 `127.0.0.1:AUTO_PORT`）；
  - 强制绑定系统进程组（Job Object / prctl），宿主退出时保证 100% 连带回收。
- **模式 B：外部提供商 (External Provider)**：
  - 用户接入已在本地或局域网运行的服务（如已有 CLIProxyAPI 监听 28317，或局域网 vLLM）；
  - ProxyWeave 仅建立反向代理路由与探针保活；
  - **核心铁律：绝对禁止向外部提供商发送 Kill 信号或强行接管其生命周期！**

### 6.6 插件生命周期状态机与四象限策略 (Lifecycle State Machine)

废除机械式的“强制只保留 1~2 个插件并盲目 LRU 杀进程”设计，推行**四象限生命周期策略**：

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

**生命周期安全保障机制**：
- **活跃请求原子计数器 (`active_requests: AtomicUsize`)**：当且仅当计数器归零且空闲超过 TTL 时才允许触发回收。**任何有正在传输的流式请求的插件绝对禁止被终止！**
- **Singleflight 冷启动互斥锁**：当多个并发请求同时命中处于休眠状态的插件时，通过 `tokio::sync::OnceCell` 或 `Singleflight` 互斥锁，仅触发一次子进程冷启动，其他请求安全挂起等待就绪探针放行，杜绝进程被重复拉起导致端口冲突。
- **就绪探针 (Readiness Probe)**：支持 HTTP GET（首选）与 TCP Connect 探针，超时后进入回退，不向未就绪的子进程发送业务流量。

### 6.7 跨平台系统级进程管控机制
- **Windows**：Win32 `CreateJobObjectW`，配置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，将每个 Managed 进程句柄绑定到该 Job，主进程无论是正常退出还是在任务管理器中被强制杀死，Windows 内核自动级联清理子进程。
- **Linux**：子进程 `CommandExt::pre_exec` 中注入系统调用 `libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL)`，当父进程死亡时内核自动发出 SIGKILL。
- **macOS**：创建父子继承的匿名管道（Inherited Pipe Watchdog），子进程或包装进程在后台线程阻塞读取该管道 EOF，一旦父进程崩溃退出，管道读破裂触发子进程自尽。

### 6.8 SSE 流式协议与 Tool Call 碎片规整
- **真伪终止符严谨识别**：
  - *正常流结束*：上游流传输完毕但遗漏尾部结束标记，安全补发 `data: [DONE]\n\n`；
  - *异常截断/抛错*：上游网络中断、超额报错或 5xx 崩溃时，向客户端推送结构化错误事件帧并关闭连接，**绝对禁止补发 `data: [DONE]`**（防止 IDE 误判生成完毕而执行残损代码）。
- **Tool Call Delta 碎片拼装**：在流式过程中跟踪 `index` 与 `tool_calls` 增量，确保多工具调用在协议转换过程中参数不发生乱序或丢失。

### 6.9 客观可度量的性能 SLA 目标与压测规范
移除未经实测的虚假宣传，确立工程级 SLA 指标：
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

---

## 7. 插件规范（Manifest Schema 1.0）与生态发现 (Plugin Specification & Ecosystem)

### 7.1 Manifest Schema 1.0 规范定义 (14 个核心维度)

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
  type: "binary" # binary | node | python | docker | external
  entrypoint:
    windows: "bin/wb2api.exe"
    darwin: "bin/wb2api"
    linux: "bin/wb2api"
  args: ["-config", "{CONFIG_PATH}", "-port", "{PORT}"]
  env:
    GIN_MODE: "release"
  working_dir: "{DATA_DIR}"

distribution:
  repository: "https://github.com/linguo2625469/workbuddy2api-panel"
  release_asset_pattern:
    windows_x86_64: ".*windows.*amd64.*\\.zip"
    darwin_aarch64: ".*darwin.*arm64.*\\.tar\\.gz"
    linux_x86_64: ".*linux.*amd64.*\\.tar\\.gz"

lifecycle:
  policy: "idle_timeout" # persistent | idle_timeout | on_demand | external
  idle_timeout_seconds: 300
  startup_timeout_seconds: 15
  stop_signal: "SIGTERM"
  graceful_timeout_seconds: 5

network:
  listen_host: "127.0.0.1"
  port_allocation: "dynamic" # dynamic | fixed | external
  health_check:
    type: "http" # http | tcp
    path: "/healthz"
    interval_seconds: 30
    timeout_seconds: 2
    expected_status: 200

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
    disallow_arbitrary_fs: true
```

### 7.2 外部模式定义样例 (External Provider Manifest)

对于用户机器上已运行的已有网关（如 CLIProxyAPI）：

```yaml
schema_version: "1.0"
metadata:
  id: "external-cpa"
  name: "Local CLIProxyAPI Instance"
  version: "7.2.115"
  description: "挂载已在本地运行的 CLIProxyAPI 实例"
  license: "Proprietary"

runtime:
  type: "external"

lifecycle:
  policy: "external" # 宿主不负责启停，永不发送 Kill 信号

network:
  listen_host: "127.0.0.1"
  port_allocation: "fixed"
  fixed_port: 28317
  health_check:
    type: "http"
    path: "/health"
    interval_seconds: 15
    timeout_seconds: 2

capabilities:
  protocols: ["openai_chat", "streaming", "tool_calling", "gemini"]
```

### 7.3 三级插件可信等级机制
为杜绝恶意代码利用插件机制进行系统提权，建立三级信任机制：
1. **Tier 1: Official / Verified (官方核准)**：收录于官方索引仓库，SHA256 签名校验通过，默认可一键安装；
2. **Tier 2: Community / Unverified (社区来源)**：用户通过 GitHub URL 自行添加，界面强制弹出黄色醒目风险提示，需用户明确勾选“已知晓外部二进制风险”方可安装；
3. **Tier 3: Local / Custom (本地开发者自建)**：本地开发 Manifest，仅限单机调试。

### 7.4 插件市场生态：GitHub Registry 与安全边界
- 建立官方插件轻量索引仓库 `proxyweave-registry`。
- 支持 GitHub Topic 检索（`#proxyweave-plugin`），但明确声明：**Topic 发现结果仅作为未审核的候选条目展示，安装时强制执行三级安全审查**。
- 对未经审核的未知仓库，启发式分析仅生成 Manifest 预览草案供开发者核验，**严禁静默执行外部下载内容**。

---

## 8. 交互体验与四大产品差异化功能 (UX & Key Differentiators)

### 8.1 极简暗色工业风桌面界面布局 (Wireframe)

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⊞ ProxyWeave v0.1.0        ● Core: 127.0.0.1:8080 (Active) [API Key: ••••]│
├───────────────┬─────────────────────────────────────────────────────────────┤
│ 🔌 PLUGINS    │ [ + Add Plugin ]  [ 🔍 Search Plugins... ]  [ 🚀 Flight Deck ]│
│               ├─────────────────────────────────────────────────────────────┤
│ ● wb2api      │ WorkBuddy 2API Engine                           v1.13.0     │
│   Managed     │ Status: RUNNING (PID: 14208) │ 127.0.0.1:52114 │ RAM: 18.2 MB │
│   Active (1)  │ Active Requests: 1 │ Idle TTL: 240s │ Models: 12 exported   │
│               │ [ Open Web Panel ]  [ View Logs ]  [ Config ]  [ ⏹ Stop ]    │
│ ○ wild-work   ├─────────────────────────────────────────────────────────────┤
│   Managed     │ wild-work 12-Channel Aggregator                 v2.4.1      │
│   Sleeping    │ Status: SLEEPING (Cold Start ~200ms) │ RAM: 0.0 MB           │
│               │ [ ⚡ Wake Now ]  [ Configure Accounts ]  [ 🗑 Uninstall ]     │
│ ◈ ext-cpa     ├─────────────────────────────────────────────────────────────┤
│   External    │ CLIProxyAPI Local Service                       v7.2.115    │
│   Fixed:28317 │ Status: ATTACHED (Unmanaged) │ Ping: 0.4ms │ Healthy        │
├───────────────┼─────────────────────────────────────────────────────────────┤
│ 🛠 TOOLKIT    │ 🩺 API Doctor: All 3 upstream routes healthy               │
│ Profile: Dev  │ 📈 Compatibility: 100% Pass (Streaming + Tool Call Verified)│
└───────────────┴─────────────────────────────────────────────────────────────┘
```

### 8.2 API Doctor (端到端脱敏调用链诊断与追踪)
当客户端（如 Cursor）报 500、401 或请求挂起时，用户无需在不同终端翻看子进程日志。API Doctor 提供一键诊断：
- 结构化捕获：`Client -> ProxyWeave (8080) -> Router -> Plugin Adapter -> Upstream`；
- 智能故障归因：明确区分是“本地端口被占”、“Token 过期 (401)”、“上游限流 (429)”还是“子进程崩溃 (502)”；
- **全链路脱敏**：对 Authorization Token、Cookie 自动执行正则表达式级别掩码，生成安全的错误报告，一键复制用于求助。

### 8.3 Compatibility Flight (多协议/流式/Tool Call 综合质检工作台)
不能只凭在 Web 页面聊天成功就断定 API 可用！Compatibility Flight 自动化执行 6 项严格检测：
1. `Non-stream Chat`：标准单轮会话，校验完整 JSON 结构；
2. `Streaming SSE`：首字延迟 (TTFT) 测量，连续 Chunk 校验，尾部合法性校验；
3. `Tool Calling`：模拟函数调用（如 getWeather），校验参数 Delta 的拼装完整性；
4. `Cancellation`：客户端流式中途断开，校验上游请求是否在 1 秒内被成功取消；
5. `Error Mapping`：上游异常中断时，确认**绝未**错误注入 `[DONE]`；
6. `Multi-turn Context`：多轮会话上下文拼接校验。

### 8.4 Configuration Profiles (IDE/客户端快速预设与一键切换)
允许用户保存不同的工作场景预设（如“Coding: 优先路由至 Claude 3.7”、“Writing: 路由至 DeepSeek-V3”）。点击一键切换，并自动调用 IDE Handoff 机制同步更新 Cursor / Claude Code 配置。

### 8.5 Safe Upgrade (版本固定、校验和比对与秒级故障回滚)
- 插件升级前自动在 `~/.proxyweave/plugins/.backup/` 保留上一版本二进制与配置；
- 下载新版本强制校验 GitHub 官方 Release 资产的 SHA256 校验和；
- 启动新版本并挂起执行 3 次就绪健康探针；若探针失败，自动无感回滚至上一版本，保障开发不中断。

---

## 9. 安全模型与数据持久化治理 (Security & Persistence)

### 9.1 系统威胁建模 (Threat Model)
- **同机非授权探测威胁**：同机运行的恶意脚本试图通过 `127.0.0.1:8080` 未授权调用用户的昂贵模型额度；
- **凭据泄露威胁**：明文将 API Key、Token 存储于磁盘文件，被扫描窃取；
- **恶意外部二进制提权威胁**：第三方插件二进制利用宿主提权执行危险系统指令。

### 9.2 凭据安全体系 (Credential Broker)
- **核心 Secret 与配置物理分离**：普通配置（端口、路由映射）保存在 JSON；高敏感凭据（API Token）通过 `keyring-rs` 托管至操作系统底层安全凭据库（Windows Credential Manager / macOS Keychain / Linux Secret Service）。
- **最小权限配置投影**：需要向子进程提供凭据时，在插件专属隔离目录临时生成单向只读配置，文件权限严格限定为 `0o600`（仅当前用户可读写），子进程退出或宿主关闭时自动粉碎清理。
- **本地 API Key 鉴权**：ProxyWeave 统一对外入口默认生成高强度随机密钥（`pw-sk-...`），客户端访问必须携带 Bearer Token，有效拦截同机未授权探测。
- **管理 API 与推理 API 物理隔离**：内部插件管理与控制接口（`/_proxyweave/admin/*`）受独立治理，严禁通过常规推理路径越权访问。

### 9.3 外部进程非沙箱边界与风险告知
- **严肃声明**：在操作系统层面，外部启动的可执行文件属于独立进程，拥有当前登录用户的 OS 权限。**外部进程不等于 Wasm/Docker 安全沙箱！**
- 宿主严禁向用户承诺未经验证的“进程绝对安全隔离”；对未知来源插件，必须履行显式的风险弹窗提示义务。

### 9.4 配置持久化治理 (No-DB 架构加固)
为确保无需数据库依然保证数据零损坏：
- 顶层配置严格声明 `schema_version: "1.0"`，包含向前兼容的自动迁移逻辑；
- 进程内采用 `parking_lot::RwLock` 保护配置状态树；
- 写入磁盘时一律采用 **“内存序列化 -> 写入 `.tmp` 临时文件 -> `fsync` 刷盘 -> 原子 `fs::rename` 替换”** 标准流程；
- 自动维护最新的 `config.json.bak` 双副本，若主文件遭遇断电损坏，秒级自动加载备份恢复。

---

## 10. 架构决策记录 (ADR - Architectural Decision Records)

### ADR-01: 为什么选择成熟脚手架 (Quotio) 改造而非从零手写桌面外壳？
- **背景**：原方案试图从零编写 Tauri 2.0 桌面外壳、托盘、窗口生命周期、自启动与系统凭证库。
- **决策**：采用成熟的 `xiaocoss/quotio-desktop`（MIT License，commit `21d75d08`）作为主脚手架基线。
- **原因**：Quotio 已经完美解决了跨平台桌面托盘、单实例锁、系统 Keyring (1192 行平台代码)、IDE 配置读写等极其繁琐的工程细节。将其中的特定平台业务（Codex/Kiro）剥离并重构为通用插件宿主，比从零研发节省至少 3 周时间且稳定性大幅提升。

### ADR-02: 为什么借鉴 TiyGate 协议与路由设计而非全盘照搬商业网关？
- **背景**：需要抹平 OpenAI Chat、Responses 与 Anthropic Messages 之间的流式与协议差异。
- **决策**：将 `tiylabs/tiygate`（Apache-2.0，commit `6db40e54`）作为协议层首要参考源，借鉴其 IR 中间表示与编解码器，但坚决不引入其 SQLx 数据库与商业计费模块。
- **原因**：TiyGate 拥有业界最成熟的 Rust 原生多协议无损转换算法。只提取协议编解码，可最大化利用其流式状态机，同时保持 ProxyWeave 的本地极简无库特色。

### ADR-03: 为什么选择微内核黑盒独立进程，而非动态链接库 (DLL/so) 或 WebAssembly？
- **背景**：插件运行形式的技术选型。
- **决策**：坚持黑盒可执行进程模型。
- **原因**：各类开源反代项目技术栈异构严重（Go、Rust、Node.js、Python）。动态链接库存在语言 ABI 冲突且单一崩溃会导致宿主同归于尽；Wasm 目前无法运行重型 Go 运行时及复杂系统 Socket。黑盒独立进程具备天然的内存隔离与故障边界。

### ADR-04: 为什么废除纯 LRU 淘汰，改为四象限生命周期与 Singleflight 互斥锁？
- **背景**：原方案采用强制限制 1~2 个进程并按 LRU 杀死。
- **决策**：引入四象限策略、`active_requests` 活跃引用计数与 Singleflight 冷启动锁。
- **原因**：LLM 会话长流式可能持续数分钟。按 LRU 杀死进程会导致正在进行的会话被腰斩；突发高并发请求命中休眠插件会导致重复拉起与端口碰撞。

### ADR-05: 为什么严禁在异常断开时盲目追加 `data: [DONE]`？
- **背景**：部分简陋网关为了强行闭合 SSE 流，在遇到任何错误时均追加 `[DONE]`。
- **决策**：严格区分正常流结束与异常断开，异常时坚决不补发 `[DONE]`。
- **原因**：当上游报错或网络截断时追加 `[DONE]`，下游代码编辑器（如 Cursor）会认为代码已生成完毕并直接应用残缺的代码，导致严重的工程事故。

### ADR-06: 为什么采用系统级凭证库/分离式加密，而非明文 JSON 存储 Secret？
- **背景**：凭据保存的安全性考量。
- **决策**：使用 `keyring-rs` 对接系统安全凭据管理器，与常规 JSON 配置物理分离。
- **原因**：个人电脑常面临恶意插件或同机脚本扫描普通配置目录的风险。将 Secret 托付给 OS 原生安全体系是桌面软件的行业标准规范。

### ADR-07: 为什么将 Reference-First 确立为强制性工程研发纪律？
- **背景**：AI 编程助手容易脱离工程实际盲目手写脆弱机制。
- **决策**：将 Reference-First 写入工程门禁，核心机制必须先有真实开源源码依据。
- **原因**：系统底层机制（进程组、端口竞态、SSE 解析）充满平台边缘陷阱，成熟开源项目经过了成千上万用户的实战检验，借鉴成熟方案是保障软件工业级质量的最优路径。

---

## 11. 重构后的实施里程碑与落地计划 (Milestones & Acceptance Gates)

实施路径从“机械式周计划”重构为**基于风险收敛与模块依赖关系的 7 个阶段**：

### Phase -1: 项目准入与门禁确认 (Launch Gate)
- **依赖项**：用户确认本规划书。
- **参考源**：官方品牌与 GitHub 规范。
- **核心开发内容**：
  1. 确认品牌名称 `ProxyWeave`，在 GitHub 创建干净独立的全新仓库；
  2. 确立 `README.md` 产品宗旨并打下 Initial Commit；
  3. 归档 `Plan.md`、`REFERENCE_REGISTRY.md` 与本审计结论。
- **验收标准**：通过 Launch Gate 检查，未写任何业务代码，仓库干净合规。

### Phase 0: 脚手架采用、基线编译与业务耦合剥离 (Runnable Baseline)
- **依赖项**：Phase -1 准入完成。
- **参考源**：Quotio Desktop (`xiaocoss/quotio-desktop`, commit `21d75d08`)。
- **核心开发内容**：
  1. 完整拉取 Quotio Desktop 基线，在本地验证可编译运行 (`npm run build` + `cargo check`)；
  2. 建立基准回归测试，锁定固定 Commit 并保留 MIT 版权声明；
  3. 执行业务解耦：剥离 `codex_*`、`kiro_sidecar`、`quota`、`scheduler` 与 `vendor` 目录；
  4. 验证桌面壳工程与 `quotio-platform` 仍可正常启动。
- **验收标准**：剔除 CPA 业务代码后，Tauri 桌面外壳与托盘正常工作，无编译报错。

### Phase 1: 通用插件运行时与进程管控 (Supervisor & Managed/External)
- **依赖项**：Phase 0 干净底座。
- **参考源**：`shared-child`、Quotio-Platform、本地 REF (`workbuddy-switch-gateway-latest`)。
- **核心开发内容**：
  1. 抽象通用 `PluginSupervisor`，实现 Win32 Job Object 与 Linux prctl 级联清理；
  2. 实现四象限生命周期调度、活跃请求计数器与 Singleflight 冷启动锁；
  3. 实现带退避的动态端口分配器 (Port Allocator)；
  4. 支持首发 Managed 插件 (`wb2api`) 与首发 External Provider (`CPA`) 的进程管控。
- **验收标准**：启动与杀死宿主，子进程 100% 连带回收，无孤儿进程，并发冷启动无碰撞。

### Phase 2: 异步网关接入与 TiyGate 协议层集成 (Gateway & Codecs)
- **依赖项**：Phase 1 进程调度可用。
- **参考源**：TiyGate (`tiylabs/tiygate`, commit `6db40e54`)、Axum 0.7、Hyper 1.0。
- **核心开发内容**：
  1. 搭建 Axum 异步 HTTP/SSE 网关，替换旧阻塞 `bridge.rs`；
  2. 移植 TiyGate 的 IR 编解码器（支持 Chat Completions 与 Messages 协议转换）；
  3. 实现严谨的 SSE 终止符状态机与取消传播 (CancellationToken)；
  4. 实现基础模型路由与虚拟别名映射。
- **验收标准**：通过 127.0.0.1:8080 向 wb2api 发起流式 Chat 会话流畅输出，中断请求立即取消上游。

### Phase 3: Manifest 引擎、凭证管理与安装升级 (Plugin Installer & Security)
- **依赖项**：Phase 2 协议网关可用。
- **参考源**：Quotio `proxy_download.rs`、Tunnel Agent Download、Keyring-rs。
- **核心开发内容**：
  1. 实现 Manifest Schema 1.0 解析与校验引擎；
  2. 泛化 GitHub Release 资产下载器，包含 SHA256 校验与解压；
  3. 对接系统 Keyring 凭据托管，实现 `0o600` 最小权限配置安全投影；
  4. 支持配置双副本持久化与损坏自动回滚。
- **验收标准**：通过 Manifest 一键下载并解压新版本插件，凭据安全存入系统凭据管理器。

### Phase 4: 用户交互、试飞工作台与 API Doctor (UX & Diagnostics)
- **依赖项**：Phase 3 插件引擎完备。
- **参考源**：DeepSeek Harness 前端、Quotio `agent_config.rs`。
- **核心开发内容**：
  1. 升级 React 前端 UI 为 ProxyWeave 极简暗色工业风格看板；
  2. 实现 API Doctor 全链路脱敏调用追踪与一键故障排查；
  3. 实现 Compatibility Flight 自动化 6 项协议质检套件；
  4. 复用 `agent_config.rs` 实现 Cursor、Claude Code 等 IDE 配置一键生成与接入。
- **验收标准**：前端可直观查看插件指标，一键诊断故障，质检套件 100% 通过，IDE 顺利接入。

### Phase 5: 插件生态分发、多平台构建与正式发布 (Ecosystem & Release)
- **依赖项**：Phase 4 交互与功能全部验收。
- **参考源**：Quotio Release 流水线、TiyGate Desktop 构建流水线。
- **核心开发内容**：
  1. 搭建官方插件源仓库 `proxyweave-registry` 与三级信任机制；
  2. 完善 Windows (NSIS / MSI)、macOS (DMG) 与 Linux (AppImage) CI 交叉构建流水线；
  3. 完善全部工程治理文档（README, ARCHITECTURE, SECURITY 等）；
  4. 发布 v0.1.0-alpha 公开版本。
- **验收标准**：三平台跨端构建顺利出包，安装运行无报错，完成首个正式发布。

---

## 12. 开源工程规范与项目文档体系 (Governance & Repository Standards)

### 12.1 规划中的开源标准文档体系 (Planned Docs)
在项目推进过程中，必须逐步建立并完善以下 10 项标准化工程治理文档：
- `README.md`：项目定位、核心宗旨、快速上手与架构全景（已完成规划设计）。
- `ARCHITECTURE.md`：详细系统分层、单向依赖拓扑与通信时序（计划 Phase 1 交付）。
- `PLUGIN_SPEC.md`：Manifest 1.0 完备技术规范与开发者编写指南（计划 Phase 3 交付）。
- `SECURITY.md`：威胁模型、漏洞披露流程、沙箱边界与凭据安全策略（计划 Phase 3 交付）。
- `CONTRIBUTING.md`：开发者贡献指南、代码风格、分支模型与 PR 规范（计划 Phase 5 交付）。
- `LICENSE`：正式开源许可证（遵循 MIT License）。
- `CHANGELOG.md`：遵循 Keep a Changelog 规范的版本演进记录。
- `COMPATIBILITY.md`：主流客户端（Cursor、Claude Code 等）与协议支持矩阵（计划 Phase 4 交付）。
- `docs/references/REFERENCE_REGISTRY.md`：开发前参考源详细档案（计划 Phase 0 建立）。
- `docs/architecture/SCAFFOLD_ADOPTION.md`：主脚手架采用审计与模块迁移记录（计划 Phase 0 建立）。
- `docs/architecture/ADR/`：架构决策记录集合。

### 12.2 CI/CD 矩阵与自动化质量门禁
- **代码风格与静态检查**：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`。
- **自动化测试**：`cargo test --workspace`（包含单元测试与 Mock 集成测试）。
- **多平台构建矩阵**：GitHub Actions 覆盖 `windows-latest`、`macos-latest` (Apple Silicon) 与 `ubuntu-22.04`。
- **安全扫描**：`cargo audit`（依赖漏洞扫描）与 Secret 泄露扫描。

### 12.3 核心纪律红线与边界约束清单
1. **绝对禁止全盘盲目 Fork 与改名欺骗**：必须执行严格的脚手架审计，保留开源协议归属与版权。
2. **绝对禁止闭门造车从零实现复杂系统机制**：非平凡功能坚决执行 Reference-First 准则。
3. **绝对禁止泄露真实凭证**：真实联调资产绝不进入版本控制。
4. **绝对禁止随意终止外部非托管进程**：严格保护 External Provider 的运行自主权。
5. **绝对禁止异常断开时虚假注入终止符**：保障流式协议真实性。
6. **当前阶段严格遵守规划纪律**：未经用户复审与 Phase -1 准入，**绝不擅自开始编写产品功能代码**。
