use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Predictable branch where condition is always true.
    pub const BRANCH_PREDICTABLE,
    name: "branch_predictable",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/branch_predictable.mir")),
    entry: "branch_predictable",
    expected: || Value::int64(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "branch"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}
