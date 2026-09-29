# STEP 11 — MVP 验收报告（v0.1.0）

日期：2026-09-29 · 提交链：db7e035 → 2215ba8 → 127b918 → 65b658a → 11b59f2 → 74e18ce → (本提交)

## L1 冒烟测试

| 项 | 命令 | 预期 | 实际 | 结论 |
|---|---|---|---|---|
| T1 编译 | `cargo check --workspace --all-targets` | 0 错误 | 0 错误 | ✅ |
| T2 格式 | `cargo fmt --all --check` | 通过 | 通过 | ✅ |
| T3 静态 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 错误 | 0 错误 | ✅ |
| T7 示例 | `cargo run -p rustforge --example {minimal_2d,minimal_3d,mixed_23d} -- --frames 3` | 3 帧正常退出 | 全部 RUN_*_OK | ✅ |

## L2 灰度测试

| 阶段 | 范围 | 结论 |
|---|---|---|
| 单 crate | rf-core(11) math(22) memory(7) event(7) task(7) ecs(20) reflection(3) serialization(6) plugin(4) | ✅ |
| workspace | `cargo test --workspace` | 157 passed / 0 failed ✅ |
| 跨后端 | Software 直连 / 降级链 / Null（rf-rhi tests） | ✅ |
| 2D/3D 双模式 | acceptance::mvp_window_and_dual_render + mixed_23d | ✅ |
| 分发产物 | （T13 于 STEP 14 执行） | 待 STEP 14 |

## L3 自验证（对照 docs/step0-constraints.md §8）

| # | 验收项 | 状态 | 证据 |
|---|---|---|---|
| 1 | 窗口 + 2D/3D 渲染 | ✅ | acceptance::mvp_window_and_dual_render（Win32 窗口经 Platform::Default；CI 用 Headless） |
| 2 | 后端可切换（降级验证） | ✅ | acceptance::mvp_backend_switch |
| 3 | ECS 2D/3D 组件共存 | ✅ | acceptance::mvp_ecs_mixed_components |
| 4 | 资产导入（PNG/QOI/BMP/TGA/PNM/OBJ/WAV/JSON/TOML/CSV + 桩） | ✅ | acceptance::mvp_asset_imports + rf-asset tests(15) |
| 5 | 编辑器面板 + 模式切换 + 撤销 | ✅ | acceptance::mvp_editor + rf-editor tests(5) |
| 6 | 调试器日志/控制台/性能/统计 | ✅ | acceptance::mvp_debugger + rf-debugger tests(5) |
| 7 | build/test/run 全通过 | ✅ | 本报告 L1/L2 |
| 8 | Git/CI/hooks/社区文件 | ✅ | acceptance::mvp_repo_infra + hooks 已在每次提交运行 |
| 9 | L1/L2/L3 全通过留档 | ✅ | 本文件 |
| 10 | 扩展文档 | ✅ | docs/ext-*.md（随本提交） |

## 接口一致性

冻结清单 IF-001~IF-333；实现期变更 3 条（CP-001~003）已记录于清单 §26.5，公开签名无破坏。

## 2D/3D 双模式对照

| 能力 | 2D | 3D |
|---|---|---|
| 相机 | Camera2D（正交/像素完美/y 翻转） | Camera3D（透视/正交/yaw-pitch） |
| 渲染 | SoftwareRenderer2D（逆映射光栅 + Alpha 混合） | render_mesh_cpu（z-buffer + Lambert + 双光源） |
| 混合 | LayerStack 交叉排序 + mixed_23d 示例 | 同左 |

## 性能对照（Target 参考，非 P0 门槛）

- 冒烟：157 测试 < 3s；示例 3 帧即时。
- ECS：10 万实体 spawn+query < 0.5s（stress_many_entities）。
- 正式 bench 对照列 P1（bench_ecs 示例规格保留）。

## 失败项与修复记录

实现期缺陷（全部已修复并回归）：ECS despawn 位置失效、查询必需组件过滤缺失、
Sprite 正/逆映射错误、BMP BGR 通道序、Buddy 级别方向、渲染图入度方向、
Profiler 嵌套借用（CP-003）。详见 git log。

## 结论

**MVP 验收通过**，进入 STEP 12（P1 域功能）。
