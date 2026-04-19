use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Sixteen argument function call stressing argument passing and copying.
    pub const CALL_MANY_ARGS_16,
    name: "call_many_args_16",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_many_args_16.mir")),
    entry: "call_many_args_16",
    expected: || Value::int64(136_000),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["calls", "call"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}
