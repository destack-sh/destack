use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Alternating branch where condition flips each iteration.
    pub const BRANCH_ALTERNATING,
    name: "branch_alternating",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/branch_alternating.mir")),
    entry: "branch_alternating",
    expected: || Value::int64(5000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "branch"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}
