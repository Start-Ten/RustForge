# STEP 2 — 总体架构、crate 依赖图、里程碑与风险

## 1. 分层架构

```
┌────────────────────────── 应用层 ──────────────────────────┐
│ examples(游戏)  tools/editor  rustforge(伞 crate)          │
├────────────────────────── 产品层 ──────────────────────────┤
│ rf-editor  rf-debugger  rf-distribute  rf-ml               │
├────────────────────────── 领域层 ──────────────────────────┤
│ rf-render rf-asset rf-physics rf-animation rf-audio        │
│ rf-network rf-ai rf-ui rf-script                           │
├────────────────────────── 图形/平台层 ─────────────────────┤
│ rf-rhi (Backend: Software/Vulkan/DX12/DX11/GL/GLES/WGPU/Metal/Null)
│ rf-platform (Headless/Win32/X11/Android)                   │
├────────────────────────── 基础层 ──────────────────────────┤
│ rf-ecs rf-reflection rf-serialization rf-task rf-event     │
│ rf-memory rf-math rf-plugin                                 │
├────────────────────────── 内核层 ──────────────────────────┤
│ rf-core (Entity/Tick/Handle/Error/Version)                 │
└─────────────────────────────────────────────────────────────┘
```

## 2. crate 与职责

| crate | 职责 |
|---|---|
| rf-core | Entity、Tick、句柄、错误基类、引擎版本、TypeKey |
| rf-math | 2D/3D 数学全套、几何、插值、曲线、噪声、随机 |
| rf-memory | Arena/Pool/Stack/Buddy 分配器、分配追踪、泄漏检测 |
| rf-event | 类型安全事件总线、优先级/一次性/延迟、输入映射 |
| rf-task | 工作窃取线程池、优先级、依赖 DAG、取消令牌 |
| rf-ecs | Archetype ECS、Query、调度器、事件双缓冲、命令缓冲、层级 |
| rf-reflection | TypeRegistry、属性/方法描述、注册宏 |
| rf-serialization | 反射驱动序列化(JSON/二进制)、版本迁移、增量 |
| rf-plugin | Plugin trait、Registry、扩展点、版本兼容 |
| rf-platform | 窗口/输入/文件/时间/生命周期抽象，Headless+Win32 实现 |
| rf-rhi | RhiDevice 等图形抽象、后端选择、Software 实现、GPU 后端骨架 |
| rf-asset | AssetImporter、导入管线、异步加载、LRU、依赖图、热重载、PAK |
| rf-render | 渲染图、Camera2D/3D、Renderer2D、批次、LayerStack、后处理、上采样 |
| rf-physics | PhysicsWorld(3D)、Physics2D、关节、确定性回放 |
| rf-animation | 2D 帧/骨骼、3D 骨骼蒙皮、动画图状态机、IK、Sequencer 时间轴 |
| rf-audio | AudioGraph、DSP 节点、WAV 编解码、混音、空间化 |
| rf-network | NetworkTransport、复制、RPC、预测回滚钩子 |
| rf-ai | 行为树、黑板、A*/流场/NavGrid、感知、群体 |
| rf-ui | 控件树、布局、9-Slice、数据绑定、虚拟化 |
| rf-script | ScriptHost、字节码 VM、可视化图编译、GAS-lite、脚本调试 |
| rf-ml | MlBackend、张量、 Dense/激活、训练循环(实验) |
| rf-editor | EditorPanel、面板模型、撤销重做、命令面板、主题、布局 |
| rf-debugger | 日志、控制台、Profiler、帧统计、远程调试接口 |
| rf-distribute | BuildProfile/Packager/签名/商店/更新/成就/云存档/崩溃/合规/SDK |
| rustforge | 门面 re-export、Engine 组装、默认运行循环 |

## 3. 依赖方向（冻结）

- 只允许上层依赖下层，禁止同层互依赖外：`rf-render → {rf-ecs, rf-rhi, rf-asset, rf-ui}`、`rf-editor → {rf-ecs, rf-render, rf-asset, rf-ui, rf-debugger}`、`rf-animation → rf-ecs`、`rf-physics → rf-ecs`、`rf-ai → rf-ecs`、`rf-audio` 独立(仅 rf-core)、`rf-network → rf-serialization`。
- **禁止**：基础层任何 crate 依赖 rf-rhi/rf-render；rf-rhi 禁止依赖 rf-render；rf-core 零外部运行时依赖（dev 依赖除外）。
- 外部依赖白名单（详见 docs/dependencies.md）：thiserror、serde/serde_json、flate2、windows-sys(仅 win32 feature)。

## 4. 里程碑

| 里程碑 | 内容 | 版本 |
|---|---|---|
| MVP (STEP 3-11) | 基础层全量 + 平台(Headless/Win32) + RHI(Software+骨架) + 资产 + 渲染 2D/3D + 编辑器/调试器 MVP + 混合示例 | v0.1.0 |
| Alpha (STEP 12) | 物理/动画/音频/网络/AI/UI/脚本全 P0-P1 + 分发 P0 | v0.2.0 |
| Beta (STEP 13) | 高级渲染(虚拟几何/GI-lite/光追 trait)、ML、大世界流送、SDK | v0.5.0 |
| 1.0 (STEP 14) | 全平台分发链、商店合规、更新、成就云存档崩溃报告、反作弊可选 | v1.0.0 |

## 5. 关键技术决策

1. **Software 后端为一等公民**：`SoftwareDevice` 在 CPU 上实现完整 RHI 资源/命令语义（framebuffer 纹理、blit、固定功能管线），供 CI、Editor 预览、无 GPU 环境使用；GPU 后端（Vulkan/DX12）以 ash*/windows-rs 骨架 + 能力查询交付，feature 门控。
2. **反射驱动序列化**：所有运行时资产/存档走 rf-reflection 类型信息，格式层(JSON/二进制)只做编码；保证一份 schema 多格式。
3. **Archetype SoA + 类型擦除列**：Column 用裸内存 + drop_fn 实现零拷贝遍历，Query 经 `QueryData` unsafe trait 构造引用，调度器以读写集保证无别名。
4. **命令式渲染图**：RenderGraph 在 CPU 侧做依赖分析与自动同步点插入（逻辑屏障），后端只执行线性命令；未来 GPU 后端将逻辑屏障翻译为本机屏障。
5. **一切可 Headless**：平台、渲染、编辑器、音频都提供无窗口/无设备实现，保证 T1–T13 在 CI 全绿。

## 6. 风险与规避

| 风险 | 概率 | 规避 |
|---|---|---|
| unsafe ECS 引发 UB | 中 | 借用仅经调度器授权；miri 抽查；模糊测试实体/组件增删 |
| GPU 后端工程量失控 | 高 | 冻结 trait + Software 全量实现保验收；GPU 后端 feature 门控不阻塞主线 |
| 编译时长膨胀 | 中 | 依赖白名单；rf-core 零依赖；windows-sys 仅平台层启用 |
| 规格与实现偏差 | 中 | 接口冻结清单 + 每 STEP 一致性自检 + 变更提案机制 |
| 平台差异(路径/换行/线程) | 低 | 抽象收敛到 rf-platform；CI 双平台 |

## 7. 目录树（STEP 3 落地形态）

```
Engine/
├── Cargo.toml  rustfmt.toml  PROGRESS.md  TASK_REQUIREMENTS.md(原始规格存档)
├── .gitignore .gitattributes .githooks/ .github/workflows/ci.yml
├── README.md LICENSE-MIT LICENSE-APACHE CONTRIBUTING.md CODE_OF_CONDUCT.md
├── SECURITY.md CHANGELOG.md
├── docs/ (step*.md, testing.md, git-commands.md, dependencies.md, reports/, ext-*.md)
├── crates/<每个 crate>/src/{lib.rs, ...} + tests/
└── examples 由 crates/rustforge/examples 提供
```
