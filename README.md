# 🌐 ProxyWeave (编织网关)

### Turn AI Proxy Projects into Plug-and-Play Apps.
**让每一个 AI 代理项目，都能成为即插即用的应用。**

> **Slogan**：Plug In. Route Anywhere.（即插即用，随心路由）  
> **定位**：Local-first AI Proxy Application Platform & Desktop Workbench（本地优先的 AI 代理应用集成平台与统一工作台）  
> **核心哲学**：上游易逝，平台长存 · Everything is a Plugin（万物皆插件） · **成熟实现直接复用优先 (Direct Reuse First)**  
> **项目状态**：Pre-development · 架构规划与脚手架准入阶段 (Phase -1 规范冻结完成，Phase 0 脚手架审计与基线准备进行中，尚未编写产品业务代码)

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

## 💡 核心设计与开发政策 (Core Principles & Engineering Policies)

### 1. Everything is a Plugin · 具体能力插件化
宿主只维护通用基础设施，不把某个服务商的登录、逆向或请求细节写死在核心中。每个插件通过版本化的 `manifest.yaml` 描述来源、能力、运行方式、配置和权限，并在独立进程或已有 HTTP 服务中提供能力。

### 2. 成熟实现直接复用优先 (Direct Reuse First) · 强制开发纪律
**找到满足需求、许可证允许且依赖可控的成熟代码后，应直接使用或移植对应功能模块，再进行必要适配。禁止阅读其代码上下文后，让 AI 重新生成一套功能相同的实现，并将这种重写称为复用。**

实现方式严格按以下顺序选择：
1. **成熟库或 SDK 可以直接依赖时，优先直接依赖**。
2. **成熟项目中存在合适模块时，提取该模块及最小必要依赖，保留可靠性处理和必要测试**。
3. **需要接入本项目接口时，在复用实现外增加薄适配层**。
4. **仅在有明确不兼容问题时修改原实现，并记录原因**。
5. **只有检索和比较后确实无法合理复用，才允许通过正式架构决策记录 (ADR) 陈述依据后自研**。

> **AI 职责边界**：AI 主要承担源码检索、方案比较、代码提取、接口适配、集成测试和缺陷修复，**不得默认重写已有成熟算法或系统机制**。

### 3. 主脚手架采用 vs 功能模块复用的清晰界限
- **主脚手架采用 (Main Scaffold)**：保留 Quotio Desktop 为首选脚手架候选，采用前必须在本地核验其固定 Commit (`21d75d08`)、许可证 (MIT)、构建情况、依赖树与业务耦合。主脚手架允许完整获取仓库用于审计和建立开发基线，也允许直接保留合适的桌面壳 (`apps/desktop/src-tauri`)、工作区、平台抽象封装 (`quotio-platform`) 和构建发布体系。在保留原作者版权和许可证的前提下，剥离不需要的特定业务（Codex/Kiro/额度轮询），再开展二次开发。**严禁为了品牌或目录统一重写已经适用的成熟基础代码**。
- **功能模块复用 (Modular Reuse)**：除主脚手架外，其他参考项目原则上按功能模块引入，不整体合并其产品和业务体系。复用模块范围包括其运行所必需的局部依赖与可靠性保障测试，**不能只复制一个裸函数而删除了异常保护与跨平台兜底**。

### 4. Local First, Lightweight by Design · 本地优先
基于 Tauri v2 + Rust 构建桌面平台，使用版本化本地单文件配置与操作系统原生凭证库，彻底杜绝把外部数据库 (MySQL/PostgreSQL)、Redis 或云端控制面作为个人使用的前提。核心常驻目标内存 < 25MB。

### 5. Independent Core, Open Ecosystem · 上游独立与黑盒接入
插件及其上游项目各自维护、独立升级。对于本地“黑盒项目”，若存在源码（如本地 `D:\workbuddy2api-panel` Go 源码），核验后可按模块复用；若仅有可执行二进制或已运行的 HTTP 服务（如 `D:\CLIProxyAPI_...`），则一律按公开接口以 External 模式接入，**严禁假称已复用其内部源码**。

---

## 🏛️ 目标系统架构 (Target Architecture)

ProxyWeave 确立了清晰的**三层分工体系与核心职责解耦**：

```text
┌─────────────────────────────────────────────────────────────┐
│                 Layer 1: Host Microkernel                   │
│   Tauri v2 UI + System Tray + Axum Async Gateway + Core     │
│   (生命周期调度、四象限策略、端口分配与恢复、安全凭据托管、模型路由)   │
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
- **Plugin Supervisor**：专管子进程生命周期、系统级进程组绑定、级联回收与 Singleflight 互斥锁。
- **API Gateway**：专管对外单端口（默认 `127.0.0.1:8080`）的 HTTP/SSE 连接与背压取消传播。
- **Model Router**：专管虚拟模型别名映射与基于插件健康度的 Fallback 故障转移。
- **Protocol Adapter**：基于 IR 中间表示，专管 OpenAI Chat、Responses 与 Anthropic Messages 协议转换、状态机与能力降级。
- **Credential Broker**：专管系统 Keyring 凭据托管与沙盒目录 `0o600` 最小权限临时配置投影。
- **Plugin Registry**：专管插件来源、Manifest 解析、版本固定与 SHA256 完整性校验。

---

## 🔄 双模运行架构与生命周期 (Dual-Mode Runtime)

### 1. Managed Plugin (托管模式)
- 宿主在用户授权后获取并校验发行产物（GitHub Release 校验 SHA256 完整性），负责配置、拉起、探针、退出和重启策略。
- **端口竞争预防与冲突恢复**：采用带随机退避的端口初筛算法；若子进程启动发生端口碰撞，立即捕获异常并自动更换端口重试（最多 3 次）。
- **系统进程组回收**：在支持 Job Object 的 Windows 系统和支持 prctl 的 Linux 系统上，将子进程绑定到系统进程组，宿主正常或异常退出时内核自动连带回收，并提供异常断电后的残留容灾探针。
- **四象限生命周期策略**：
  1. `persistent`：核心常驻插件，随宿主启动常驻，不休眠；
  2. `idle_timeout`：空闲回收插件，无流量超过 TTL（默认 5 分钟）后优雅退出；
  3. `on_demand`：冷门重型插件，请求到达时冷启动唤醒，处理完毕后释放；
  4. `external`：外部独立服务，宿主不负责启停，永不发送 Kill 信号。
- **长会话保护**：活跃请求原子计数器保证有正在传输的流式请求时绝不强行中断。

### 2. External Provider (外部接入模式)
- 连接用户已在本地或局域网部署运行的 HTTP API 服务（如已有 CLIProxyAPI 监听 28317，或局域网 vLLM / Ollama）。
- 完成模型自动发现、请求透明转发与诊断。
- **核心铁律**：**不视外部进程为宿主所有，绝对禁止向外部服务发送 Kill 信号或强行终止该进程。**

---

## 🌊 SSE 流式协议与终止符规整 (Streaming & Termination)

在流式输出中，必须严格区分正常传输完毕与异常网络中断：

1. **补发终止符的前提条件**：**必须具有协议层正常完成证据**（例如 HTTP 状态码为 200，且上游已发出包含合法 `finish_reason` 的尾帧，或上游流按规范自然关闭）。此时若上游遗漏尾部结束标记，才允许规范补发终止符。
2. **异常截断坚决不补发**：若上游因网络异常中断、超时截断或 5xx 报错关闭连接，**绝对禁止盲目追加 `data: [DONE]`**，必须向下游转发错误事件帧或直接断开连接，防止下游代码编辑器（如 Cursor）误判生成完毕而直接应用残损代码。
3. **区分各协议独立终止规则**：
   - OpenAI Chat Completions 协议：正常终止事件为 `data: [DONE]\n\n`；
   - Anthropic Messages 协议：正常终止事件为 `event: message_stop` 事件帧；
   - OpenAI Responses API：正常终止事件为 `response.completed` 事件帧。

---

## 📊 协议兼容矩阵 (Protocol Compatibility Matrix)

ProxyWeave 将协议适配能力明确划分为 5 级支持标准，坚决不做含糊的“全协议无损互转”绝对承诺：

| 源客户端协议 | 目标插件协议 | 支持等级 | 转换与降级处理规范 |
|---|---|---|---|
| **OpenAI Chat** | **OpenAI Chat** | **透传 (Passthrough)** | Headers/Body 原样透传，零序列化与流式延迟损耗。 |
| **OpenAI Chat** | **Anthropic Messages** | **支持转换 (Supported)** | 消息角色映射、多轮对话拼接、标准 Tool Calls 参数片段规整。Anthropic 独有的 `thinking` 块降级为 Markdown 折叠文本或 System Prompt 引导。 |
| **OpenAI Responses** | **OpenAI Chat** | **支持转换 (Supported)** | Responses API 状态机与 Chat Completions 双向映射，流式事件转换。 |
| **Anthropic Messages** | **OpenAI Chat** | **支持转换 (Supported)** | `tools` 结构转换为 OpenAI Functions/Tools 规范，流式 `content_block` 映射为 `delta`。 |
| **多模态图像 (Image)** | **多模态图像 (Image)** | **降级适配 (Degraded)** | 统一将 Base64 / URL 图像数据块转换为目标协议支持的对象格式；超出分辨率限制时按比例降采样。 |
| **音频流 / 实时 WebRTC** | **文本 API 插件** | **明确拒绝 (Rejected)** | 存在不可逾越的协议与介质鸿沟，网关直接返回 `400 Bad Request` 伴随明确错误码，拒绝假转换。 |
| **厂商自定义专属扩展** | **任意目标协议** | **未验证 (Unverified)** | 包含未知自定义私有字段的请求，在未通过 Compatibility Flight 验证前默认剥离非标字段或报错。 |

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
  permissions:
    allow_internet_access: true
    allow_local_storage: true
```

---

## 🔒 安全透明声明与概念边界 (Security Boundaries)

为保障工程诚实性与用户知情权，ProxyWeave 严谨界定三大安全概念：

1. **权限声明 vs 实际操作系统权限**：Manifest 中的 `security.permissions` 属于**声明式契约与用户告知机制**，用于在界面展示风险并在宿主内部控制入参；在未运行于内核沙箱的前提下，外部启动的可执行文件具备当前操作系统用户的执行权限。**权限声明不等于操作系统底层安全沙箱！**
2. **SHA256 完整性校验 vs 密码学数字签名**：宿主对 GitHub Release 资产执行 SHA256 校验仅用于验证安装包在下载过程中未损坏且与清单哈希一致（完整性验证）；在发布者未提供公私钥证书（GPG/Authenticode）进行签名验证前，该校验不代表密码学数字签名。
3. **独立子进程 vs 安全沙箱**：Managed 插件运行于独立子进程中，具备内存隔离与崩溃隔离（插件崩溃不影响宿主）；但这属于多进程架构特性，**独立子进程不等于安全沙箱**，用户必须谨慎评估安装来源。

---

## 🔬 开源脚手架与参考来源 (Reference-First)

ProxyWeave 坚持 **成熟实现直接复用优先**：非平凡系统机制坚决不从零造轮子，先阅读成熟开源源码，再评估、移植与解耦。

| 项目 | 在 ProxyWeave 中的角色 | 重点借鉴与复用范围 | 隔离与剥离边界 | 验证状态 |
| --- | --- | --- | --- | --- |
| **[Quotio Desktop](https://github.com/xiaocoss/quotio-desktop)** | **首选主脚手架基线** | Tauri v2 桌面壳、系统托盘、`quotio-platform` (跨平台路径与 Keyring 凭据托管)、`agent_config.rs` (IDE 接入)、`proxy_download.rs` (下载器) | 彻底剥离 Codex 多开、Kiro 专属集成、额度轮询及多账号业务；替换同步阻塞的 `bridge.rs`；切断 SQLite 依赖 | 🟢 **源码已核验**<br>(Commit `21d75d08`) |
| **[TiyGate](https://github.com/tiylabs/tiygate)** | **网关与协议模块参考** | Axum 0.7 异步网关流水线、N*M 协议 IR 中间表示编解码、SSE 状态机与 Tool Call 碎片拼装 | 坚决剥离其 10 余个表的 SQLx 数据库迁移依赖、管理员 REST API 与企业 OAuth | 🟢 **源码已核验**<br>(Commit `6db40e54`) |
| **[Tunnel Agent](https://github.com/beyondhumane/tunnel-agent)** | **多引擎管理与交互参考** | 多异构引擎统一调度交互体验、GitHub Release 资产探测锁定流程、Win32 Job Object 单元测试用例 | 坚决不引入其 C#/.NET 运行时与 Avalonia 技术栈 | 🟢 **源码已核验**<br>(Commit `8cfbd02`) |
| **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)** | **微内核设计哲学参考** | Everything is a Plugin、Cordis 可逆副作用生命周期（Reversible Effects，退出自动销毁端口与配置）、暗色工业风看板 | 坚决不引入其重型 Node.js/Agent 运行时与复杂智能体体系 | ⚪ **待核验**<br>(Commit `d7432673`) |

---

## 🗺️ 研发路线图 (Roadmap)

所有阶段均经过严格的模块依赖与风险收敛推导，**未获真实证据的门禁严禁标为通过**：

| 阶段 | 主要交付物 | 关键验收门禁 | 状态 |
| --- | --- | --- | --- |
| **Phase -1 · 准入与治理** | 品牌确立 (ProxyWeave)、独立 GitHub 仓库、产品宗旨 README、Plan.md 架构规范冻结 | 需求边界清晰，参考源档案建立，无未经审计的盲目开发 | **Done (已完成)** |
| **Phase 0 · 脚手架采用与解耦** | 引入 Quotio Desktop (`21d75d08`)，本地验证可编译构建 (`npm run build` + `cargo check`)，剥离特定业务代码 | 桌面壳与托盘正常工作，跨平台系统抽象通过测试，工作区纯净 | **In Progress (进行中)** |
| **Phase 1 · 核心契约与运行时** | **最小 Manifest Schema 1.0 冻结**、**凭证安全传递与沙箱目录 0o600 投影**、`PluginSupervisor` 进程组清理、端口冲突重试恢复、首发 Managed (`wb2api`) 与 External (`CPA`) 挂载 | 退出连带回收无孤儿进程，并发冷启动无端口碰撞，禁杀外部进程 | Planned |
| **Phase 2 · 异步网关与协议** | Axum 0.7 异步 HTTP/SSE 流式网关、TiyGate 协议 IR 编解码、SSE 终止符状态机、协议兼容矩阵与模型别名路由 | 客户端流式输出流畅，中断请求即刻取消上游，异常断开坚决不补发 `[DONE]` | Planned |
| **Phase 3 · 插件引擎与持久化** | Manifest 完整解析校验引擎、带 SHA256 完整性校验的资产下载器、版本固定与安全回滚、配置单文件原子写入 (`.tmp -> fsync -> rename`) 与 `.bak` 双副本 | 插件一键解析安装，配置损坏秒级自动回滚 | Planned |
| **Phase 4 · 桌面看板与诊断** | 极简暗色工业风 UI、API Doctor 脱敏排障链、Compatibility Flight 质检工作台、IDE Handoff 自动写入 | 诊断链准确归因故障，质检套件通过，Cursor/Claude Code 顺畅接入 | Planned |
| **Phase 5 · 生态索引与发布** | 官方插件源 `proxyweave-registry`、Windows (MSI/NSIS) / macOS (DMG) / Linux (AppImage) CI 构建与发布 | 多平台安装包构建正常，发布 v0.1.0-alpha 公开版本 | Planned |

---

## 📁 仓库核心文档导航 (Documentation Map)

- 📜 **[Plan.md](./Plan.md)**：项目最高权威规划书 (v3.0.0)，涵盖 12 章节详细推导。
- 🏛️ **[docs/PLAN-ProxyWeave.md](./docs/PLAN-ProxyWeave.md)**：架构设计归档镜像。
- 📦 **[docs/references/REFERENCE_REGISTRY.md](./docs/references/REFERENCE_REGISTRY.md)**：15 大核心模块开源参考源登记档案库。
- ⚖️ **[docs/architecture/ADR/](./docs/architecture/ADR/)**：架构决策记录 (ADR-01 至 ADR-07)。
- 🧩 **[manifests/](./manifests/)**：已验证插件 Manifest 样例清单。
- 📜 **[THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)**：第三方开源许可与致谢。
- 🗄️ **[docs/PLAN-GlueHarness.md](./docs/PLAN-GlueHarness.md)**：[已归档] 早期 GlueHarness 草案历史记录。

---

## ⚖️ 许可证 (License)

本项目基于 [MIT License](./LICENSE) 开源发布。  
Copyright (c) 2026 Cary & ProxyWeave Contributors.

---

**ProxyWeave — Turn AI Proxy Projects into Plug-and-Play Apps.**  
*让分散的代理项目，成为可以自由组合的应用生态。*
