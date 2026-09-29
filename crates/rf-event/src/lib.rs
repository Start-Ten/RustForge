//! RustForge 类型安全事件总线：优先级/一次性/延迟/限流/输入映射
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-event"
}
