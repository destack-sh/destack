use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Burst allocations with ten allocations per iteration.
    pub const ALLOC_BURST,
    name: "alloc_burst",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/alloc_burst.mir")),
    entry: "alloc_burst",
    expected: || Value::int64(1000),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["memory", "alloc"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}
