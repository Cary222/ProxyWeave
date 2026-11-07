# ADR-008：Managed 插件动态端口分配与证据驱动冲突恢复

状态：已采用（spec v10 / Plan.md §6.3）。

## 背景与目标

根据 Plan.md §6.3 与架构红线：
1. Managed 插件在运行时不得回退硬编码端口（如 8080），必须在 `[20000, 60000]` 范围内动态分配回环端口。
2. 试探可用与子进程绑定之间存在必然的 TOCTOU 竞态窗口，必须在架构上严格区分“预防措施”（预防性初筛）与“恢复兜底”（证据驱动重试循环）。
3. 遵循 Direct Reuse First：优先成熟库、已有模块提取、薄适配，自研必须附带 ADR。
4. 冲突判定必须由确凿 OS 证据驱动（如 `AddrInUse` / `EADDRINUSE` / Winsock 10048），慢启动、凭据缺失或 503 饱和绝不得推断为端口冲突。

## 决策与 Direct Reuse First 分析

1. **库选型评估（Direct Reuse First 第 1 级）**：
   - 调研 `portpicker-rs`：其核心实现为无状态单次端口探测，无法表达进程级租约台账、多实例排除黑名单（blacklist）以及 ADR-003 所需的结构化 `AddrInUse` 错误证据提取。
2. **已有模块提取与复用（Direct Reuse First 第 2 级）**：
   - 复用工程既有经过严格测试的模块 `crates/proxyweave-plugin/src/port.rs` 中的 `check_port_available` 作为内核真实绑定探针。
3. **薄适配与进程级租约台账（Direct Reuse First 第 3 级）**：
   - 在 `crates/proxyweave-plugin/src/port_allocator.rs` 中实现 `PortAllocator`：
     - 范围锁定 Plan.md §6.3 A 规定的 `[20000, 60000]`。
     - 初始候选端口采用轻量 PRNG 随机退避，随后顺次步进探测，消除雷群竞争。
     - 具备引用计数的 RAII `PortLease`，生命周期随 `ManagedInstance`，Drop 或 explicit release 自动归还，杜绝泄漏。
     - 支持 `allocate_excluding`，将已冲突端口拉入启动尝试黑名单。
4. **冲突恢复循环（Plan.md §6.3 B）**：
   - 在 `RuntimeManager::start_managed` 中实现恢复循环：
     - 子进程退出时，严格排查 stderr 的 bind conflict 错误字串或退出后端口仍被占用事实。
     - 确认真实冲突证据后，将原端口加入黑名单，重新初筛，更新 `{{PORT}}` 渲染并重拉子进程。
     - 严格上限重试 3 次；若仍失败，插件标记为 `Failed` 并生成结构化 `PortConflictDiagnostic` 供 API Doctor 诊断。

## 结果与红线保证

- 0 数据库、0 凭据写入、外部 External 插件零生命周期干预。
- 保证无任何适配器特定硬编码（无特定插件特判分支）。
