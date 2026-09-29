//! RustForge 分发：构建/打包/签名/商店/更新/成就/云存档/合规/SDK
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-distribute"
}
