use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Cast mix covering integer truncation and float conversions.
    pub const CAST_MIX,
    name: "cast_mix",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/cast_mix.mir")),
    entry: "cast_mix",
    expected: || Value::int64(10_001),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["arithmetic", "cast"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}
