//! RustForge RHI：设备/纹理/缓冲/管线抽象 + Software/Null 后端
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-rhi"
}
