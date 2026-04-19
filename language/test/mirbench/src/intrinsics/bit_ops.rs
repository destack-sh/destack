use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Bit manipulation intrinsics including leadingZeroCount, trailingZeroCount, and populationCount.
    pub const BIT_OPS,
    name: "bit_ops",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/bit_ops.mir")),
    entry: "bit_ops",
    expected: || bit_ops(10_000, 773738404492881801),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["intrinsics", "bit"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the bit ops benchmark.
fn bit_ops(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut value = seed;

    // run bit ops loop
    while index < iterations {
        // evaluate bit intrinsics
        let _clz = (value as u64).leading_zeros();
        let _ctz = (value as u64).trailing_zeros();
        let _pop = (value as u64).count_ones();

        // update state
        let shifted = value.wrapping_shl(1);
        value ^= shifted;
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(value)
}
