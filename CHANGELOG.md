# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与语义化版本。

## [Unreleased]

### Added
- STEP 0-2.5：约束、功能矩阵、架构、接口冻结清单（docs/）。
- STEP 3：Cargo workspace、CI、Git hooks、社区健康文件。
- STEP 4-14：全部 26 crate 实现（见 docs/reports/）。

## [0.2.0-alpha] — 2026-09-29
### Added
- 物理（3D/2D SAT/关节/角色控制器）、动画（骨骼/IK/时间轴）、网络（复制/RPC/预测）、AI（BT/A*/感知）、UI（布局/9-Slice）、脚本（VM/GAS）。
- ML 实验模块（张量/MLP/XOR 训练）。
- 分发全链路：构建计划/ZIP/PAK/安装脚本/HMAC 签名/4 商店清单/块差分更新/成就/云存档/崩溃报告/分析/合规/DRM/反作弊/C SDK+FFI。
- 229 项测试全绿；clippy -D warnings 0 错误。

## [0.1.0-mvp] — 2026-09-29
### Added
- 基础层：core/math/memory/event/task/ecs/reflection/serialization/plugin。
- 平台层：Headless + Win32 窗口 + VFS。
- RHI：trait + Software CPU 光栅化 + Null + 降级链。
- 资产：15 种格式导入器 + PAK + 图集 + 热重载。
- 渲染：相机/2D 批次/渲染图/后处理/上采样。
- 编辑器/调试器 MVP；Engine 组装与三个示例；MVP 验收套件。

## [0.1.0-mvp] — 计划
- MVP：见 docs/step0-constraints.md §8 验收标准。
