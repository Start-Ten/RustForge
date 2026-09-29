//! RustForge 平台抽象：窗口/输入/文件/时间（Headless+Win32）
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-platform"
}
