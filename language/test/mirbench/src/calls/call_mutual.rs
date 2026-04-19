use super::super::{Program, scale_axis_range};
use destack_vm::Value;

declare_program! {
    /// Mutually recursive even and odd functions.
    pub const CALL_MUTUAL,
    name: "call_mutual",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_mutual.mir")),
    entry: "call_mutual",
    expected: || Value::int64(500),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["calls", "call", "mutual"],
    scales: &[
        scale_axis_range("iter", 0, 500, 1000, 1000, true, 1, 1000),
    ],
}
