//! RustForge 渲染：渲染图/相机/2D 批次/3D 光栅/后处理/上采样
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-render"
}
