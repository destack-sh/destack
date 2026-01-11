use super::super::{Program, scale_axis};
use destack_vm::memory::Value;

declare_program! {
    /// Recursive fibonacci with exponential call tree.
    pub(crate) const FIB_RECURSIVE,
    name: "fib_recursive",
    source: include_str!("fib_recursive.mir"),
    entry: "fib",
    expected: || Value::int64(55),
    default_args: |_interp| vec![Value::int64(10)],
    tags: &["arithmetic", "fib"],
    scales: &[
        scale_axis("iter", 0, 10, 20, 30, true),
    ],
}
