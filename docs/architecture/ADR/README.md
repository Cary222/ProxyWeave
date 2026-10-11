# ProxyWeave Architectural Decision Records (ADR)

本文档归档 ProxyWeave 自立项以来的 7 项关键架构决策：

- **ADR-01**: 为什么选择成熟脚手架 (Quotio) 改造而非从零手写桌面外壳？
- **ADR-02**: 为什么借鉴 TiyGate 协议与路由设计而非全盘照搬商业网关？
- **ADR-03**: 为什么选择微内核黑盒独立进程，而非动态链接库 (DLL/so) 或 WebAssembly？
- **ADR-04**: 为什么废除纯 LRU 淘汰，改为四象限生命周期与 Singleflight 互斥锁？
- **ADR-05**: 为什么严禁在异常断开时盲目追加 `data: [DONE]`？
- **ADR-06**: 为什么采用系统级凭证库/分离式加密，而非明文 JSON 存储 Secret？
- **ADR-07**: 为什么将 Reference-First 确立为强制性工程研发纪律？

详细论证与分析见项目根目录 [Plan.md](../../Plan.md) 第 10 章节。
