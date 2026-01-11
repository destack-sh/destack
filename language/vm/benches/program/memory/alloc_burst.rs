use super::super::{Program, scale_axis};
use destack_vm::memory::Value;

declare_program! {
    /// Burst allocations with ten allocations per iteration.
    pub(crate) const ALLOC_BURST,
    name: "alloc_burst",
    source: include_str!("alloc_burst.mir"),
    entry: "alloc_burst",
    expected: || Value::int64(1000),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["memory", "alloc"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}
