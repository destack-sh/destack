use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Eight argument function call measuring argument passing.
    pub const CALL_MANY_ARGS,
    name: "call_many_args",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_many_args.mir")),
    entry: "call_many_args",
    expected: || Value::int64(36000),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["calls", "call"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}
