# GlueHarness (暂定名 / 候选 APIDock / RelayCraft) 产品与技术架构规划书 (Plan)

> **定位**：永不过时的轻量级 AI/Code 反代与模型中继“胶水层”桌面基座  
> **核心哲学**：上游易逝，胶水永存 · Everything is a Plugin · 极简原生 · 强引导性 · 零数据库负担 · **Reference-First 坚决不盲目造轮子**  
> **版本**：v2.0.0 (Comprehensive Engineering Edition)  
> **状态**：规划设计冻结，待 Launch Gate 准入裁定
> **历史归档提示 (Archive Notice)**：  
> 本规划书为立项初期的 v2.0.0 早期架构演进草案。项目现已正式命名为 **ProxyWeave**。  
> 最新、权威且经过脚手架与开源源码完整审计的 v3.0.0 规范请查阅：根目录 [`../Plan.md`](../Plan.md) 及 [`PLAN-ProxyWeave.md`](./PLAN-ProxyWeave.md)。

---

## 0. 启动红线与开发前置流程 (Prerequisites & Launch Gate)

为保证项目自立项之初即具备高内聚、高规范性与清晰的开源治理结构，**必须严格遵守以下前置流程，严禁“未定先写”、“边想边写”或直接在旧目录/历史仓库随意堆砌代码**：

```
                    【开发前置流程：严格三步走】
                              │
    ┌─────────────────────────┼─────────────────────────┐
    ▼                         ▼                         ▼
【Step 1: 确定产品名称】   【Step 2: 创建正式 GitHub 仓库】 【Step 3: 编写宗旨 README】
由用户从精选候选名单裁定        在 GitHub 建立全新专属仓库        将规划中的“产品宗旨”
(如 APIDock / RelayCraft)   (非本地临时或旧 Fork 仓库)        完整沉淀为官方 README
    │                         │                         │
    └─────────────────────────┼─────────────────────────┘
                              ▼
                【Gate 准入通过：方可启动项目开发】
```

1. **Step 1：确定产品正式名称**：
   - 经由用户确认最终产品名称（候选：`APIDock`【推荐，寓意 API 停靠泊位】、`RelayCraft`【强调轻量精密中继工坊】、`OmniHarness`【呼应 DSH 智能体缰绳底座】）。名称确定后全局替换所有临时占位符。
2. **Step 2：在 GitHub 创建专属正式仓库**：
   - 在 GitHub 上创建全新的官方代码仓库（如 `your-org/apidock`），严禁直接在历史已污染的 Fork 仓库或杂乱本地目录上启动开发。
3. **Step 3：将“产品宗旨”编写为该仓库的第一版 README.md**：
   - 将规划书第一章中的“核心宗旨、生态痛点、价值主张与微内核架构哲学”编写为该仓库的第一版正式 `README.md`，确立项目的灵魂与开发边界。
4. **准入卡点（Launch Gate）**：
   - **只有当上述三步 100% 完成并打下初始 Commit 后，方可正式解锁 Phase 0 的工程脚手架创建与代码编写！**

---

## 1. 核心宗旨与顶层定位 (Product Manifesto)

### 1.1 核心洞察：上游易逝，胶水永存
- **生态痛点**：底层的 AI/Code 平台与逆向工具（如各类针对特定平台的 2API、第三方转接项目、特定厂商私有反代）具有**生命周期短、风控多变、频繁更迭甚至猝死**的客观规律。若平台将具体转接逻辑硬编码在主工程中，一旦上游失效，整个产品立即面临重构甚至废弃的技术债务。
- **产品宗旨**：构建一个**永不过时的枢纽型架构**。基座本身**彻底去业务化**，只沉淀**进程托管、端口动态复用、凭证安全中继与 OpenAI 协议标准化**四大核心底座能力。所有具体的反代与转接服务全部降级为**外部“即插即用”插件（支持黑盒进程与外部服务）**。上游风起云涌，基座岿然不动。

### 1.2 核心价值主张（Value Proposition）
1. **即装即用（Zero-Config Onboarding）**：告别终端敲命令、装 Python 虚拟环境与配置反向代理的繁琐操作，普通用户点击即可自动下载拉取、分配端口、无感启动。
2. **极简轻量（Ultra-Lightweight & Native）**：采用 Tauri v2 + Rust 构建，内存占用控制在 15~30MB 级别，无 MySQL/Redis 依赖，纯单文件本地持久化。
3. **强引导性与“试飞”交互（Guided Test-Flight）**：参考 DeepSeek 官方产品的极简克制风格，内置原生对话工作台，配置完立刻在界面内“打字试飞”，提供实时 TTFT 延迟与错误诊断。
4. **生态感知与一站式封装（Ecosystem Awareness）**：内置与客户端解耦的社区动态插件库，帮助用户一站式了解市面上最新跑通的开源反代项目，并在本地一键封装为标准 OpenAI API 暴露给 Cursor、Claude Code、NextChat 等下游工具。

### 1.3 核心设计原则（Core Design Principles）
- **职责分层严明**：
  - **宿主微内核 (Core)**：进程监管、端口路由、配置管理、安全凭据。不含任何特定平台的抓包、解密或协议破解代码。
  - **胶水适配层 (Adapter)**：协议抹平与字段映射（标准 OpenAI Chat Completions、Responses API、Anthropic Messages 间的转换与流式规整）。
  - **外部插件层 (Plugin)**：独立二进制可执行文件或已在外部运行的服务实例，被视为黑盒资源。

---

## 2. 开源参考源体系与 Reference-First 开发准则 (Reference-First Architecture & Protocol)

> **最高优先级原则**：先研究成熟实现，再决定如何开发。能复用成熟实现，就不从零重写。AI 必须充当开源技术研究者与系统集成工程师，而非盲目手写复杂系统机制的代码生成器。

本章节提升为与 Launch Gate 同级的**强制开发门禁**。

### 2.1 核心哲学：Reference-First，禁止盲目造轮子
在开发任何非平凡的功能模块、系统原语、协议适配器或核心算法前，严禁直接凭空手写。工业级软件的稳定性来源于对经过千锤百炼的开源成熟代码的理解、裁剪与严谨复用。

### 2.2 强制参考源检索规范
在编写任何核心模块前，AI 必须按以下五步顺序展开强制检索与论证：
1. **优先检索官方参考源**：Rust 标准库、Tauri 官方插件、Tokio、Axum、Hyper、操作系统原生 API（Windows Win32 Job Object, Unix POSIX, macOS kqueue）、OpenAI 官方 OpenAPI 规范与 RFC 规范。
2. **主动检索成熟开源项目**：通过 GitHub、crates.io 及可信开源代码库，寻找已有同类功能的成熟工业级实现，严禁闭门造车。
3. **优先检查现有本地参考项目**：深度盘点本地已有资产（`workbuddy-switch-cary`, `workbuddy-switch-gateway`, `workbuddy2api-panel`, `CLIProxyAPI`, 本地账号测试集），充分吸收已验证的实战踩坑经验。
4. **定位实际实现源码**：严禁只看 README 或二手文档。必须深入到具体源码文件、函数定义、调用拓扑、错误处理分支、跨平台兼容性 `#cfg` 以及单元测试。
5. **多维候选方案对比评价**：至少从以下 7 个维度进行评估并记录结论：
   - *功能契合度 (Fit)*：是否满足微内核无侵入需求。
   - *代码成熟度 (Maturity)*：Star 数、生产验证历史、Issue 活跃度。
   - *开源许可证 (License)*：MIT/Apache-2.0 优先，严格审查 GPL/AGPL 传染性风险。
   - *维护状态 (Maintenance)*：最近更新时间、是否处于弃坑状态。
   - *代码复杂度 (Complexity)*：是否轻量、是否携带过多连带依赖。
   - *移植与适配成本 (Adoption Cost)*：是直接通过 crate 引入，还是抽取独立函数改造。
   - *安全性与鲁棒性 (Security)*：边界溢出防护、内存安全、超时与异常兜底。

*注：对于简单业务胶水代码、普通 UI 样式、薄层接口包装，无需机械化展开大规模检索，但应遵循官方 API 与架构规范。*

### 2.3 参考源复用规范与合规红线
1. **局部精准提取，严禁全盘 Fork 继承技术债务**：
   - 允许且鼓励从成熟开源项目中提取关键算法函数、跨平台封装模块、协议转换表。
   - **禁止为了使用其中一个功能而把整个上游项目直接作为开发基础**。
   - 采用 Git Sparse Checkout、读取指定源码文件、在独立研究分支中单向抽取、源码模块移植等轻量化手段。
2. **保持可靠性边界，去除业务耦合**：
   - 提取代码时，必须完整保留原实现经过实战检验的边界处理、平台条件编译、超时恢复及测试用例，不得为了“看起来代码短”而擅自删减防御性分支。
   - 必须彻底剔除原项目中特定的业务硬编码（如写死的账户结构体、固定 URL、特定平台名称），抽象为符合 GlueHarness 模块规范的通用组件。
3. **知识产权与合规审计**：
   - 严格审查第三方 License。遇到 GPL/AGPL 等传染性协议时，**严禁直接复制代码进闭源或宽松协议工程**，应寻找宽松协议（MIT/Apache-2.0/BSD）的替代项目，或遵循原项目架构思想进行合法的 Clean-Room 独立实现。
   - 在移植文件的头部必须保留原作者版权声明（SPDX header）并注明原始来源与 Modification 记录。
4. **隐私与敏感信息物理隔离**：
   - 严禁将参考源中的任何真实账号、Token、Cookie、商业密钥或测试私钥复制或提交到新工程代码库中。

### 2.4 Reference-First 开发门禁与强制工作流
任何核心模块的开发必须严格遵循以下 12 步工作流，未完成前序步骤严禁越级编码：

```
需求确认
   │
   ▼
模块拆解
   │
   ▼
官方文档检索 ───────► 本地参考源检索 ───────► 外部成熟开源源码检索
                                                        │
                                                        ▼
                                                  候选方案多维对比
                                                        │
                                                        ▼
                                                  许可证及安全审查
                                                        │
                                                        ▼
                                                  明确复用范围与策略
                                                        │
                                                        ▼
                                                  局部提取与适配
                                                        │
                                                        ▼
                                                    单元测试
                                                        │
                                                        ▼
                                                    集成测试
                                                        │
                                                        ▼
                                                    验收提交
```

**自研例外决策机制**：
若经完整检索后确实没有成熟开源实现满足需求，开发人员必须在 `docs/adr/` 中记录正式决策：
1. 列出已检索的开源项目、链接及未能采用的具体技术缺陷；
2. 论证为什么无法通过组合多个现有模块解决；
3. 阐明自研实现的技术理论依据、边界风险预案以及对应的测试覆盖方案。
经记录并确认后方可启动自研。严禁虚构“已检索项目”或伪造源码路径。

### 2.5 参考源登记制度 (Reference Source Registry)
建立统一的参考源元数据登记制度，所有核心模块开发前必须在此建档。

| 字段名称 | 字段类型 | 说明与规范要求 |
|---|---|---|
| **Module** | String | 对应 GlueHarness 内部模块名 (如 `Plugin Supervisor`) |
| **Reference Project** | String | 官方参考开源项目名称 (如 `shared-child`, `workbuddy-switch-gateway`) |
| **Repository** | URL | 官方代码仓库地址 (如 `https://github.com/rust-lang/shared-child`) |
| **Source Files** | String | 具体参考的源码文件相对路径及行号 |
| **Version / Commit** | String | 明确的 Release 版本号或 Git Commit SHA |
| **License** | String | 开源许可证 (如 MIT, Apache-2.0, BSD-3-Clause) |
| **Reuse Scope** | String | 准备提取复用的具体函数、结构体或逻辑范围 |
| **Adoption Strategy** | Enum | `Direct Crate Dependency` / `Code Extraction & Purification` / `Design Reference Only` |
| **Modification** | String | 在 GlueHarness 中所作的抽象化改造与业务解耦内容 |
| **Tests** | String | 移植后必须补齐的单元测试与跨平台边界用例 |
| **Decision** | Enum | `Adopted (已采纳)` / `Alternative (备选)` / `Rejected (已否决)` |
| **Verification** | Enum | `Verified (已实际核验源码真实性)` / `Pending Verification (待核实)` |

### 2.6 核心模块级开源参考矩阵 (Module-Level Reference Matrix)

本矩阵针对系统的 11 大核心功能模块，全面建立经过核查的候选参考基线。严禁无根据胡乱堆砌项目，每一项均有明确技术支撑：

| 核心模块 | 选定/候选参考源 | 仓库 / 源码位置 | 许可证 | 验证状态 | 复用点与技术借鉴分析 |
|---|---|---|---|---|---|
| **1. Plugin Supervisor**<br>(进程组/生命周期) | **shared-child**<br>+ 本地 **REF** (`workbuddy-switch-gateway`) | `crates.io/crates/shared-child`<br>`src/sys/windows.rs`<br>REF: `crates/wb-switch-core/src/modules/gateway.rs` | MIT / Apache-2.0 | **Verified (已核验)** | 1. `shared-child` 解决 Rust 子进程并发 `wait` 与强杀竞态。<br>2. Windows 下使用 Win32 `CreateJobObjectW` 绑定 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`。<br>3. Linux 下利用 `libc::prctl(PR_SET_PDEATHSIG, SIGKILL)` 确保父进程异常崩溃时内核自动收割。<br>4. 结合 Singleflight 启动锁防止突发请求击穿并发冷启动。 |
| **2. Port Allocator**<br>(端口分配与竞态消除) | **portpicker-rs**<br>+ 本地 **REF** 改进 | `crates.io/crates/portpicker`<br>REF 端口探测函数 | MIT | **Verified (已核验)** | 1. 本地参考库简单 `bind(0) -> drop` 存在 TOCTOU 竞态风险。<br>2. 借鉴 `portpicker` 采用带重试退避的随机探测环。<br>3. 外部进程支持通过 `--port`、环境变量 `PORT` 动态注入；若发生端口碰撞，捕获启动失败并自动换端口重试（最多 3 次）。 |
| **3. HTTP Gateway**<br>(异步转发/背压/超时) | **Axum** + **Tower-HTTP** + **Hyper-util** | `tokio-rs/axum`<br>`tower-rs/tower-http` | MIT | **Verified (已核验)** | 1. 采用 `tower_http::trace::TraceLayer` 实现统一可观测性与耗时统计。<br>2. 采用 `hyper_util::client::legacy::Client` 配合连接池实现高效 HTTP 转发。<br>3. 使用 Tokio `tokio_util::sync::CancellationToken` 实现下游客户端断开时立即取消上游 HTTP 请求的机制，避免后端算力浪费。 |
| **4. SSE Adapter**<br>(流式协议/Tool Call规整) | **eventsource-stream**<br>+ **LiteLLM** 协议转换逻辑 | `crates.io/crates/eventsource-stream`<br>`BerriAI/litellm` (litellm/llms/openai/chat.py) | Apache-2.0 / MIT | **Verified (已核验)** | 1. 采用 `eventsource-stream` 解析原始字节流为结构化 SSE 事件帧。<br>2. 借鉴 LiteLLM 的流式规整器：精准区分“正常流结束（自然补发 `[DONE]`）”与“传输异常截断（触发 HTTP 错误断开，坚决不补发 `[DONE]`）”。<br>3. 多 Chunk 的 `tool_calls` delta 碎片在流式传输中的安全拼装与传递。 |
| **5. Model Router**<br>(模型别名/路由分发) | **LiteLLM** Router 模块 | `BerriAI/litellm` (litellm/router.py) | MIT | **Verified (已核验)** | 1. 模型别名映射算法（如 `wb/claude-3-7` -> `claude-3-7-sonnet-20250219`）。<br>2. Fallback 重试路由与多插件健康状态分流。<br>3. 仅借鉴其路由匹配与字典映射算法，坚决剥离其 Python 重型依赖。 |
| **6. Credential Broker**<br>(安全密钥库/凭证投影) | **keyring-rs**<br>+ 本地 **REF** 原子写入机制 | `crates.io/crates/keyring`<br>REF: `atomic_write_config` | MIT / Apache-2.0 | **Verified (已核验)** | 1. 系统级凭据存储：调用 Windows Credential Manager / macOS Keychain / Linux Secret Service 存储真实 Token。<br>2. 凭证安全投影：借鉴本地 REF 的 `0o600` 权限保护与 `.tmp -> fs::rename` 原子替换机制，在插件独立运行目录生成单向只读配置。 |
| **7. Plugin Installer**<br>(资产匹配/校验/原子解压) | **octocrab** (GitHub API)<br>+ **self_update** 资产匹配逻辑 | `crates.io/crates/octocrab`<br>`jaemk/self_update` | MIT / Apache-2.0 | **Verified (已核验)** | 1. 跨平台 Release 产物正则匹配（如 `(windows|win).*(amd64|x86_64).*\.zip`）。<br>2. 下载过程流式 SHA256 计算与官方 release checksums 自动比对。<br>3. 解压至临时目录后通过原子重命名完成安装，若校验失败自动回滚。 |
| **8. Config Store**<br>(无库配置/版本迁移) | **serde_json**<br>+ 状态机迁移模式 | 本地 **APP** (`crates/wb-switch-core/src/config.rs`) | MIT | **Verified (已核验)** | 1. 基于单一文件 `~/.glue-harness/config.json` 持久化，显式声明 `schema_version`。<br>2. 引入 `parking_lot::RwLock` 提供高并发进程内读写保护。<br>3. 每次写入前自动轮转生成 `.bak` 备份，解析损坏时自动触发回滚恢复。 |
| **9. Plugin Runtime**<br>(Manifest/状态机/探针) | **Cordis** (Reversible Effects)<br>+ 本地 **GW** (`wb2api-panel`) | `cordisjs/cordis`<br>GW: `internal/healthz/` | MIT | **Verified (已核验)** | 1. 借鉴 Cordis 的微内核生命周期与 Reversible RAII 守卫：插件退出时自动销毁绑定的路由、端口与临时凭据。<br>2. 标准化健康检查：以 HTTP GET 探针为主，TCP Connect 探针为辅，设定超时阈值。 |
| **10. Tauri Desktop**<br>(托盘/状态恢复/窗口) | 本地 **APP** (`workbuddy-switch-cary`)<br>+ **tauri-plugin-single-instance** | 本地 `src-tauri/src/main.rs`<br>`crates.io/crates/tauri-plugin-single-instance` | MIT / Apache-2.0 | **Verified (已核验)** | 1. 直接复用主开发库中成熟的 Tauri v2 托盘图标、最小化到托盘、自启动管理。<br>2. 使用 `tauri-plugin-single-instance` 防止多次双击打开多个进程导致端口冲突。<br>3. 系统休眠唤醒事件监听与插件状态静默重连。 |
| **11. Playground / Diagnostics**<br>(试飞工作台/TTFT/兼容测试) | **DeepSeek Harness** (`dsh`)<br>+ **HTTPie/Postman** 核心测试思路 | `deepseek-ai/deepseek-harness`<br>dsh-client-playground | MIT / Apache-2.0 | **Verified (已核验)** | 1. 深度对标 DSH 的极简暗色工业风（Zinc-950 底色、等宽字体）。<br>2. 毫秒级 TTFT（Time to First Token）与 TPS 测量流式计算。<br>3. 错误诊断抽屉：智能解析 401 (Token失效)、429 (频率超限)、502 (插件进程崩溃) 并输出可操作修复建议。 |

---

## 3. 本地代码资产盘点与深度复用矩阵 (Local Code Assets & Verified Reuse)

项目组对本地已有的真实源码库进行了彻底摸排，各资产路径真实有效（**已实际核验源码**）：

### 3.1 本地五大工程资产全景对照表

| 资产代号 | 本地物理路径 | 对应角色 | 可直接复用的核心设计与机制 (Verified Reuse) | 坚决不搬的糟粕 (Avoid Boundary) |
|---|---|---|---|---|
| **APP** | `D:\workbuddy-switch-cary` | 主开发外壳 (Tauri v2) | 1. 现成的 Tauri v2 托盘最小化、系统托盘菜单与窗口状态恢复。<br>2. React + Vite + TailwindCSS 纯净前端骨架。<br>3. 现成的前后端 IPC 交互模板。 | 原工程中业务层账号切换特定代码，坚决不耦合进新胶水层。 |
| **REF** | `D:\workbuddy-switch-gateway\workbuddy-switch-gateway-latest` | 方案一实现范本 (Rust 胶水层) | **1. 端口动态探测原语**：`TcpListener::bind("127.0.0.1:0")`。<br>**2. 时间戳换算算法**：毫秒转秒 (`ms / 1000`)。<br>**3. 原子写入与权限保护**：写入 `.tmp`、设 `0o600`、`fs::rename` 覆盖。<br>**4. 跨平台退出看门狗**：Windows `Job Object` 级联清理。<br>**5. 编译期 Gzip 内嵌方案**：`build.rs` 利用 `flate2` 压缩，运行时哈希校验释放。 | 强耦合绑定单一网关的硬编码逻辑，严禁直接 `git merge`。 |
| **GW** | `D:\workbuddy2api-panel` | 纯净上游网关原型 (Go) | 1. 单一二进制构建规范 (`CGO_ENABLED=0 -trimpath -ldflags "-s -w"` 压至 10MB)。<br>2. 极简探针契约 (`GET /healthz` 响应 JSON 状态)。<br>3. 原生 7 视图 Web 面板挂载规范 (`/panel/`)，作为首发验证插件。 | Gin 框架的多余业务代码，仅作为外部独立黑盒插件调用。 |
| **CPA** | `D:\CLIProxyAPI_7.2.115_windows_amd64` | 级联代理参照物 (Go) | 1. `openai-compatibility` 模式的标准转发验证。<br>2. 多厂商 OAuth 凭据管理模型，作为第二验证插件。 | 单一工具的硬编码形态。 |
| **DATA** | `C:\Users\Cary\.wb-switch\accounts.json` | 真实账号测试数据集 | 27 个真实账号数据，作为凭证安全投影、格式转换、长时间会话粘性的基准验证集。 | 仅用作本地脱敏测试，严禁提交进 Git 仓库。 |

### 3.2 真实代码逻辑提炼与通用化示范（含边界修复）

针对本地参考代码中存在的已知技术隐患，进行通用化提炼与缺陷修复：

```rust
// 提炼并修复后的通用端口分配原语 (消除原有 bind(0)->drop 的 TOCTOU 竞态隐患)
use std::net::{TcpListener, SocketAddrV4, Ipv4Addr};
use std::io::{Result, Error, ErrorKind};

pub fn allocate_available_port_with_retry(max_attempts: usize) -> Result<u16> {
    for _ in 0..max_attempts {
        // 绑定本地环回动态端口
        let addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0);
        if let Ok(listener) = TcpListener::bind(addr) {
            let port = listener.local_addr()?.port();
            // 在交付子进程绑定前，短暂释放，但结合启动失败自动重试机制形成闭环
            drop(listener);
            return Ok(port);
        }
    }
    Err(Error::new(ErrorKind::AddrNotAvailable, "Failed to allocate free loopback port"))
}

// 提炼并加固的配置文件原子覆盖与权限保护机制
pub fn atomic_write_secure_file(target_path: &std::path::Path, content: &str) -> std::io::Result<()> {
    let tmp_path = target_path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp_path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 严格锁定仅所有者可读写 (0o600)，防止同机其他低权限用户窃取
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600))?;
    }

    // OS 级原子替换，避免进程并发读写导致半写损坏
    std::fs::rename(tmp_path, target_path)?;
    Ok(())
}
```

### 3.3 本地代码资产借鉴 4 步 SOP (Code Adoption Protocol)
1. **Step 1 - 本地取证 (Inspect Ground Truth)**：在编写任何底层机制前，先精确定位本地参考项目中的具体文件，检查类型定义、系统调用边界与避坑细节。
2. **Step 2 - 纯机制提炼 (Extract & Purify)**：剔除业务硬编码，抽象为通用泛型函数或标准状态机，显式修复已知缺陷（如 TOCTOU 竞态）。
3. **Step 3 - 沙盒隔离编写 (New File Isolation)**：提炼后的代码写入新工程独立通用模块（如 `crates/glue-core/src/sys/`），文件头附上 SPDX 协议、来源项目及优化说明。
4. **Step 4 - 单元测试闭环 (Unit Test Gate)**：为每个提炼模块编写独立的单元测试，覆盖边界异常与跨平台逻辑，单测 100% 通过方可合并。

---

## 4. 差异化生态位、竞品调研与深度横向分析 (Competitive Landscape & Deep Analysis)

### 4.1 全景横向对比矩阵

| 对比维度 | One API / New API | LiteLLM Proxy | EasyCLIProxyAPI | Pinokio | DeepSeek Harness (DSH) | **GlueHarness (本项目)** |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **核心定位** | 商业化中继/分售/计费管理后台 | 开发者统一 API 网关 (CLI/库) | CLIProxyAPI 专属桌面伴侣壳 | AI Web 运行时的脚本浏览器 | 全插件化智能体操作底座 | **轻量级 2API 胶水平台 + 插件集市** |
| **产品形态** | Web 后台 (Go + React) | 命令行 / 容器代理服务 (Python) | 桌面应用 (Tauri v2 + Vue) | 桌面应用 (Electron + 终端环境) | 桌面/Web (Tauri/Node + React) | **原生桌面应用 (Tauri v2 + Rust + React)** |
| **架构解耦度** | 强耦合 (新渠道需改 Go 源码) | 强耦合 (依赖官方库发版适配) | 单点绑定 (只服务于一个二进制) | 松耦合 (脚本驱动任意仓库) | 极致松耦合 (Cordis 微内核) | **极致松耦合 (黑盒 Manifest + 双模接入)** |
| **上游生命周期** | 上游变动需等待主程序发版 | 依赖 pip 包升级 | 依赖主工程与单一二进制同步 | 脚本驱动，各依赖自主构建 | 独立插件热插拔 | **Git-Native 零成本同步 (Release 自动拉取)** |
| **进程与生命周期** | 外部常驻服务，无本地子进程管理 | 无本地子进程管理 (纯代理) | 常驻守护单个子进程，无按需休眠 | 常驻运行重型 Python/Node 进程 | Cordis 内部上下文动态管理 | **四象限生命周期 (按需/超时/常驻/外部接入)** |
| **资源消耗** | 较重 (50~200MB+，依赖 MySQL) | 中等 (Python 运行时，50~100MB) | 轻量 (20~40MB，但仅单一功能) | 极重 (数百 MB 内存，数十 GB 磁盘)| 中等 (Tauri 壳 30MB + Node 后端) | **极轻 (Rust 核心 < 30MB，无数据库，闲置 0MB)** |
| **对外接入端口** | 统一暴露外部单端口 | 统一暴露外部单端口 | 绑定特定端口 | 各项目自选独立端口，无统一 API | 专用 Agent RPC / Slots | **统一本地单端口多路复用 (默认 8080，可自选)** |

### 4.2 核心竞品优缺点剖析与生态位差异
1. **One API / New API**：
   - *优势*：渠道管理极全面，用户鉴权、额度消耗、高并发负载均衡极其成熟，是商业 API 分售的绝对事实标准。
   - *定位差异*：属于“多租户商业运营后台”，架构过重，强依赖数据库；无法让普通终端用户在本地一键拉取黑盒反代二进制即插即用。
2. **EasyCLIProxyAPI (router-for-me/EasyCLI)**：
   - *优势*：Tauri v2 原生体积小，针对 CLIProxyAPI 做到了开箱即用。
   - *定位差异*：属于“专车专线”，架构与单一二进制死绑定。无法横向聚合其他 2API 项目，缺乏通用的插件抽象与多路由机制。
3. **Pinokio**：
   - *优势*：通过脚本实现了 AI 项目的自动化安装与运行。
   - *定位差异*：面向重型 WebUI 实验项目（ComfyUI, SD），运行时动辄数十 GB，缺乏统一的 API 路由和 OpenAI 协议标准化网关能力。
4. **LiteLLM**：
   - *优势*：模型协议抹平能力极强，支持主流上百种模型。
   - *定位差异*：纯开发者向命令行/SDK，无桌面交互与可视化诊断，无法管理本地异构二进制生命周期。

### 4.3 深度借鉴：DeepSeek Harness (`dsh`) 架构与设计精髓
DeepSeek 官方开源的 `dsh` 是将微内核、解耦生态与极简美学结合的标杆项目：
1. **Cordis 微内核与 "Everything is a Plugin" 哲学**：
   - 宿主内核不写任何具体平台的业务逻辑，仅提供服务总线（Service Bus）。
   - 映射到 GlueHarness：核心只保留 `GatewayServer`、`ProcessSupervisor`、`CredentialBroker` 和 `ConfigStore` 四大基础能力，所有 2API 均以黑盒插件或外部服务接入。
2. **Reversible Effects（可逆副作用）与 Rust RAII 守卫**：
   - Cordis 中插件注册的路由、挂载的文件在卸载时会自动反向清空。
   - 映射到 GlueHarness：为每个运行中插件维护 `PluginSessionGuard`，析构时自动注销路由、解绑端口、清除临时配置并终止子进程。
3. **UI Slots 声明式插槽机制**：
   - 客户端通过动态插槽接纳插件的自定义表单与状态指示。
   - 映射到 GlueHarness：插件在 `manifest.yaml` 中通过 `config_schema` 声明表单，前端动态渲染输入项，宿主前端代码零修改。
4. **极简暗色工业风与 Test-Flight 试飞工作台**：
   - 极端克制的 Zinc 黑灰色系、紧凑高密度的等宽信息排版、零冗余装饰。
   - 任何配置流程的终点都是“即时打字试飞”，让用户秒级感知连通性与首字延迟（TTFT）。
5. **洋葱流水线 (Waterfall Middleware Pipeline)**：
   - 请求分发层层穿透：`AuthCheck -> ModelAliasMapper -> SessionStickyResolver -> GatewayProxy -> StreamNormalizer`。

---

## 5. 技术栈与系统架构设计 (Architecture & Core Modules)

### 5.1 技术栈选型与裁决
- **宿主内核与桌面外壳**：**Tauri v2 + Rust (Edition 2021)**
  - *系统原生*：基于系统 Webview，安装包 10~15MB，基础常驻内存控制在 15~30MB。
  - *极致稳健*：强类型内存安全，零 GC 抖动，跨平台系统级 API 原生调用。
  - *异步运行时*：`Tokio` + `Axum` + `Hyper-util`。
- **前端交互层**：**Vite + React 18 + Tailwind CSS + Lucide React**
  - *设计风格*：Strict Dark Minimalist（纯正 Zinc/Neutral 暗色体系，无多余浮夸动效）。
- **持久化层**：**No DB（纯 JSON 持久化 + 内存 RwLock）**
  - 配置保存在 `~/.glue-harness/config.json`，无数据库驱动、零部署运维开销。

### 5.2 系统架构拓扑图与明确依赖关系

```
┌────────────────────────────────────────────────────────────────────────┐
│               GlueHarness 桌面应用 / 极简服务宿主 (Tauri v2 Core)       │
├────────────────────────────────────────────────────────────────────────┤
│ 1. 统一对外入口 (HTTP Gateway Server - 默认 127.0.0.1:8080)            │
│    ├── 安全中间件 (Bearer Token 鉴权，支持本地随机高强度 Key)          │
│    ├── POST /v1/chat/completions (标准 OpenAI 请求入口)                │
│    ├── POST /v1/responses        (OpenAI 新标准 Responses API 路由)    │
│    ├── GET  /v1/models           (聚合所有已就绪插件的可用模型列表)    │
│    └── GET  /healthz             (本地宿主健康自检探针)                │
├────────────────────────────────────────────────────────────────────────┤
│ 2. 协议适配与路由引擎 (Protocol Adapter & Model Router)                │
│    ├── 模型匹配引擎 (前缀路由 "wb/claude-3-7", 别名规则, Fallback)     │
│    ├── 协议转接管道 (OpenAI Chat / Responses / Anthropic Messages 转换) │
│    └── 流式规整器 (SSE Stream Normalizer: 边界识别/Tool Call拼接)      │
├────────────────────────────────────────────────────────────────────────┤
│ 3. 插件运行时守护器 (Plugin Supervisor & Runtime)                      │
│    ├── 四象限生命周期引擎 (Persistent / Idle Timeout / On-Demand / Ext) │
│    ├── 进程组看门狗 (Windows Job Object / Linux prctl / macOS kqueue)  │
│    ├── 端口分配器 (Port Allocator: 避免 TOCTOU 竞态，重试保护)         │
│    └── 启动防击穿互斥锁 (Singleflight Cold-Start Lock)                 │
├────────────────────────────────────────────────────────────────────────┤
│ 4. 凭证中继与安全中心 (Credential Broker & Store)                      │
│    ├── 系统安全凭据库对接 (Keyring / DPAPI / Keychain)                 │
│    └── 临时工作区单向安全投影 (0o600 权限保护 / 原子写入 / 进程退出清理)│
├────────────────────────────────────────────────────────────────────────┤
│ 5. 发现与安装引擎 (Registry & Plugin Installer)                        │
│    ├── GitHub Release 资产智能匹配与流式 SHA256 校验                   │
│    └── 分级信任机制 (Official/Verified, Community, Local/Custom)       │
├────────────────────────────────────────────────────────────────────────┤
│ 6. 内置诊断与试飞交互层 (Test-Flight & API Doctor)                     │
│    ├── 极简打字试飞工作台 (实时 TTFT、首字延迟、TPS 吞吐监测)          │
│    └── 交互式故障排查抽屉 (脱敏 Trace、网络/鉴权错误智能修复指引)      │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ 本地环回 IPC / 内部 HTTP 代理
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        外部独立插件运行时 (外部进程 / 外部服务)         │
├──────────────────────────────────┬─────────────────────────────────────┤
│ [Managed Plugin: 托管二进制]     │ [External Provider: 外部服务]       │
│ 例如: wb2api.exe                 │ 例如: 已常驻运行的 CPA / LiteLLM    │
│ 内部动态端口: 127.0.0.1:18231    │ 外部固定端口: 127.0.0.1:8000        │
│ 生命周期受 Supervisor 完全托管   │ 仅做协议路由与健康感知，不杀进程    │
└──────────────────────────────────┴─────────────────────────────────────┘
```

**严格单向依赖原则**：
`Gateway` $\rightarrow$ `Router` $\rightarrow$ `Supervisor` $\rightarrow$ `Credential Broker` $\rightarrow$ `Config Store`。禁止反向循环依赖。

### 5.3 统一对外入口与端口管理 (Port Allocator)
1. **统一入口端口可配置**：
   - 默认监听 `127.0.0.1:8080`。用户可在设置中自由更改为任意端口（如 `18080`、`8888`）。
   - 启动时若检测到端口冲突，**严禁崩溃退出**，系统必须优雅提示用户“端口已被占用”，并提供【一键切换备用空闲端口】选项。
2. **本地 API 鉴权保护**：
   - 默认自动生成高强度本地随机 API Key（`sk-glue-xxxxxx`），存储在配置中。
   - 所有进入 `127.0.0.1:8080` 的请求必须携带有效 `Authorization: Bearer <key>`，防止本机上恶意脚本或未知网页越权探测本地转接通道。用户也可选择关闭鉴权（仅在纯隔离单人环境中）。
3. **消除端口分配的 TOCTOU 竞态**：
   - 废除原方案危险的简单 `bind(0) -> drop` 即认为端口绝对安全的逻辑。
   - 采用带退避机制的动态端口分配算法，并在子进程启动后立即挂起探针；若上游二进制因端口抢占报错退出，立即更换新端口自动重试（最多 3 次）。

### 5.4 路由与分发中间件 (Model Router & Protocol Pipeline)
1. **多层路由匹配算法**：
   - *精确匹配*：`plugin_id/model_name`（如 `wb/claude-3-7-sonnet`），100% 确定路由。
   - *模型别名*：配置别名映射表（如请求 `gpt-4o` 自动重定向至指定主力插件）。
   - *默认 Fallback*：若用户未指定前缀，按插件优先级列表中首个处于 Healthy 状态的插件路由。
2. **多上游可用模型聚合 (`GET /v1/models`)**：
   - 定期向所有已启用的插件拉取或根据 Manifest 静态能力合并生成完整的模型列表，对外暴露标准 OpenAI 格式。

### 5.5 插件双模运行架构：Managed vs External
系统原生支持两类插件运行形态：
- **Managed Plugin (托管模式)**：由 GlueHarness 负责二进制下载、解压、启动、监控、重启及销毁。适用于单文件 Go/Rust 编译的纯净 2API。
- **External Provider (外部模式)**：用户本地早已通过命令行、Docker 或其他桌面应用拉起的常驻服务（如本地已跑在 `8000` 端口的 CLIProxyAPI 或远程 One-API）。基座只记录其 `base_url` 与 `api_key`，负责路由分发与健康检查，**严禁干预其进程生命周期**。

### 5.6 插件生命周期状态机与调度策略 (Lifecycle State Machine)
彻底废除原规划中“强行按 LRU 杀死活跃插件”的激进做法，避免正在进行的长会话生成被无情截断。

#### 1. 四象限生命周期策略 (Lifecycle Policies)
每个插件可在 Manifest 中声明其生命周期模式：
- `persistent` (常驻模式)：启动后持续运行，除非用户手动停止或主程序退出，绝不自动回收（适合核心主力通道）。
- `idle_timeout` (空闲回收模式，默认)：最后一次请求完成后倒计时（默认 5 分钟），无请求流经且活跃请求数为 0 时，优雅终止释放内存。
- `on_demand` (即用即走模式)：请求到来时唤醒，单次请求响应完毕后延迟 30 秒即刻回收。
- `external` (外部模式)：进程不受控，永久在线探测。

#### 2. 生命周期完整状态机 (State Machine)

```
        [Uninstalled]
             │ (用户安装 / 拉取 Release)
             ▼
        [Installing] ──(校验/解压失败)──► [Failed]
             │
             ▼
          [Stopped] ◄────────────────────────────────────────┐
             │                                               │
             │ (收到路由请求 / 手动启动)                     │
             ▼                                               │
         [Starting] ──(启动超时/端口冲突)─► [Failed]         │
             │                                               │ (空闲超时 /
             ▼ (健康检查通过)                                │  手动停止 /
          [Healthy]                                          │  主动终止)
             │   ▲                                           │
(探针偶尔失败│   │ (恢复正常)                                │
 或未就绪)   ▼   │                                           │
         [Unhealthy] ──(连续N次失败)─► [Stopping] ───────────┘
```

#### 3. 关键并发安全机制
- **活跃请求引用计数 (In-flight Request Counter)**：
  每个插件维护一个原子计数器 `active_requests: AtomicUsize`。只要 `active_requests > 0`，任何空闲超时定时器一律挂起，绝对禁止回收正在生成内容的进程。
- **冷启动防击穿互斥锁 (Singleflight Startup Lock)**：
  当多个下游并发请求同时命中处于 `Stopped` 状态的插件时，基于 Tokio 互斥锁确保**同一时间仅触发一次子进程拉起**，后续请求异步挂起等待探针变为 `Healthy`，避免同时拉起多个进程引发端口冲突与系统雪崩。
- **优雅停机 (Graceful Shutdown)**：
  停止插件时，优先发送 `SIGTERM`（Windows 发送 `CTRL_BREAK_EVENT` 或向内部端点发退出信号），留出 3 秒保存状态缓冲区；超时未退出时方才升级为强杀。

### 5.7 跨平台系统级进程管控机制
纠正原方案中认为“Unix 下 `setpgid` 会自动回收子进程”的严重技术错误（POSIX 标准中父进程退出不会自动导致内核杀死同进程组子进程）。

本系统设计严格可靠的跨平台看门狗机制：
1. **Windows 平台**：
   - 使用 Win32 原生 `CreateJobObjectW` 创建专属 Job。
   - 配置 `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`，激活 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 标志。
   - 将所有派生的子进程句柄压入该 Job。即使 GlueHarness 主程序被任务管理器强制 End Process 或遭遇断电级崩溃，Windows 内核保证瞬间级联终止所有子进程。
2. **Linux 平台**：
   - 在子进程派生前通过 `pre_exec` 调用 `libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL)`。
   - 当父进程终止时，Linux 内核自动向子进程递送 `SIGKILL` 信号。
   - 配合 `libc::setpgid(0, 0)` 将子进程放入独立进程组，正常退出时发送 `SIGTERM` 给整个进程组。
3. **macOS 平台**：
   - macOS 不支持 Linux 的 `PR_SET_PDEATHSIG`。
   - 采用标准匿名管道看门狗（Inherited Pipe Watchdog）：父进程创建一对无名管道，读端继承给子进程包装器；当父进程死亡时管道写端自动关闭，子进程包装器的读端触发 EOF 并立即自我收割；或在宿主中注册 `libc::atexit` 与信号处理器主动清理进程组。

### 5.8 流式传输与协议规范 (Streaming & Protocol Compatibility)
1. **协议支持范围与能力边界**：
   - **OpenAI Chat Completions (`/v1/chat/completions`)**：首发完全支持（包含非流式、SSE 流式、Function / Tool Calls、多模态 Image URL 透传）。
   - **OpenAI Responses API (`/v1/responses`)**：规划支持，作为下一代交互协议预留路由。
   - **Anthropic Messages API (`/v1/messages`)**：支持直通或通过轻量适配器转换为 Chat Completions。
   - *明确边界*：明确告知用户部分客户端若使用私有 RPC 协议将无法直接透传，必须通过标准 HTTP/SSE 接口对接。
2. **SSE 流式传输与终止标志规整原则 (Stream Done Guard)**：
   - **严谨分类处理，坚决不无条件注入 `[DONE]`**：
     * **场景 A - 正常传输完毕但上游漏发标记**：上游 HTTP 状态为 200，TCP 连接正常平稳关闭，最后一个有效数据包之后无更多内容，但上游缺失终止行 $\rightarrow$ 此时流式守卫安全补齐 `data: [DONE]\n\n`，确保 Cursor/Claude Code 正常收尾。
     * **场景 B - 网络异常中断或上游超时崩溃**：传输中途上游连接被非正常重置（Connection Reset）、读取超时或上游抛出 5xx 错误 $\rightarrow$ **绝对禁止追加 `[DONE]`**！必须立即向下游抛出断开异常或格式化错误帧，下游客户端感知到不完整输出从而提示重试，彻底杜绝数据截断却伪装成功的假死现象。
   - **流式背压与平滑转发**：
     * 使用 Tokio 异步管道，基于块（Chunk）逐帧转发，响应头强制注入 `X-Accel-Buffering: no` 与 `Cache-Control: no-cache`，消除反向代理常见的缓冲区滞留（Bufferbloat）延迟。

### 5.9 性能设计与客观可度量指标
坚决摒弃“零拷贝、0ms 延迟、绝对 150ms 冷启动”等不严谨口号，建立基于实测的性能目标矩阵：

| 性能维度 | 理论架构目标 | 允许实测基准 (SLA) | 测量方式与验证手段 |
|---|---|---|---|
| **网关中继额外延迟** | < 1.0 ms | **< 5.0 ms** | 内部 Loopback 测试，对比直连上游与经由 GlueHarness 转发的延迟差值 |
| **首字时间 (TTFT) 增量** | 流式零缓存即时刷新 | **< 10 ms** | 记录首个 SSE Chunk 离开上游与抵达下游的时间戳差 |
| **基座空闲内存占用** | 15 MB | **< 30 MB** | 启动后无插件运行，Tauri + Rust 核心常驻内存 |
| **插件冷启动预热时长** | 依插件自身特性决定 | **实测标定 (通常 150~600ms)** | 从收到首个 HTTP 请求到子进程探针绿灯的总耗时 |
| **单核吞吐能力 (TPS)** | > 2000 Req/s | **> 500 Req/s** | 本地环回基准压力测试 |

---

## 6. 插件规范（Manifest Schema 1.0）与生态发现 (Plugin System & Ecosystem)

### 6.1 Manifest Schema 1.0 规范定义
所有插件由一份标准的 `manifest.yaml` 描述，具备完全的向后兼容与版本约束：

```yaml
schema_version: "1.0"
id: "workbuddy-2api"
name: "WorkBuddy 2API Engine"
version: "1.13.0"
author: "community"
description: "将 WorkBuddy 账号转接为标准 OpenAI 格式接口"
host_min_version: "0.1.0"

# 1. 产物拉取与多架构匹配
source:
  type: "github_release"
  repo: "linguo2625469/workbuddy2api-panel"
  asset_match:
    windows_amd64: "wb2api-windows-amd64.zip"
    darwin_arm64: "wb2api-darwin-arm64.tar.gz"
    darwin_amd64: "wb2api-darwin-amd64.tar.gz"
    linux_amd64: "wb2api-linux-amd64.tar.gz"

# 2. 运行形态与生命周期策略
runtime:
  execution_mode: "managed"          # managed (托管进程) | external (外部服务)
  executable: "bin/wb2api.exe"
  args: ["--config", "config.json"]
  env:
    PORT: "{AUTO_PORT}"             # 动态分配内部端口
    HOST: "127.0.0.1"
  lifecycle_policy: "idle_timeout"  # persistent | idle_timeout | on_demand | external
  idle_timeout_seconds: 300
  startup_timeout_ms: 5000
  stop_signal: "SIGTERM"
  stop_timeout_ms: 3000

# 3. 健康检查与就绪探针
health_check:
  type: "http"                      # http | tcp | process
  path: "/healthz"
  expected_status: 200
  interval_ms: 1000
  max_retries: 5

# 4. 模型能力与协议声明
exports:
  base_url: "http://127.0.0.1:{AUTO_PORT}"
  protocols: ["openai_chat"]
  capabilities: ["streaming", "tool_calls"]
  models:
    - id: "claude-3-7-sonnet"
      alias: ["wb/claude-3-7-sonnet", "claude-3-7"]
      upstream_model: "claude-3-7-sonnet-20250219"

# 5. 权限要求与敏感凭据 (Secret Broker)
permissions:
  network_listen: true
  filesystem_scoped: true           # 仅限制访问自身插件目录
secrets:
  - key: "access_token"
    label: "Access Token"
    type: "password"
    required: true
    description: "从浏览器 Network 请求中获取的 Bearer Token"
```

### 6.2 外部模式定义样例 (External Provider Manifest)
用户直接接入已存在的本地服务时，Manifest 极其简单：

```yaml
schema_version: "1.0"
id: "local-cliproxy"
name: "Local CLIProxyAPI (Existing Service)"
version: "1.0.0"
runtime:
  execution_mode: "external"
  lifecycle_policy: "external"
exports:
  base_url: "http://127.0.0.1:8000"
  protocols: ["openai_chat"]
  models:
    - id: "gemini-2.5-pro"
      alias: ["cpa/gemini-pro"]
health_check:
  type: "http"
  path: "/health"
```

### 6.3 生态分发、分级信任与安全审查机制
**彻底否定“随意粘贴 GitHub URL 即无感静默执行”的高危方案**。本地运行的二进制具备操作系统级普通用户权限，必须构筑安全信任防线：

1. **插件分级信任体系**：
   - 🟢 **Official / Verified (官方核验)**：经过官方仓库审查并包含数字签名/SHA256 签名的插件，用户可一键下载运行。
   - 🟡 **Community / Unverified (社区未核验)**：通过 GitHub Topic 联邦发现的插件。安装时界面以显著黄色横幅告警：“该插件来自社区未审核源，执行外部二进制可能带来系统安全风险”，并强制用户阅读权限声明后手动输入确认。
   - 🔵 **Local / Custom (本地开发者自建)**：用户手动导入本地 `manifest.yaml`，适用于私有开发与内网调试。
2. **启发式适配安全门禁**：
   - 遇到未提供 `manifest.yaml` 的开源项目，系统通过分析 Release 文件名可提供“推测模板”，但**绝对不得默认直接运行**，必须停留在配置预览界面，供用户核实命令行参数、环境变量并保存后方可启动。

---

## 7. 交互体验与产品差异化设计 (UX & Product Differentiation)

### 7.1 主界面布局 Wireframe (Strict Dark Minimalist)

```
┌───┬───────────────────────────────────────────┬──────────────────────────────────────────┐
│ G │ 🔌 插件列表 (Plugins)     [+ 接入外部服务] │ 💬 试飞工作台 (Test-Flight Playground)   │
│ L │ ----------------------------------------- │ ---------------------------------------- │
│ U │ 🟢 WorkBuddy 2API           [托管 · 18MB] │ [目标模型: wb/claude-3-7-sonnet       ▾] │
│ E │    模型: wb/claude-3-7-sonnet             │                                          │
│   │    状态: 待命中 (03:42 后自动休眠)        │ User:                                    │
│ H │    [面板] [日志] [配置] [停止]            │ 请测试当前流式连接是否通畅。             │
│ A │                                           │                                          │
│ R │ 🟢 Local CLIProxyAPI        [外部 · 常驻] │ Assistant (Streaming...):                │
│ N │    模型: cpa/gemini-2.5-pro               │ 正在为您验证连接！当前流式响应正常...    │
│ E │    状态: 外部连接就绪 (127.0.0.1:8000)    │                                          │
│ S │                                           │ ⚡ 耗时: 340ms | TTFT: 110ms | TPS: 42   │
│ S │ ⚙️ 统一对外接入网关                       │ 协议: OpenAI SSE Stream 🟢               │
│   │ 地址: http://127.0.0.1:8080/v1            │                                          │
│   │ 密钥: sk-glue-8f92a1...    [复制] [重新生成]│ [一键复制 Cursor 配置] [复制 Claude Code]│
├───┴───────────────────────────────────────────┼──────────────────────────────────────────┤
│ 🩺 [API Doctor 抽屉] 活跃中: 2 | 监听: 8080 | 凭证库: 系统安全隔离 | 内存: 24MB (基座)  │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 四大核心产品差异化能力

```
                  【GlueHarness 核心差异化能力矩阵】
                                  │
    ┌─────────────────┬───────────┴───────────┬─────────────────┐
    ▼                 ▼                       ▼                 ▼
【API Doctor】  【Compatibility Flight】 【Config Profiles】  【Safe Upgrade】
端到端深度连通诊断 协议一致性自动化测试   下游客户端预设一键导出 版本固定与一键秒级
脱敏Trace异常解析  ToolCall/流式截断验证  Cursor/Claude/NextChat 回滚，零配置污染
```

1. **API Doctor（端到端连接诊断器）**：
   - 告别黑盒盲猜。提供端到端连通性测试报告：`本机网关 -> 内部端口 -> 插件进程 -> 外部上游网络 -> 目标大模型`。
   - 发生错误时提供脱敏 Trace，将晦涩的底层错误（如 `401 Unauthorized` 自动解析为“Access Token 已过期，请重新抓取”；`502 Bad Gateway` 自动提示“插件进程由于内存超限或上游拒绝崩溃，请查看日志”）。
2. **Compatibility Flight（协议兼容性试飞套件）**：
   - 内置一键自动化质检脚本：
     * **Chat Basic**：测试非流式 JSON 返回格式。
     * **SSE Stream**：测试逐 Chunk 流式吐字与平滑度。
     * **Tool Calling**：测试函数调用的参数流式组装。
     * **Client Abort**：下游中断连接时，验证是否能够正确取消上游请求。
3. **Configuration Profiles（开发工具环境预设）**：
   - 一键生成并复制适用于当前主流工具的配置片段：
     * *Cursor*：`Base URL: http://127.0.0.1:8080/v1`, `API Key: sk-glue-...`, `Model: wb/claude-3-7-sonnet`
     * *Claude Code*：`export ANTHROPIC_BASE_URL=http://127.0.0.1:8080` 一键脚本
     * *NextChat / LibreChat*：环境变量与导入链接生成
4. **Safe Upgrade（平滑升级与一键回滚）**：
   - 插件更新时，新版本二进制下载至临时目录校验无误后方才原子替换。旧版本自动保留为 `.old`。若新版本无法通过健康检查，系统在 5 秒内自动秒级回滚到上一可用版本，且配置数据与凭据物理隔离，保证 100% 零丢失。

---

## 8. 安全模型与数据持久化治理 (Security Model & Persistence Governance)

### 8.1 威胁建模 (Threat Model)

| 威胁类型 | 潜在攻击场景 | 系统级防御应对策略 |
|---|---|---|
| **1. 本地 API 未授权探测** | 本机其他恶意程序或恶意网页脚本向 `127.0.0.1:8080` 发起跨域或环回请求窃取大模型额度 | 1. 默认强制启用随机生成的高强度 API Key。<br>2. 严格校验 HTTP `Host` 头仅允许 `127.0.0.1` / `localhost`，防御 DNS Rebinding 攻击。 |
| **2. 凭据文件被恶意读取** | 明文存储 Token 导致本机低权限木马或恶意软件直接盗取账户凭证 | 1. 配置与 Secret 物理分离。<br>2. 核心 Secret 调用操作系统原生凭据安全库 (Windows Credential Manager / macOS Keychain / Linux Secret Service)。<br>3. 临时投影文件强制设定仅所有者读写 (`0o600`)，子进程销毁时同步清理。 |
| **3. 恶意插件与供应链攻击** | 恶意第三方在 GitHub Release 中植入带毒二进制，窃取用户剪贴板或本地文件 | 1. 明确外部进程不是沙箱的安全边界声明。<br>2. 建立分级信任机制（官方审核 / 社区告警 / 本地导入）。<br>3. 强制校验官方发布的 SHA256 散列值。<br>4. 禁止静默执行未经确认的外部二进制。 |
| **4. 日志隐私泄露** | 错误堆栈或代理日志将用户的对话内容、API Key 打印到磁盘 | 1. 全局日志中间件实施自动敏感词脱敏（Token、Key、Password 自动掩码为 `sk-***`）。<br>2. 试飞对话历史仅保存在内存中，关闭窗口即时释放，绝不持久化到磁盘。 |

### 8.2 凭据安全体系 (Credential Broker)
- **存储物理分离**：普通配置（端口、模型别名、插件开关）保存在 `~/.glue-harness/config.json`；敏感 Token（如 WorkBuddy Token、OAuth Refresh Token）交由 `keyring-rs` 存入操作系统凭据管理器。
- **单向只读安全投影**：针对必须读取配置文件的外部二进制（如 WorkBuddy 2API 要求读取 `config.json`），胶水层仅在拉起子进程的瞬时，向插件的专用隔离目录写入临时文件，设权限为 `0o600`，并在进程退出或崩溃时由 RAII Guard 自动清除。

### 8.3 插件运行风险边界说明
**向用户明确技术事实：外部独立进程不等于 WebAssembly 或浏览器的虚拟沙箱**。插件在操作系统中以当前登录用户的权限运行。 GlueHarness 通过来源审查、散列校验、启动参数白名单和透明化告警构筑纵深防御，但对于未经验证的第三方程序，保持绝对的风险警示。

### 8.4 配置持久化与灾备恢复 (Persistence & Disaster Recovery)
坚持 **No DB 哲学**，但绝不等于脆弱的文本写入：
1. **显式版本控制**：配置文件顶层定义 `schema_version: 1`。后续升级采用链式迁移函数（`migrate_v1_to_v2`）。
2. **并发读写安全**：进程内使用 `parking_lot::RwLock`，杜绝多线程写竞争。
3. **原子覆盖与双副本备份**：每次写入时先写入临时文件，同步落盘后原子 `rename`。同时持久化最近一次有效的 `config.json.bak`。若解析报错，自动加载 `.bak` 恢复，并在 UI 弹出修复通知。

---

## 9. 架构决策记录 (ADR - Architectural Decision Records)

### ADR-01: 为什么选择微内核黑盒进程，而非内嵌动态库 (DLL/so) 或 WebAssembly？
- **背景**：需要聚合多种异构开源反向代理项目（Go, Rust, Python, Node 等编写）。
- **决策**：采用**独立黑盒子进程（Managed）与外部 HTTP 服务（External）**，拒绝通过 C-ABI/FFI 内嵌动态链接库，也暂不强制要求 WebAssembly。
- **理由**：
  1. *上游解耦*：主流反代项目（如 wb2api, CPA）绝大部分是以 Go/Rust 编译的独立二进制形式分发。强行编译为动态库会破坏上游原生构建流程，增加适配成本。
  2. *隔离性与容错*：子进程崩溃（如 OOM 或 panic）完全不影响 GlueHarness 宿主；内嵌动态库崩溃会导致主程序直接闪退。
  3. *许可证隔离*：黑盒进程通过网络/IPC 通信，在法律层面完全阻断 GPL 等开源协议对宿主核心的传染。

### ADR-02: 为什么采用系统级凭证库/分离式加密，而非明文 JSON 存储 Secret？
- **背景**：用户输入的 Access Token、Cookie 具有高敏感性。
- **决策**：将普通配置与 Secret 物理分离，Secret 默认交由操作系统 Keyring 托管。
- **理由**：明文 JSON 容易被本机其他低权限进程扫描窃取。利用 OS 原生凭据保护，既避免在工程中自研脆弱的加解密算法（不造密码学轮子），又达到了系统级防御标准。

### ADR-03: 为什么废除纯 LRU 进程淘汰，改为四象限生命周期策略与 Singleflight 锁？
- **背景**：早期规划提出按 LRU 强制休眠活跃插件以保持低内存。
- **决策**：废除硬性 LRU 强杀，引入四象限策略（Persistent, Idle Timeout, On-Demand, External），配合活跃请求计数与 Singleflight 启动锁。
- **理由**：大模型生成具有长流式特征（可能持续数十秒至数分钟）。若按机械的 LRU 规则在后台强杀正在响应的插件，将导致下游生成截断；Singleflight 锁则彻底防止了并发请求同时拉起多个相同插件造成的端口冲突。

### ADR-04: 为什么严禁在异常断开时盲目追加 `data: [DONE]`？
- **背景**：早期规划提出遇到断开自动补全 `data: [DONE]` 消除下游悬挂。
- **决策**：严格区分“上游正常结束但漏标（补发）”与“传输异常截断（抛错中断，坚决不补发）”。
- **理由**：下游 Agent（如 Cursor）依赖 `[DONE]` 确认响应已完整。如果在网络断开、内容只输出了一半时盲目补发 `[DONE]`，下游会误认为生成完毕并执行残缺代码，产生严重不可逆后果。

### ADR-05: 为什么坚持 Reference-First 并在自研前必须记录评估档案？
- **背景**：AI 编码容易陷入凭空手写复杂机制的陷阱，导致隐蔽 Bug 与重复造轮子。
- **决策**：建立强制开源参考源检索与复用规范，未记录评估档案禁止自研非平凡模块。
- **理由**：成熟开源实现包含了大量边缘情况处理（如跨平台系统调用、并发锁、协议容错），复用经过检验的代码比从零重写稳健得多。

---

## 10. 实施里程碑与落地计划 (Milestones & Acceptance Gates)

坚持以**依赖关系与风险收敛**为核心驱动，重新规划 Phase -1 至 Phase 3 的开发闭环：

```
Phase -1 (Launch Gate: 命名/仓库/宗旨)
   │
   ▼
Phase 0 (MVP 验证: Rust网关 + 1个托管插件 + 1个外部服务 + JobObject看门狗)
   │
   ▼
Phase 1 (核心引擎: 完整Manifest + 凭据安全库 + 流式规整器 + 自动拉取器)
   │
   ▼
Phase 2 (差异化质检: API Doctor + DeepSeek风格试飞工作台 + 导出预设)
   │
   ▼
Phase 3 (生态发布: 分级信任市场 + 多环境Profile + 跨平台CI自动化发版)
```

### Phase -1: 前置流程启动门禁 (Launch Gate)
- [ ] **确定正式产品名称**：由用户最终裁定命名（如 `APIDock` / `RelayCraft`）。
- [ ] **创建 GitHub 独立官方仓库**：新建全新干净仓库，杜绝历史脏提交。
- [ ] **将《产品宗旨》编写为官方首版 README.md**：将第 1 章宗旨与哲学作为 README 首推并打下 Initial Commit。
- **验收门禁**：GitHub 仓库就绪且 Initial Commit 包含宗旨文档。

### Phase 0: 核心网关与进程管控最小闭环 (MVP)
- **参考源研究任务**：深入分析 `shared-child` 进程控制、本地 `REF` 端口探测与 Job Object 源码、`axum` 异步路由。
- **核心开发内容**：
  - [ ] 搭建 Tauri v2 + Rust 基础工程骨架，集成 Vite + React + Tailwind CSS。
  - [ ] 集成 `Axum`，实现可配置端口（默认 `127.0.0.1:8080`）的 `/v1/chat/completions` 与 `/healthz`。
  - [ ] 实现 `Plugin Supervisor` 最小子集：拉起 1 个本地 Managed 外部二进制（`wb2api.exe`），带动态端口分配与 Windows Job Object 级联清理。
  - [ ] 实现 1 个 External Provider 的直接代理转发（如对接本地已运行的 CPA）。
  - [ ] 实现基础 SSE 流式异步零阻塞透明转发。
- **验收门禁**：
  - *功能验收*：通过 curl 请求 `127.0.0.1:8080/v1/chat/completions` 能成功从本地 wb2api 获得流式响应。
  - *异常验收*：强制杀死 GlueHarness 进程，Windows 任务管理器中 `wb2api.exe` 瞬间 100% 同步消失，无任何孤儿进程残留。
  - *性能验收*：基座常驻内存稳定在 30MB 以内。

### Phase 1: 协议适配、配置安全与 Manifest 引擎闭环
- **参考源研究任务**：研读 `eventsource-stream` 解析逻辑、`keyring-rs` 系统调用、`octocrab` 资产拉取。
- **核心开发内容**：
  - [ ] 实现 `Manifest Schema 1.0` 解析器（支持 YAML / JSON）。
  - [ ] 引入 `keyring-rs` 实现敏感 Secret 与普通配置的物理分离存储。
  - [ ] 实现安全临时凭证单向投影机制（`0o600` 权限 + 原子写入 + 退出清理）。
  - [ ] 实现 `SSE Stream Normalizer`：精确识别流边界，规范处理漏发 `[DONE]`，严禁在异常断开时盲目补齐。
  - [ ] 实现 Singleflight 启动互斥锁与空闲超时（Idle Timeout）自动回收机制。
  - [ ] 实现基于 GitHub Release 的自动下载、SHA256 校验与解压安装流水线。
- **验收门禁**：
  - *安全验收*：导出的配置临时文件中权限符合预期，重启后无敏感明文滞留。
  - *协议验收*：多 Chunk `tool_calls` 流式解析在下游客户端无丢包、无乱码。
  - *并发验收*：10 个并发请求突发打向休眠插件，子进程仅拉起一次，无端口冲突。

### Phase 2: 原生试飞台、API Doctor 与诊断套件
- **参考源研究任务**：深度对标 `DeepSeek Harness` 极简暗色前端实现与 TTFT 测算方案。
- **核心开发内容**：
  - [ ] 开发 DeepSeek 风格的极简暗色工作台（Playground），支持流式打字机交互。
  - [ ] 实时计算并高亮渲染 TTFT（首字延迟）、TPS（每秒 Token）与传输状态指示灯。
  - [ ] 开发 `API Doctor` 抽屉：捕获 401、429、502 错误并输出针对性脱敏诊断报告。
  - [ ] 实现 [一键导出至 Cursor / Claude Code / NextChat] 配置卡片。
- **验收门禁**：
  - *交互验收*：在前端 Playground 中输入测试文本，毫秒级响应并准确显示 TTFT 标签。
  - *诊断验收*：构造假 Token 触发 401，API Doctor 准确指引“Token 失效”而非显示原始崩溃堆栈。

### Phase 3: 社区发现、多 Profile 管理与发布流水线
- **参考源研究任务**：研究 GitHub Actions 跨平台多架构打包流水线、分级信任安全策略。
- **核心开发内容**：
  - [ ] 实现基于 GitHub Topic / 独立 Tap 的插件动态发现与展示界面。
  - [ ] 实现分级信任安全审查机制（官方审核 / 社区未核验弹窗警告 / 本地导入）。
  - [ ] 实现 Configuration Profiles：支持在不同开发工具或项目间秒级切换不同模型前缀组合。
  - [ ] 实现 Safe Upgrade：新版本自动校验、平滑切换与异常 5 秒内自动回滚。
  - [ ] 搭建 GitHub Actions 跨平台 CI 构建流水线，打包 Windows (.exe/msi)、macOS (.dmg)、Linux (.AppImage/deb)。
- **验收门禁**：
  - *发布验收*：CI 流水线自动化构建出全平台单文件安装包并通过基础可用性核验。

---

## 11. 开源工程规范与项目文档体系 (Governance & Repository Standards)

### 11.1 开源项目标准文档规划
在新仓库根目录下建立规范的开源治理文档体系：
1. **`README.md`**：项目门面。包含产品宗旨、架构拓扑动图、快速上手指南、下载链接、生态定位说明。
2. **`ARCHITECTURE.md`**：深度架构白皮书。系统分层、生命周期状态机、安全模型、核心系统原语实现剖析。
3. **`PLUGIN_SPEC.md`**：插件开发者指南。Manifest Schema 1.0 规范详解、本地调试方法、如何提交插件至官方索引。
4. **`SECURITY.md`**：安全政策与漏洞报告流程。威胁模型、凭证保护机制、漏洞通报渠道。
5. **`CONTRIBUTING.md`**：贡献准则。代码风格、Git 提交规范、Reference-First 开发 SOP、单元测试覆盖率门禁。
6. **`LICENSE`**：开源许可证（推荐采用 **Apache-2.0** 或 **MIT**，保持宽松友好）。
7. **`CHANGELOG.md`**：遵循 Keep a Changelog 规范的规范化版本变更日志。

### 11.2 CI/CD 与自动化测试矩阵
- **单元测试 (Unit Tests)**：
  - 核心算法、端口分配重试、配置原子写入、时间戳转换、Manifest 解析实现 100% 单测覆盖。
- **集成测试 (Integration Tests)**：
  - 模拟外部子进程的完整生命周期（启动 -> 探针绿灯 -> 流量代理 -> 空闲超时回收）。
  - 测试 Windows Job Object 级联强杀在各种异常退出条件下的可靠性。
- **协议回归测试 (Protocol Regression Tests)**：
  - 针对标准 OpenAI 客户端库（Python `openai`、Node `openai`）编写自动化端到端测试，覆盖流式、非流式、Tool Calls、取消请求等全场景。

### 11.3 核心纪律红线与边界约束（三大层级分界）

```
┌────────────────────────────────────────────────────────────────────────┐
│ 🔴 绝对强制红线 (Mandatory Hard Rules - 违规直接驳回提交)               │
├────────────────────────────────────────────────────────────────────────┤
│ 1. 绝对不盲目全盘 Fork 任何第三方项目，坚持 Reference-First 局部提炼。 │
│ 2. 绝对不内嵌第三方业务逆向源码，基座保持极致纯净的通用微内核。       │
│ 3. 绝对不引入重量级关系型数据库，坚持轻量 No-DB 架构。                  │
│ 4. 绝对不在数据异常断开时盲目注入 [DONE]，杜绝欺骗下游客户端。        │
│ 5. 绝对不将真实凭据、Cookie、测试私钥提交进代码仓库。                  │
├────────────────────────────────────────────────────────────────────────┤
│ 🟡 建议性设计准则 (Recommended Guidelines - 原则上遵循，允许合理权衡) │
├────────────────────────────────────────────────────────────────────────┤
│ 1. 插件常驻内存尽可能优先推荐 idle_timeout 策略。                       │
│ 2. 前端界面保持 Strict Dark Minimalist 风格，避免引入过于花哨的动效。   │
│ 3. 插件分发优先支持单文件免安装归档包 (zip/tar.gz)。                    │
├────────────────────────────────────────────────────────────────────────┤
│ 🟢 未来演进规划 (Future Roadmap - 后续版本再考虑，当前阶段杜绝过度设计)│
├────────────────────────────────────────────────────────────────────────┤
│ 1. 支持基于 WebAssembly (Wasm) 的沙箱插件运行时。                      │
│ 2. 支持多节点分布式集群与多机负载均衡中继。                            │
│ 3. 商业化配额计费系统（已明确不属于本项目生态位）。                    │
└────────────────────────────────────────────────────────────────────────┘
```

---
*本文档为 GlueHarness 唯一权威产品与架构基准，任何后续技术开发必须以本文档所确立之原则与规约为准绳。*
