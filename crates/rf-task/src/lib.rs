//! RustForge 任务系统：工作窃取线程池/优先级/依赖 DAG/取消
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-task"
}
