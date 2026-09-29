# 扩展指南（渲染后端 / 资产导入器 / 编辑器 / 调试器）

## 新增 RHI 后端
1. 在 `rf-rhi/src/` 新建 `mybackend.rs`，实现 `RhiDevice` + `CommandList`（含 `as_any`）。
2. `Backend` 枚举加变体（可扩展项，见冻结清单 IF-140）。
3. `create_device` 匹配分支接构造；建议 feature 门控（如 `gpu`）。
4. 通过 `rf-rhi/tests/software.rs` 同型测试（clear/blit/三角/深度/纹理往返）。

## 新增资产导入器
1. 实现 `rf-asset::AssetImporter`（name/extensions/import）。
2. 产出 `ImportedAsset { kind_name, data: Box<dyn AssetAny>, .. }`。
3. `pipeline.register_importer(Box::new(MyImporter))`；或加入 `builtin_importers()`。
4. 依赖经 `ctx.add_dep()` 声明（依赖图/热重载自动生效）。

## 新增编辑器面板
1. 实现 `rf-editor::EditorPanel`（kind/title/update）。
2. `EditorApp::new` 面板列表追加，或运行时经模型组合。
3. 面板输出写入 `ctx.log`（`HIERARCHY/PROPS` 前缀），UI 层解析渲染。

## 新增调试器数据源
1. 计数器：`Profiler::counter_add/set`（2D 专用计数：sprites/batches/tile 数）。
2. 远程：实现 `DebugTransport`（如 TCP/adb），接入 `RemoteDebugServer`。
3. 渲染统计：填 `RenderStats`（IF-305）由 Profiler 面板展示。

## 2D/3D 混合开发
- 同一 World 混合组件；渲染经 `LayerStack::add(name, kind, order)` 交叉编排。
- 参考示例 `crates/rustforge/examples/mixed_23d.rs`。
