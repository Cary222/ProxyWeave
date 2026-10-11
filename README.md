# 🌐 ProxyWeave (编织网关)

### Turn AI Proxy Projects into Plug-and-Play Apps.
**让每一个 AI 代理项目，都能成为即插即用的应用。**

> **Slogan**：Plug In. Route Anywhere.（即插即用，随心路由）  
> **定位**：Local-first AI Proxy Application Platform & Desktop Workbench（本地优先的 AI 代理应用集成平台与统一工作台）  
> **核心哲学**：上游易逝，平台长存 · Everything is a Plugin（万物皆插件） · Reference-First（成熟实现优先）  
> **项目状态**：Pre-development · 架构规划与脚手架准入阶段 (Phase -1 Launch Gate 通过，步入 Phase 0)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021%20%7C%202024-orange.svg)](https://www.rust-lang.org/)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2.0-24C8DB.svg)](https://v2.tauri.app/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)]()

---

## 📖 为什么做 ProxyWeave？(Vision & Problem)

优秀的开源 AI 反向代理、2API 转接工具和模型网关层出不穷（如 CLIProxyAPI、各类 *2API、Token 轮换网关等）。然而，每个项目往往都有自己的安装命令、运行依赖、凭证格式、监听端口、API 协议和更新方式。对开发者而言，最大的痛点不仅是“如何调用一个模型”，更是“如何长期维护多个彼此独立的代理程序”：

- **发现与部署分散**：需要在多个 GitHub 仓库间搜索、下载、编译、安装和排障。
- **配置反复劳动**：不同工具的启动参数、凭证格式（毫秒/秒、明文/加密）和客户端接入方法完全不一致。
- **生命周期割裂与冲突**：多个常驻后台进程频繁产生端口冲突（Port Conflict）、父进程退出后子进程残留僵死，资源无序占用。
- **上游变化脆弱易逝**：当某特定平台的接口、Cookie 抓取或前端协议发生更迭，单体聚合工具极易全盘崩溃。
- **能力无法自然组合**：即使运行了多个代理，也缺少共同的模型发现、虚拟别名路由、实时试飞与端到端脱敏排障入口。

> **核心判断**：ProxyWeave **不打算重新实现所有上游代理**。它要为这些已有项目提供一个标准的、可复用的、本地优先的**应用宿主（Host & Workbench）**。  
> **上游易逝，平台长存。** 代理项目可以更迭，插件可以替换，但插件契约、用户配置与统一接入体验应当持续演进。

---

## 💡 核心设计原则 (Design Principles)

### 1. Everything is a Plugin · 具体能力插件化
宿主只维护通用基础设施，不把某个服务商的登录、逆向或请求细节写死在核心中。每个插件通过版本化的 `manifest.yaml` 描述来源、能力、运行方式、配置和权限，并在独立进程或已有 HTTP 服务中提供能力。

### 2. Reference First, Build Less · 先研究，再复用
优先阅读官方规范与成熟开源项目的真实源码，评估许可证、安全性和移植成本；能合理复用的系统机制绝不从零造轮子。主脚手架在合规审计后作为开发基线，其他参考项目优先按模块复用，严禁无审查整体合并。

### 3. Local First, Lightweight by Design · 本地优先
基于 Tauri v2 + Rust 构建桌面平台，使用版本化本地单文件配置与操作系统原生凭证库，彻底杜绝把外部数据库 (MySQL/PostgreSQL)、Redis 或云端控制面作为个人使用的前提。核心常驻目标内存 < 25MB。

### 4. Independent Core, Open Ecosystem · 上游独立
插件及其上游项目各自维护、独立升级。ProxyWeave 的价值来自稳定的插件契约、进程生命周期管控、接口兼容、故障隔离和使用体验，而不是绑定某个特定代理引擎。

---

## 🏛️ 目标系统架构 (Architecture)

ProxyWeave 确立了清晰的**三层分工体系与核心职责解耦**：

```text
┌─────────────────────────────────────────────────────────────┐
│                 Layer 1: Host Microkernel                   │
│   Tauri v2 UI + System Tray + Axum Async Gateway + Core     │
│   (生命周期调度、四象限策略、端口动态绑定、安全凭证托管、模型路由)   │
└──────────────────────────────┬──────────────────────────────┘
                               │ Manifest 1.0 契约 / 统一控制面
┌──────────────────────────────▼──────────────────────────────┐
│                 Layer 2: Glue Adapter                       │
│   协议编解码 (TiyGate Codec IR) + 流式规整 + 凭据投影 + 探针检测   │
└──────────────────────────────┬──────────────────────────────┘
                               │ Loopback HTTP / IPC / Process
┌──────────────────────────────▼──────────────────────────────┐
│                 Layer 3: Blackbox Plugins                   │
│  [Managed Plugin]          [External Provider]              │
│  wb2api (Go 二进制)          已运行的 CLIProxyAPI (127.0.0.1:28317)│
│  wild-work (Node 二进制)     远程 vLLM / Ollama 局域网端点       │
└─────────────────────────────────────────────────────────────┘
```

### 核心职责分离 (Separation of Concerns)
- **Plugin Supervisor**：专管进程生命周期、Win32 Job Object 级联回收与 Singleflight 互斥锁。
- **API Gateway**：专管对外单端口（默认 `127.0.0.1:8080`）的 HTTP/SSE 连接与背压取消传播。
- **Model Router**：专管虚拟模型别名映射与基于插件健康度的 Fallback 故障转移。
- **Protocol Adapter**：基于 IR 中间表示，专管 OpenAI Chat、Responses 与 Anthropic Messages 协议转换与 SSE 状态机。
- **Credential Broker**：专管系统 Keyring 凭据托管与沙盒目录 `0o600` 最小权限临时配置投影。
- **Plugin Registry**：专管插件来源、Manifest 解析、版本固定与校验和。

---

## 🔄 双模运行架构 (Dual-Mode Runtime)

### 1. Managed Plugin (托管模式)
- 宿主在用户授权后获取并校验发行产物（GitHub Release 校验 SHA256），负责配置、拉起、探针、退出和重启策略。
- 分配动态回环端口（如 `127.0.0.1:AUTO_PORT`），消除端口冲突。
- 强制绑定 Win32 Job Object (`KILL_ON_JOB_CLOSE`) 与 Linux `prctl(PR_SET_PDEATHSIG)`，**宿主退出时 100% 级联清理子进程，绝无孤儿进程残留**。
- 支持四象限生命周期：`persistent`（高频常驻）、`idle_timeout`（空闲 5 分钟安全回收）、`on_demand`（冷启动唤醒）。
- **长会话保护**：活跃请求原子计数器保证有正在传输的流式请求时绝不强行中断。

### 2. External Provider (外部接入模式)
- 连接用户已在本地或局域网部署运行的 HTTP API 服务（如已有 CLIProxyAPI 监听 28317，或局域网 vLLM / Ollama）。
- 完成模型自动发现、请求透明转发与诊断。
- **核心铁律**：**不视外部进程为宿主所有，绝对禁止向外部服务发送 Kill 信号或强行终止该进程。**

---

## 📋 插件契约：Manifest First (Schema 1.0)

插件通过版本化的 `manifest.yaml` 描述元数据与运行规范：

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
```

### 插件发现与三级信任等级 (Trust Tiers)
- **Tier 1: Verified (官方核验)**：经项目维护者核验的来源与固定版本；提供一键安装与完整性验证。
- **Tier 2: Community (社区索引)**：社区注册表中的第三方插件，清晰标注未审查范围并强制弹窗风险告知。
- **Tier 3: Local / Custom (本地自建)**：开发者自定义 Manifest 或本地调试程序。

> **安全透明声明**：插件具有本机执行能力。GitHub Topic 检索、下载和 SHA256 校验**都不等于安全沙箱**；ProxyWeave 不承诺“随意粘贴任意仓库就能安全运行”，所有安装过程均包含显式权限告知与用户确认。

---

## 🚀 预期使用路径 (User Journey)

```text
发现兼容插件 / 挂载已有外部服务
              ↓
确认来源 / 权限 / 许可证与平台兼容性
              ↓
安装或连接 → 引导式凭证配置 → 就绪探针检测
              ↓
模型发现 → Playground 试飞 / Compatibility Flight 质检
              ↓
获取本地单入口 (127.0.0.1:8080) → 一键配置 Cursor / Claude Code
              ↓
通过 API Doctor 定位异常 / 安全热更新与故障秒级回滚
```

---

## 🎯 核心能力与差异化功能 (Key Differentiators)

| 领域 | 目标能力与差异化实现 |
| --- | --- |
| **API Gateway** | 本地永久单入口（默认 `127.0.0.1:8080`），模型虚拟别名映射，健康度 Fallback 分流 |
| **协议适配 (IR)** | 借鉴 TiyGate 编解码算法，无损互转 OpenAI Chat、Responses API 与 Anthropic Messages |
| **流式状态机** | 严格区分正常结束（补发 `[DONE]`）与异常截断（**坚决不补发 `[DONE]`**，防止下游 IDE 应用残缺代码） |
| **运行与生命周期** | 四象限策略，活跃请求计数防中断，Singleflight 互斥锁防止突发高并发冷启动碰撞 |
| **API Doctor** | 全链路脱敏调用追踪：`Client -> 8080 -> Router -> 插件 -> 上游`，智能归因 401/429/502 |
| **Compatibility Flight** | 内置 6 项自动化协议与流式质检套件（TTFT 首字延迟、Tool Call 碎片拼装、取消传播验证） |
| **IDE Handoff** | 自动探测并一键更新 Cursor、Claude Code、Windsurf 等 IDE 配置文件 |
| **安全存储 (Zero-DB)** | 核心 Token 托管至系统底层 Keyring，配置单文件原子写入 (`.tmp -> fsync -> rename`) 与 `.bak` 双副本 |

---

## 🔬 开源脚手架与参考来源 (Reference-First)

ProxyWeave 坚持 **Reference-First**：非平凡系统机制坚决不从零造轮子，先阅读成熟开源源码，再评估、移植与解耦。

| 项目 | 在 ProxyWeave 中的角色 | 重点借鉴与复用范围 | 隔离与剥离边界 |
| --- | --- | --- | --- |
| **[Quotio Desktop](https://github.com/xiaocoss/quotio-desktop)** | **首选主脚手架** | Tauri v2 桌面壳、系统托盘、`quotio-platform` (1192 行成熟跨平台路径与 Keyring 凭据库)、`agent_config.rs` (IDE 接入)、`proxy_download.rs` (下载器) | 彻底剥离 Codex 多开、Kiro 专属集成、额度轮询及多账号业务；替换同步阻塞的 `bridge.rs`；切断 SQLite 依赖 |
| **[TiyGate](https://github.com/tiylabs/tiygate)** | **网关与协议参考** | Axum 0.7 异步网关流水线、N*M 协议 IR 中间表示编解码、SSE 状态机与 Tool Call 碎片拼装 | 坚决剥离其 10 余个表的 SQLx 数据库迁移依赖、管理员 REST API 与企业 OAuth |
| **[Tunnel Agent](https://github.com/beyondhumane/tunnel-agent)** | **多引擎管理与交互参考** | 多异构引擎统一调度交互体验、GitHub Release 资产探测锁定流程、Win32 Job Object 单元测试用例 | 坚决不引入其 C#/.NET 运行时与 Avalonia 技术栈 |
| **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)** | **微内核设计哲学** | Everything is a Plugin、Cordis 可逆副作用生命周期（Reversible Effects，退出自动销毁端口与配置）、暗色工业风看板 | 坚决不引入其重型 Node.js/Agent 运行时与复杂智能体体系 |

---

## 🗺️ 研发路线图 (Roadmap)

所有阶段均经过严格的模块依赖与风险收敛推导：

| 阶段 | 主要交付物 | 关键验收门禁 | 状态 |
| --- | --- | --- | --- |
| **Phase -1 · 准入与治理** | 品牌确立 (ProxyWeave)、独立 GitHub 仓库、产品宗旨 README、Plan.md 架构规范冻结 | 需求边界清晰，参考源档案建立，无未经审计的盲目开发 | **Done (已完成)** |
| **Phase 0 · 脚手架与解耦** | 引入 Quotio Desktop 基线，本地验证编译构建，彻底剥离专有业务代码，保留桌面壳与系统库 | 桌面壳与托盘正常工作，跨平台系统抽象通过测试，工作区干净 | **In Progress (进行中)** |
| **Phase 1 · 运行时微内核** | `PluginSupervisor`、Win32 Job Object / Linux prctl 级联清理、动态端口分配器、首发 Managed (`wb2api`) 与 External (`CPA`) 挂载 | 退出 100% 回收无孤儿进程，并发冷启动无端口碰撞，禁杀外部进程 | Planned |
| **Phase 2 · 异步网关与协议** | Axum 0.7 异步 HTTP/SSE 流式网关、TiyGate 协议 IR 编解码、SSE 终止符状态机、基础模型别名路由 | 客户端流式输出流畅，中断请求即刻取消上游，异常断开不乱补 `[DONE]` | Planned |
| **Phase 3 · 插件引擎与安全** | Manifest 1.0 解析校验引擎、带 SHA256 校验的资产下载器、Keyring 系统安全凭据托管、No-DB 原子持久化 | Manifest 一键解析安装，Secret 安全隔离投影，配置损坏秒级回滚 | Planned |
| **Phase 4 · 桌面看板与诊断** | 极简暗色工业风 UI、API Doctor 脱敏排障链、Compatibility Flight 质检工作台、IDE Handoff 自动写入 | 诊断链准确归因故障，质检套件 100% 通过，Cursor/Claude Code 顺畅接入 | Planned |
| **Phase 5 · 生态索引与发布** | 官方插件源 `proxyweave-registry`、Windows (MSI/NSIS) / macOS (DMG) / Linux (AppImage) CI 构建与发布 | 三平台安装包构建正常，发布 v0.1.0-alpha 公开版本 | Planned |

---

## 🔒 安全、合规与风险边界 (Security & Compliance)

- **本地优先与最小暴露**：默认绑定 `127.0.0.1` 本地回环地址，管理端点与推理端点物理隔离，本地 API Key 默认鉴权拦截同机探测。
- **凭据物理隔离**：Token 等 Secret 通过系统安全凭据管理器隔离存储，严禁在代码仓库、示例数据或明文日志中硬编码真实密钥。
- **真实性保障**：严格区分流式正常完成与异常截断，不将故障伪装为成功。
- **版权合规**：完整保留所有参考开源项目（Quotio, TiyGate 等）的版权声明与许可证；不以规避上游安全策略为目的。

---

## 📁 仓库核心文档导航 (Documentation Map)

- 📜 **[Plan.md](./Plan.md)**：项目最高权威规划书 (v3.0.0)，涵盖 12 章节详细推导。
- 🏛️ **[docs/PLAN-ProxyWeave.md](./docs/PLAN-ProxyWeave.md)**：架构设计归档副本。
- 📦 **[docs/references/REFERENCE_REGISTRY.md](./docs/references/REFERENCE_REGISTRY.md)**：15 大核心模块开源参考源登记档案库。
- ⚖️ **[docs/architecture/ADR/](./docs/architecture/ADR/)**：架构决策记录 (ADR-01 至 ADR-07)。
- 🧩 **[manifests/](./manifests/)**：已验证插件 Manifest 样例清单。
- 📜 **[THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)**：第三方开源许可与致谢。

---

## ⚖️ 许可证 (License)

本项目基于 [MIT License](./LICENSE) 开源发布。  
Copyright (c) 2026 Cary & ProxyWeave Contributors.

---

**ProxyWeave — Turn AI Proxy Projects into Plug-and-Play Apps.**  
*让分散的代理项目，成为可以自由组合的应用生态。*
