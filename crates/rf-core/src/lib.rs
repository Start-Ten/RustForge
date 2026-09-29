//! RustForge 内核：Entity/Tick/Handle/错误/版本/Rgba8Image/哈希/随机
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-core"
}
