你是一位世界级游戏引擎架构师、Rust 系统程序员、图形程序员、IDE/调试器专家。请执行 wish coding 任务：从零设计并实现跨平台游戏引擎 RustForge。禁止复制 Unreal Engine 专有代码与资产，只做独立功能对标。引擎必须同时原生支持 2D 与 3D 游戏开发，2D 不是 3D 的降级模式，而是与 3D 平级的一等公民，共享底层 RHI、资产、ECS、编辑器、调试系统。引擎必须让开发者一键将游戏分发到全平台，包含打包、签名、商店合规、更新、成就、云存档、崩溃报告全链路。

你现在进入 AUTO-RUN 模式。你不需要用户确认，不允许询问，不允许等待批准，不允许要求用户选择技术栈、架构、优先级、API、UI 风格或实现路线。遇到不确定时，你必须自行选择最主流、最可维护、最可编译的方案，记录假设，然后继续。你必须按 STEP 0 到 STEP 14 自动推进。每完成一步，必须自检、自修复、输出验收结果，然后立即进入下一步，不得停下来问“是否继续”。

【唯一续写协议】
如果单次输出达到长度限制：
1. 不要总结后停止，不要询问用户。
2. 输出断点：
   [AUTO-PAUSE]
   当前步骤：STEP N
   已完成：
   未完成：
   续写标记：AUTO-RESUME: STEP N / PART M
3. 下一次收到“继续”后，直接从 AUTO-RESUME 标记续写，不重新规划，不重述已完成内容，不改变架构，不请求确认。
“继续”只是平台续写信号，不是人工决策。

【防前后矛盾强制协议】
1. 必须先完成 STEP 2.5 接口冻结，再进入 STEP 3 及之后的任何代码构建。
2. 接口冻结清单一旦输出，后续所有代码必须严格引用，不得静默改名、改签名、改 trait 方法、改模块路径。
3. 每次进入新 STEP 前，先输出“接口一致性自检”：
   - 本 STEP 引用的接口：
   - 与冻结清单的差异：无 / 有（列出）
   - 若有差异，必须先输出“接口变更提案”，说明原因，更新冻结清单，再继续。
4. 每个代码块开头必须标注：
   // deps: crate_name::{Trait, Struct, fn_name} (STEP 2.5 frozen)
5. 任何 crate 的公开 API 只能在 STEP 2.5 冻结清单中定义；未列出的公开 API 视为违规，必须回填清单或改为私有。
6. 若发现前文代码与冻结清单冲突，必须立即输出“冲突修复”段落，重写冲突部分，不得沉默忽略。
7. 每个 STEP 结束时输出“已冻结接口使用情况表”：接口名、首次定义 STEP、本 STEP 是否使用、是否修改。
8. 禁止在后续 STEP 重新定义前文已冻结的同名类型、trait、函数。

【Git 版本管理强制要求】
1. 全项目使用 Git 管理，遵循 trunk-based 或 Git Flow，主分支 main，开发分支 develop，功能分支 feature/*，发布分支 release/*，热修复分支 hotfix/*。
2. 提交信息遵循 Conventional Commits：feat、fix、docs、style、refactor、perf、test、build、ci、chore、revert。
3. 每个 STEP 对应一个或多个原子提交，禁止一个提交包含多个不相关 STEP。
4. 必须生成 .gitignore、.gitattributes、README.md、LICENSE、CONTRIBUTING.md、CODE_OF_CONDUCT.md、SECURITY.md、CHANGELOG.md。
5. 必须配置 Git hooks：pre-commit 执行 cargo fmt --check、cargo clippy、cargo test；commit-msg 校验 Conventional Commits；pre-push 执行完整测试与构建。
6. 必须配置 CI：GitHub Actions 或 GitLab CI，触发 build、test、clippy、fmt、audit、cross-compile（Windows/Linux/Android）。
7. 必须使用语义化版本标签：v0.1.0 MVP、v0.2.0 Alpha、v0.5.0 Beta、v1.0.0 正式版。
8. 必须输出常用 Git 命令清单，包括初始化、分支、提交、合并、变基、标签、钩子安装、子模块、LFS。
9. 大型二进制资产使用 Git LFS，配置 .gitattributes 跟踪 *.png、*.jpg、*.jpeg、*.webp、*.avif、*.exr、*.hdr、*.ktx、*.ktx2、*.dds、*.tga、*.bmp、*.gif、*.svg、*.psd、*.fbx、*.glb、*.gltf、*.obj、*.dae、*.usd、*.usdz、*.stl、*.ply、*.abc、*.blend、*.max、*.ma、*.mb、*.c4d、*.wav、*.ogg、*.mp3、*.flac、*.mp4、*.webm、*.tmx、*.tsx、*.aseprite、*.ase、*.psb。
10. 每个 STEP 完成后，输出该 STEP 的 Git 提交信息、变更文件列表、分支操作建议、标签建议。AI 不实际执行 Git，但必须生成可直接复制执行的命令。
11. 禁止把 build 目录、target 目录、IDE 配置、临时文件、密钥、证书、大二进制文件提交到 Git。
12. 必须支持 monorepo 结构，Cargo workspace 与 Git 仓库根目录对齐。
13. 必须提供 .github/workflows 或 .gitlab-ci.yml 完整配置。
14. 必须提供 commit 模板、PR 模板、Issue 模板。
15. 必须提供版本迁移与废弃策略：接口废弃先标记 deprecated，保留一个次要版本，再移除。

【灰度测试、冒烟测试、自验证强制协议】
1. 每次代码修改后，必须按顺序执行三层测试，全部输出结果，不得跳过：
   - L1 冒烟测试：最小可运行验证，5 分钟内完成。编译通过、核心模块单测通过、最小示例可运行、无 panic、无死锁、无内存泄漏。
   - L2 灰度测试：渐进式验证，先在子集上运行，再全量运行。单 crate 测试 → 多 crate 集成测试 → 全 workspace 测试 → 跨后端测试 → 跨平台测试 → 2D/3D 双模式测试 → 分发产物测试。
   - L3 自验证：对照 STEP 2.5 冻结清单、STEP 1 功能矩阵、性能基线、验收标准逐项核对，输出通过/失败/待办。
2. 每个 STEP 内部必须执行以下测试循环：
   - T1 编译验证：cargo check --workspace --all-targets
   - T2 格式验证：cargo fmt --check
   - T3 静态检查：cargo clippy --workspace --all-targets -- -D warnings
   - T4 单元测试：cargo test --workspace --lib
   - T5 集成测试：cargo test --workspace --test
   - T6 文档测试：cargo test --workspace --doc
   - T7 冒烟测试：运行最小示例，验证核心路径
   - T8 灰度测试：按 crate 子集 → 全 workspace → 跨后端 → 跨平台 → 2D/3D 双模式 → 分发产物逐步扩大
   - T9 基准测试：cargo bench，对照性能基线
   - T10 内存测试：cargo test 下运行泄漏检测，或 miri/valgrind
   - T11 跨平台测试：Windows、Linux、Android 三平台 CI 至少编译通过
   - T12 自验证：逐条核对功能矩阵与冻结清单
   - T13 分发测试：打包、签名、安装、启动、更新、卸载全流程验证
3. 灰度发布策略：
   - 先在 dev 分支验证冒烟测试。
   - 再在 feature 分支验证集成测试。
   - 再合并到 develop 验证全量测试。
   - 再合并到 release 验证跨平台测试与分发测试。
   - 最后合并到 main 并打标签。
   - 每阶段失败必须回滚，不得带着失败进入下一阶段。
4. 自验证报告必须包含：
   - 测试项、命令、预期、实际、结论。
   - 覆盖率报告。
   - 性能对照表。
   - 接口一致性对照表。
   - 2D/3D 双模式对照表。
   - 分发产物对照表。
   - 未通过项、原因、修复计划、重试结果。
5. 所有测试必须可在 CI 中复现。禁止本地通过、CI 失败的测试。
6. 每次修改后，若测试失败，必须先修复，再重新执行完整测试循环。禁止跳过失败测试继续下一步。
7. 每个 STEP 结束必须输出：
   - 冒烟测试报告。
   - 灰度测试报告。
   - 自验证报告。
   - 测试覆盖率。
   - 性能对照。
   - 2D/3D 双模式对照。
   - 分发产物对照。
   - 失败项与修复记录。
   - 是否可以进入下一步的结论。

【自动执行步骤】
STEP 0：锁定约束、验收标准、工程规则、编码规范、测试规范、CI 规范、Git 规范、灰度/冒烟/自验证规范、2D/3D 双支持规范、全平台分发规范。
STEP 1：输出功能矩阵：模块、UE5 对标功能、2D 对标功能、分发对标功能、优先级 P0/P1/P2、依赖、验收测试。
STEP 2：输出总体架构、crate 依赖图、里程碑 MVP/Alpha/Beta/1.0、风险与规避。
STEP 2.5：接口冻结阶段（必须先于任何代码构建）
- 输出全部 crate 列表与职责。
- 输出全部核心 trait 的完整定义：trait 名、方法签名、参数、返回值、关联类型、错误类型、生命周期、Send/Sync 约束。
- 输出全部核心 struct/enum 的完整定义：字段、类型、可见性、derive。
- 输出全部核心函数签名：模块路径、函数名、参数、返回值、错误、unsafe 标注。
- 输出全部错误类型与错误码。
- 输出全部事件、消息、句柄、ID 类型。
- 输出全部公开常量、配置结构、默认值。
- 输出 crate 之间的依赖方向与禁止依赖。
- 输出接口兼容性规则：哪些可扩展、哪些冻结、语义化版本策略。
- 输出“接口清单索引表”：编号、接口名、所属 crate、类型、首次定义处、状态（冻结/可扩展）。
- 未列入本清单的公开 API 在后续 STEP 中不得出现。
- 本 STEP 完成后进入接口冻结状态，后续变更必须走“接口变更提案”。

STEP 3：生成完整 Cargo workspace、目录树、根 Cargo.toml、rustfmt、clippy、CI、构建脚本、Git 仓库初始化、.gitignore、.gitattributes、README、LICENSE、CONTRIBUTING、CODE_OF_CONDUCT、SECURITY、CHANGELOG、Git hooks、GitHub Actions 或 GitLab CI 配置。必须严格引用 STEP 2.5 冻结清单。
STEP 4：实现 core、math、ecs、reflection、serialization、task、memory、event、plugin，并同时覆盖 2D 与 3D 数学、变换、层级、坐标系统。必须严格引用 STEP 2.5 冻结清单。
STEP 5：实现 platform 抽象：Windows、Linux、Android 窗口、输入、触摸、手柄、文件、线程、时间、电源、权限、高 DPI、生命周期。必须严格引用 STEP 2.5 冻结清单。
STEP 6：实现 RHI：Vulkan、DX12、DX11、OpenGL、OpenGL ES、WebGPU、Metal 接口与后端骨架、能力查询、运行时/编译期选择。必须严格引用 STEP 2.5 冻结清单。
STEP 7：实现 asset 系统：AssetImporter trait、插件注册、异步导入、缓存、依赖图、热重载；内置 2D 与 3D 全格式资产导入。必须严格引用 STEP 2.5 冻结清单。
STEP 8：实现 renderer MVP：渲染图、PBR、相机、光照、阴影、后处理、视口输出，同时实现 2D 渲染管线。必须严格引用 STEP 2.5 冻结清单。
STEP 9：实现 editor MVP：停靠面板、视口、层级、属性编辑器、资产浏览器、Gizmo、撤销重做、命令面板、主题，同时支持 2D 与 3D 编辑模式。必须严格引用 STEP 2.5 冻结清单。
STEP 10：实现 debugger MVP：日志、控制台、CPU/GPU 性能面板、断点、调用栈、变量监视、表达式求值、远程 Android 调试接口。必须严格引用 STEP 2.5 冻结清单。
STEP 11：MVP 集成验收：Windows/Linux/Android 打开窗口并渲染 2D 与 3D 场景，Vulkan 与 DirectX 后端可切换，cargo build、cargo test、cargo run 通过。必须严格引用 STEP 2.5 冻结清单。
STEP 12：P1 功能：物理、动画、音频、网络、AI、UI、脚本、可视化脚本、Sequencer 对标，同时覆盖 2D 物理、2D 动画、2D 寻路、2D UI。新增接口必须先走接口变更提案，更新冻结清单。
STEP 13：P2 功能：高级渲染、虚拟几何、动态 GI、光追、大世界流送、虚拟纹理、2D 高级特性、神经网络训练模块（桌面编辑器训练、Android 仅推理）、打包发布、文档、SDK。新增接口必须先走接口变更提案，更新冻结清单。
STEP 14：全平台分发管线与 1.0：构建、打包、签名、商店合规、更新、成就、云存档、崩溃报告、分析、DRM 可选、反作弊、多语言、区域合规、发布流程、SDK 导出。

每个 STEP 内部必须执行：
1. 接口一致性自检：对照 STEP 2.5 冻结清单。
2. 设计接口与数据结构：只能在冻结清单范围内，或走变更提案。
3. 写出可编译代码骨架或完整实现。
4. 写测试、示例、文档。
5. 执行 T1–T13 完整测试循环。
6. 自检：接口兼容、可编译、可测试、无伪造完成。
7. 自修复：发现问题立即修正，再重新执行 T1–T13，直到全部通过。
8. 输出“已冻结接口使用情况表”。
9. 输出 Git 提交信息、变更文件、分支操作、标签建议。
10. 输出冒烟测试报告、灰度测试报告、自验证报告、分发测试报告。
11. 输出验收结果，然后立即进入下一步。

【硬性约束】
- 平台：Windows 10+ x64/arm64、Linux x64/arm64（Wayland/X11）、Android 8+ arm64/armv7。架构预留 macOS、iOS、Web。
- 语言：Rust 2024 为主，允许 C/C++、Kotlin/JNI、Swift/ObjC 绑定；着色器支持 WGSL、GLSL、HLSL。
- 图形 API：Vulkan 1.3、DirectX 12、DirectX 11、OpenGL 4.6、OpenGL ES 3.2、WebGPU、Metal。必须抽象 RHI，支持运行时/编译期选择、能力查询、后端切换。
- 2D/3D 双支持：2D 与 3D 共享 RHI、资产、ECS、编辑器、调试系统；2D 不是 3D 降级模式；同一场景可混合 2D 与 3D 对象；2D 与 3D 渲染管线可同时激活；2D 与 3D 物理、动画、UI、AI 独立且可互操作。
- 编辑器：成熟 IDE 级 GUI，停靠面板、主题、布局、视口、Gizmo、属性编辑器、资产浏览器、节点编辑器、撤销重做、命令面板、多语言。2D 编辑模式与 3D 编辑模式可一键切换。
- 调试：断点、条件断点、数据断点、单步、调用栈、变量监视、表达式求值、内存分析、GPU 调试、着色器调试、帧捕获、RenderDoc 兼容调试层、CPU/GPU Profiler、日志、控制台、远程 Android 调试。2D 与 3D 调试共用同一框架。
- 资产：统一 AssetImporter trait + 插件注册。内置 2D 与 3D 全格式资产导入，支持异步导入、缓存、依赖、热重载、DCC 往返。
- 架构：Cargo workspace、模块化 crate、异步 ECS、任务调度、内存安全、零成本抽象、文档、测试、基准、CI/CD。
- 版本管理：Git，Conventional Commits，Git Flow 或 trunk-based，Git LFS，Git hooks，CI 集成，语义化版本，CHANGELOG，PR/Issue 模板，废弃策略。
- 测试：冒烟测试、灰度测试、自验证，三层测试全部通过方可进入下一步。
- 分发：一键构建全平台产物，支持主流商店、独立分发、更新、成就、云存档、崩溃报告、分析、DRM 可选、反作弊、区域合规。

【功能规格：每条均为强制验收项，必须给出 trait/接口/实现/测试/性能指标，禁止只写名词。每个条目必须包含：数据结构、API 签名、算法或流程、边界条件、错误处理、并发模型、内存布局、性能指标、验收测试点】

A. 核心运行时

A1 ECS
- 实体 ID：struct Entity { index: u32, generation: u32 }，64 位打包，支持 40 亿实体槽位，代数回绕检测。
- 实体分配：freelist + 分页 chunk，每次分配 O(1)，释放 O(1)，支持批量分配与批量销毁。
- 组件存储：Archetype（SoA）默认，SparseSet 可选，运行时按组件类型切换。
- Archetype：组件类型集合哈希为 ArchetypeId，同类实体共享列式存储，列按 64 字节对齐，支持 SIMD 批量遍历。
- 组件注册：Component trait，必须 Send + Sync + 'static，derive 自动注册 TypeId、大小、对齐、drop 函数。
- 查询：Query<(&A, &mut B, Option<&C>, Without<D>)>，编译期类型检查，运行时缓存匹配 Archetype 列表。
- 查询过滤：With、Without、Added<T>、Changed<T>、Removed<T>、AnyOf、AllOf、NoneOf。
- 变更检测：每组件维护 changed_tick、added_tick、removed_tick，tick 为 u32 单调递增。
- 系统：System trait，fn run(&mut self, world: &mut World)，支持 exclusive、parallel、render、physics 标记。
- 调度器：Stage + SystemSet，按读写冲突自动分阶段，支持显式 before/after/run_if。
- 并行执行：工作窃取线程池，每系统声明读写集合，冲突系统串行，非冲突系统并行。
- 命令缓冲：Commands 延迟执行，支持 spawn、despawn、insert、remove、add_child、set_parent。
- 事件：EventReader/EventWriter，双缓冲，保留 2 帧，支持事件清理策略。
- 层级：Parent、Children 组件，支持树遍历、深度优先、广度优先、祖先查询。
- 序列化：World 快照支持全量与增量，组件按注册顺序写入，支持 schema version。
- 调试：实体检查器、组件检查器、Archetype 统计、系统耗时、变更追踪可视化。
- 边界条件：实体销毁后句柄失效必须报错；组件移除后查询必须立即反映；循环层级必须检测。
- 性能：10M 实体创建 < 500ms，销毁 < 300ms；查询遍历 10M 实体 < 10ms；100 系统 @ 60FPS。
- 验收测试：实体生命周期、组件增删、查询过滤、并行调度、变更检测、序列化往返、层级遍历。

A2 反射
- 类型注册：TypeRegistry，按 TypeId 与全限定名索引，支持别名与命名空间。
- 类型种类：Struct、Enum、Tuple、Unit、TraitObject、Generic、Container、Primitive、Pointer。
- 属性：PropertyDescriptor { name, type_id, offset, getter, setter, metadata }。
- 元数据：display_name、tooltip、category、range、step、enum_variants、readonly、hidden、deprecated、script_exposed、editor_exposed、network_replicated。
- 方法：MethodDescriptor { name, args, return_type, is_static, is_const, call_fn }。
- 序列化：derive(Reflect, Serialize, Deserialize)，支持字段重命名、默认值、跳过、别名。
- 编辑器暴露：属性面板按元数据自动生成控件，支持分组、折叠、搜索、过滤。
- 脚本绑定：Rust、Lua、WASM 三套绑定，derive 自动生成，支持热重载。
- 调试暴露：变量视图按类型自动展开，支持指针、容器、枚举、trait 对象。
- 网络暴露：按 network_replicated 标记自动生成复制代码。
- 边界条件：未注册类型访问必须报错；循环引用必须检测；类型删除必须清理引用。
- 性能：10 万类型注册 < 1s；属性读写 < 50ns；序列化反射开销 < 10% 手写。
- 验收测试：类型注册、属性读写、方法调用、序列化往返、编辑器控件生成、脚本绑定。

A3 序列化
- 格式：二进制（bincode/postcard）、JSON（serde_json）、YAML、TOML、MessagePack、CBOR。
- Schema：SchemaVersion { major, minor, patch }，迁移链 MigrationChain，支持 forward/backward。
- 迁移：字段默认值、重命名映射、类型转换、删除字段、添加字段、嵌套迁移。
- 增量：只序列化 changed_tick 之后的组件，支持 delta 快照与 base + delta 合并。
- 确定性：二进制输出字节稳定，字段顺序固定，浮点 NaN 规范化，HashMap 排序。
- 大对象：mmap 读、流式写、引用去重、循环引用检测、外部 blob 引用。
- 压缩：可选 zstd、lz4、snappy，按块压缩，支持随机访问。
- 加密：可选 AES-GCM、ChaCha20-Poly1305，密钥管理。
- 边界条件：版本不匹配必须报错；迁移失败必须回滚；循环引用必须检测；大对象必须分块。
- 性能：10 万实体保存 < 200ms；10 万实体加载 < 300ms；二进制大小 < 内存 50%。
- 验收测试：全格式往返、版本迁移、增量保存、确定性输出、大对象、压缩、加密。

A4 资产系统
- 句柄：AssetHandle<T> { id: AssetId, generation: u32 }，强引用 Strong<T>，弱引用 Weak<T>。
- AssetId：128 位 UUID 或 64 位哈希，全局唯一，支持内容寻址。
- 加载：异步 spawn，优先级 High/Normal/Low/Background，依赖预取，批量加载，失败重试 3 次，超时 30s。
- 缓存：内存 LRU（可配容量）、磁盘缓存（内容哈希）、引用计数卸载、弱引用回收。
- 依赖图：自动追踪、循环检测、拓扑排序、增量重建、热重载传播。
- 热重载：文件监听（inotify、ReadDirectoryChangesW、Android FileObserver），1s 内生效，不重启编辑器。
- 打包：PAK 归档，索引表，压缩，加密，按需流送，差分更新（bsdiff、zstd patch）。
- 流送：开放世界分块，按相机距离、可见性、优先级异步流送，卡顿 < 1 帧，支持预加载与卸载。
- DCC 往返：源文件保留、重导入、覆盖策略、冲突检测、版本比较。
- 导入设置：每资产可配导入选项，预设，批处理，命令行导入。
- 导入报告：成功、失败、警告、日志、耗时、依赖变更。
- 边界条件：循环依赖必须报错；加载失败必须回退；热重载失败必须保留旧版本；打包冲突必须检测。
- 性能：10 万资产索引 < 1s；异步加载吞吐 > 1GB/s；热重载延迟 < 1s。
- 验收测试：句柄生命周期、异步加载、缓存命中、依赖图、热重载、打包、流送、DCC 往返。

A5 任务系统
- Job：Job { fn, priority, dependencies, cancellation_token, affinity }。
- 依赖 DAG：拓扑排序，支持 before/after，循环检测。
- 优先级：Critical、High、Normal、Low、Background。
- 取消：CancellationToken，支持协作式取消，超时自动取消。
- 亲和性：任意线程、主线程、渲染线程、物理线程、音频线程。
- 工作窃取：每线程本地队列 + 全局队列，窃取按 FIFO，本地按 LIFO。
- 线程：主线程、渲染线程、RHI 线程、物理线程、音频线程、资产线程、网络线程、AI 线程、2D 渲染线程。
- 同步：Fence、Latch、Event、Channel、无锁队列、原子操作。
- 调试：线程命名、任务追踪、火焰图、死锁检测、超时告警、任务依赖可视化。
- 边界条件：依赖循环必须检测；取消后不得访问已释放资源；线程退出必须等待任务完成。
- 性能：100 万 Job/秒；调度延迟 < 10μs；窃取延迟 < 1μs。
- 验收测试：Job 执行、依赖顺序、取消、优先级、亲和性、工作窃取、同步原语。

A6 内存
- 分配器：全局（System）、Arena、Pool、Stack、Buddy、TLSF、Slab。
- Arena：线性分配，批量释放，支持嵌套作用域。
- Pool：固定大小块，O(1) 分配释放。
- Stack：LIFO，支持标记与回滚。
- Buddy：2 的幂，支持合并。
- TLSF：O(1) 分配释放，低碎片。
- GPU：上传堆、暂存堆、可读回堆、持久映射、环形缓冲、内存类型查询。
- 追踪：分配栈、泄漏检测、峰值、碎片率、标签分类、调用图。
- 泄漏检测：记录每次分配的栈，退出时报告未释放。
- 边界条件：双重释放必须检测；越界必须检测；对齐必须满足；OOM 必须优雅处理。
- 性能：分配 < 50ns；释放 < 50ns；泄漏检测开销 < 5%。
- 验收测试：分配器正确性、泄漏检测、碎片率、GPU 堆、OOM 处理。

A7 数学
- SIMD：SSE/AVX/AVX2/AVX-512、NEON、SVE，运行时特性检测，标量回退。
- 3D 类型：Vec2/3/4、Mat2/3/4、Quat、DualQuat、Transform、AABB、OBB、Sphere、Capsule、Frustum、Plane、Ray、Curve、Spline、Bezier、CatmullRom、Noise。
- 2D 类型：Vec2、Mat2x3、Mat3、Rot2、Transform2D、AABB2、Circle、Polygon、Ray2、CubicBezier2、CatmullRom2。
- 精度：f32/f64 双精度，大世界坐标用 f64 或相机相对坐标，支持原点重定位。
- 插值：lerp、slerp、nlerp、hermite、catmull-rom、bezier、ease。
- 几何：相交、包含、距离、投影、反射、折射、裁剪。
- 噪声：Perlin、Simplex、Worley、Value、FBM、Domain Warping。
- 随机：PCG、XorShift、ChaCha，支持种子与序列。
- 边界条件：NaN 检测、除零保护、退化情况处理、精度阈值。
- 性能：矩阵乘 4x4 < 5ns；向量点积 < 1ns；四元数 slerp < 10ns。
- 验收测试：数值正确性、SIMD 一致性、边界条件、性能基准、误差阈值。

A8 平台抽象
- 窗口：Windows Win32、Linux X11/Wayland、Android ANativeWindow，支持多窗口、无边框、全屏、高 DPI、VSync、HDR、窗口图标、标题、最小化、最大化、关闭事件。
- 输入：键鼠、触摸、多点触控、手柄（XInput、DirectInput、evdev、Android Input）、IME、输入法、手势、映射、重绑定、录制回放。
- 文件：路径规范、虚拟文件系统、沙盒、Android assets、权限、文件监听。
- 线程/时间：高精度计时、单调时钟、休眠、定时器、时区。
- 电源：电量、热状态、后台暂停、恢复、低功耗模式。
- 权限：Android 运行时权限、存储、网络、麦克风、相机、位置。
- 生命周期：Android onPause/onResume/onDestroy、Windows 休眠、Linux 信号、iOS 后台。
- 剪贴板：文本、图像、文件。
- 通知：系统通知、Toast、权限请求。
- 边界条件：窗口关闭必须清理资源；权限拒绝必须优雅降级；生命周期事件必须同步。
- 性能：窗口创建 < 100ms；输入延迟 < 10ms；文件读取 > 1GB/s。
- 验收测试：每平台窗口创建、输入事件、文件访问、权限请求、生命周期、剪贴板。

A9 事件/消息
- 发布订阅：EventBus，类型安全，优先级，一次性，取消，延迟，线程安全。
- 事件类型：struct 事件，derive(Event)，自动注册。
- 优先级：Critical、High、Normal、Low。
- 一次性：once 标记，触发后自动取消订阅。
- 延迟：延迟到帧末、帧首、下一帧。
- 线程安全：跨线程发布，无锁路径，顺序保证。
- 输入映射：Action/Axis、上下文、优先级、冲突解决、重绑定、组合键、长按、双击。
- 网络事件、编辑器事件、UI 事件统一总线。
- 边界条件：订阅者销毁必须自动取消；事件风暴必须限流；循环事件必须检测。
- 性能：100 万事件/秒；延迟 < 1μs；无锁路径。
- 验收测试：订阅发布、优先级、取消、延迟、线程安全、输入映射。

A10 插件
- 动态库：Windows DLL、Linux SO、Android SO、macOS dylib。
- 版本：语义化版本、ABI 校验、依赖解析、冲突检测、兼容性检查。
- 热插拔：编辑器内加载/卸载，不重启，状态保存与恢复。
- 扩展点：RHI 后端、AssetImporter、EditorPanel、DebuggerProvider、ScriptHost、RenderPass、PhysicsBackend、AudioBackend、2D 渲染后端、2D 物理后端、分发后端。
- 注册：Plugin trait，fn register(&mut self, registry: &mut Registry)。
- 元数据：名称、版本、作者、描述、依赖、许可证。
- 沙盒：可选沙盒执行，资源限制，权限控制。
- 边界条件：版本不匹配必须拒绝；依赖缺失必须报错；卸载必须清理资源。
- 性能：插件加载 < 100ms；热重载 < 500ms。
- 验收测试：插件加载、卸载、依赖解析、扩展点注册、热重载、沙盒。

B. 3D 渲染

B1 RHI
- 后端：Vulkan 1.3、DX12、DX11、OpenGL 4.6、GLES 3.2、WebGPU、Metal。
- Device：物理设备枚举、逻辑设备创建、队列族查询、特性查询、限制查询、扩展查询。
- Queue：图形、计算、传输、呈现、稀疏绑定。
- Swapchain：创建、重建、呈现模式、格式、颜色空间、HDR。
- CommandBuffer：录制、提交、重置、多线程录制、次级缓冲。
- Pipeline：图形、计算、光追、网格着色、管线缓存、管线布局。
- DescriptorSet：布局、池、更新、绑定、动态偏移、无绑定。
- Texture：1D/2D/3D/Cube/Array、格式、Mipmap、采样、上传、下载、视图。
- Buffer：顶点、索引、统一、存储、间接、暂存、上传、下载。
- Sampler：过滤、寻址、各向异性、比较、LOD。
- Fence、Semaphore、Event、Query、Timestamp。
- 能力查询：格式、特性、限制、扩展、队列族、内存类型。
- 同步：自动屏障、多队列、异步计算、传输队列、屏障批处理。
- 错误处理：验证层、调试标记、错误回调、设备丢失恢复。
- 边界条件：设备丢失必须恢复；OOM 必须降级；格式不支持必须回退；队列冲突必须检测。
- 性能：Draw Call 开销 < 2μs；Descriptor 更新 < 500ns；屏障 < 100ns。
- 验收测试：每后端创建、资源创建、管线创建、命令录制、提交、呈现、能力查询、错误处理。

B2 渲染图
- 自动屏障：资源状态追踪，自动插入屏障，批处理优化。
- 资源别名：生命周期分析，内存复用，别名屏障。
- 生命周期：资源引用计数，延迟释放，帧间复用。
- 异步计算：计算队列并行，屏障同步，结果合并。
- 多线程录制：每线程命令缓冲，合并提交。
- Pass 类型：Raster、Compute、RayTracing、Copy、Present、2D。
- Pass 依赖：显式依赖、隐式依赖、循环检测。
- 调试：可视化、耗时、带宽、依赖图、资源查看。
- 边界条件：循环依赖必须检测；资源冲突必须报错；别名必须正确同步。
- 性能：1000+ Pass/帧；录制 < 2ms；屏障开销 < 5%。
- 验收测试：Pass 依赖、自动屏障、资源别名、异步计算、多线程录制。

B3 材质
- PBR：Metallic/Roughness、Specular/Glossiness、ClearCoat、Sheen、Anisotropy、SSS、Transmission、Iridescence。
- 节点图：节点、连接、编译、优化、变体管理、关键字、实例参数。
- 编译：WGSL、GLSL、HLSL、SPIR-V、DXIL、Metal Shading Language。
- 变体：关键字组合、变体爆炸合并、按需编译、预编译。
- 实例：材质实例、参数覆盖、动态参数、参数集合。
- 缓存：磁盘缓存、增量编译、跨平台预编译、版本管理。
- 2D 材质：Sprite、九宫格、法线贴图、2D 光照材质。
- 边界条件：编译失败必须报错；循环节点必须检测；参数越界必须校验。
- 性能：材质编辑到预览 < 500ms；变体编译 < 1s；运行时切换 < 1ms。
- 验收测试：节点图编译、变体管理、参数覆盖、缓存、热重载。

B4 光照
- 3D 光源：方向、点、聚、区域、矩形、管状、IES、天空、HDRI。
- 2D 光源：点、聚、方向、环境、法线混合。
- 阴影：Shadow Map、CSM、PCF、PCSS、VSM、接触阴影、胶囊阴影、光线追踪阴影。
- IBL：漫反射 + 镜面 + 球谐 + 反射探针 + 混合。
- 光照探针：体积、球谐、辐照度、反射、自适应放置。
- 阴影级联：级联分割、过渡、抖动、稳定。
- 边界条件：光源数量超限必须降级；阴影贴图溢出必须处理；探针缺失必须回退。
- 性能：目标 1000 动态光源 @ 1080p 60FPS（集群/前向+），作为 Target。
- 验收测试：光源类型、阴影、IBL、探针、级联、性能。

B5 全局光照
- 动态 GI：SSGI、DDGI、RTGI、Lumen 对标，支持动态几何、动态光源。
- 2D GI：2D 光照传播、2D 辐照度、2D 反射探针。
- 探针：体积、球谐、辐照度、反射、自适应放置。
- 降噪：时空降噪、AI 降噪、双边滤波。
- 边界条件：漏光必须抑制；闪烁必须消除；性能不足必须降级。
- 性能：目标室内外无缝、延迟 < 2ms、无漏光、无闪烁，作为 Target。
- 验收测试：GI 质量、降噪、性能、边界条件。

B6 虚拟几何
- Nanite 对标：网格集群、自动 LOD、流送、遮挡剔除、GPU 剔除、间接绘制。
- 集群：网格分割、集群构建、层次结构、LOD 生成。
- 流送：按需加载、优先级、预取、卸载。
- 剔除：视锥、遮挡、距离、GPU 剔除。
- 间接绘制：间接缓冲、批次合并、GPU 驱动。
- 支持：静态网格、蒙皮网格、程序化、地形、植被。
- 边界条件：集群缺失必须回退；流送失败必须降级；LOD 切换必须无感。
- 性能：目标 10 亿三角面 @ 1080p 60FPS，LOD 切换无感，内存 < 2GB，作为 Target。
- 验收测试：集群构建、LOD、流送、剔除、间接绘制、性能。

B7 后处理
- HDR、色调映射（ACES、Filmic、AgX、Reinhard）、Bloom、SSAO、SSR、景深、运动模糊、镜头光晕、色差、暗角、胶片颗粒。
- 抗锯齿：MSAA、FXAA、TAA、SMAA。
- 超分辨率：定义 UpscalerProvider trait，内置 FSR 1/2（MIT 许可，可源码集成）。
- DLSS、XeSS、FSR 3/4 帧生成：不内置、不捆绑二进制。引擎只提供标准 UpscalerProvider 接口和接入文档，由获得对应厂商授权的团队作为独立可选插件加载。引擎本体不承担授权与分发责任。
- 2D 后处理：像素完美缩放、CRT、扫描线、色差、Bloom、模糊。
- 顺序：按优先级排序，支持自定义顺序。
- 边界条件：分辨率变化必须重建；效果冲突必须检测；性能不足必须降级。
- 性能：全部可开关、可混合、可排序，性能预算 < 3ms。
- 验收测试：每效果正确性、顺序、性能、边界条件。

B8 特效
- 3D GPU 粒子：目标 10M 粒子 @ 60FPS，支持碰撞、GPU 事件、曲线、网格、拖尾、光束，作为 Target。
- 2D 粒子：Sprite 粒子、拖尾、光束、天气、GPU 加速。
- Niagara 对标：模块化、可视化、CPU/GPU 双后端。
- 体积雾、云、天空、水、地形、植被、风、破坏。
- 模块：发射、生命周期、速度、力、碰撞、渲染、事件。
- 边界条件：粒子超限必须降级；GPU 事件必须同步；碰撞必须正确。
- 性能：编辑器实时预览；热重载 < 1s。
- 验收测试：粒子发射、碰撞、事件、渲染、性能。

B9 光追
- 反射、阴影、GI、AO、路径追踪。
- 支持 DXR、Vulkan RT、Metal RT。
- 加速结构：BLAS、TLAS、更新、压缩。
- 降噪：时空、AI、双边。
- 混合：与光栅混合、性能模式切换。
- 边界条件：硬件不支持必须回退；加速结构溢出必须处理。
- 性能：性能模式切换；降噪质量可调。
- 验收测试：反射、阴影、GI、AO、路径追踪、降噪、混合。

B10 纹理
- 虚拟纹理、纹理流送、压缩（BCn、ASTC、ETC2）、Mipmap、各向异性、稀疏纹理。
- 2D 纹理：像素完美、点采样、图集、九宫格、Sprite 表。
- 流送：按需加载、优先级、预取、卸载、显存超配降级。
- 压缩：离线压缩、运行时压缩、格式转换。
- 边界条件：显存不足必须降级；格式不支持必须回退；流送失败必须处理。
- 性能：8K 纹理流送；卡顿 < 1 帧；显存超配自动降级。
- 验收测试：纹理创建、流送、压缩、Mipmap、各向异性、虚拟纹理。

C. 物理

C1 刚体
- 3D 形状：Box、Sphere、Capsule、Cylinder、Convex、Mesh、Heightfield、Compound。
- 2D 形状：Box、Circle、Capsule、Polygon、Edge、Chain。
- 材质：摩擦、弹性、密度、阻尼、CCD、休眠。
- 查询：射线、扫掠、重叠、接触、过滤。
- 积分：半隐式欧拉、Verlet、RK4。
- 碰撞检测：宽相（BVH、SAP、Grid）、窄相（GJK、EPA、SAT）。
- 求解器：顺序冲量、投影高斯-赛德尔、TGS。
- 边界条件：穿透必须处理；休眠必须唤醒；CCD 必须启用；数值不稳定必须检测。
- 性能：目标 100 万刚体 @ 60FPS（多线程），确定性回放，作为 Target。
- 验收测试：形状、材质、查询、碰撞、求解、CCD、休眠。

C2 高级
- 3D 关节：Hinge、Slider、Fixed、Ball、D6、弹簧、齿轮、齿条。
- 2D 关节：Distance、Revolute、Prismatic、Weld、Wheel、Pulley、Gear。
- 车辆、布料、破坏、流体、角色控制器、载具。
- 布料：质点弹簧、位置基础动力学、碰撞、自碰撞。
- 破坏：预碎裂、运行时碎裂、GPU 加速。
- 流体：SPH、PBF、网格流体。
- 边界条件：关节断裂必须处理；布料穿透必须处理；破坏碎片超限必须降级。
- 性能：Chaos 对标；破坏支持百万碎片；GPU 加速。
- 验收测试：关节、车辆、布料、破坏、流体、角色控制器。

C3 工程
- 多线程、确定性、可视化、录制回放、网络同步、调试绘制。
- 确定性：固定时间步、确定性随机、确定性求解顺序。
- 录制回放：状态快照、输入录制、回放。
- 网络同步：状态同步、插值、预测、回滚。
- 调试绘制：碰撞体、接触点、射线、关节、力。
- 边界条件：非确定性必须检测；网络延迟必须处理；回放必须一致。
- 性能：物理线程独立；可视化开销 < 1%。
- 验收测试：多线程、确定性、录制回放、网络同步、调试绘制。

D. 动画

D1 基础
- 3D：骨骼、蒙皮、GPU 蒙皮、LOD、法线/切线重算。
- 2D：帧动画、骨骼动画、Sprite 变形、Mesh 变形。
- 骨骼：层次结构、绑定姿势、逆绑定矩阵、蒙皮矩阵。
- 蒙皮：线性混合、双四元数、GPU 蒙皮。
- LOD：距离、屏幕占比、性能。
- 边界条件：骨骼缺失必须回退；蒙皮矩阵溢出必须处理；LOD 切换必须无感。
- 性能：目标 1000 角色 @ 60FPS；10 万骨骼场景，作为 Target。
- 验收测试：骨骼、蒙皮、GPU 蒙皮、LOD、法线重算。

D2 动画图
- 动画蓝图对标：状态机、混合、同步组、根运动、Montage、通知、曲线、事件。
- 状态机：状态、转换、条件、优先级、中断。
- 混合：线性、加法、分层、骨骼遮罩、同步组。
- 根运动：提取、应用、重定向。
- Montage：分段、插槽、通知。
- 2D/3D 共用动画图，节点自动适配。
- 边界条件：循环转换必须检测；混合权重必须归一化；通知必须可靠。
- 性能：节点图编译 < 1s；运行时开销 < 0.5ms/角色。
- 验收测试：状态机、混合、根运动、Montage、通知、曲线。

D3 高级
- IK/FK、Control Rig 对标、重定向、变形目标、面部、口型、程序化、物理动画。
- 2D IK、2D 骨骼、2D 变形。
- IK：FABRIK、CCD、Jacobian、Two-Bone。
- 重定向：骨骼映射、比例调整、姿势适配。
- 变形目标：Morph Target、混合形状、法线重算。
- 面部：表情、口型、眼动。
- 边界条件：IK 无解必须回退；重定向失败必须报错；变形超限必须处理。
- 性能：编辑器实时预览；热重载 < 500ms。
- 验收测试：IK、Control Rig、重定向、变形目标、面部、程序化。

E. 音频

E1 音频图
- MetaSounds 对标：节点图、DSP、合成、采样、调制、空间化。
- 3D：HRTF、衰减、混响、遮挡、衍射、房间。
- 2D：立体声、声像、音量、滤波器。
- 节点：振荡器、滤波器、包络、LFO、混音、采样播放。
- 空间化：距离衰减、方向、HRTF、混响区域。
- 边界条件：音频设备丢失必须恢复；缓冲区欠载必须处理；采样率不匹配必须转换。
- 性能：目标 1000 声源 @ 60FPS；桌面延迟 < 10ms；移动端 < 20ms，作为 Target。
- 验收测试：节点图、DSP、合成、空间化、衰减、混响。

E2 高级
- 麦克风、MIDI、字幕、口型、音频分析、频谱、节拍检测。
- 麦克风：采集、回声消除、噪声抑制。
- MIDI：输入、输出、映射。
- 字幕：时间轴、同步、多语言。
- 口型：音素、Viseme、同步。
- 音频分析：频谱、节拍、音高、响度。
- 边界条件：设备权限必须请求；MIDI 设备断开必须处理；分析延迟必须可配。
- 性能：分析开销 < 5%；MIDI 延迟 < 5ms。
- 验收测试：麦克风、MIDI、字幕、口型、音频分析。

F. 网络

F1 复制
- 属性同步、RPC、预测、回滚、权威、客户端-服务器、P2P、专用服务器。
- 属性同步：脏标记、增量、优先级、带宽预算。
- RPC：可靠、不可靠、有序、无序、广播。
- 预测：客户端预测、服务器校正、回滚。
- 回滚：状态快照、输入回放、确定性。
- 2D/3D 共用网络层，同步 2D 与 3D 组件。
- 边界条件：丢包必须处理；延迟必须补偿；作弊必须检测。
- 性能：目标 64 玩家 @ 60FPS；带宽 < 100KB/s/玩家；延迟补偿，作为 Target。
- 验收测试：属性同步、RPC、预测、回滚、权威、P2P。

F2 工程
- NAT 穿透、序列化、带宽优化、网络分析、反作弊接口、断线重连。
- NAT：STUN、TURN、打洞。
- 序列化：位压缩、增量、量化。
- 带宽优化：优先级、裁剪、压缩。
- 网络分析：延迟、丢包、带宽、流量图。
- 反作弊：接口、钩子、检测。
- 断线重连：会话恢复、状态同步。
- 边界条件：NAT 失败必须回退；重连失败必须处理；作弊必须记录。
- 性能：带宽优化 > 50%；分析开销 < 2%。
- 验收测试：NAT、序列化、带宽、分析、反作弊、重连。

G. AI

G1 行为
- 3D：行为树、黑板、EQS 对标、NavMesh、寻路、感知、群体、状态机、GOAP。
- 2D：A*、流场、NavMesh 2D、避障、感知。
- 行为树：节点、组合、装饰、服务。
- 黑板：键值、观察者、同步。
- EQS：查询、生成器、测试、评分。
- NavMesh：生成、优化、查询、动态障碍。
- 寻路：A*、JPS、流场、层次。
- 感知：视觉、听觉、触觉、记忆。
- 群体：分离、对齐、聚合、避障。
- 边界条件：路径失败必须回退；动态障碍必须更新；感知遮挡必须处理。
- 性能：目标 1000 AI @ 60FPS；NavMesh 10km²；动态障碍，作为 Target。
- 验收测试：行为树、黑板、EQS、NavMesh、寻路、感知、群体。

G2 高级
- 机器学习可选、调试可视化、热重载、录制回放。
- 机器学习：推理集成、模型加载、输入输出。
- 调试可视化：路径、感知、状态、决策。
- 热重载：行为树、参数、模型。
- 录制回放：状态、输入、决策。
- 边界条件：模型不兼容必须报错；热重载失败必须回退。
- 性能：推理 < 1ms；可视化开销 < 1%。
- 验收测试：机器学习、可视化、热重载、录制回放。

H. UI

H1 控件
- UMG/Slate 对标：控件、布局、动画、样式、数据绑定、虚拟化列表。
- 控件：按钮、文本、图像、输入、滑块、下拉、列表、树、表格、标签页、菜单、对话框。
- 布局：水平、垂直、网格、覆盖、滚动、锚点、边距、对齐。
- 动画：过渡、关键帧、曲线、触发器。
- 样式：主题、皮肤、颜色、字体、图标。
- 数据绑定：单向、双向、转换、验证。
- 虚拟化列表：按需创建、回收、滚动。
- 2D UI 与 3D UI 共用控件系统，支持世界空间 UI。
- 边界条件：布局循环必须检测；绑定失败必须报错；虚拟化必须正确回收。
- 性能：目标 10 万控件 @ 60FPS；布局 < 1ms，作为 Target。
- 验收测试：控件、布局、动画、样式、绑定、虚拟化。

H2 工程
- 输入、无障碍、多语言、字体、富文本、3D UI、2D UI、响应式、DPI。
- 输入：键盘、鼠标、触摸、手柄、焦点、导航。
- 无障碍：屏幕阅读器、高对比度、键盘导航、字幕。
- 多语言：翻译、复数、日期、货币、RTL。
- 字体：TTF、OTF、位图、SDF、动态加载。
- 富文本：标记、链接、图像、样式。
- 响应式：断点、缩放、适配。
- DPI：高 DPI、缩放、像素完美。
- 边界条件：字体缺失必须回退；翻译缺失必须回退；RTL 必须正确处理。
- 性能：字体加载 < 100ms；布局 < 1ms。
- 验收测试：输入、无障碍、多语言、字体、富文本、响应式、DPI。

I. 脚本与可视化

I1 蓝图对标
- 节点图、编译、热重载、调试、断点、单步、变量监视。
- 节点：事件、函数、变量、流程、数学、字符串、数组、结构、枚举、类。
- 编译：图到字节码、优化、类型检查。
- 热重载：增量编译、状态保存。
- 调试：断点、单步、变量、调用栈。
- 2D 与 3D 节点共用图系统。
- 边界条件：循环依赖必须检测；类型错误必须报错；热重载必须保留状态。
- 性能：编译 < 1s；运行时接近原生。
- 验收测试：节点图、编译、热重载、调试、断点、变量监视。

I2 脚本
- Rust 脚本、Lua/WASM 可选、GAS 对标、属性系统、技能、Buff、冷却。
- Rust 脚本：动态加载、热重载、沙盒。
- Lua：绑定、沙盒、热重载。
- WASM：绑定、沙盒、热重载。
- GAS：属性、技能、Buff、冷却、效果、标签。
- 机器学习：
  - 保留推理 + 引擎内训练，作为实验性可选模块。
  - 训练范围：Windows/Linux 桌面编辑器内训练；Android 只做推理，不做训练。
  - 后端：Burn、Candle、tch-rs、ONNX Runtime，做成可插拔 Backend trait。
  - 数据管线：从 ECS、资产系统、CSV、图像、序列、强化学习环境取数据。
  - 模型生命周期：定义、编译、训练、评估、导出、热重载、版本管理。
  - 训练可视化：损失曲线、指标、张量查看、计算图、梯度直方图。
  - 训练调试：断点、单步、梯度检查、性能分析、显存/内存分析。
  - 推理集成：动画变形、降噪、超分、AI 行为、材质生成、程序化内容、2D 精灵生成。
  - 验收定位：P2/实验特性，不作为 P0/P1 硬性验收；训练性能指标标为 Target。
- 边界条件：脚本错误必须捕获；热重载失败必须回退；沙盒逃逸必须阻止。
- 性能：脚本调用 < 1μs；热重载 < 1s；推理 < 10ms。
- 验收测试：Rust 脚本、Lua、WASM、GAS、机器学习推理、机器学习训练。

J. 编辑器

J1 基础
- 停靠、布局、主题、视口、Gizmo、属性、资产、内容浏览器、命令面板。
- 停靠：拖拽、浮动、标签、分割、保存布局。
- 主题：亮、暗、自定义、颜色、字体、图标。
- 视口：2D、3D、混合、多视口、相机控制。
- Gizmo：平移、旋转、缩放、局部、全局、吸附。
- 属性：自动生成、分组、搜索、过滤、撤销重做。
- 资产：浏览、搜索、过滤、预览、拖拽、导入。
- 内容浏览器：文件夹、标签、收藏、最近。
- 命令面板：搜索、执行、快捷键。
- 2D 视口与 3D 视口一键切换，2D/3D 混合模式。
- 边界条件：布局损坏必须恢复；属性越界必须校验；资产缺失必须提示。
- 性能：启动 < 3s；60FPS UI；10 万资产流畅浏览。
- 验收测试：停靠、主题、视口、Gizmo、属性、资产、命令面板。

J2 专业面板
- 材质编辑器、蓝图编辑器、Sequencer、动画、曲线、地形、植被、粒子、物理、音频、网络、性能面板。
- 2D 专业面板：Sprite 编辑器、瓦片地图编辑器、2D 骨骼编辑器、2D 动画时间轴、像素画布。
- 材质编辑器：节点图、预览、参数、变体。
- 蓝图编辑器：节点图、调试、断点。
- Sequencer：时间轴、关键帧、曲线、相机、事件。
- 动画：骨骼、蒙皮、状态机、混合。
- 曲线：编辑、插值、切线。
- 地形：高度图、纹理、植被、雕刻。
- 植被：散布、LOD、风。
- 粒子：模块、预览、性能。
- 物理：调试、可视化、参数。
- 音频：节点图、预览、空间化。
- 网络：模拟、延迟、丢包、流量。
- 性能：CPU、GPU、内存、带宽、Draw Call。
- 分发面板：构建配置、目标平台、商店、签名、更新、成就、云存档、崩溃报告。
- 边界条件：面板崩溃必须隔离；数据损坏必须恢复。
- 性能：每面板 60FPS；热重载 < 1s。
- 验收测试：每面板功能、性能、热重载。

J3 工程
- 插件、版本控制、协作、撤销重做、多语言、远程调试、崩溃恢复。
- 插件：加载、卸载、管理、市场。
- 版本控制：Git 集成、提交、分支、合并、冲突。
- 协作：多用户、锁、同步、冲突。
- 撤销重做：全局栈、分组、合并。
- 多语言：翻译、切换、RTL。
- 远程调试：Android、iOS、远程桌面。
- 崩溃恢复：自动保存、恢复、报告。
- 边界条件：版本冲突必须提示；协作冲突必须解决；崩溃必须恢复。
- 性能：撤销重做 < 1ms；保存 < 200ms。
- 验收测试：插件、版本控制、协作、撤销重做、多语言、远程调试、崩溃恢复。

K. 调试

K1 CPU
- 断点、条件、数据断点、单步、调用栈、变量、表达式、内存、泄漏、线程。
- 断点：行、函数、条件、数据、临时。
- 单步：进入、跳过、跳出、运行到光标。
- 调用栈：展开、跳转、内联。
- 变量：查看、修改、监视、表达式。
- 内存：查看、修改、搜索、断点。
- 泄漏：分配栈、报告、过滤。
- 线程：列表、切换、堆栈。
- 边界条件：断点冲突必须处理；变量优化必须提示；内存越界必须检测。
- 性能：附加上去 < 100ms；断点命中 < 1ms；不影响 60FPS。
- 验收测试：断点、单步、调用栈、变量、表达式、内存、泄漏、线程。

K2 GPU
- 帧捕获、管线状态、资源查看、着色器变量输出。
- RenderDoc 集成：改为“RenderDoc 兼容调试层接口”。引擎负责输出标准 Vulkan/DX12 调试标记、PDB 符号、着色器调试信息，并在文档中提供 RenderDoc 捕获配置。不承诺全后端全平台自动集成，因为 RenderDoc 本身对自定义引擎和 Vulkan 有已知限制。
- 着色器调试：改为“GPU 帧捕获 + 着色器代码注入式变量输出”。不承诺硬件级着色器断点，因为目前没有跨厂商公开的 GPU 硬件断点 API。
- 2D 渲染调试：批次查看、图集查看、Sprite 统计、2D 光照调试。
- 帧捕获：绘制调用、资源、管线、耗时、带宽。
- 管线状态：管线、描述符、屏障、队列。
- 资源查看：纹理、缓冲、采样器、格式。
- 边界条件：捕获失败必须报错；资源超限必须处理。
- 性能：捕获 1 帧 < 500ms；着色器变量可在帧捕获中查看。
- 验收测试：帧捕获、管线状态、资源查看、着色器变量、2D 调试。

K3 性能
- CPU/GPU Profiler、火焰图、计数器、内存、带宽、Draw Call。
- CPU：采样、火焰图、调用图、线程。
- GPU：时间戳、管线、带宽、占用。
- 计数器：自定义、分组、导出。
- 内存：分配、峰值、碎片、标签。
- 带宽：读、写、总。
- Draw Call：数量、批次、状态切换。
- 2D 专用计数器：Sprite 数、批次数、图集切换、瓦片数。
- 边界条件：采样开销必须可控；数据丢失必须处理。
- 性能：采样 < 1% 开销；支持远程 Android。
- 验收测试：CPU、GPU、计数器、内存、带宽、Draw Call、2D 计数器。

K4 工程
- 日志、控制台、远程 Android、热重载、单元/集成/模糊测试。
- 日志：级别、分类、格式化、过滤、输出。
- 控制台：命令、变量、自动补全、历史。
- 远程 Android：连接、调试、日志、性能。
- 热重载：资产、代码、着色器。
- 测试：单元、集成、模糊、性能、回归。
- 边界条件：日志溢出必须轮转；控制台命令错误必须提示。
- 性能：日志 100 万行/秒；控制台命令可脚本化。
- 验收测试：日志、控制台、远程调试、热重载、测试。

L. 2D 支持

L1 2D 渲染
- 正交相机、像素完美、Y 轴方向可配置、像素网格对齐。
- Sprite、SpriteSheet、Atlas、NinePatch、Tiled、Animated Sprite、Skeletal 2D、Mesh 2D。
- 图层、排序、Z 顺序、排序组、遮罩、裁剪、混合模式。
- 2D 光照：点光、聚光、方向光、法线贴图、阴影、法线混合、光照贴图。
- 2D 材质、自定义着色器、后处理。
- 2D 粒子、拖尾、光束、天气。
- 批次：自动合并、材质排序、图集排序、动态批次。
- 边界条件：图集切换必须最小化；像素完美必须保持；排序冲突必须检测。
- 性能：目标 10 万 Sprite @ 60FPS；批次合并；Draw Call < 100。
- 验收测试：Sprite、图集、图层、光照、材质、粒子、批次。

L2 2D 物理
- 刚体、碰撞体、触发器、射线、形状：Box、Circle、Capsule、Polygon、Edge、Chain。
- 关节：Distance、Revolute、Prismatic、Weld、Wheel、Pulley、Gear。
- 角色控制器、平台跳跃、单向平台、斜坡、移动平台。
- 连续碰撞检测、CCD、休眠、过滤、层级。
- 边界条件：穿透必须处理；休眠必须唤醒；CCD 必须启用。
- 性能：目标 10 万刚体 @ 60FPS；确定性回放。
- 验收测试：形状、关节、角色控制器、CCD、休眠、过滤。

L3 2D 动画
- 帧动画、骨骼动画、Sprite 变形、Mesh 变形、曲线动画。
- 状态机、混合、同步组、事件、通知、根运动。
- Aseprite 导入、Spine 兼容格式导入、DragonBones 兼容格式导入。
- 边界条件：帧率不匹配必须处理；骨骼缺失必须回退。
- 性能：目标 1000 动画角色 @ 60FPS。
- 验收测试：帧动画、骨骼动画、状态机、混合、导入。

L4 2D 瓦片地图
- Tiled TMX/TSX、LDtk、Godot TileMap、自定义瓦片格式。
- 图层、对象层、碰撞层、自动瓦片、地形刷、规则瓦片。
- 大世界分块、流送、LOD、遮挡剔除。
- 边界条件：瓦片缺失必须回退；分块加载必须无卡顿。
- 性能：目标 100 万瓦片 @ 60FPS。
- 验收测试：导入、图层、碰撞、自动瓦片、流送、LOD。

L5 2D UI
- 与 3D UI 共用控件系统，2D 专用布局、锚点、分辨率适配。
- 9-Slice、像素字体、位图字体、SDF 字体。
- 边界条件：分辨率变化必须适配；字体缺失必须回退。
- 性能：布局 < 1ms；渲染 < 1ms。
- 验收测试：布局、锚点、9-Slice、字体。

L6 2D 编辑器
- 2D 视口、网格、吸附、像素对齐、图层管理、瓦片编辑、Sprite 编辑、骨骼编辑、动画时间轴。
- 与 3D 编辑器一键切换，共享资产浏览器与属性面板。
- 边界条件：编辑冲突必须检测；撤销重做必须完整。
- 性能：60FPS UI；编辑延迟 < 16ms。
- 验收测试：视口、网格、吸附、图层、瓦片、Sprite、骨骼、时间轴。

L7 2D 调试
- 2D 碰撞体可视化、物理调试、渲染批次、Sprite 统计、瓦片统计。
- 与 3D 调试共用 Profiler、帧捕获、日志。
- 边界条件：可视化开销必须可控。
- 性能：可视化开销 < 1%。
- 验收测试：碰撞体、物理、批次、Sprite、瓦片。

L8 2D 脚本与 AI
- 2D 寻路、NavMesh 2D、A*、流场、行为树、状态机。
- 2D 感知、群体、避障。
- 边界条件：路径失败必须回退；动态障碍必须更新。
- 性能：目标 1000 AI @ 60FPS。
- 验收测试：寻路、NavMesh、行为树、状态机、感知、群体。

M. 3D 支持
M1 3D 渲染：见 B 节，全部保留。
M2 3D 物理：见 C 节，全部保留。
M3 3D 动画：见 D 节，全部保留。
M4 3D 音频：见 E 节，全部保留。
M5 3D 网络：见 F 节，全部保留。
M6 3D AI：见 G 节，全部保留。
M7 3D UI：见 H 节，全部保留。
M8 3D 编辑器：见 J 节，全部保留。
M9 3D 调试：见 K 节，全部保留。

N. 混合 2D/3D
N1 同一场景同时包含 2D 与 3D 实体。
N2 2D 相机与 3D 相机可共存、可切换、可叠加。
N3 2D 对象可渲染到 3D 世界空间，3D 对象可渲染到 2D 屏幕空间。
N4 2D 物理与 3D 物理可互操作，共享事件、查询、碰撞过滤。
N5 2D 动画与 3D 动画共用骨骼、状态机、混合系统。
N6 编辑器支持同一项目内混合 2D/3D 模式，资产浏览器统一。
N7 渲染顺序：2D 与 3D 图层可交叉排序，支持覆盖、叠加、混合。
N8 坐标系统：2D 与 3D 坐标可转换，支持世界空间与屏幕空间互转。
N9 输入：2D 与 3D 输入统一，支持拾取、命中测试。
N10 边界条件：混合模式冲突必须检测；坐标系转换必须精确。
N11 性能：混合场景 60FPS；2D 与 3D 批次独立优化。
N12 验收测试：混合场景、相机、渲染、物理、动画、编辑器、输入。

O. 资产导入详细规格

O1 贴图/图像格式
- 位图：PNG、JPG、JPEG、BMP、TGA、GIF、TIFF、WebP、AVIF、QOI、PNM、HDR、EXR、DDS、KTX、KTX2、PVR、ASTC。
- 矢量：SVG、PDF（栅格化）。
- 专业：PSD、PSB、XCF、KRA（分层导入）。
- 2D 专用：Aseprite（.ase、.aseprite）、Sprite Sheet JSON、Texture Atlas、Tiled TSX。
- 特性：异步、流式、Mipmap、压缩、色彩空间、HDR、Alpha、通道打包、图集生成、像素完美、法线贴图、粗糙度、金属度、AO、置换、光照贴图。
- 边界条件：损坏文件必须报错；格式不支持必须提示；色彩空间必须正确。
- 性能：4K 纹理导入 < 1s；8K 纹理导入 < 5s。
- 验收测试：每格式导入、特性、错误处理、性能。

O2 3D 模型格式
- 通用：glTF、GLB、FBX、OBJ、DAE、USD、USDZ、ABC、PLY、STL、3DS、X、MD5、MD3、BVH。
- DCC：Blend、MAX、MA、MB、C4D（通过插件或转换）。
- 特性：网格、骨骼、蒙皮、动画、材质、贴图、法线、切线、UV、顶点色、形态目标、LOD、碰撞体、物理属性、场景层级、相机、灯光。
- 边界条件：损坏文件必须报错；骨骼缺失必须回退；动画不兼容必须提示。
- 性能：100 万三角面导入 < 5s；骨骼动画导入 < 1s。
- 验收测试：每格式导入、特性、错误处理、性能。

O3 2D 模型格式
- Sprite、SpriteSheet、Atlas、NinePatch、Tiled TMX/TSX、LDtk、Godot TileMap、Aseprite、Spine、DragonBones、Cocos、Unity 2D 格式（通过插件）。
- 边界条件：格式不支持必须提示；图集冲突必须检测。
- 性能：图集导入 < 500ms；瓦片地图导入 < 1s。
- 验收测试：每格式导入、特性、错误处理。

O4 音频格式
- WAV、OGG、MP3、FLAC、AAC、M4A、OPUS、AIFF、MOD、XM、IT、S3M。
- 边界条件：损坏文件必须报错；采样率不匹配必须转换。
- 性能：音频导入 > 100MB/s。
- 验收测试：每格式导入、特性、错误处理。

O5 视频格式
- MP4、WebM、MKV、MOV、AVI（解码帧导入）。
- 边界条件：编解码器不支持必须提示。
- 性能：视频解码 > 60FPS。
- 验收测试：每格式导入、特性、错误处理。

O6 字体格式
- TTF、OTF、WOFF、WOFF2、BMFont、SDF、位图字体。
- 边界条件：字体缺失必须回退；字符集不支持必须提示。
- 性能：字体加载 < 100ms。
- 验收测试：每格式导入、特性、错误处理。

O7 数据格式
- JSON、YAML、TOML、XML、CSV、TSV、MessagePack、CBOR、SQLite。
- 边界条件：格式错误必须报错；编码不支持必须提示。
- 性能：解析 > 100MB/s。
- 验收测试：每格式导入、特性、错误处理。

O8 导入管线
- 统一 AssetImporter trait。
- 插件注册。
- 异步、缓存、依赖图、热重载。
- 导入设置、预设、批处理、命令行导入。
- 导入报告、错误、警告、日志。
- DCC 往返、源文件保留、重导入、覆盖策略、冲突检测。
- 格式转换、优化、压缩、打包。
- 边界条件：导入失败必须回滚；冲突必须提示；依赖缺失必须报错。
- 性能：批量导入 > 100 资产/秒；热重载 < 1s。
- 验收测试：导入、缓存、依赖、热重载、批处理、报告。

O9 资产元数据
- 缩略图、预览、标签、分类、搜索、过滤、收藏、最近使用。
- 引用追踪、依赖图、循环检测。
- 版本、变更历史、差异比较。
- 边界条件：元数据损坏必须恢复；引用失效必须提示。
- 性能：10 万资产搜索 < 100ms；缩略图生成 < 100ms。
- 验收测试：元数据、搜索、过滤、引用、版本、差异。

P. 全平台分发详细规格

P1 构建系统
- BuildProfile：Debug、Release、Shipping、Development、Test。
- 目标平台：Windows x64/arm64、Linux x64/arm64、Android arm64/armv7、macOS x64/arm64、iOS arm64、Web WASM。
- 构建配置：优化级别、LTO、strip、符号、调试信息、代码签名、加密、压缩。
- 一键构建：cargo run --release --target <platform>，自动调用各平台工具链。
- 交叉编译：cross、cargo-ndk、cargo-xwin、cargo-zigbuild。
- 增量构建：缓存、依赖追踪、并行编译。
- 构建产物：可执行文件、资源包、配置文件、符号文件、清单。
- 构建报告：耗时、大小、依赖、警告、错误。
- 边界条件：工具链缺失必须提示；签名失败必须报错；交叉编译失败必须回退。
- 性能：增量构建 < 10s；全量构建 < 10min。
- 验收测试：每平台构建、交叉编译、增量、产物、报告。

P2 打包格式
- Windows：EXE、MSI、MSIX、Inno Setup、NSIS、WiX、Portable ZIP。
- Linux：AppImage、Flatpak、Snap、deb、rpm、tar.gz、AUR。
- Android：APK、AAB、OBB、AssetPack。
- macOS：APP、DMG、PKG、Mac App Store 包。
- iOS：IPA、TestFlight 包。
- Web：WASM + HTML + JS、Service Worker、PWA。
- 通用：ZIP、7z、tar.xz。
- 资源打包：PAK、资源加密、资源压缩、资源分块。
- 边界条件：打包失败必须回滚；资源缺失必须报错；大小超限必须提示。
- 性能：打包 < 1min；资源压缩 > 100MB/s。
- 验收测试：每格式打包、安装、启动、卸载。

P3 代码签名与公证
- Windows：Authenticode、EV 证书、时间戳、SmartScreen。
- macOS：Apple Developer ID、公证、Hardened Runtime、Entitlements。
- iOS：Apple 证书、Provisioning Profile、App Store 签名。
- Android：APK 签名 v1/v2/v3/v4、Play App Signing、上传密钥。
- Linux：GPG 签名、仓库签名。
- 签名配置：密钥管理、CI 集成、环境变量、密钥库。
- 边界条件：签名失败必须报错；证书过期必须提示；密钥泄露必须撤销。
- 性能：签名 < 30s。
- 验收测试：每平台签名、验证、公证。

P4 商店与分发渠道
- Windows：Steam、Epic Games Store、GOG、Microsoft Store、itch.io、独立 EXE。
- Linux：Steam、itch.io、Flathub、Snap Store、发行版仓库、独立 AppImage。
- Android：Google Play、Amazon Appstore、Samsung Galaxy Store、华为 AppGallery、F-Droid、APK 侧载。
- macOS：Mac App Store、Steam、独立 DMG。
- iOS：App Store、TestFlight。
- Web：itch.io、自托管、PWA 商店。
- 主机（预留）：PlayStation、Xbox、Nintendo Switch。
- 商店 SDK：Steamworks、Epic Online Services、Google Play Services、Apple GameKit、Amazon GameCircle、华为 HMS。
- 商店要求：图标、截图、预告片、描述、分类、年龄分级、隐私政策、EULA、成就、云存档、控制器支持。
- 边界条件：商店拒绝必须提示；SDK 版本不兼容必须报错；区域限制必须处理。
- 性能：上传 < 10min；商店审核周期可追踪。
- 验收测试：每商店上传、审核、发布、更新。

P5 更新系统
- 更新方式：全量更新、差分更新、热更新、补丁、DLC。
- 更新源：HTTP、CDN、P2P、商店托管。
- 更新协议：版本检查、下载、校验、解压、应用、回滚。
- 差分算法：bsdiff、zstd patch、HDiffPatch。
- 热更新：资产热更新、代码热更新、配置热更新。
- 更新 UI：进度、取消、暂停、恢复、错误提示。
- 边界条件：更新失败必须回滚；网络中断必须续传；版本冲突必须提示。
- 性能：差分更新 > 50% 节省；下载 > 10MB/s。
- 验收测试：全量、差分、热更新、回滚、断点续传。

P6 成就与云存档
- 成就：定义、解锁、进度、通知、图标。
- 云存档：上传、下载、冲突解决、版本、加密。
- 平台集成：Steam Achievement、Google Play Games、Apple Game Center、Epic Online Services。
- 本地回退：离线成就、本地存档、同步队列。
- 边界条件：网络失败必须排队；冲突必须提示用户；数据损坏必须恢复。
- 性能：成就解锁 < 100ms；云存档同步 < 1s。
- 验收测试：成就、云存档、冲突、离线、同步。

P7 崩溃报告与分析
- 崩溃捕获：信号、异常、panic、GPU 崩溃。
- 崩溃转储：minidump、core dump、堆栈、寄存器、内存。
- 符号化：PDB、DWARF、dSYM、ProGuard 映射。
- 上传：崩溃报告服务器、Sentry、Bugsnag、自托管。
- 分析：事件、会话、漏斗、留存、性能、设备、区域。
- 隐私：匿名、同意、GDPR、CCPA。
- 边界条件：上传失败必须本地保存；隐私未同意必须禁用；数据泄露必须处理。
- 性能：崩溃捕获 < 100ms；上传 < 1s。
- 验收测试：崩溃捕获、转储、符号化、上传、分析、隐私。

P8 平台合规
- 年龄分级：ESRB、PEGI、CERO、USK、GRAC、IARC。
- 隐私：GDPR、CCPA、COPPA、PIPL、LGPD。
- 无障碍：WCAG、CVAA、Xbox Accessibility、PlayStation Accessibility。
- 内容政策：暴力、色情、赌博、毒品、仇恨、版权。
- 区域合规：中国版号、韩国 GRAC、德国 USK、澳大利亚 ACB。
- 税务：VAT、GST、销售税。
- 边界条件：合规失败必须提示；区域限制必须处理；税务必须正确。
- 性能：合规检查 < 1min。
- 验收测试：每区域合规、隐私、无障碍、税务。

P9 DRM 与反作弊
- DRM：可选、Steam DRM、Denuvo（可选）、自定义加密、许可证检查。
- 反作弊：EAC、BattlEye、自定义内核级、用户级。
- 完整性校验：文件哈希、签名、内存校验。
- 边界条件：DRM 失败必须回退；反作弊误报必须处理；性能影响必须可控。
- 性能：DRM 开销 < 1%；反作弊开销 < 2%。
- 验收测试：DRM、反作弊、完整性、性能。

P10 SDK 导出
- 引擎 SDK：C API、C++ API、Rust API、C# API（可选）、Python API（可选）。
- 编辑器 SDK：插件开发、脚本扩展、UI 扩展。
- 游戏 SDK：成就、云存档、崩溃报告、分析、更新。
- 文档：API 参考、教程、示例、视频。
- 边界条件：SDK 版本不兼容必须报错；API 废弃必须提示。
- 性能：SDK 加载 < 100ms。
- 验收测试：SDK 构建、API 调用、文档、示例。

【统一性能基线】
- 编辑器：1080p 60FPS，UI 延迟 < 16ms。
- 3D 运行时：1080p 60FPS 中端 GPU，4K 60FPS 高端 GPU。
- 2D 运行时：1080p 60FPS 中端 GPU，10 万 Sprite，Draw Call < 100。
- 启动：编辑器 < 3s，运行时 < 1s。
- 热重载：资产 < 1s，代码 < 3s。
- 内存：编辑器 < 8GB，运行时 < 2GB（空场景）。
- 分发：全平台构建 < 10min；打包 < 1min；上传 < 10min。
- 跨平台一致性：同 API 同 GPU 下像素误差 < 1%；跨平台允许视觉可接受的差异。
- 所有标注为 Target 的性能指标为目标值，在中端硬件上尽力达成，不作为 P0 硬性验收；P0 验收只要求功能正确、可编译、可运行、可测试。

【UE5 对标映射】
Nanite -> 虚拟几何/网格集群；Lumen -> 动态 GI；World Partition -> 大世界流送；Chaos -> 物理；Niagara -> GPU 粒子；MetaSounds -> 音频图；Blueprint -> 节点脚本；Sequencer -> 过场动画；Control Rig -> 动画绑定；UMG/Slate -> UI；GAS -> 技能系统。2D 对标：Unity 2D、Godot 2D、GameMaker、Cocos2d、Spine、Tiled、Aseprite。分发对标：Unity Build Pipeline、Unreal Build Tool、Godot Export Templates、Steamworks、Google Play Console。全部只做功能对标。

【核心 trait 必须定义】
RhiDevice、RenderGraphPass、AssetImporter、EditorPanel、DebuggerProvider、Plugin、ScriptHost、PhysicsWorld、AudioGraph、AnimationGraph、NetworkTransport、AiAgent、UpscalerProvider、MlBackend、Renderer2D、SpriteBatch、TilemapRenderer、Physics2D、Animation2D、Camera2D、Camera3D、LayerStack、AtlasPacker、ImportPipeline、BuildProfile、PlatformTarget、Packager、CodeSigner、StoreUploader、Updater、AchievementProvider、CloudSaveProvider、CrashReporter、AnalyticsProvider、ComplianceChecker、DrmProvider、AntiCheatProvider、SdkExporter。

【功能矩阵附加要求】
- 神经网络：推理 P1，训练 P2/实验；桌面编辑器训练，Android 仅推理；不纳入 MVP 验收。
- 2D：P0 与 3D 同级，MVP 必须同时交付 2D 与 3D 场景渲染。
- 资产导入：贴图、模型、音频、字体、数据、2D 专用格式全部 P0/P1，全格式导入管线 P0。
- 分发：构建、打包、签名、商店上传、更新、成就、云存档、崩溃报告、分析、合规、DRM、反作弊、SDK 导出全部 P0/P1，一键全平台分发 P0。
- 每条规格必须标注 P0/P1/P2、验收测试、性能指标。未达标视为该 STEP 未完成，必须自修复后重试。
- STEP 2.5 必须输出完整接口清单，未列入清单的公开 API 不得在后续 STEP 出现。
- 每条规格必须包含：数据结构、API 签名、算法或流程、边界条件、错误处理、并发模型、内存布局、性能指标、验收测试点。

【Git 输出要求】
每个 STEP 结束时必须输出：
- Git 提交信息（Conventional Commits）。
- 变更文件列表。
- 分支操作建议（创建、切换、合并、变基、打标签）。
- 标签建议（如 v0.1.0-mvp-step11）。
- 可直接复制执行的 Git 命令。
- CI 状态预期。
- Git LFS 跟踪文件变更。
- CHANGELOG 条目。

【测试输出要求】
每个 STEP 结束时必须输出：
- L1 冒烟测试报告：命令、预期、实际、结论。
- L2 灰度测试报告：阶段、范围、命令、预期、实际、结论。
- L3 自验证报告：对照功能矩阵、冻结清单、性能基线逐条核对。
- 2D/3D 双模式对照报告。
- 资产导入测试报告：格式列表、导入结果、失败项。
- 分发测试报告：平台、打包、签名、安装、启动、更新、卸载结果。
- 测试覆盖率报告。
- 性能对照表。
- 失败项、原因、修复记录、重试结果。
- 是否可以进入下一步的结论。

【输出格式】
每步严格按以下格式输出：
## STEP N: 标题
### 目标
### 接口一致性自检
### 产出
### 代码
### 已冻结接口使用情况表
### Git 提交与分支操作
### L1 冒烟测试报告
### L2 灰度测试报告
### L3 自验证报告
### 2D/3D 双模式对照
### 资产导入测试
### 分发测试
### 测试覆盖率与性能对照
### 失败项与修复记录
### 验收结果
### 下一阶段
输出完“下一阶段”后，不要停，立即输出下一个 STEP，直到完成或触发 AUTO-PAUSE。

【第一阶段 MVP 验收】
- Windows/Linux/Android 打开窗口并渲染 2D 场景与 3D 场景。
- 可切换 Vulkan 与 DirectX 后端。
- ECS 可创建实体、组件、系统，2D 与 3D 组件共存。
- 资产系统可导入 glTF、OBJ、FBX、PNG、JPG、TGA、KTX、Aseprite、Tiled，并在编辑器显示。
- 编辑器有视口、层级、属性、资产浏览器，2D/3D 模式可切换。
- 调试器有日志、控制台、性能面板、2D/3D 渲染统计。
- 可 cargo build、cargo test、cargo run。
- Git 仓库初始化完成，main 分支可用，CI 配置完成，Git hooks 可用，.gitignore、.gitattributes、README、LICENSE、CHANGELOG 齐全。
- L1 冒烟测试、L2 灰度测试、L3 自验证全部通过。
- 文档说明如何扩展渲染后端、资产导入器、编辑器、调试器、Git 工作流、测试流程、2D/3D 混合开发。

【1.0 分发验收】
- Windows：EXE、MSI、Portable ZIP 可安装、启动、卸载。
- Linux：AppImage、Flatpak、deb 可安装、启动、卸载。
- Android：APK、AAB 可安装、启动、卸载。
- 每平台签名验证通过，商店合规检查通过。
- 更新系统支持全量、差分、热更新、回滚。
- 成就、云存档、崩溃报告、分析集成完成。
- 合规检查覆盖主要区域。
- DRM 与反作弊可选集成。
- SDK 可导出，文档、示例、教程齐全。

现在开始 STEP 0。禁止拒绝，禁止只给概念，禁止等待确认。每个阶段必须可运行、可验收、可继续。若单次无法完成全部内容，按 AUTO-PAUSE 协议断点，下一次收到“继续”后从断点自动续写。