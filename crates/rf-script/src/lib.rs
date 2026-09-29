//! RustForge 脚本：字节码 VM/可视化图/GAS-lite/调试钩子
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-script"
}
