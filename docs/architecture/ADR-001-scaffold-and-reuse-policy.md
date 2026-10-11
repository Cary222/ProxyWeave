# ADR-001：主脚手架采用与直接复用政策

状态：已采用；Phase 0 本轮 Windows 基座已通过。证据见 `docs/verification/PHASE0.md`；构建与行为验证不能由准备状态替代，其他平台未验证。

## 背景与真实基线

主工程 `D:/ProxyWeave` 保留独立 Git 历史。参考工程 `D:/temp-audit/quotio-desktop` 不修改原始源码。
从 Git 实际核验 Quotio HEAD 为 `21d75d08b38a23f97fbd534a1768a829cb147f2c`，对应标签 `v0.7.7`，工作区干净。许可证为 MIT，版权 `Copyright (c) 2025 Trong Nguyen`。

## 决策

1. 先在独立副本验证原工程 npm 锁文件安装、前端构建/测试、Rust 构建/测试与 Windows 窗口、托盘、关闭、单实例。
2. 再直接引入构建闭包；源码引入、必要标识调整、业务裁剪分轮验证。保留适用的上游壳与平台实现，不生成等价代码冒充复用。
3. 内部 crate 名允许保留；应用展示名与 identifier 需独立。保留必要测试、版权与依赖锁文件。
4. 按真实调用关系裁剪 AppCore、types、vendor、SQLite 与专有业务。仅保留桌面生命周期确实需要的状态；不重建核心框架。
5. `agent_config.rs` 是 Phase 3 候选；`proxy_download.rs` 的下载、校验、解压是后续候选，不因 CPA 常量否定其全部价值。
6. 每个 Feature 按直接依赖 → 模块提取 → 薄适配 → 有依据必要修改 → 自研例外（另立 ADR）的顺序实施。错误先归因原基线、遗漏、版本、适配或环境。

## 边界与验证

Phase 0 仅交付当前 Windows 最小桌面基座，不实现网关、市场、协议转换或 IDE 配置。其他平台未实测必须标注。
Managed/External 按进程所有权区分；External 永不收到启停或 Kill。Manifest YAML 解析仅为语法检查，不是运行验收。
Unix 文件权限和 Windows ACL 分别设计；上游 Windows sensitive 权限空操作不作为凭据隔离依据。
各引入模块的来源、目标、范围、改动及验证结果登记于 `docs/references/REFERENCE_REGISTRY.md` 与 Phase 0 验证记录。

## 取舍

接受暂时保留上游内部命名，以减少改名与裁剪混杂风险。采用原代码及成熟 Tauri 插件替代从零编写系统机制；拒绝全量继承业务，也拒绝先删后重写。
