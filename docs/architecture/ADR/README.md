# ProxyWeave Architectural Decision Records (ADR)

权威规划：[Plan.md](../../../Plan.md)。当前阶段：Phase 0 本轮 Windows 最小桌面基座已通过，F1 已独立 Verified；F2 六具体 DEF 已 Verified，F3 已独立 Verified；F4 桌面宿主已实现并验证，真实插件与最终原生桌面 MCP 验收仍开放。验收证据见 [PHASE0.md](../../verification/PHASE0.md)。
历史 ADR-01–ADR-07 是决策摘要，不是完成了全部模块审计或跨平台验证的证据。

## 决策索引

- [ADR-001：主脚手架采用与直接复用政策](../ADR-001-scaffold-and-reuse-policy.md)：先验证真实上游，再按闭包引入和分轮裁剪；直接依赖/提取优先，禁止等价重写冒充复用。替代历史 ADR-01/ADR-07 的失效链接。
- [ADR-002：Windows 本地插件契约与登记](../ADR-002-local-plugin-contract.md)：Feature 1 的直接依赖/来源比较、真实插件核验与最终 Inspector 原生桌面 MCP 完成门禁。
- [ADR-003：插件 ACL、Keyring 持久化与 Owned Runtime](../ADR-003-runtime-job-acl-and-lifecycle.md)：Feature 2 的 Windows Job Object 进程生命周期、真实 Windows 文件 DACL 隔离、Keyring 持久化、端口冲突确凿判定、wb2api 503 探针语义与 External 零 Kill 规则。
- [ADR-004：统一异步网关、本地 API Key 鉴权与 SSE 同协议流式透传](../ADR-004-unified-gateway-and-sse-passthrough.md)：Feature 3 的 Axum 0.8.9 / axum-reverse-proxy 2.2.0 直接依赖异步网关、本地 API Key 鉴权、真实模型发现与同协议透传、SSE 异常截断防伪造及取消传播。
- [ADR-005：极简桌面宿主、IPC 状态管理与 UI 闭环](../ADR-005-minimal-desktop-host.md)：Feature 4 的 Tauri IPC 胶水层设计、细粒度异步锁、密码零回读与安全清空、白名单生命周期事件、网关与插件联动、以及测试调用前端闭环。
- [协议复用边界（历史 ADR-02）](../../../Plan.md#3-协议与网关核心参考源tiygate-深度分析-tiygate-protocol-reference)：TiyGate 是候选，Phase 2 开发前核验实际模块、测试与依赖；不引入 SQLx。
- [黑盒进程与所有权（历史 ADR-03）](../../../Plan.md#65-插件双模运行架构managed-vs-external-provider)：Managed/External 由进程所有权决定，External 禁启停禁 Kill；独立进程不是安全沙箱。
- [生命周期（历史 ADR-04）](../../../Plan.md#66-插件生命周期状态机与四象限策略-lifecycle-state-machine)：活跃请求与冷启动互斥，仍为待实现设计。
- [SSE 正常终止证据（历史 ADR-05）](../../../Plan.md#68-sse-流式协议与终止符规整-streaming--termination)：异常截断绝不补发 `[DONE]`，HTTP EOF 本身不足以证明协议正常完成。
- [凭据边界（历史 ADR-06）](../../../Plan.md#91-系统安全模型与凭据隔离)：Keyring 与普通配置分离；Unix 权限与 Windows ACL 分别设计验证，Phase 0 不宣称凭据投影完成。

## 验证范围

Quotio 实际 Git HEAD：`21d75d08b38a23f97fbd534a1768a829cb147f2c`（`v0.7.7`）。
其平台封装并非全部通用；源码调用闭包、构建和 Windows 行为需分别登记。后续 Feature 按需完成前置审计，不以 M01–M15 历史全绿表格作为依据。
