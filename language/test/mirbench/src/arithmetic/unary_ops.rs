use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Unary operations: negation and bitwise not.
    pub const UNARY_OPS,
    name: "unary_ops",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/unary_ops.mir")),
    entry: "unary_ops",
    expected: || unary_ops(10_000, 12345),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["arithmetic", "unary"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the unary ops benchmark.
fn unary_ops(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut value = seed;

    // run unary loop
    while index < iterations {
        // apply unary sequence
        let value1 = !value.wrapping_neg();
        let value2 = !value1.wrapping_neg();
        let value3 = !value2.wrapping_neg();
        let value4 = !value3.wrapping_neg();

        // advance state
        value = value4;
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(value)
}
