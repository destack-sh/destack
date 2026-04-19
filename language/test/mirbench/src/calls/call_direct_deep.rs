use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Twenty deep call chain measuring deep call stacks.
    pub const CALL_DIRECT_DEEP,
    name: "call_direct_deep",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_direct_deep.mir")),
    entry: "call_direct_deep",
    expected: || Value::int64(1000),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["calls", "call"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}
