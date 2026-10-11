# ADR-002：Windows 本地插件包、Manifest 与登记闭包

状态：实施中；Feature 1 未独立 Verified，不授权依赖运行行为。

## 范围与决策

承接已冻结 Phase 0（70b4ae145afee9860b2257eb386005c291549661），不重建桌面壳。按 Feature 1 → 2 → 3 → 4 开发并逐个交付可检出提交；审查员在独立副本复验。最终实际 Windows 用户操作须由 Inspector 通过对应 MCP 全项明确 Verified，才可申请目标完成。真实模型/凭据缺失不能以 Mock 替代。`docs/DEFECTS_LEDGER.md` 及 `.pi/inspection/**` 不由开发会话写入。

本地导入只接受 Manifest 及明确列出的文件：当前 Windows 可执行文件、可选非敏感资源、许可证。不自动识别/编译任意源码项目，不递归搬运源码目录或 auths/data。每个文件通过 SHA256 校验后复制到宿主登记目录；用户配置/凭据与包、源码分离。相同 id/相同内容重复导入幂等；相同 id 不同内容拒绝覆盖，初版不做在线升级。

Manifest 使用严格类型，未知字段/重复键/多文档拒绝，限制大小；只支持本轮声明的 Schema 1.0 子集。Managed（宿主启动并拥有）与 External（只挂载端点）使用互斥运行时分支，绝不以源码可得性区分。路径使用可移植相对路径，拒绝 traversal、绝对路径、Windows ADS/设备名、链接/重解析点；不复制未声明文件。登记采用已复用平台原子写入和每包目录发布，不重新实现原子覆盖算法。

## Direct Reuse First 前置比较

- 已采用：Quotio v0.7.7 / 21d75d08b38a23f97fbd534a1768a829cb147f2c，`crates/quotio-platform/src/lib.rs` → 当前同路径 `atomic_write`/Windows `MoveFileExW` 与适用原测试。非敏感登记使用 `sensitive=false`；不以此宣称 Windows ACL。
- Serde 已在工程使用，直接依赖 derive + deny_unknown_fields；不手写 YAML 解析器。候选 `serde_yaml` 0.9.34+deprecated 已缓存但停止维护；选择 `serde_yaml_ng` 0.10.0（MIT）直接依赖，审阅发布源 `src/de.rs`、`tests/test_error.rs` 的重复键/深度/alias repetition 保护，先在独立依赖审计工程运行原测试与调用烟测。
- SHA256：直接依赖已在 lock 中的 RustCrypto `sha2` 0.10.9（MIT OR Apache-2.0），不手写散列算法。
- 对照 Hermes IDE `f9445bff4dc21247cdd0ba19b274b988fcfc9194` `src-tauri/src/plugins.rs` 与 LICENSE（公开源码）：LICENSE 为 Business Source License（未来 Change License Apache-2.0），不将其误报为 MIT/无条件开源；其产品插件目录/manifest_json 登记不是本项目 Managed 黑盒运行契约，许可证及依赖边界均不宜提取；本项目只写契约验证、文件边界和调用既有库的必要胶水，不复制其实现。实际发布源码页面/依赖文档已查看，候选不等于复用。
- 官方资料：Serde attributes、serde_yaml_ng 发布源码/测试、sha2 API；成熟库保留其源码与测试，应用层针对本契约增加边界回归。需自行定义的字段与登记策略是本产品契约，ADR 在此明确范围，非重写成熟系统机制。

## 真实 wb2api 核验（不运行、不读取原凭据）

- Git HEAD：84614197edb08068cd9dabf471b0ebef2a27af0b；describe：v1.13.0-3-g8461419；源码 appVersion=1.13.0-panel。
- MIT：Copyright (c) 2026 Sliverkiss；Copyright (c) 2026 linguo2625469。
- 工作区不干净：go.mod 将 redis/go-redis/v9 从 indirect 调整为 direct；另有未跟踪 gateway.log。未修改原目录，不读取 auths/data/log 内容。
- 候选 exe SHA256：38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485；Go buildinfo go1.27.1、windows/amd64、CGO_ENABLED=0、vcs.revision=上述 HEAD、vcs.modified=true。不得称其为干净 commit 构建。
- 原 README/CI 的构建：`go build -trimpath -ldflags="-s -w" -o wb2api.exe ./cmd/server`；当前现成产物 `-h` 成功退出 0，只支持 `-config`，没有 `-port`。单二进制内嵌前端，不需运行仓库源码。
- 实际配置是 JSON；`listen`/`api_key`/`auth_dir`/`state_file`，可由 WB2A_LISTEN/WB2A_API_KEY/WB2A_AUTH_DIR/WB2A_STATE_FILE 覆盖。宿主预投影完整配置且 fail-closed，不能触发缺失配置自动生成并在日志输出 api_key 的分支。关闭原默认后台账号任务，避免测试账户额外副作用。
- `/healthz` 无鉴权，X-Service=workbuddy2api；Pool.ServableNow() 为 false（缺账号或饱和）返回 503。进程存活、HTTP 可达、业务 Ready 分别记录；503+身份不是 Ready，也不是端口冲突/崩溃证据；Busy 时不得因饱和503重启/中断流。
- 路由：GET /v1/models、POST /v1/chat/completions（Bearer api_key）。模型实际发现，不从草案假定模型名称。凭据只经用户安全入口注入宿主数据，原上游 auth parser 支持嵌套 auth/account 或扁平 accessToken/uid；文件须 workbuddy*.json，不读取现存账户。

## 验证门禁

Feature 1：严格 YAML/语义、包完整性、路径边界、幂等/冲突、登记重载/配置原子写入/失败保持原状态，既有 Rust/TS/桌面构建回归。Feature 2 前独立验证 Windows Job Object fixture，并等待审查门禁；首次 wb2api 启动不得先于此。后续 Windows ACL/Keyring、取消/异常、真实推理与桌面 MCP 逐项报告，不将局部 Phase 2/3 标整阶段完成。
