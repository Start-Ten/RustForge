# 测试协议（L1/L2/L3 与 T1–T13）

## 三层测试

| 层 | 目的 | 范围 | 门槛 |
|---|---|---|---|
| L1 冒烟 | 最小可运行验证 ≤5min | T1–T7 | 全绿才进 L2 |
| L2 灰度 | 渐进扩大 | T8：单 crate → workspace → 跨后端(Software/Null) → 2D/3D 双模式 → 分发产物 | 失败回滚 |
| L3 自验证 | 对照清单核对 | T12：冻结清单 + 功能矩阵 + 性能基线 | 输出报告 |

## T1–T13 命令表

| # | 项 | 命令 |
|---|---|---|
| T1 | 编译验证 | `cargo check --workspace --all-targets` |
| T2 | 格式验证 | `cargo fmt --all --check` |
| T3 | 静态检查 | `cargo clippy --workspace --all-targets -- -D warnings` |
| T4 | 单元测试 | `cargo test --workspace --lib` |
| T5 | 集成测试 | `cargo test --workspace --test`（集成测试置于各 crate `tests/`） |
| T6 | 文档测试 | `cargo test --workspace --doc` |
| T7 | 冒烟示例 | `cargo run -p rustforge --example minimal_2d -- --frames 3`（CI/无窗口模式） |
| T8 | 灰度 | 按 crate 子集 `cargo test -p rf-ecs` 逐级扩大；跨后端用例见 rf-rhi tests |
| T9 | 基准 | `cargo run -p rustforge --release --example bench_ecs`（输出对照表） |
| T10 | 内存 | leak 检测单测（rf-memory tests）+ 建议 `cargo +nightly miri -p rf-ecs` |
| T11 | 跨平台 | CI matrix（ubuntu/windows）+ aarch64-linux-gnu check + android check |
| T12 | 自验证 | `cargo test -p rustforge --test acceptance`（MVP 验收清单逐条断言） |
| T13 | 分发 | `cargo test -p rf-distribute`（打包-解包-签名-更新-回滚全流程） |

## 灰度发布策略

1. `develop`：冒烟（T1–T7）。
2. `feature/*`：crate 集成（T8 子集）。
3. 合并 `develop`：全量（T1–T9）。
4. `release/*`：跨平台 + 分发（T11、T13）。
5. `main`：打标签（vX.Y.Z），失败一律回滚，不携带失败前进。

## 报告模板（docs/reports/stepN.md）

- 测试项 / 命令 / 预期 / 实际 / 结论
- 覆盖率（按 crate tests 数与断言数估算）
- 性能对照（实测 vs Target）
- 接口一致性（本 STEP 引用 vs 冻结清单）
- 2D/3D 双模式对照、资产导入结果、分发产物结果
- 失败项与修复记录
- 是否可进入下一步：结论
