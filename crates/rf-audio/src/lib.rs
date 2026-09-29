//! RustForge 音频：DSP 节点图/混音/空间化/WAV 编码
//!
//! 接口契约见 docs/step2.5-interface-freeze.md。公开 API 与冻结清单严格一致。
#![forbid(unsafe_op_in_unsafe_fn)]

pub fn crate_id() -> &'static str {
    "rf-audio"
}
