//! RustForge 物理：3D 刚体冲量求解 + 2D SAT/关节/角色控制器
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-physics"
}
