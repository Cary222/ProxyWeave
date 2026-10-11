# Third-Party Notices & Open Source Attributions

ProxyWeave (编织网关) 严格遵守开源软件知识产权与合规伦理。本项目在架构规划、脚手架选型与功能开发过程中，参考了社区优秀开源项目的优秀实践与设计模式。

> **声明分类原则**：
> 1. **第一部分（计划引入）**：经审计拟作为主脚手架基线或作为独立功能模块提取复用的成熟开源项目。在代码实际引入后，完整保留其原版权声明与许可证。
> 2. **第二部分（仅架构/算法设计参考）**：仅用于吸收其系统架构、设计哲学或规避已知缺陷经验，**未整体合并其代码仓库**，亦不引入其非必要运行时的参考源。

---

## 第一部分：计划采用的主脚手架与待引入代码模块 (Planned for Direct Adoption & Modular Reuse)

### 1. Quotio Desktop
- **仓库地址**: https://github.com/xiaocoss/quotio-desktop
- **锁定版本 / Commit**: `21d75d08b38a23f97fbd534a1768a829cb147f2c` (v0.7.8)
- **开源许可证**: MIT License
- **版权所有**: Copyright (c) 2024-2026 xiaocoss
- **计划复用范围**: 作为首要主脚手架候选基线（Tauri v2 桌面壳、系统托盘、`quotio-platform` 跨平台系统抽象、`agent_config.rs` IDE 配置分发）。彻底剥离其 Codex/Kiro 专有业务与 SQLite 依赖。

### 2. shared-child
- **仓库地址**: https://github.com/oconnor663/shared-child.rs
- **固定版本**: v1.0.1 (crates.io)
- **开源许可证**: MIT License / Apache License 2.0
- **版权所有**: Copyright (c) 2017 Jack O'Connor
- **计划复用范围**: 计划在 M01 Supervisor 模块中作为直接依赖或核心进程等待模块，解决子进程跨平台并发等待与 Win32 Job Object 绑定清理。

### 3. portpicker-rs
- **仓库地址**: https://github.com/Cargo-Prebuilt/portpicker-rs
- **固定版本**: v0.1.1 (crates.io)
- **开源许可证**: MIT License / Apache License 2.0
- **版权所有**: Copyright (c) 2015 The Rust Project Developers; Copyright (c) 2021 Cargo-Prebuilt
- **计划复用范围**: 计划在 M02 Port Allocator 模块中引入作为动态回环端口初筛算法，结合外层端口冲突捕获重试循环使用。

### 4. keyring-rs
- **仓库地址**: https://github.com/hwchen/keyring-rs
- **固定版本**: v3.6.2 (crates.io)
- **开源许可证**: MIT License / Apache License 2.0
- **版权所有**: Copyright (c) 2016-2024 keyring-rs contributors
- **计划复用范围**: 计划在 M07 Credential Broker 模块中引入，提供操作系统底层安全凭据库（Windows Credential Manager / macOS Keychain / Linux Secret Service）的统一托管。

### 5. eventsource-stream
- **仓库地址**: https://github.com/johanbrandhorst/eventsource-stream
- **固定版本**: v0.2.3 (crates.io)
- **开源许可证**: MIT License / Apache License 2.0
- **版权所有**: Copyright (c) 2020 Johan Brandhorst-Satzkorn
- **计划复用范围**: 计划在 M04 流式规整模块中引入，提供高效异步流式 Server-Sent Events (SSE) 事件帧解码。

---

## 第二部分：架构设计与算法参考源（仅作设计参考，不整体引入代码仓库） (Design & Architectural References Only)

### 1. TiyGate
- **仓库地址**: https://github.com/tiylabs/tiygate
- **参考 Commit**: `6db40e54b6ea083b33db237d8e87adbfb97cbbec`
- **开源许可证**: Apache License 2.0
- **版权所有**: Copyright (c) 2024-2026 TIY Labs
- **设计借鉴说明**: 借鉴其基于 Axum 0.7 的异步反向代理流水线、统一中间表示 (IR) 编解码思想与流式状态机；**坚决不引入其 SQLx 数据库与商业化计费模块**。

### 2. Tunnel Agent
- **仓库地址**: https://github.com/beyondhumane/tunnel-agent
- **参考 Commit**: `8cfbd02251a2dc810ddb32bf949aa2bf41e54823`
- **开源许可证**: MIT License
- **版权所有**: Copyright (c) 2024-2026 Beyond Humane & Villoh
- **设计借鉴说明**: 借鉴其多异构引擎统一调度交互体验、GitHub Release 资产正则匹配与测试用例；**坚决不引入其 C#/.NET 运行时与 Avalonia 框架**。

### 3. DeepSeek Harness (DSH)
- **仓库地址**: https://github.com/deepseek-ai/deepseek-harness
- **参考 Commit**: `d7432673d3ca24d35eb7b6f6580f4f72db1900be`
- **开源许可证**: MIT License
- **版权所有**: Copyright (c) 2025 DeepSeek AI
- **设计借鉴说明**: 借鉴其基于 Cordis 的微内核插件化哲学（Everything is a Plugin）、可逆副作用生命周期（Reversible Effects）与暗色工业风看板；**坚决不引入其重型 Node.js/Agent 运行时**。

### 4. workbuddy-switch-gateway (REF)
- **本地路径 / 来源**: `D:\workbuddy-switch-gateway\workbuddy-switch-gateway-latest` (v0.8.16, commit `ebd9427`)
- **开源许可证**: GPL-3.0 (历史)
- **设计借鉴说明**: 仅作为本地踩坑经验与算法逻辑参考（时间戳毫秒与秒换算、`0o600` 临时配置原子写入）；**由于 GPL 许可证传染风险，严禁直接复制或合并其任何源码文件**。
