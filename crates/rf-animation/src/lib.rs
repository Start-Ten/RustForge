//! RustForge 动画：骨骼/蒙皮/状态机/IK/时间轴(Sequencer)
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-animation"
}
