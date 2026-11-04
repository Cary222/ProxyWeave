# ADR-004：同协议字节透传网关与请求生命周期

范围/状态：本 ADR 保留原 Feature 3 同协议透传决策；该历史 standalone 冻结后来已有独立 receipt，后续异协议复用不是此 ADR 原门禁的自动扩展。源码基线 `3471d077d657adc7e64826bac0a9d09f91d35347` 的唯一当前进度与未关闭项见 [PHASE1_REVIEW_STATUS](../verification/PHASE1_REVIEW_STATUS.md)，不得将旧“实施中”或局部 Verified 当全项结论。

## 目标与边界

首个本地 Managed 可用版本：loopback HTTP/1.1，本地 API Key，`GET /v1/models`、`POST /v1/chat/completions`。只透传 OpenAI Chat；没有协议 IR 转换、模型伪造、SQLx、外部服务生命周期或自动下载。F4 才将真实桌面 UI 接入；fixture 不是 wb2api 真实凭据 E2E 的替代。

## 直接复用与必要薄适配

- 直接依赖 `axum 0.8.9`、`axum-reverse-proxy 2.2.0`（关闭默认 rustls，采用 native-tls）。原包本地实际路径：`D:/temp-audit/proxyweave-feature3-proxy-audit/axum-reverse-proxy-2.2.0`；发布源码 commit `7350d118a69b6d98ae19a767b9d71630b2a3bc80`，MIT Copyright 2024 Tom Lubenow。包与源码不修改。
- **Axum 版本变更显式记录**：原 Plan 设计写 Axum 0.7；真实原发布包 `Cargo.toml` 的 `[dependencies.axum] version = "^0.8"` 是直接复用 2.2.0 所需。当前 `crates/proxyweave-gateway/Cargo.toml` 固定 `axum = "=0.8.9"` / 原代理 `=2.2.0`，Cargo.lock 匹配；选择兼容真实复用闭包，而非暗自保留 0.7 假前提或降低门禁。原包与宿主不同 patch 版本须分别测试，历史失败范围照实保留。
- 采用 `src/proxy.rs` 的 `ReverseProxy::new_with_client` / `proxy_request` / `ProxyPolicy` 和 `src/forward.rs` 的请求与响应 header 筛选、URI 拼接、响应头超时、直接 `Body::new(Incoming)`。请求/响应实体不重新序列化，保留上游状态码、Content-Type 与其他端到端 headers；hop-by-hop/framing 由原模块与 Hyper 处理。不会强设 SSE Content-Type。
- 使用自建 Hyper client 配置，不读取系统代理、不跟随重定向、禁用取消请求重试；构造前校验 endpoint，禁止 userinfo/query/fragment。
- 原代理模块会 trace 完整 URI/query 和 error chain。以官方 `tracing::WithSubscriber` 在数据面 future 上配置 NoSubscriber，同时清掉 response extensions 的 ProxyError；不把这些原始敏感诊断接入应用日志。仍不宣称插件 stdout/stderr 或所有日志全面安全。
- 官方 Tower HTTP `ValidateRequestHeaderLayer::custom` 及 Bearer 实现为鉴权来源：薄适配仅固定安全错误、敏感 HeaderValue、禁止重复 Authorization、避免原 Bearer Debug 回显。本地 Key 不能转发；只使用单独配置的上游 Key，并剥除客户端 Cookie/Proxy-Authorization。处理 Connection 所提名的全部字段后才加入上游凭据。
- `axum::serve` 官方连接循环 (`axum-0.8.9/src/serve/mod.rs:280-479`) 是 listener/TokioIo/TowerToHyperService 的来源。最小适配为 HTTP1 + JoinSet：拥有连接任务；关闭接入时取消并 drain 这些连接，不在无限 SSE 上永久等待 graceful shutdown。半关闭不作为继续消费服务的理由。取消必须由客户端原生 TCP 断连与上游 socket EOF/写失败证据验证，不凭 Drop 推断。
- 保留 F2 的同一 Tokio FIFO 生命周期锁：请求 admission 与显式 stop 不竞态。`RequestLease` RAII 原子计数覆盖连接、等待响应头、响应 body 全部帧，EOF/错误/Drop 释放。显式 Managed stop 有活跃请求时返回 `ActiveRequests`（不悄悄取消）；网关 shutdown 先取消/drain，再能停止 Managed。External stop 永远拒绝，绝不 Kill。
- TiyGate 实际源码 `D:/temp-audit/tiygate` commit `6db40e54663455fb99d675bd8985e4dc50f68a14`、Apache-2.0 已比较：server ingress 依赖 store/SQLx、IR/UsageAccumulator/StreamCapture；完整 SSE 管道有 `upstream_error && !upstream_terminal` 补 marker 分支。本初版不提取该管道，也不重新实现解析器；同协议无需解析 SSE。

## 后续提取范围（不覆盖原同协议门禁）

后来已将 TiyGate 的 `crates/core/src/ir/mod.rs`、`crates/core/src/protocol/{mod,lossy,structured_output}.rs`、routing 错误类型及 `crates/protocols/src/{chat_completions,messages,responses,error_body}.rs` 闭包提取到 `crates/proxyweave-protocols/`；仅三协议及适用原测试 subset，未整体引入 server/store/SQLx/计费数据库。真实来源、版权/适配、测试路径与原测试缺口见 [REFERENCE_REGISTRY](../references/REFERENCE_REGISTRY.md)。

当前网关同协议仍原 `Body` 字节/Content-Type/未知字段透传（安全 header 过滤除外），不在该路径插入 parser 或合成完成；异协议才走 TiyGate IR/codecs + 官方 `eventsource-stream` 0.2.3。旧自建解析器存在过不等于当前接入实现；本次核查实际 `.eventsource()` 调用与 Cargo/lock。

异协议只有源协议正常完成证据才能编码目标终止；HTTP 200/clean EOF 不证明完成，异常 EOF/解析或协议错误绝不补成功。能力损失门禁仅针对源 IR 能表达的维度，不宣称任意扩展无损。历史独立 Messages ingress → Chat/Responses backend 工具关联与 usage 四失败未关闭；当前未重跑。原 F3 standalone receipt 不会自动覆盖此后来数据面，具体证据只引用当前状态汇总。
## 终止与错误真实性

所有收到的上游 bytes 原样转发，包括上游自己的 `[DONE]`。本实现从不合成结束标记，也不解析 finish_reason。异常 transport body error 保留错误并中止，header deadline 是安全 504；上游已返回的 4xx/5xx body 原样保留。完整 HTTP framing 但无 `[DONE]` 的 SSE EOF 仍仅透传 EOF，不保证所有 SDK 会报告失败，也不将其称为业务正常完成。

## 验证计划

1. 原包 local lib/header/rfc9110/streaming/policy/routing/service/error tests，保留真实失败与限定通过范围，不改源码或断言。
2. 实际 Owned fixture：不同本地/上游 Key、missing/wrong/duplicate auth 无上游请求；真实模型 JSON、未知请求字段、query、response bytes/header/status 一致。
3. 分裂 UTF-8/CRLF/SSE event、合法 DONE、Content-Length 截断与正常 HTTP EOF 无 DONE。
4. 原生 TCP header-wait 断连 / streaming 断连，上游 socket 与 lease 归零；header timeout/连接拒绝/并发请求/显式 stop/有界 gateway shutdown/端口冲突。
5. 保留六 DEF 的默认并行 Rust 回归和 External 零 Kill；fmt、scoped Clippy、frontend tests/build、embedded Tauri build。独立冻结后由 Inspector 复验。

实现和测试尚未构成真实 wb2api 登录或最终 Windows 桌面可用验收；日志安全、跨平台权限、HTTP2、release installer 不据此扩大。
