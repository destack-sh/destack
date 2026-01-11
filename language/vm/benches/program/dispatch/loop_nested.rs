use super::super::{Program, scale_axis};
use destack_vm::memory::Value;

declare_program! {
    /// Nested loop with outer times inner iterations.
    pub(crate) const LOOP_NESTED,
    name: "loop_nested",
    source: include_str!("loop_nested.mir"),
    entry: "loop_nested",
    expected: || Value::int64(10000),
    default_args: |_interp| vec![Value::int64(100)],
    tags: &["dispatch", "loop"],
    scales: &[
        scale_axis("iter", 0, 100, 1000, 10000, true),
    ],
}
