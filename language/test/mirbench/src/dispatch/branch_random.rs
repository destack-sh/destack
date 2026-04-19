use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Random branch behavior using an LCG selector.
    pub const BRANCH_RANDOM,
    name: "branch_random",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/branch_random.mir")),
    entry: "branch_random",
    expected: || branch_random(10_000, 12345),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(12345)],
    tags: &["dispatch", "branch"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the random branch benchmark.
fn branch_random(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // run branch loop
    while index < iterations {
        // compute selector
        let mixed = index.wrapping_mul(1103515245).wrapping_add(seed);
        let byte = mixed & 255;

        // update accumulator
        if byte < 128 {
            acc = acc.wrapping_add(index);
        } else {
            acc ^= mixed;
        }

        // advance counter
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(acc)
}
