//! RustForge 2D/3D 数学库：向量/矩阵/四元子/几何/插值/曲线/噪声
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-math"
}
