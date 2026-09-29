# STEP 12-14 — Alpha 验收报告（v0.2.0）

提交链：...→ 65a76eb(MVP) → a25b687(STEP12) → b66b06a(STEP13-14) · 标签：v0.1.0-mvp, v0.2.0-alpha

## L1 冒烟（T1-T3/T7）

| 项 | 命令 | 结果 |
|---|---|---|
| T1 编译 | cargo check --workspace --all-targets | ✅ 0 错误 |
| T2 格式 | cargo fmt --all --check | ✅ |
| T3 静态 | cargo clippy --workspace --all-targets -- -D warnings | ✅ 0 错误 |
| T7 示例 | minimal_2d --frames 3 | ✅ 退出 0 |

## L2 灰度（T4/T5/T8）

| 范围 | 结果 |
|---|---|
| cargo test --workspace | **229 passed / 0 failed** ✅ |
| 逐 crate 子集 | 全绿（见 git log 中 pre-commit 输出） |
| 跨后端 | Software/Null/降级链 ✅（rf-rhi） |
| 2D/3D 双模式 | acceptance + mixed_23d ✅ |

## L3 自验证（STEP 12-14 功能矩阵对照）

| 模块 | 验收点 | 状态 | 证据 |
|---|---|---|---|
| C1/C2/L2 物理 | 3D 冲量+休眠+确定性回放；2D SAT+多边形+关节+单向平台+角色控制器 | ✅ | rf-physics 10 tests |
| D1-D3 动画 | 骨骼/蒙皮矩阵/2D 骨骼/帧动画事件/状态机淡入/Two-Bone+FABRIK IK/时间轴 | ✅ | rf-animation 9 tests |
| F1/F2 网络 | 回环对（延迟/丢包）/UDP/复制增量+预算/RPC/预测回滚 | ✅ | rf-network 6 tests |
| G1 AI | 行为树组合节点/黑板观察者/A* 禁切角/流场导航/感知视锥记忆/boids | ✅ | rf-ai 7 tests |
| H1/L5 UI | 控件树确定性布局/命中/显示列表/9-Slice/绑定/虚拟化窗口 | ✅ | rf-ui 7 tests |
| I1/I2 脚本 | 字节码 VM（表达式/控制流/原生调用）+ 图编译 + GAS-lite + 断点调试 | ✅ | rf-script 8 tests |
| I2 ML | 张量运算/MLP XOR 训练收敛/存取往返/确定性 | ✅ | rf-ml 5 tests |
| P1 构建 | BuildProfile/PlatformTarget×10/BuildPlan 命令行生成 | ✅ | rf-distribute |
| P2 打包 | ZIP（自实现中央目录）/PAK/InnoSetup/Nsis/AppImage/deb/Gradle 脚本 | ✅ | |
| P3 签名 | 自实现 SHA-256（NIST 向量）+CRC32（IEEE 向量）+HMAC 签名/验签/防篡改 + 原生命令生成 | ✅ | |
| P4 商店 | Steam/Play/itch/GOG 清单生成 + dry-run 报告 | ✅ | |
| P5 更新 | 清单往返/块差分（省带宽验证）/应用+哈希失败回滚 | ✅ | |
| P6 成就/云存档 | 解锁/持久化/通知；槽位上传下载/冲突策略 | ✅ | |
| P7 崩溃/分析 | GDPR 同意门控/本地持久化；JSONL 批处理 | ✅ | |
| P8 合规 | GDPR/CCPA/年龄分级/商店渠道/中国版号提示 | ✅ | |
| P9 DRM/反作弊 | NoDrm/SteamDrm 命令；完整性哈希扫描（缺失/篡改） | ✅ | |
| P10 SDK | C 头文件导出 + FFI 三函数（版本/初始化/sha256） | ✅ | |

## 接口一致性

冻结清单 IF-001~IF-334 + CP-001~003 变更提案（§26.5）。无未记录公开 API。

## 2D/3D 双模式对照

物理（3D 场景 vs 2D 场景）、动画（Skeleton vs Skeleton2）、渲染（render_mesh_cpu vs SoftwareRenderer2D）
各自独立实现、共享 math/ECS/调试框架 —— 满足规格「2D 非 3D 降级」要求。

## 失败项与修复记录（STEP 12-14 期间）

FABRIK 根锚定缺失、赋值语句编译次序、i32→u32 cores、LoopbackTransport 半双工接线、
RPC 类型复杂度重构、SHA-256/HMAC 实现修正。全部已修复并回归（229 绿）。

## 结论

**Alpha（STEP 12-14）验收通过**。全部 14 个 STEP 完成。
P2/P1 后续项已在功能矩阵标注（GPU 后端 feature `gpu`、动态库插件、JPEG 解码等）。
