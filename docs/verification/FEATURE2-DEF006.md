# Feature 2 DEF-006 不透明模板数据与安全 Debug 修复

状态：开发门禁通过，ReadyForReview；尚未 Inspector Verified。父提交为 DEF-005 冻结 `1dd2bcd7fc698e6ab4c394a746e0f737a6fc25f7`；Inspector 已独立确认 DEF-001～005 的具体修复，不扩大为 F2 整体或真实插件/原生桌面验收。

## 根因与限定修复

- `crates/proxyweave-plugin/src/projection.rs` 原先对不断变化的输出重复 String::replace，然后在输出里查找 SECRET。凭据中合法的模板形文本被重新解释，BTreeMap 键序影响结果；API Key/路径数据也可能被解释或被当缺少 secret 拒绝。
- 直接依赖锁文件已有 `regex = "=1.12.3"`，复用官方 `Regex::replace_all` 的 Fallibility 示例：迭代原始输入的非重叠匹配，复制输入字面段，逐个查询固定标准 token 或 secret，再 push_str 插入不透明值。输出永不被再次匹配；不用正则 replacement string，`$1` 也不会扩展。
- 薄适配：不需要 capture，采用 find_iter；只允许 Manifest 已支持的 token；原模板不匹配/嵌套/多余花括号和真正缺少 secret 安全失败。错误使用固定类别，不回显 token 名或已插入数据。
- `ProjectionContext` 不再派生 Debug。手写 Debug 仅公开 port 和非穷尽标记，省略所有 supplied id、路径、API Key 与 secret 值（包括 secret 名）。保留 Clone 与现有 public API。
- 不改 RuntimeManager、HTTP probe、Win32 Job、atomic_write/DACL、Manifest schema、External 生命周期、F3 草案及审查端资产。

## Direct Reuse 溯源

- 原发布包：`D:/build-tools/cargo/registry/src/rsproxy.cn-e3de039b2554c837/regex-1.12.3`，`.cargo_vcs_info.json` 记录 commit `b028e4f40eac8959d05e82abf8404906b1c565c0`；许可证 `MIT OR Apache-2.0`，本次按 MIT 使用，Copyright 2014 The Rust Project Developers。
- 官方可失败替换算法：`src/regex/string.rs:776-808`；保留原输入匹配、边界段复制、失败短路和尾段复制。官方原测试 `tests/replace.rs` 包含替换、closure 与 NoExpand/$ 字面值覆盖。
- 原包完整复制到 `D:/temp-audit/proxyweave-def006-regex-original`：56 个文件逐字节比较一致。`Cargo.toml` SHA256 `a41c45254782e4ae11fe4d213a0cb4d17ee0f6bc094eea90dd331ad6cbaff837`；`Cargo.lock` SHA256 `f6d82a7e9d794d96d162b50dcf3099b27966c2a80122510e54b4bf74c54df4ef`；`src/regex/string.rs` SHA256 `9f7686e10535fe385a767063132d39ee1a1af1a20a119d78df479f110822e274`；`tests/replace.rs` SHA256 `78ff9bf7f78783ad83a78041bb7ee0705c7efc85b4d12301581d0ce5b2a59325`；`LICENSE-MIT` SHA256 `6485b8ed310d3f0340bf1ad1f47645069ce4069dcc6bb46c7d5c6faf41de1fdb`。
- 不提取/改写 regex 搜索器，不引入完整模板框架。原测试在隔离 target 运行，不修改发布源码/测试断言，不将其独立依赖版本混入 workspace。

## 回归范围

1. 原模板 a→z / z→a 的两种键序，重复 SECRET 和全部标准 token；替换值含不存在的 SECRET、标准 token、花括号、Unicode、美元及引用文本仍完全不透明。
2. 原模板 missing/unknown/malformed 拒绝；错误不回显已插入值或未知名字；普通/pretty Debug 不含 supplied data。
3. JSON 嵌套/数组/数字/布尔/null 保持类型；序列化后重新解码的凭据 UTF-8 bytes 与输入一致；不渲染 JSON 对象键，沿用现有契约。
4. 实际 Owned Rust harness Managed 子进程：随机回环端口，独立 Keyring namespace，仅 dummy 凭据；真实 args/path 和 env/key/secret 经生产 RuntimeManager 渲染；config.json 与 credential JSON 由既有敏感 atomic_write 写入；子进程捕获用于逐字节核对，stop 后 Windows OS 确认 PID 退出。
5. 相同真实调用路径中的 missing-secret 触发 ConfigProjectionFailed 与缓存 Failed，错误/状态不带 dummy 凭据，spawn 次数不增加；Keyring 项显式删除并核对不存在，Drop 提供失败路径清理。

上述 fixture 捕获文件只含测试 dummy 值，存在隔离 tempfile 下，不是生产日志功能；没有使用真实上游凭据，也没有宣称所有日志或插件自身 stdout/stderr 全面安全。

## 门禁

定向 task `b54a19f54` terminal exit 0：3 个 DEF-006 lib 测试 + 1 个真实 Managed args/env/Keyring 测试通过；plugin/platform all-targets Clippy `--no-deps --locked -j 1 -- -D warnings` 零警告。原始收据 `feature2-def006-targeted.txt`。之后额外增加了真实 missing-secret/状态/Keyring 显式清理断言，定向轮不能当作这些新增断言通过；由下述完整门禁核验最终源码。
原包第一轮 task `b750117b2` terminal exit 1（cmd 将 cargo 非零归一为 1）：原发布包有意排除 `tests/fuzz/mod.rs`，且不含 `testdata/fowler/{basic,nullsubexpr,repetition}.toml`，因此 `tests/lib.rs` 编译报 `error[E0583]: file not found for module fuzz` 与 3 个 include_bytes 文件缺失；无测试运行，doctest 也未执行。同时产生原测试的 4 个 std::usize::MAX 弃用警告，不是 workspace 门禁警告。原始收据 `feature2-def006-original-package-failure.txt`。归因：发布包测试资产缺失（不是宿主适配失败）；不删断言或修改原测试。改以独立 Cargo test target 直接引用原 `tests/replace.rs`（该文件自包含），只验证替换子集；随后在未改原包中单独运行 replace_all doctest。完整 integration suite 仍未验证。
独立子集 task `b91702d2e` terminal exit 0：未修改原 tests/replace.rs 的 26 个测试通过；原包 replace_all 的 2 个 doctest（含 Fallibility 示例）通过，其他 240 个 doctest 被过滤。替换 harness 的初次解析选择了 regex-automata 0.4.18 / regex-syntax 0.8.11 / aho-corasick 1.1.5 / memchr 2.8.3；原包 doctest 则沿用发布锁 0.4.14 / 0.8.9 / 1.1.3 / 2.7.6。因此该轮不是全链路相同依赖验证，另做以下精确四版本补充轮；workspace 锁及原包所有 56 个文件均保持不变。原始收据 `feature2-def006-original-subset-pass.txt`。
精确原包依赖 task `bf4a9f93c` terminal exit 0：harness 四依赖精确降到原发布 Cargo.lock 的 0.4.14 / 0.8.9 / 1.1.3 / 2.7.6 后，26 个原替换测试 + 2 个官方 doctest 再次通过，未修改任何原源码/断言/锁。原始收据 `feature2-def006-original-exact-pass.txt`；最后重验原包 56 文件逐字节一致。此处精确指原包依赖，并非 workspace 依赖（差异已单列）；完整原 integration suite 仍未验证。
完整 task `bb880e31d` terminal exit 0：fmt check、workspace locked 默认并行 Rust **56 passed / 0 failed / 1 fixture ignored**（plugin41=lib11+registry13+runtime_defects13+lifecycle4，platform8，types7）、scoped all-targets Clippy 零警告、npm ci、Vitest2、tsc/Vite 与 embedded Tauri debug --no-bundle 均通过。包括新增真实 missing-secret/状态/Keyring 清理断言，沿用既有 Protected DACL/Job/External/DEF-001～005 回归，不调整全量 test-threads。原始收据 `feature2-def006-full-pass.txt`。本轮桌面构建仅证明基座可构建，不是 F2 UI 或原生 MCP 操作验收。
workspace 对比父提交仅新增 plugin→regex 依赖边，版本无变：regex1.12.3 / regex-automata0.4.14 / regex-syntax0.8.10 / aho-corasick1.1.4 / Tokio1.52.3 / reqwest0.13.4 / Keyring3.6.3。
LSP 主动检查 projection.rs / runtime_defects.rs：2 个文件均 inconclusive，零诊断不是 clean；最终以真实测试/构建门禁为准。

## 复验命令

```cmd
set "RUSTUP_HOME=D:\build-tools\rustup"
set "CARGO_HOME=D:\build-tools\cargo"
set "PATH=D:\build-tools\cargo\bin;%PATH%"
cargo fmt --all -- --check
cargo test --workspace --locked -j 1
cargo clippy -p proxyweave-plugin -p quotio-platform --all-targets --no-deps --locked -j 1 -- -D warnings
cd apps\desktop
npm ci
npm test
npm run build
npm run tauri -- build --debug --no-bundle
```

独立原包：使用其未修改 Cargo.lock，设置 `CARGO_TARGET_DIR=D:\temp-audit\proxyweave-def006-regex-target` 后分别执行 `cargo test --locked -j 1 --test integration replace::` 与 `cargo test --locked -j 1 --doc regex::string::Regex::replace_all`。

发布 integration 缺少资产时，不改原包：在相邻 `proxyweave-def006-replace-harness/Cargo.toml` 建立如下 test target，并对 harness 的锁按原包四版本执行 cargo update --precise（原包锁和源码不改）后 `cargo test --offline --locked -j 1 --test original_replace`：

```toml
[package]
name = "proxyweave-def006-original-replace-tests"
version = "0.0.0"
edition = "2021"
publish = false
[dependencies]
regex = { version = "=1.12.3", path = "../proxyweave-def006-regex-original" }
[[test]]
name = "original_replace"
path = "../proxyweave-def006-regex-original/tests/replace.rs"
```

## 冻结边界

限定 Cargo.lock、`crates/proxyweave-plugin/Cargo.toml`、`src/projection.rs`、`tests/runtime_defects.rs`、`THIRD_PARTY_NOTICES.md`、`docs/references/licenses/Regex-MIT.txt`、本报告与原始门禁收据。不 stage F3、ADR index/registry 草案，绝不修改 `docs/DEFECTS_LEDGER.md` 或 `.pi/inspection/**`。最终 commit / SHA256 待门禁后交付，不在报告中嵌入自身 hash。
