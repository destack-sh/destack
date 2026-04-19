use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Five deep call chain measuring call overhead.
    pub const CALL_DIRECT_SHALLOW,
    name: "call_direct_shallow",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_direct_shallow.mir")),
    entry: "call_direct_shallow",
    expected: || Value::int64(1000),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["calls", "call"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}
