use super::super::{Program, scale_axis_range};
use destack_vm::Value;

declare_program! {
    /// Tail recursive countdown (not optimized).
    pub const CALL_RECURSIVE_TAIL,
    name: "call_recursive_tail",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_recursive_tail.mir")),
    entry: "call_recursive_tail",
    expected: || Value::int64(1000),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["calls", "call", "tail"],
    scales: &[
        scale_axis_range("iter", 0, 500, 1000, 1000, true, 1, 1000),
    ],
}
