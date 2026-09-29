//! RustForge 序列化：JSON/二进制/版本迁移/世界快照/增量
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-serialization"
}
