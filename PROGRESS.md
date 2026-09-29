# RustForge AUTO-RUN 进度（AUTO-PAUSE 断点协议载体）

> 本文件由 AUTO-RUN 流程维护；每次 STEP 完成/中断即更新。收到「继续」后从断点恢复，不重新规划。

| STEP | 状态 | 提交 | 备注 |
|---|---|---|---|
| 0 约束 | ✅ | chore(docs) | docs/step0-constraints.md |
| 1 功能矩阵 | ✅ | chore(docs) | docs/step1-feature-matrix.md |
| 2 架构 | ✅ | chore(docs) | docs/step2-architecture.md |
| 2.5 接口冻结 | ✅ | chore(docs) | docs/step2.5-interface-freeze.md（IF-001~IF-333） |
| 3 工程脚手架 | ✅ | chore | workspace/CI/hooks/git init，commit db7e035 |
| 4 基础层 | ✅ | feat | 9 crate + rf-ecs-derive，87 测试（commit 2215ba8） |
| 5 平台层 | ✅ | feat | Headless+Win32+VFS+FrameTimer（并入 127b918） |
| 6 RHI | ✅ | feat | traits+Software 光栅化+Null+降级链+as_any(CP-002)（commit 127b918） |

## 断点标记

[AUTO-PAUSE]
当前步骤：STEP 7 资产系统
已完成：STEP 0–6（commit: db7e035 / 2215ba8 / 127b918；累计 101 测试全绿）
未完成：STEP 7 起全部
续写标记：AUTO-RESUME: STEP 7 / PART 1

## 剩余工作清单（按序）

1. STEP 7 rf-asset：AssetId/AssetImporter trait/ImportPipeline(LRU+依赖图+热重载 notify_changed)/PAK(write_pak+PakReader)/AtlasPacker(Shelf)/内置导入器(PNG-with-flate2/QOI/BMP/TGA/PNM/OBJ/glTF-lite/WAV/TMX-lite/JSON/TOML-mini/CSV/SpriteSheetJson/FontJson + NotYet 桩) + rf-audio AudioAsset 先行
2. STEP 8 rf-render：Camera2D/3D、Renderer2D trait+Software 实现(sprite 批次/像素完美)、Renderer3D(经 rf-rhi software draw)、RenderGraphPass/RenderGraph(拓扑+循环检测+BarrierPlan)、LayerStack、后处理(tone_map/bloom/pixel_perfect/crt/aberration/vignette)、UpscalerProvider、Gizmo
3. STEP 9 rf-editor：EditorMode/PanelKind/Selection/EditorContext/EditorPanel trait/UndoStack/CommandPalette/Theme/DockLayout/EditorModel/内置面板/EditorApp
4. STEP 10 rf-debugger：LogLevel/Logger(全局+宏)/CVar/Console/Profiler(ScopeGuard+flame)/RenderStats/DebugTransport+RemoteDebugServer
5. STEP 11 rustforge：App trait/EngineConfig/Engine(run+run_headless) + examples minimal_2d/minimal_3d/mixed_23d + tests/acceptance.rs 逐条断言 MVP 清单
6. STEP 12 域 crate：rf-physics(3D 冲量+2D SAT+关节+角色控制器)、rf-animation(骨骼/蒙皮/状态机/IK/Timeline)、rf-audio(节点图 DSP+WAV)、rf-network(loopback+udp+复制+RPC+预测)、rf-ai(BT/黑板/A*/流场/感知/boids)、rf-ui(控件树/布局/9-slice/绑定/虚拟化 UiCommand)、rf-script(VM+图编译+GAS+调试器)
7. STEP 13 rf-ml(Tensor/MlpBackend/XOR 训练) + 高级渲染 trait 桩
8. STEP 14 rf-distribute：crc32/sha256/crc? BuildProfile/PlatformTarget/BuildPlan/ZipPackager(自写 zip)/PakPackager/ScriptPackager/HmacSigner/StoreUploader/UpdateManifest+block_diff+Updater/LocalAchievements/LocalCloudSaves/LocalCrashReporter/LocalAnalytics/ComplianceChecker/IntegrityAntiCheat/CSdkExporter+ffi
9. 收尾：全量 T1-T13、git tag v0.1.0-mvp、docs/reports/、docs/ext-*.md、CHANGELOG

## 关键决策存档

见 docs/step0-constraints.md §10（A1–A7）。实现期补充：
- D1 AudioAsset 定义于 rf-audio，rf-asset 依赖 rf-audio（导入器产出类型）。
- D2 rf-ui 不依赖 rf-render：产出 UiCommand 显示列表，由伞 crate 适配渲染。
- D3 ECS QueryData/Bundle 为 unsafe trait，安全性由调度器读写集分析保证。

## 测试状态

- 最近一次 T1（cargo check）：✅ workspace 全绿
- 最近一次 T2（fmt --check）：✅
- 最近一次 T3（clippy -D warnings）：✅ 0 错误
- 最近一次 T4/T5（cargo test --workspace）：✅ 101 passed / 0 failed（含 platform 7 + rhi 7）
