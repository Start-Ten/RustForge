# STEP 0 — 约束、规范与验收标准（锁定）

本文件为 RustForge 引擎的顶层约束，全部后续 STEP 不得违反。标注【假设】的条目为 AUTO-RUN 模式下由架构师自行裁定的决策记录。

## 1. 平台与语言约束

| 项 | 约束 |
|---|---|
| 桌面平台 | Windows 10+ x64/arm64、Linux x64/arm64（X11/Wayland） |
| 移动平台 | Android 8+ arm64/armv7 |
| 预留 | macOS、iOS、Web(WASM) —— 仅架构预留，不做 P0 验收 |
| 语言 | Rust 2024 edition（rustc 1.85+，CI 固定 stable） |
| 绑定 | 允许 C ABI 导出供 C/C++/Kotlin(JNI) 调用；本仓库 MVP 提供 C ABI 骨架 |
| 着色器 | WGSL 为主（trait 层接受 WGSL 源），GLSL/HLSL 作为导入转换目标（P2） |

【假设】MVP 阶段 GPU 后端（Vulkan/DX12）以 trait 完整 + 能力查询 + 编译骨架交付；**Software（CPU）后端为 P0 默认可用后端**，保证 2D/3D 渲染在任何 CI/无 GPU 环境可运行、可测试、可截图验收。运行时后端选择协议已实现：请求后端不可用时按 Software → Null 顺序降级。

## 2. 2D/3D 双支持约束

1. 2D 与 3D 共享：RHI、资产系统、ECS、编辑器框架、调试器框架、任务系统、事件总线。
2. 同一 World 中 2D 与 3D 组件共存（混合场景为 P0 验收项）。
3. 渲染顺序上 2D 图层与 3D 图层可通过 `LayerStack` 交叉编排。
4. 2D 物理与 3D 物理独立实现、共享查询/事件接口。
5. 编辑器 `EditorMode::{TwoD, ThreeD, Mixed}` 一键切换。

## 3. 工程规范

- 结构：Cargo workspace monorepo，仓库根 = workspace 根。
- 命名：crate 前缀 `rf-`，伞 crate `rustforge`。
- 格式：rustfmt 默认 + `use_small_heuristics="Max"`；CI `cargo fmt --check`。
- 静态检查：`cargo clippy --workspace --all-targets -- -D warnings`。
- 测试：三层（L1 冒烟 / L2 灰度 / L3 自验证），T1–T13 循环见 docs/testing.md。
- 提交：Conventional Commits；每 STEP ≥1 个原子提交；分支 main/develop/feature/*。
- 版本：语义化，MVP=v0.1.0，Alpha=v0.2.0，Beta=v0.5.0，1.0=v1.0.0。
- 禁止提交：target/、IDE 配置、密钥证书、大二进制（走 Git LFS 声明）。

## 4. 接口冻结规范

- STEP 2.5 输出的《接口冻结清单》(docs/step2.5-interface-freeze.md) 是唯一公开 API 来源。
- 未列入清单的 `pub` API 视为违规：必须回填清单（走“接口变更提案”小节，追加编号并标注变更 STEP）或改为私有。
- 变更提案格式：接口编号、旧签名、新签名、原因、影响面、兼容性处理（deprecate 一个次要版本再移除）。
- 每个 STEP 开头输出“接口一致性自检”。

## 5. 测试规范（摘要，全文见 docs/testing.md）

| 层 | 内容 | 门槛 |
|---|---|---|
| L1 冒烟 | 编译 + 核心单测 + 最小示例运行 ≤5min | 全绿才可进 L2 |
| L2 灰度 | 单 crate → workspace → 跨后端(Software/Null) → 2D/3D 双模式 → 分发产物 | 逐级扩大，失败回滚 |
| L3 自验证 | 对照冻结清单 + 功能矩阵 + 性能基线逐项核对 | 输出通过/失败/待办 |

## 6. 性能基线与 Target 声明

- P0 硬性验收 = 功能正确、可编译、可运行、可测试。
- 全部数值型性能指标为 Target（中端硬件尽力达成），不做 P0 硬门槛；每 STEP 输出实测对照。
- 统一基线：编辑器 1080p60、运行时 1080p60（中端）、启动<3s、热重载<1s、内存运行时<2GB。

## 7. 法律与合规约束

- 不复制 Unreal Engine 专有代码与资产，仅做独立功能对标。
- FSR 1/2 为 MIT 许可可源码集成；DLSS/XeSS 不内置不捆绑，仅提供 `UpscalerProvider` 接口与接入文档。
- RenderDoc：提供兼容调试层（调试标记、PDB、着色器调试信息）+ 捕获配置文档，不承诺自动集成。
- 仓库许可证：MIT OR Apache-2.0 双许可。

## 8. 验收标准（MVP = STEP 11 完成态）

1. Windows 打开窗口并渲染 2D 与 3D 场景（Software 后端；Vulkan/DX12 骨架可编译可查询）。
2. 后端可切换（请求 Vulkan 失败时降级 Software，行为可验证）。
3. ECS：实体/组件/系统可创建，2D 与 3D 组件共存。
4. 资产：PNG/QOI/BMP/TGA/OBJ/glTF(JSON)/TMX/JSON/TOML 导入可用，损坏文件报错。
5. 编辑器：视口/层级/属性/资产浏览器面板模型可用，2D/3D 模式切换，撤销重做。
6. 调试器：日志、控制台（命令+cvar）、性能面板数据、2D/3D 渲染统计。
7. `cargo build && cargo test && cargo run --example minimal_2d` 全通过。
8. Git 仓库、main 分支、CI 配置、hooks、.gitignore/.gitattributes/README/LICENSE/CHANGELOG 齐全。
9. L1/L2/L3 全部通过并留档 docs/reports/。
10. 扩展文档：渲染后端/资产导入器/编辑器面板/调试器/工作流/2D-3D 混合开发。

## 9. 编码规范要点

- 公开 API 必须 `#[doc]` 注释；unsafe 块必须 `// SAFETY:` 注释。
- 错误处理：库 crate 统一 `thiserror` 错误类型 + `rf_core::Result` 别名约定；禁止裸 panic 于库代码（expect 仅限测试与明确不变量）。
- 并发：跨线程数据必须 Send+Sync；任务系统承担并行调度；ECS 借用冲突由调度器读写集分析保证。
- 依赖最小化：每引入外部 crate 需在 docs/dependencies.md 登记【假设】。

## 10. AUTO-RUN 假设记录

| # | 假设 | 理由 |
|---|---|---|
| A1 | MVP 默认后端为 Software(CPU) | 无 GPU 环境可编译可测试可验收；GPU 后端 trait 完整 + 骨架 |
| A2 | 反射采用注册式 + macro_rules 派生，不用 proc-macro | 控制编译时长与依赖面；接口预留 derive 升级 |
| A3 | 热重载文件监听 MVP 用轮询(500ms) | 跨平台零依赖；inotify/ReadDirectoryChangesW 列 P1 |
| A4 | PAK 压缩 MVP 用 Deflate(flate2) | zstd/lz4 列 P1 依赖升级 |
| A5 | 音频实时设备输出列 P1，MVP 交付 DSP 图 + WAV 渲染 | 无设备 CI 也可测 |
| A6 | Android MVP 交付 NDK 构建脚本 + JNI 骨架，不上商店 | 本机无 Android SDK |
| A7 | GIF/JPG/FBX/KTX2/BLEND 二进制解析 P1 | 实现成本高，导入器注册后返回明确 NotYetSupported 错误 |
