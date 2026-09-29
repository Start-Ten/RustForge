# RustForge Engine

跨平台 2D/3D 游戏引擎（Rust）。**2D 与 3D 平级**：共享 RHI、资产、ECS、编辑器、调试系统；同一场景可混合 2D 与 3D 对象。

> 独立实现，仅做功能对标（UE5/Unity/Godot），不包含任何 Epic 专有代码或资产。

## 快速开始

```bash
cargo build --workspace            # T1
cargo test  --workspace            # T4/T5
cargo run --example minimal_2d     # 窗口 + 2D 渲染
cargo run --example minimal_3d     # 窗口 + 3D 渲染
cargo run --example mixed_23d      # 混合 2D/3D
```

无 GPU 环境自动使用 **Software(CPU) 后端**（`rf-rhi::software`），渲染管线语义一致。

## Workspace 布局

| crate | 职责 |
|---|---|
| rf-core / rf-math / rf-memory / rf-event / rf-task | 内核与基础层 |
| rf-ecs / rf-reflection / rf-serialization / rf-plugin | 数据与扩展层 |
| rf-platform / rf-rhi / rf-asset | 平台与图形/资产层 |
| rf-render / rf-physics / rf-animation / rf-audio / rf-network / rf-ai / rf-ui / rf-script | 领域层 |
| rf-ml / rf-editor / rf-debugger / rf-distribute | 产品层 |
| rustforge | 伞 crate（prelude + Engine 组装） |

## 文档

- 规格存档：`TASK_REQUIREMENTS.md`
- 约束：`docs/step0-constraints.md` · 功能矩阵：`docs/step1-feature-matrix.md`
- 架构：`docs/step2-architecture.md` · **接口冻结清单：`docs/step2.5-interface-freeze.md`**
- 测试协议：`docs/testing.md` · Git 工作流：`docs/git-commands.md` · 依赖白名单：`docs/dependencies.md`
- 扩展指南：`docs/ext-*.md` · 验收报告：`docs/reports/`

## 分支与版本

trunk-based：`main`（可发布）+ `develop`（集成）+ `feature/*`。Conventional Commits。
版本：v0.1.0 MVP → v0.2.0 Alpha → v0.5.0 Beta → v1.0.0。

## 许可

MIT OR Apache-2.0，见 LICENSE-MIT / LICENSE-APACHE。
