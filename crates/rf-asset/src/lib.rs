//! RustForge 资产：导入器/异步/LRU/依赖图/热重载/PAK/图集
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-asset"
}
