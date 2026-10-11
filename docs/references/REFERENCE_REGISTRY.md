# ProxyWeave Reference Source Registry（参考源档案）

版本：v2.3 — Phase 0 Windows 基座通过；F1 已独立 Verified；F2 六具体 DEF 已 Verified；F3 已独立 Verified；F4 桌面宿主已实现并验证，真实插件与最终原生桌面 MCP 验收仍开放。

## 状态纪律

- 来源 Git/许可证核验、源码审计、构建测试、运行行为是不同证据，不能相互替代。
- 本阶段仅对实际引入的桌面闭包完成前置审计；其余 Feature 在开发前核验。
- 历史 v2.0 的全绿状态、错误完整 Hash、假设源码路径及未留证据的行为结论不再作为验收依据。
- 直接依赖 → 模块提取并保留测试 → 薄适配 → 有依据修改 → 自研例外 ADR。

## 实际 Git 基线

以下值从各本地仓库 `git rev-parse HEAD`、`git describe --tags --always`、`git tag --points-at HEAD` 与 `git status --short` 读取；此次均为干净工作区，不根据短 Hash 补写。

| 来源 | 本地路径 | 完整 Commit | 版本/标签 | 本阶段验证范围 |
|---|---|---|---|---|
| Quotio Desktop | `D:/temp-audit/quotio-desktop` | `21d75d08b38a23f97fbd534a1768a829cb147f2c` | `v0.7.7` | MIT LICENSE（版权 2025 Trong Nguyen）已读；实际桌面闭包已审计；独立副本与裁剪后 Windows 构建/测试/行为验证通过，详见 PHASE0.md |
| TiyGate | `D:/temp-audit/tiygate` | `6db40e54663455fb99d675bd8985e4dc50f68a14` | 无 HEAD 标签 | 仅 Git 基线核验；协议模块开发前再审计、测试，不视为已提取 |
| Tunnel Agent | `D:/temp-audit/tunnel-agent` | `9bfb7779b80de5e205a388e2e02d5840a3ed8b47` | 无 HEAD 标签 | 仅 Git 基线核验；不引入 C#/.NET |
| DeepSeek Harness | `D:/temp-audit/deepseek-harness` | `d743267388641bc76f17c45ce8b4c231aed1d32c` | `dsh-v0.2.1-alpha.2` | 仅 Git 基线核验；不引入 Node 作为宿主核心 |

## M01–M15 候选矩阵

| 编号 | Feature | 来源候选与原路径 | 决策/当前状态 |
|---|---|---|---|
| M01 | Process Supervisor | Win32 Job Object 官方 API (`windows-sys` 0.61.2) | Feature 2 实施中：基于 Win32 Job Object 实现 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 进程生命周期绑定；External 严禁 Kill。详见 ADR-003 |
| M02 | Port Allocator | 标准库 `std::net::TcpListener` 端口检测 | Feature 2 实施中：回环端口分配与占用预检；收集明确冲突证据（`AddrInUse`），就绪超时绝不单独推定为端口冲突。详见 ADR-003 |
| M03 | Async Gateway | axum 0.8.9 / axum-reverse-proxy 2.2.0 / Hyper 1 / Tower HTTP | F3 实施中：直接依赖原代理，原包 `D:/temp-audit/proxyweave-feature3-proxy-audit/axum-reverse-proxy-2.2.0`，MIT commit7350d118a69b6d98ae19a767b9d71630b2a3bc80；连接所有权/凭据为薄适配，无 SQLx。来源及验证边界见 ADR-004 |
| M04 | SSE | 原代理 `src/forward.rs` 的 `Body::new(Incoming)` | 同协议原字节/原 Content-Type 透传，不引入 TiyGate 业务状态机，不合成 DONE；原包测试及原生 TCP 断连回归进行中，未 Verified。见 ADR-004 |
| M05 | Protocol IR | TiyGate `crates/protocols/` 等实际路径待核验 | Phase 2 候选，保留原测试，禁止整体业务合并 |
| M06 | Model Router | TiyGate/LiteLLM 设计候选 | 未审计，不预先授权重新实现同质算法 |
| M07 | Credential Broker | Quotio `crates/quotio-platform`；`keyring` 3.6.3；`windows-sys` 0.61.2 | Feature 2 实施中：Keyring 命名空间隔离与持久化；实现真实 Windows 文件 DACL 限制（阻断继承、独占当前用户），废除空操作。详见 ADR-003 |
| M08 | Installer | Quotio `crates/quotio-core/src/proxy_download.rs` | 下载/校验/解压候选，未引入；后续按范围核验与泛化，不能整体否定 |
| M09 | Config Store | Quotio platform `atomic_write`/`replace_file` 与原测试 | 已读平台文件及对应 Windows/Unix 分支；原子写入不等于双副本容灾或 Windows 凭据隔离 |
| M10 | Manifest Engine | 直接依赖 serde_yaml_ng 0.10.0 / Serde；Quotio atomic_write | Feature 1 严格契约/本地包登记已 Inspector Verified；来源/测试见 ADR-002 与 FEATURE1.md；非运行支持声明 |
| M11 | Lifecycle | DSH 可逆副作用设计候选 | 未审计，后续按实际 Rust 依赖选型 |
| M12 | IDE Handoff | Quotio `crates/quotio-core/src/agent_config.rs` | 登记为 Phase 3 候选；不是 Phase 0 必须引入资产 |
| M13 | Desktop Shell | Quotio `apps/desktop/src-tauri/`、React bootstrap | 已直接复用；本轮 Windows 构建/测试与窗口/托盘/关闭/单实例行为验证通过；Feature 4 极简桌面宿主、IPC 胶水、网关/插件/事件管理与测试调用 UI 已实现并通过构建与测试，详见 ADR-005 与 FEATURE4.md |
| M14 | API Doctor | tower-http trace 候选 | 未引入；版本、脱敏契约与测试开发前核验 |
| M15 | Compatibility Flight | DSH 交互参考与协议测试候选 | 未引入；Phase 2 功能验收，Phase 4 发布回归 |

## 本阶段模块溯源与验证

已从 Quotio `21d75d08b38a23f97fbd534a1768a829cb147f2c` 分步引入与裁剪；原路径、目标路径及必要修改见下表。原样引入 238 文件的 SHA-256 历史快照为 `../verification/phase0-introduction.json`（不是当前文件校验清单）。

| 原路径 → 目标路径 | 复用范围与修改 | 验证 |
|---|---|---|
| `apps/desktop/src-tauri/src/{lib,main}.rs` → 同路径 | 直接提取窗口命令、单实例首注册、托盘菜单/图标/定位；去除 AppCore、业务 IPC、更新/后台采集；新增 native-close 隐藏胶水、托盘恢复复用 unminimize | 原样与提取后 Rust check/build/test 已通过；最终独立前端嵌入与桌面行为见报告 |
| `apps/desktop/src/main.tsx`、`components/AppShell.tsx`、CSS → 同路径 | 保留启动揭示计时器、原窗口按钮和关闭/最小化函数/对话框；本地偏好命名空间 ProxyWeave；状态页及小面板为薄胶水 | 裁剪后 2 frontend tests + Vite build 通过 |
| `crates/quotio-platform/src/lib.rs` → 同路径 | 通用路径/原子写入/备份/命令/Keyring 封装和原测试；命名空间调整，按调用闭包删除 CPA/Kiro/Dream-Skin 资源及专用账号常量 | Windows 原 6 测试保留；最终裁剪回归见报告；ACL/Keyring 未验收 |
| `crates/quotio-types` → 同路径 | 暂保留原类型及 7 测试，不凭类型名盲删 | 7/7 原测试通过；后续按依赖渐进裁剪 |
| Cargo/npm/Tauri/Vite/TS 配置与图标 → 同路径 | 内部 crate 名保留；展示名/独立 ID 调整；移除资源包/更新器/业务依赖并提交锁文件；图标暂保留上游资产 | 各步骤构建及最终冻结证据见报告 |
详细过程与日志证据见 `../verification/PHASE0.md`；准备成功或 metadata 成功不能登记为构建通过。

## Feature 1 直接依赖与登记胶水（已独立 Verified）

| 成熟来源/版本 | 发布源码 Commit | 原路径 → 目标使用 | 许可证/必要适配 |
|---|---|---|---|
| serde_yaml_ng 0.10.0 | 45bce7bd7efb754ffc9dc910f3d1bae7edd0adf4 | 发布 `src/de.rs`/`src/mapping.rs`、`tests/test_error.rs` → 直接 crate 依赖 | MIT；不改源码，严格类型/映射预检胶水；原测试Rust1.99兼容lint仅独立审计使用 |
| sha2 0.10.9 | 82c36a428f8d6f05f3bfccdedb243e9d1f85359d | RustCrypto `sha2` → 直接依赖 SHA256 | MIT OR Apache-2.0；无散列算法重写 |
| semver 1.0.28 | 7625c7aa3f0e8ba21e099d1765bcebcb72aa8816 | Version::parse → 直接依赖 | MIT OR Apache-2.0 |
| url 2.5.8 | d6ea13c5f8e7e6e627f6390161b3e185bda5e5ce | URL origin/parser → 直接依赖 | MIT OR Apache-2.0 |
| tempfile 3.27.0 | 5c8fa12eb584931b4f1bccfde87eb72fbfa7dc61 | 原 TempDir/Builder及原测试 → staging目录 | MIT OR Apache-2.0 |
| 已采用 Quotio platform | 21d75d08b38a23f97fbd534a1768a829cb147f2c | `atomic_write`/`replace_file` → 当前同路径直接调用 | MIT；登记非敏感，原6测试保持，非Windows ACL验收 |

以上 Commit 来自各发布包 `.cargo_vcs_info.json`，非补写短Hash。原发布代码/测试在独立副本实跑，结果及失败归因见 `../verification/FEATURE1.md`；测试执行中不作提前通过声明。产品契约/边界胶水位于 `crates/proxyweave-plugin/`；对照 Hermes固定源因BSL许可证及IDE业务契约不匹配未提取，见 ADR-002。wb2api候选exe/buildinfo/dirty差异见同ADR；无真实凭据/Ready/推理验收。

## 本地插件与隐私边界

| 资产 | 本地路径 | 本阶段状态/约束 |
|---|---|---|
| wb2api | `D:/workbuddy2api-panel` | Phase 1 首个 Managed 候选；此轮未编译/探测，不沿用历史验证声明 |
| 已有 CPA | `D:/CLIProxyAPI_7.2.115_windows_amd64` | 本地已有二进制，28317 未于此轮探测；不能推断无公开源码或专有许可证。作为 External 接入时禁启停禁 Kill |
| 历史 REF | `D:/workbuddy-switch-gateway/workbuddy-switch-gateway-latest` | 用户禁止复制代码；仅避坑经验参考，不作为本轮代码来源 |
| 用户真实账号/配置 | 用户私有目录 | 本阶段无需读取，不读取/复制/提交真实凭据 |

Managed/External 由进程所有权决定，不由源码可得性决定。所有 manifests 是待运行验证草案；Unix `0o600` 与 Windows ACL 分开设计验证。
