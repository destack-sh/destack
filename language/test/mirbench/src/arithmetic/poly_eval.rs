use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Evaluate a cubic polynomial with Horner's method.
    pub const POLY_EVAL,
    name: "poly_eval",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/poly_eval.mir")),
    entry: "poly_eval",
    expected: || poly_eval(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["arithmetic", "poly"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the polynomial evaluation benchmark.
fn poly_eval(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // run loop
    while index < iterations {
        // evaluate polynomial
        let t1 = index.wrapping_mul(3).wrapping_add(5);
        let t2 = t1.wrapping_mul(index).wrapping_add(7);
        let t3 = t2.wrapping_mul(index).wrapping_add(11);
        acc = acc.wrapping_add(t3);

        // advance cursor
        index = index.wrapping_add(1);
    }

    // return value
    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
