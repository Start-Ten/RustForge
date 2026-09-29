//! RustForge 编辑器：面板/撤销重做/命令面板/主题/2D-3D 模式
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-editor"
}
