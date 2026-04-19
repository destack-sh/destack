use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Rolling sum and sum of squares.
    pub const ROLLING_STATS,
    name: "rolling_stats",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/rolling_stats.mir")),
    entry: "rolling_stats",
    expected: || rolling_stats(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["arithmetic", "stats"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the rolling stats benchmark.
fn rolling_stats(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut sum = 0i64;
    let mut sumsq = 0i64;

    // run loop
    while index < iterations {
        sum = sum.wrapping_add(index);
        sumsq = sumsq.wrapping_add(index.wrapping_mul(index));
        index = index.wrapping_add(1);
    }

    let mixed = mix_result(sum.wrapping_add(sumsq), iterations);
    Value::int64(mixed)
}
