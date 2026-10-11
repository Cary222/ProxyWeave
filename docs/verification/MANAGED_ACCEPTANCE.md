# 首个本地 Managed 插件目标：实施与独立验收契约

Goal：`mv44jirh-kepwhn`（regular；完整原始 objective 保存在 goal 系统中，不自行修改）。

## 完成门禁

- 按 Feature 1 → 2 → 3 → 4 实施，每个 Feature 提供独立 Git commit 或完整 SHA256 冻结快照；来源版本、原/目标路径、必要适配、原测试与本地回归命令、未通过项及独立复验步骤齐全后送 Inspector。
- 自测通过不等于 Verified。送审后冻结相关文件；审查通过前不启动依赖该 Feature 的运行行为。Job Object 先独立 fixture 验证并获得门禁，再首次启动 wb2api。
- 最终复验对象是实际 ProxyWeave Windows 桌面客户端，不是 Pi Web、浏览器壳、仅 API 脚本或 MCP 工具可用性。Inspector 必须经对应桌面 MCP 亲自点击/输入/导入/配置/启动/停止/重启/重开完成全部用户流程并明确最终全项 Verified，才可申请 goal complete。
- 最终交付冻结源码、对应内嵌 Windows exe 指纹、可导入真实插件包指纹、非敏感操作说明、隔离测试 profile 启动方式、真实 E2E 复验步骤。凭据经安全配置入口或 Keyring 注入，不在报告/intercom中明文传送，不读取/搬运原插件 auths/data。
- 有效测试凭据/上游缺失时真实 Ready/推理保持未通过，继续其他独立工作并如实报告阻塞。goal blocked 遵从工具生命周期：同一阻塞连续三个 goal turns 后设置；不以 Mock 完成真实请求。
- 开发会话不写 `docs/DEFECTS_LEDGER.md`、`.pi/inspection/**` 或审查员复验沙箱，不使用共享 dev server 作为对方复验来源。

## 进度

| 交付 | 状态 | 未验证边界 |
|---|---|---|
| Phase 0 | 已通过限定 Windows 门禁，70b4ae145afee9860b2257eb386005c291549661 | 非跨平台/完整产品 |
| Feature 1 Manifest/本地登记 | 实施中，尚未送审/Verified | 解析不等于运行支持 |
| Feature 2 安全配置/Owned Managed | 待 Feature 1 Verified；独立 Job fixture 门禁可先审计 | 未启动 Managed，ACL/Keyring/回收未验收 |
| Feature 3 统一网关/真实调用 | 待依赖门禁 | 不以 Mock 代替真实模型/凭据 |
| Feature 4 最小桌面操作 | 待依赖门禁 | 最终实际客户端由 Inspector MCP 独立复验 |

此目标包含 Phase 1、必要 Phase 2 局部行为以及最小 Phase 3 UI；不把局部能力写成完整 Phase 2/3 完成。不扩展在线市场/自动编译/下载升级/完整 API Doctor/IDE/跨平台发布。
