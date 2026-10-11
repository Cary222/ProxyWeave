# Feature 1：Manifest 与本地登记（实施中，未 Verified）

## 范围

新增 `crates/proxyweave-plugin`：严格 Schema 1.0 子集、Managed/External 所有权、显式本地包文件/完整性校验、登记幂等/冲突、非敏感设置持久化/重载。Tauri 只增加 import/list/save-settings 异步阻塞池 IPC 和独立 profile 根；无启动/停止/网关接口，未启动 wb2api。UI 导入入口留待 Feature 4；当前 CLI 示例为独立复验入口，不宣称用户桌面闭环完成。

来源/策略见 [ADR-002](../architecture/ADR-002-local-plugin-contract.md)。原 wb2api 目录不修改，auths/data/log 内容不读取；现成 exe 的 Git buildinfo 与 dirty 差异如实记录，非干净来源构建声明。

## 合同与明确拒绝

- `schema_version` 仅 1.0；metadata id/name/semver/license/source_commit；runtime ownership 二选一；Managed 明确 executable/hash/args/env/resources，External 只有 HTTP origin，不含 owned executable/secret projection/idle/port retry。
- health_check path/status/identity-header/value/timeout；业务 Ready 固定要求 HTTP200+声明身份，503 不等于崩溃或冲突；Feature 1 只声明、不发探针。
- capabilities 仅本轮 openai_chat 同协议模型/推理路径与 streaming 声明；不假定模型列表、不做协议转换。未知键、重复键、多 YAML 文档、未支持协议、非法模板明确拒绝，不静默丢字段。
- `manifests/cpa.manifest.yaml` 与 `manifests/wild-work.manifest.yaml` 是历史规划草案，包含本轮未支持字段，当前严格引擎会拒绝；Phase0 PyYAML语法通过仍成立，但不表示这些草案可导入或对应能力已实现。本轮可导入契约只按 actual `wb2api.manifest.yaml` 和明确二选一所有权规范构建。
- 配置声明 non-sensitive scalar settings、JSON config_template、credential_files 模板及 required_secrets 名称。登记接口没有 secret values 入参；Feature 2 才实现安全配置/Keyring/ACL。插件运行能力未实现。
- 包文件采用可移植相对路径，拒绝 traversal/盘符/绝对路径/ADS/Windows设备名/链接及重解析点；每个显式文件 SHA256 对照，限每文件256MiB/总512MiB，Manifest256KiB。不复制未列出文件。
- 非敏感登记用已复用 Quotio atomic_write；staging 来自 tempfile，完成后发布整个包目录。同id/同内容幂等，不同内容明确冲突不覆盖。损坏记录或已登记exe篡改重载失败，不报告成功。
- 本机同用户具有文件修改权限不是恶意同用户OS沙箱，path检查+哈希不宣称防所有竞态攻击；Feature2私有ACL另验。

## 可重复命令（独立 git archive 副本）

```cmd
set RUSTUP_HOME=D:\build-tools\rustup
set CARGO_HOME=D:\build-tools\cargo
set PATH=D:\build-tools\cargo\bin;%PATH%
set CARGO_BUILD_JOBS=1
cargo test -p proxyweave-plugin --locked -j 1 -- --test-threads=1
cargo check --workspace --locked -j 1
cargo test --workspace --locked -j 1 -- --test-threads=1
cargo fmt --all -- --check
cd apps\desktop
npm ci
npm test
npm run build
npm run tauri -- build --debug --no-bundle
```

独立操作无运行副作用：`cargo run -p proxyweave-plugin --example local_registry -- <isolated-profile> import <package>/manifest.yaml`；再导入一次应 already_registered=true，`list` 重开仍保留；`settings <id> <non-sensitive-JSON>`。不要将凭据传命令行。包制作只复制审核过的 exe/LICENSE/Manifest，不复制原 config/auths/data。

桌面隔离：`set PROXYWEAVE_PROFILE=<absolute-directory-outside-repo>` 后运行内嵌产物。当前 UI 仍 Phase0，只有后端登记IPC；没有假显示 Ready/调用结果。最终 UI/MCP 用户验收在 Feature4。

## 执行结果与冻结

- 依赖调用烟测 `bcab2b542`：退出 0，valid/duplicate/unknown/multi-document 四项通过。
- wb2api 独立源码副本 `bb2e521dd`：退出 0，`go test ./cmd/server ./internal/auth ./internal/httpauth ./internal/server` 四个包通过；未启动原server或读原auths/data。副本仅保留170 tracked文件及已公开记录的go.mod direct标记差异。
- 首轮 `bb25b4eb4`：8/10通过，两个 Windows 测试断言不准确（canonical extended-path前缀、junction被标准库识别成symlink的措辞），修正断言后继续回归，不降低路径拒绝。
  - 完整首次失败输出保留在 `feature1-first-assertion-failures.txt`（SHA256 `ad7319adae94dc35ed5ac4a9055249a1dad2a3e6f1feeacdc01e648330f44ab3`）；原断言/修正对照见 `feature1-assertion-adjustments.diff`。数据叶目录尚未创建，仅将预期的现存父路径 canonicalize 后接同名叶；未削弱包根 starts_with 或 link/reparse 拒绝。Junction测试仍要求 Err + list空，不仅检查措辞。
- 原依赖 `bb1005826`：原serde_yaml_ng顶层测试在Rust1.99的 dangerous_implicit_autorefs deny 阻断，属于原版本/新编译器兼容；仅隔离原测试命令 `RUSTFLAGS=-A dangerous_implicit_autorefs` 兼容，不改其源码。作为应用依赖通过正常编译（Cargo cap-lints）。`b44cc176a` 原unit/integration通过，doc-test并发触发std metadata/link环境失败，完整文档测试未通过；`bcb35bdb4` 原 lib/tests 串行回归退出 0：serde_yaml_ng 91、sha2 8、semver 34、url 67、tempfile 60 个测试通过；仅 lib/integration 范围，doc-test 完整验证仍未通过。sha2 deprecated 及 semver cfg 测试警告保留在日志，不重写原库。
- `be5f87d99` 因新增桌面依赖使Cargo.lock待更新而 --locked拒绝，后续检查未执行；针对性更新锁，再冻结 --locked回归。
- `b3a3513f3`：退出 0，应用契约 13 tests + workspace check 通过；不传 `RUSTFLAGS` 兼容豁免。
- `bade692ad`：退出 0，真实候选 wb2api 包在隔离 profile 首次导入 false、重复导入 true、重载 list 持久化；fingerprint=`0d3c10f437d3fa3aa2e3c9250f9d50a87c3224af1322618fe9e2b4c9bd8eb13d`。未运行 exe，不等于 Ready/推理。
- `b7b8a0326`：fmt check 已过，Clippy 因环境缺组件终止；`bd04ec4c0` 安装退出 0 后单独补验，不宣称该失败链的后续 build/test 执行过。
- `b3bcb803d`：退出 0，冻结 fmt/check/workspace test 全通过：13 plugin + 6 platform + 7 types = 26 Rust tests；npm ci、2 frontend tests、TypeScript/Vite build、embedded debug Tauri build 全通过，无 build warning/error。不是 Managed/网关/完整用户操作验收。
- `bd555c7a8`：Clippy `-D warnings` 全依赖范围被未改的 Quotio types 5 个历史 lint 阻断（4 个 derivable_impls、1 个 too_many_arguments），属上游基线 lint；不为此重写/扩大修改旧类型。新 crate 使用 `--no-deps` 单独严格 lint，并将基线 lint 保留为已知事项。

## 原成熟依赖测试复验（不改原源码）

Cargo cache 发布包复制到 `D:/temp-audit/proxyweave-feature1-upstream-tests/{crate-version}`；`.cargo_vcs_info.json` Commit 与许可证见登记表。独立复验使用以下命令（RUSTFLAGS 只针对原库顶层测试的新版编译器 lint，不用于产品）：

```cmd
set RUSTUP_HOME=D:\build-tools\rustup
set CARGO_HOME=D:\build-tools\cargo
set PATH=D:\build-tools\cargo\bin;%PATH%
set CARGO_BUILD_JOBS=1
set RUSTFLAGS=-A dangerous_implicit_autorefs
cargo test --manifest-path <published-source-copy>\Cargo.toml --lib --tests -j 1 -- --test-threads=1
```

逐个 `serde_yaml_ng-0.10.0`、`sha2-0.10.9`、`semver-1.0.28`、`url-2.5.8`、`tempfile-3.27.0`。原源码的 `src/`、`tests/` 不变；生成 Cargo.lock/target 不属于来源改动。doc-test 未全过单独记为未验证，不据此重写库。

实跑副本 `src/` 与 `tests/` 已逐文件字节对比缓存发布源，五库均完全一致（28/22/14/12/19 文件）。可由 `.cargo_vcs_info.json`、Cargo.lock registry checksum 与原测试命令重现；测试命令只生成独立 lock/target，没有移植后等价重写。

## 非敏感实际插件包收据

`D:/temp-audit/proxyweave-wb2api-package` 仅三文件：Manifest SHA256 `c2857bdf9bf27643cce96a356f7c38aaa03f858ca29289a02fdd3011b2f8e72b`；`LICENSE` SHA256 `49b9e27ddfc7cceab55e19eb7d405bda30a8229de3cc094e6b4d0397ab76a34b`；`bin/wb2api.exe` SHA256 `38c4d8e0421c6c8c24ec738459899c13d07e9a56730db3394a4021e460c1d485`。重新制作只从审核 exe/LICENSE 与冻结 `manifests/wb2api.manifest.yaml` 复制，不包含任何 auths/data/config/log；Manifest字段不是运行验收证明。

可复验制作命令：`node docs/verification/prepare-wb2api-package.cjs D:/workbuddy2api-panel <new-absolute-package-directory>`，逐个校验三个固定 SHA256 再写入，拒绝已有目录；本轮实跑到 `D:/temp-audit/proxyweave-feature1-review-package` 成功。Inspector 自选其独立目录，不依赖开发 profile。

## 最终端到端门禁收据

- 最终统一冻结门禁 `bae01159c`（退出码 0）：
  - `cargo fmt --all -- --check`：通过，无格式差异
  - `cargo clippy -p proxyweave-plugin --all-targets --no-deps --locked -j 1 -- -D warnings`：通过，0 warnings / 0 errors
  - `cargo test --workspace --locked -j 1 -- --test-threads=1`：通过，26 tests passed（13 plugin + 6 platform + 7 types）
  - `cd apps/desktop && npm test`：通过，2 tests passed
  - `npm run build`：通过，Vite 生产构建完成
  - `npm run tauri -- build --debug --no-bundle`：通过，内嵌调试壳构建完成，0 warnings / 0 errors
- 独立三文件真实包制作与离线登记复跑已闭环，非敏感证据文件哈希与 diff 均已归档并固化入库。
