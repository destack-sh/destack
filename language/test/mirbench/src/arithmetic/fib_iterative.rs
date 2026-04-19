use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Iterative fibonacci using SSA block parameters.
    pub const FIB_ITERATIVE,
    name: "fib_iterative",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/fib_iterative.mir")),
    entry: "fib_iter",
    expected: || Value::int64(55),
    default_args: |_interp| vec![Value::int64(10)],
    tags: &["arithmetic", "fib"],
    scales: &[
        scale_axis("iter", 0, 10, 100, 1000, true),
    ],
}
