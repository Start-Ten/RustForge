# STEP 1 — 功能矩阵（模块 × 对标 × 优先级 × 验收）

优先级定义：P0=MVP 必交付；P1=Alpha；P2=Beta/实验。验收列给出验收测试点（对应 crates 内 tests）。

## A. 核心运行时

| 模块 | UE5/行业对标 | 2D 对标 | 分发对标 | 优先级 | 依赖 | 验收测试 |
|---|---|---|---|---|---|---|
| A1 ECS | Mass/Gameplay framework | 同 | — | P0 | rf-core, rf-math | 实体生命周期/查询过滤/变更检测/事件/命令/层级/并行调度 |
| A2 反射 | UProperty | 同 | — | P0 | rf-core | 类型注册/属性读写/方法调用/元数据 |
| A3 序列化 | SaveGame/PackageName | 同 | 存档 | P0 | rf-reflection | JSON/二进制往返/版本迁移/增量 |
| A4 资产 | AssetRegistry/ImportTask | Sprite/图集/Tiled | PAK | P0 | rf-task, rf-reflection | 句柄/异步/缓存/依赖图/热重载/PAK |
| A5 任务 | TaskGraph | 同 | — | P0 | rf-core, rf-memory | Job/依赖/取消/优先级/亲和性/窃取 |
| A6 内存 | 低级分配器 | 同 | — | P0 | rf-core | Arena/Pool/Stack/Buddy/追踪/泄漏检测 |
| A7 数学 | Kismet 数学库 | 2D 数学全套 | — | P0 | rf-core | 向量/矩阵/四元数/几何/插值/随机 |
| A8 平台 | RHI 平台层 | 同 | 平台目标 | P0 | rf-core | 窗口/输入/文件/时间/生命周期(Headless+Win32) |
| A9 事件 | 消息总线 | 同 | — | P0 | rf-core | 订阅发布/优先级/延迟/线程安全/输入映射 |
| A10 插件 | Plugin system | 同 | 商店插件 | P0 | rf-core | 注册/扩展点/版本校验/状态保存恢复；动态库 P1 |

## B. 3D 渲染

| 模块 | 对标 | 优先级 | 验收测试 |
|---|---|---|---|
| B1 RHI | RHI 层 | P0(trait+Software+Vulkan/DX12 骨架) | 后端创建/能力查询/资源/命令/降级链 |
| B2 渲染图 | RDG | P0(逻辑图+自动依赖) | Pass 依赖/循环检测/执行顺序 |
| B3 材质 | Material Editor | P1(参数模型+内置着色模型) | 参数覆盖/实例 |
| B4 光照 | Lumen/Lightmass-lite | P0(方向/点/聚+Lambert) P1(阴影贴图/IBL) | 光源类型/着色正确性 |
| B5 全局光 | Lumen | P2(DDGI-lite 探针) | 探针插值 |
| B6 虚拟几何 | Nanite | P2(集群+LOD+剔除) | LOD 选择/剔除正确性 |
| B7 后处理 | PostProcess | P0(色调映射/Bloom/像素完美/CRT) | 每效果单测+顺序 |
| B8 特效 | Niagara | P1(CPU 模块化粒子+GPU 接口) | 发射/生命周期/事件 |
| B9 光追 | RT | P2(trait+软件回退路径) | 接口/回退 |
| B10 纹理 | VT/流送 | P1(图集/mipmap/压缩 BCn ASTC trait) | mipmap/图集 |

## C. 物理

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| C1 刚体 3D | Chaos | P0(球/箱+冲量+休眠) P1(凸/网格) | 积分/碰撞/求解/休眠/射线 |
| C2 高级 | 关节/车辆/布料 | P1(距离/铰接关节) P2(布料/破坏/流体) | 关节约束稳定 |
| C3 工程 | 确定性/回放 | P0(固定步长+确定性随机) | 录制回放一致 |

## D. 动画

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| D1 基础 | SkeletalMesh | P0(骨骼/蒙皮矩阵/2D 帧) | 蒙皮正确/帧动画 |
| D2 动画图 | AnimBP | P0(状态机/混合) | 转换/混合权重 |
| D3 高级 | Control Rig/IK | P1(Two-Bone/FABRIK/2D IK) | IK 收敛 |

## E. 音频

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| E1 音频图 | MetaSounds | P0(节点图+DSP+WAV) | 振荡器/包络/混音/空间衰减 |
| E2 高级 | 麦克风/MIDI | P2(trait) | 接口 |

## F. 网络

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| F1 复制 | Replication | P0(脏标记/增量/优先级/loopback+UDP) | 丢包容忍/增量正确 |
| F2 工程 | NAT/带宽 | P1(trait+文档) P2(打洞) | 重连 |

## G. AI

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| G1 行为 | BT/EQS/NavMesh | P0(BT/黑板/A*/流场/感知) | 节点执行/寻路最优性 |
| G2 高级 | ML | P2(rf-ml) | 见 I2 |

## H. UI

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| H1 控件 | UMG/Slate | P0(控件树/布局/9-Slice/绑定/虚拟化) | 布局确定性/命中测试 |
| H2 工程 | 无障碍/多语言 | P1(主题/字体/RTL 数据模型) | 回退行为 |

## I. 脚本与可视化

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| I1 可视化脚本 | Blueprint | P0(图模型+编译到字节码+调试钩子) | 图编译/断点/单步 |
| I2 脚本/ML | GAS/learn | P0(ScriptHost+VM+GAS-lite) P2(训练,桌面 only/Android 推理) | 训练收敛(XOR)/推理 |

## J. 编辑器

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| J1 基础 | Editor UI | P0(面板模型/布局/主题/撤销/命令面板/2D-3D 切换) | 面板注册/撤销栈/模式切换 |
| J2 专业面板 | 各编辑器 | P1(属性网格+资产浏览器+控制台+性能) P2(节点编辑器/Sequencer UI) | 面板数据正确 |
| J3 工程 | 协作/版本控制 | P1(自动保存/崩溃恢复) | 恢复 |

## K. 调试

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| K1 CPU | 调试器 | P0(脚本断点/单步/调用栈/变量) P1(原生 attach 文档) | 断点命中 |
| K2 GPU | RenderDoc 兼容层 | P0(帧统计/Pass 审计/标记输出接口) | 捕获清单 |
| K3 性能 | Insights | P0(CPU 作用域/计数器/火焰数据) | 作用域计时 |
| K4 工程 | 日志/控制台 | P0 | 日志过滤/命令执行 |

## L. 2D 专用（与 3D 平级）

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| L1 渲染 | Unity2D/Godot2D | P0(正交相机/精灵/批次/图层/混合) | 批次合并/排序稳定 |
| L2 物理 | Box2D | P0(圆/AABB/凸多边形+SAT+关节) | 叠加分离/关节 |
| L3 动画 | Spine-lite | P0(帧/骨骼 2D) | 混合 |
| L4 瓦片 | Tiled | P0(TMX 导入+分块) | 图层/碰撞 |
| L5 UI | UGUI | P0(锚点/9-Slice) | 适配 |
| L6 编辑器 | 2D 视口 | P0(网格/吸附/图层) | 吸附 |
| L7 调试 | 2D 统计 | P0(Sprite/批次计数) | 计数 |
| L8 AI | 2D 寻路 | P0(A*/流场) | 路径 |

## M. 3D 专用：见 B/C/D/E/F/G/H/J/K 各节，全部保留。

## N. 混合 2D/3D

| 项 | 优先级 | 验收 |
|---|---|---|
| N1-N3 同场景共存/相机共存/空间互投 | P0 | 混合示例测试 |
| N4 物理互通事件 | P1 | 事件路由 |
| N5 共用动画图 | P1 | — |
| N6-N11 编辑器/顺序/坐标/输入/性能 | P0 | LayerStack 交叉序 |

## O. 资产导入

| 类 | 格式 | 优先级 | 状态 |
|---|---|---|---|
| O1 图像 | PNG/QOI/BMP/TGA/PNM | P0 | 解码器内置(含 flate2 inflate) |
| O1 图像 | JPG/GIF/WebP/AVIF/EXR/DDS/KTX2 | P1/P2 | 注册+NotYetSupported 明确错误 |
| O2 模型 | OBJ/glTF(JSON+bin) | P0 | 内置 |
| O2 模型 | FBX/USD/BLEND | P1 | 注册+NotYetSupported |
| O3 2D | TMX(JSON 变体)/SpriteSheet JSON/Atlas JSON | P0 | 内置 |
| O4 音频 | WAV | P0 | 内置 |
| O5 视频 | MP4 等 | P2 | 接口 |
| O6 字体 | 位图字体(JSON+PNG) | P0 | 内置 |
| O7 数据 | JSON/TOML(子集)/CSV | P0 | 内置 |
| O8 管线 | ImportPipeline | P0 | 异步/缓存/依赖/热重载/报告 |
| O9 元数据 | 索引/搜索/引用 | P0 | 索引/引用 |

## P. 全平台分发

| 模块 | 对标 | 优先级 | 验收 |
|---|---|---|---|
| P1 构建 | UBT | P0(BuildProfile/目标/报告) | 构建计划生成 |
| P2 打包 | 打包器 | P0(ZIP/PAK/清单) | 打包-解包往返 |
| P3 签名 | 签名 | P0(trait+命令生成+自研哈希签名) | 签名验证 |
| P4 商店 | 商店 | P0(清单生成) P1(上传 trait) | 清单 schema 校验 |
| P5 更新 | 补丁系统 | P0(版本清单+哈希校验+块差分) | 差分往返/回滚 |
| P6 成就/云存档 | OnlineServices | P0(本地实现+trait) | 解锁/冲突解决 |
| P7 崩溃/分析 | CrashReport | P0(minidump-lite+队列+本地持久化) | 崩溃捕获 |
| P8 合规 | 合规 | P0(检查清单执行器) | 检查报告 |
| P9 DRM/反作弊 | 可选 | P0(trait+哈希完整性) | 完整性校验 |
| P10 SDK | SDK | P0(C 头+文档导出) | 生成物存在且编译(语法检查) |

## 附：神经网络（I2 细则）

- 推理：P1（`MlBackend` trait + 内置纯 Rust 张量后端）。
- 训练：P2 实验性，仅桌面编辑器；Android 仅推理。
- 不纳入 MVP 硬性验收；性能指标全部标 Target。
