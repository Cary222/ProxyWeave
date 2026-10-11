# 🌐 ProxyWeave (编织网关)

> **Plug In. Route Anywhere.**（即插即用，随心路由）  
> **Local-first AI API Plugin Host & Unified Desktop Workbench**  
> 本地优先的 AI API 插件宿主与统一工作台

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021%20%7C%202024-orange.svg)](https://www.rust-lang.org/)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2.0-24C8DB.svg)](https://v2.tauri.app/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)]()

---

## 📖 产品宗旨与核心定位 (Manifesto)

### 核心哲学：上游易逝，胶水永存 · Everything is a Plugin

在大模型转接与开发代理生态中，各类开源反向代理与中继工具层出不穷（如 2API 工具、CLIProxyAPI、Token 轮换网关等）。然而，开发者与团队在本地日常使用中常面临诸多痛点：
- **上游工具割裂多开**：每个工具监听不同端口，客户端（Cursor、Claude Code、Windsurf 等）需频繁修改端口与配置；
- **进程残留与死锁**：后台子进程频繁残留、僵死，导致本地端口冲突（Port Conflict）；
- **逆向逻辑脆弱易逝**：当某特定平台调整前端协议或封控策略时，单体整合版往往全盘崩溃；
- **流式 SSE 协议残损**：简陋代理在网络抖动或异常报错时乱补或丢失 `[DONE]`，导致下游 IDE 误判完成而写入残损代码。

**ProxyWeave 应运而生。**  
它是一个轻量、安全、可扩展的**本地 AI API 插件宿主与统一接入工作台**。它运行于个人电脑（系统托盘 / 无头守护进程模式），将繁杂割裂的各类开源反代（2API）、CLI 代理或本地推理服务，以**黑盒插件（Blackbox Plugin）**的标准范式进行统一编排、按需启停、凭据托管、协议标准化与本地聚合路由。

---

## 🏛️ 三层架构体系 (Architecture)

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

---

## ✨ 核心特性矩阵 (Key Features)

- 🔌 **Universal Blackbox Plugins**：遵循统一的 `manifest.yaml` 规范，支持外部可执行文件、跨语言运行时与外部现存服务。
- ⚓ **Single Local Port**：对外暴露永久稳定的本地单入口（默认 `127.0.0.1:8080`，可自定义），免去频繁切换端口烦恼。
- 🔄 **Dual-Mode Runtime (双模运行)**：
  - **Managed Mode**：宿主负责生命周期管理、动态端口分配，绑定 Win32 Job Object / Linux `prctl` 保证父进程退出时 100% 连带清理。
  - **External Provider**：原生挂载本地已运行或远程局域网端点，**绝对禁止强行杀死或干涉外部进程**。
- ⚡ **Four-Quadrant Lifecycle**：细分为常驻 (`persistent`)、空闲回收 (`idle_timeout`)、按需唤醒 (`on_demand`) 与外部挂载 (`external`)，配备原子请求计数器与 Singleflight 互斥锁，长流式防腰斩，冷启动防击穿。
- 🧩 **TiyGate Protocol Codec (IR)**：借鉴 TiyGate 成熟的协议中间表示，支持 OpenAI Chat Completions、Responses API 与 Anthropic Messages 协议无损互转与流式事件解析。
- 🛡️ **Zero-DB & Keyring Security**：核心 Token 托管至操作系统底层安全凭据库 (Windows Credential Manager / macOS Keychain)，配置采用无数据库单文件原子读写与双副本容灾。
- 🩺 **API Doctor**：全链路脱敏调用追踪与端到端诊断，智能归因端口占用、401 凭证过期、429 上游限流与 502 子进程异常。
- ✈️ **Compatibility Flight**：内置 6 项全自动协议与流式质量检测套件（TTFT 测量、Tool Call 碎片拼装、取消传播与错误映射）。
- ⚙️ **IDE Handoff**：自动检测与一键同步 Cursor、Claude Code、Windsurf 等 IDE 配置文件。

---

## 📋 插件规范示例 (Manifest 1.0)

每个接入 ProxyWeave 的服务均由声明式 `manifest.yaml` 驱动：

```yaml
schema_version: "1.0"
metadata:
  id: "wb2api"
  name: "WorkBuddy 2API Engine"
  version: "1.13.0"
  description: "WorkBuddy 账号转接代理网关"
  license: "MIT"

runtime:
  type: "binary"
  entrypoint:
    windows: "bin/wb2api.exe"
    darwin: "bin/wb2api"
    linux: "bin/wb2api"
  args: ["-config", "{CONFIG_PATH}", "-port", "{PORT}"]

lifecycle:
  policy: "idle_timeout"
  idle_timeout_seconds: 300
  startup_timeout_seconds: 15

network:
  listen_host: "127.0.0.1"
  port_allocation: "dynamic"
  health_check:
    type: "http"
    path: "/healthz"

capabilities:
  protocols: ["openai_chat", "streaming", "tool_calling"]
```

---

## 🗺️ 文档导航与开发指引 (Documentation Map)

- 📜 **[Plan.md](./Plan.md)**：项目最高权威规划书 (v3.0.0)，包含脚手架采用审计、15 大核心模块对照矩阵、架构决策记录 (ADR) 与 7 阶段落地计划。
- 🏛️ **[docs/PLAN-ProxyWeave.md](./docs/PLAN-ProxyWeave.md)**：架构设计与规划规范镜像归档。
- 📦 **[docs/references/REFERENCE_REGISTRY.md](./docs/references/REFERENCE_REGISTRY.md)**：Reference-First 15 大核心模块开源参考源登记档案。
- ⚖️ **[docs/architecture/ADR/](./docs/architecture/ADR/)**：架构决策记录集合 (ADR-01 至 ADR-07)。
- 🧩 **[manifests/](./manifests/)**：已验证插件清单 (wb2api, CLIProxyAPI, wild-work)。
- 📜 **[THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)**：开源第三方组件版权与许可证声明。

---

## 🛠️ 当前研发状态 (Status: Phase -1 -> Phase 0)

项目当前处于 **Phase -1 Launch Gate 准入完毕，正步入 Phase 0（脚手架基线编译与业务解耦）**：
1. [x] **Phase -1 品牌确立与独立主干建立**：正式命名为 ProxyWeave，建立全新专属代码仓库，完成宗旨沉淀。
2. [ ] **Phase 0 脚手架基线与解耦**：引入 `xiaocoss/quotio-desktop` (Commit `21d75d08`)，保留 Tauri 2 桌面壳与 `quotio-platform` 跨平台基础设施，彻底剥离专有业务代码。
3. [ ] **Phase 1 进程管理微内核**：实现 `PluginSupervisor`、Win32 Job Object 与四象限生命周期调度。
4. [ ] **Phase 2 异步网关与协议集成**：引入 Axum 0.7 与 TiyGate 协议 IR 编解码流水线。
5. [ ] **Phase 3 插件引擎与凭证安全**：实现 Manifest 解析、GitHub Release 资产下载器与 Keyring 凭据托管。
6. [ ] **Phase 4 桌面工作台与 API Doctor**：上线极简暗色工业风看板与兼容性质检台。
7. [ ] **Phase 5 跨平台打包与公开首发**：发布 v0.1.0-alpha。

---

## ⚖️ 许可证 (License)

本项目基于 [MIT License](./LICENSE) 开源发布。  
Copyright (c) 2026 Cary & ProxyWeave Contributors.
