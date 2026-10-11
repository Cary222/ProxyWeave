# ProxyWeave Reference Source Registry (参考源档案库)

> **版本**：v2.0.0 (Strict Audit & Feature SOP Edition)  
> **核心政策**：成熟实现直接复用优先（Direct Reuse First）。非平凡系统机制（进程组清理、端口防竞态分配、流式解码、协议转换、凭据隔离等）编码前必须在此建立档案。  
> **状态标注规范（严禁无证据全标 Verified）**：
> - 🟢 **源码已核验 (Source Inspected)**：已在本地拉取指定 Commit/源码，人工逐行审计其实现逻辑、错误恢复与依赖树，确认契合度与许可证。
> - 🟡 **行为已验证 (Behavior Verified)**：已在本地运行真实二进制或服务，黑盒验证其 CLI、HTTP 探针或交互行为（尚未完成源码审计或无公开源码）。
> - ⚪ **待核验 (Pending Verification)**：规划中的候选库或模块，尚未锁定具体 Commit 或尚未完成深度源码与依赖审计，编码前必须先执行审计。

---

## 1. 核心模块与参考源对照矩阵 (M01 ~ M15)

| 模块编号 | 模块名称与职责 | 选定/候选参考源 | 仓库与具体版本/Commit | 精确源码路径与组件 | 最小依赖与许可证依据 | 验证状态 | 采用决策与复用/适配范围 |
|---|---|---|---|---|---|---|---|
| **M01** | **Process Supervisor**<br>子进程生命周期、系统进程组绑定、级联回收与互斥锁 | **shared-child**<br>+ **Quotio-Platform** | `oconnor663/shared-child.rs` (v1.0.1)<br>`xiaocoss/quotio-desktop` (Commit `21d75d08`) | `shared-child/src/sys/windows.rs` (`SharedChild` 跨平台同步)<br>`quotio-platform/src/lib.rs` (`execute_command`) | `libc`, `winapi`<br>MIT / Apache-2.0 (已核验 LICENSE 文件) | 🟢 **源码已核验** | **提取模块复用**。提取 shared-child 跨平台子进程并发等待与 Win32 Job Object (`KILL_ON_JOB_CLOSE`) 配置逻辑；外部封装薄适配层注入 Singleflight 启动互斥锁与状态机转换。 |
| **M02** | **Port Allocator**<br>本地回环动态端口初筛与冲突重试恢复 | **portpicker-rs**<br>+ **REF 经验** | `Cargo-Prebuilt/portpicker-rs` (v0.1.1)<br>本地 REF 库 | `portpicker-rs/src/lib.rs` (`pick_unused_port` 循环绑定算法) | 无外部重型依赖<br>MIT / Apache-2.0 (已核验) | 🟢 **源码已核验** | **直接依赖 + 薄适配**。引入 portpicker 作为初筛；明确其存在 TOCTOU 竞态窗口，核心可靠性由外层“捕获子进程启动端口冲突并自动递增重试 (Collision Retry Loop)”提供保障。 |
| **M03** | **Async HTTP Gateway**<br>异步 HTTP/SSE 反向代理、连接池与取消传播 | **TiyGate Server**<br>+ **Axum / Hyper** | `tiylabs/tiygate` (Commit `6db40e54`)<br>`tokio-rs/axum` (v0.7.7) | `tiygate/crates/server/src/app.rs`<br>`crates/server/src/routes/` | `axum`, `hyper`, `tokio`<br>Apache-2.0 / MIT (已核验) | 🟢 **源码已核验** | **架构与流式流水线借鉴**。彻底淘汰 Quotio 同步阻塞的 `bridge.rs` (`tiny_http`)；提取 TiyGate 基于 Axum 0.7 的异步反向代理流水线与 CancellationToken 取消传播机制；剥离其 SQLx 数据库。 |
| **M04** | **SSE Normalizer & Stream**<br>流式事件解析、分块状态机与终止符规整 | **TiyGate Protocols**<br>+ **eventsource-stream** | `tiylabs/tiygate` (Commit `6db40e54`)<br>`eventsource-stream` (v0.2.3) | `tiygate/crates/protocols/src/chat_completions.rs` (`process_sse_chunk`) | `bytes`, `futures-core`<br>Apache-2.0 / MIT (已核验) | 🟢 **源码已核验** | **提取模块复用**。提取 TiyGate 状态机级 SSE 解析器；补发终止符必须有协议层正常完成证据（HTTP 200 + 包含 finish_reason 的数据帧），严禁因底层 HTTP EOF 盲目补发。 |
| **M05** | **Protocol Codec (IR)**<br>Chat/Messages/Responses 协议双向结构化转换与降级映射 | **TiyGate Protocols** | `tiylabs/tiygate` (Commit `6db40e54`) | `tiygate/crates/protocols/src/`<br>`tiygate/crates/core/src/ir/` | `serde`, `serde_json`<br>Apache-2.0 (已核验) | 🟢 **源码已核验** | **提取模块与 IR 复用**。提取 `chat_completions.rs`, `messages.rs`, `responses.rs` 与统一 `IR` 结构体；保留 Tool Call 碎片拼装逻辑；增加协议兼容性降级说明。 |
| **M06** | **Model Router**<br>虚拟模型别名映射与基于健康的 Fallback 路由 | **LiteLLM Router**<br>+ **TiyGate Routing** | `BerriAI/litellm` (v1.60+)<br>`tiylabs/tiygate` (Commit `6db40e54`) | `litellm/router.py` (设计思想)<br>`tiygate/crates/core/src/routing/` | 内存 DashMap<br>MIT / Apache-2.0 | ⚪ **待核验** | **设计参考后轻量自研**。仅借鉴其虚拟模型别名与失败降级分流算法；因原项目为 Python，在 Rust 侧基于 `dashmap` 编写极简无锁路由表，不引入 Python 运行时。 |
| **M07** | **Credential Broker**<br>系统密钥库托管与沙盒目录 0o600 凭据投影 | **Quotio-Platform**<br>+ **keyring-rs**<br>+ **REF 经验** | `xiaocoss/quotio-desktop` (Commit `21d75d08`)<br>`hwchen/keyring-rs` (v3.6.2) | `quotio-platform/src/lib.rs` (`keyring` 封装)<br>REF 库原子配置写入 | `keyring` (v3.6.2)<br>MIT / Apache-2.0 (已核验) | 🟢 **源码已核验** | **模块提取与复用**。完整复用 quotio-platform 的系统级 Keyring 托管封装；结合 REF 库的 `.tmp -> fsync -> rename` 原子覆盖与 `0o600` 文件权限保护。 |
| **M08** | **Plugin Installer**<br>GitHub Release 探测、SHA256 完整性校验与安全解压 | **Quotio Download**<br>+ **Tunnel Agent** | `xiaocoss/quotio-desktop` (Commit `21d75d08`)<br>`beyondhumane/tunnel-agent` (Commit `8cfbd02`) | `quotio-core/src/proxy_download.rs` | `reqwest`, `sha2`, `zip`, `flate2`<br>MIT (已核验) | 🟢 **源码已核验** | **提取模块泛化**。泛化 Quotio 的下载器（剥离其写死的 CPA URL），改为接收 Manifest 的 Repository 与 Regex 规则；包含进度回调与解压回滚机制。 |
| **M09** | **Config Store**<br>无数据库单文件原子持久化与双副本容灾 | **Quotio AppConfig**<br>+ **serde_json** | `xiaocoss/quotio-desktop` (Commit `21d75d08`) | `quotio-platform/src/lib.rs` (`AppConfig` 读写)<br>`serde_json` | `parking_lot`, `serde_json`<br>MIT (已核验) | 🟢 **源码已核验** | **模块提取复用**。提取基于纯 JSON 的单文件配置读写逻辑，切断其 SQLite 依赖；增加内存 `RwLock` 与磁盘 `.bak` 双副本故障回滚保护。 |
| **M10** | **Manifest Engine**<br>插件 Schema 1.0 声明式解析与语义校验 | **serde_yaml** | `dtolnay/serde-yaml` (v0.9.34) | 官方标准 YAML 反序列化机制 | `serde`, `serde_yaml`<br>MIT / Apache-2.0 (已核验) | 🟢 **源码已核验** | **直接依赖 + 薄校验层**。直接引入 serde_yaml，编写 Rust 类型校验器，在首个 Managed 插件开发前完成 Schema 1.0 的数据契约固化。 |
| **M11** | **Lifecycle Manager**<br>四象限状态机、活跃请求引用计数与空闲回收 | **DeepSeek Harness**<br>+ **Cordis** (思想) | `deepseek-ai/deepseek-harness` (Commit `d7432673`) | `packages/core/` (Reversible Effects 契约) | 状态机 Rust 实现<br>MIT (已核验) | ⚪ **待核验** | **设计参考**。借鉴其可逆副作用（退出自动注销路由、解绑端口、清理临时配置）思想；Rust 侧基于 `AtomicUsize` 与 RAII Guard 严谨实现状态转换表。 |
| **M12** | **IDE Handoff Engine**<br>Cursor/Claude Code 等开发工具配置自动接入 | **Quotio Agent Config**<br>+ **Tunnel Agent** | `xiaocoss/quotio-desktop` (Commit `21d75d08`) | `quotio-core/src/agent_config.rs` (54KB 源码) | `serde_json`, `dirs`<br>MIT (已核验) | 🟢 **源码已核验** | **完整提取复用**。提取 quotio-core 中经过实战检验的各 IDE 配置文件路径定位与 JSON/YAML 注入算法；仅修改目标端点指向 ProxyWeave 统一入口。 |
| **M13** | **Desktop Shell**<br>Tauri v2 桌面外壳、系统托盘、自启动与单实例 | **Quotio Desktop Shell** | `xiaocoss/quotio-desktop` (Commit `21d75d08`) | `apps/desktop/src-tauri/` | Tauri v2 官方生态插件<br>MIT (已核验) | 🟢 **源码已核验** | **主脚手架基线保留**。作为首选主脚手架，直接保留其经过验证的 Tauri 2.0 托盘菜单、窗口隐藏与单实例锁，避免因重命名而无谓重写。 |
| **M14** | **API Doctor**<br>全链路脱敏追踪、智能故障归因与报告导出 | **Tower-HTTP Trace**<br>+ 自研流水线 | `tower-rs/tower-http` (v0.5.2) | `tower-http/src/trace/` | `tower`, `tracing`<br>MIT (已核验) | ⚪ **待核验** | **模块引入与适配**。引入 Tower TraceLayer 捕获 HTTP 阶段耗时；外加脱敏过滤器（屏蔽 Authorization、Cookie）与 401/429/502 错误归因分析器。 |
| **M15** | **Compatibility Flight**<br>多协议、流式首字延迟与 Tool Call 自动化质检 | **DeepSeek Harness UI**<br>+ 自研自动化质检套件 | `deepseek-ai/deepseek-harness` (Commit `d7432673`) | `packages/client/` (测试看板交互) | 客户端测试用例<br>MIT (已核验) | ⚪ **待核验** | **交互参考与用例构建**。借鉴其测试面板布局；基于 Rust/TypeScript 编写 6 项端到端质检自动化测试用例。 |

---

## 2. 本地黑盒与验证项目核验记录

| 项目代号 | 本地路径 / 来源 | 项目类型 | 许可证 | 验证状态 | 核验结论与接入规范 |
|---|---|---|---|---|---|
| **GW (wb2api)** | `D:\workbuddy2api-panel` | Go 源码工程 (v1.13.0, commit `8461419`) | MIT (已核验) | 🟢 **源码已核验**<br>🟡 **行为已验证** | 已完成本地编译 (`wb2api.exe` 10.3MB)，已在本地通过 `GET /healthz` 与 Web 面板验证。作为 Phase 1 首发 Managed 插件目标。 |
| **CPA** | `D:\CLIProxyAPI_7.2.115_windows_amd64` | 纯外部可执行二进制 (v7.2.115) | 专有/未公开 | 🟡 **行为已验证**<br>（无公开源码） | 仅在本地运行验证过 28317 端口服务。**明确按 External Provider 公开 HTTP 接口接入，严禁假称复用其内部源码**。 |
| **REF** | `D:\workbuddy-switch-gateway\workbuddy-switch-gateway-latest` | 方案一历史参考库 (v0.8.16, commit `ebd9427`) | GPLv3 (历史) | 🟢 **源码已核验**<br>（仅参考算法） | 核验了其毫秒/秒换算逻辑与原子写模板。**由于许可证传染风险，严禁直接复制代码文件，仅作为避坑经验与算法逻辑参考**。 |
| **DATA** | `C:\Users\Cary\.wb-switch\accounts.json` | 本地真实账号凭据文件 | 私有数据 | 🟡 **行为已验证** | 本地真实联调资产（27 个账号数据），严禁提交至 Git 仓库。 |
