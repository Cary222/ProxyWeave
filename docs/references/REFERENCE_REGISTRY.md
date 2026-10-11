# ProxyWeave Reference Source Registry（参考源档案）

版本：v2.1 — Phase 0 本轮 Windows 最小桌面基座通过；Phase 1 待启动。

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
| M01 | Process Supervisor | shared-child；Win32 Job Object 官方 API | 后续核验具体版本、实现与测试；shared-child 本身不等于进程树 Job Object 回收。External 禁 Kill |
| M02 | Port Allocator | portpicker 候选 | 后续版本/许可证/测试核验；绑定探测后释放仍有 TOCTOU；就绪超时不单独证明端口冲突 |
| M03 | Async Gateway | Axum/Hyper、TiyGate server | 待 Feature 审计；不采用 Quotio 阻塞 bridge，不引入 SQLx |
| M04 | SSE | TiyGate protocols、eventsource-stream 候选 | 待 Feature 审计；协议正常完成证据优先，异常不补 DONE |
| M05 | Protocol IR | TiyGate `crates/protocols/` 等实际路径待核验 | Phase 2 候选，保留原测试，禁止整体业务合并 |
| M06 | Model Router | TiyGate/LiteLLM 设计候选 | 未审计，不预先授权重新实现同质算法 |
| M07 | Credential Broker | Quotio `crates/quotio-platform/src/lib.rs`；keyring 3.6.3（原配置） | 已读平台文件。含业务命名空间，Windows sensitive 权限为空操作；Keyring/ACL 功能本阶段不宣称已验证 |
| M08 | Installer | Quotio `crates/quotio-core/src/proxy_download.rs` | 下载/校验/解压候选，未引入；后续按范围核验与泛化，不能整体否定 |
| M09 | Config Store | Quotio platform `atomic_write`/`replace_file` 与原测试 | 已读平台文件及对应 Windows/Unix 分支；原子写入不等于双副本容灾或 Windows 凭据隔离 |
| M10 | Manifest Engine | Rust YAML 解析库待选择 | Phase 1 首个 Feature：Schema 与所有权/凭据契约；当前仅样例 PyYAML 6.0.3 语法与正则检查通过，非插件运行验收 |
| M11 | Lifecycle | DSH 可逆副作用设计候选 | 未审计，后续按实际 Rust 依赖选型 |
| M12 | IDE Handoff | Quotio `crates/quotio-core/src/agent_config.rs` | 登记为 Phase 3 候选；不是 Phase 0 必须引入资产 |
| M13 | Desktop Shell | Quotio `apps/desktop/src-tauri/`、React bootstrap | 已直接复用；本轮 Windows 构建/测试与窗口/托盘/关闭/单实例行为验证通过；其他平台未验证 |
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

## 本地插件与隐私边界

| 资产 | 本地路径 | 本阶段状态/约束 |
|---|---|---|
| wb2api | `D:/workbuddy2api-panel` | Phase 1 首个 Managed 候选；此轮未编译/探测，不沿用历史验证声明 |
| 已有 CPA | `D:/CLIProxyAPI_7.2.115_windows_amd64` | 本地已有二进制，28317 未于此轮探测；不能推断无公开源码或专有许可证。作为 External 接入时禁启停禁 Kill |
| 历史 REF | `D:/workbuddy-switch-gateway/workbuddy-switch-gateway-latest` | 用户禁止复制代码；仅避坑经验参考，不作为本轮代码来源 |
| 用户真实账号/配置 | 用户私有目录 | 本阶段无需读取，不读取/复制/提交真实凭据 |

Managed/External 由进程所有权决定，不由源码可得性决定。所有 manifests 是待运行验证草案；Unix `0o600` 与 Windows ACL 分开设计验证。
