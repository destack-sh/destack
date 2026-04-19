use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Collatz sequence with unpredictable branching and varied operations.
    pub const COLLATZ,
    name: "collatz",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/collatz.mir")),
    entry: "collatz_sum",
    expected: || Value::int64(3142),
    default_args: |_interp| vec![Value::int64(100)],
    tags: &["arithmetic", "collatz"],
    scales: &[
        scale_axis("iter", 0, 100, 1000, 10000, true),
    ],
}
