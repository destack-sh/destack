use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Xorshift pseudo-random mix with accumulation.
    pub const XORSHIFT_SUM,
    name: "xorshift_sum",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/xorshift_sum.mir")),
    entry: "xorshift_sum",
    expected: || xorshift_sum(100_000, 88172645463325252),
    default_args: |_interp| vec![Value::int64(100_000), Value::int64(88172645463325252)],
    tags: &["arithmetic", "xorshift"],
    scales: &[
        scale_axis("iter", 0, 100000, 1000000, 10000000, true),
    ],
}

/// Compute the expected value for the xorshift sum benchmark.
fn xorshift_sum(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut state = seed;
    let mut sum = 0i64;

    // run xorshift loop
    while index < iterations {
        // apply xorshift mix
        let shift_left = state.wrapping_shl(13);
        let mut value = state ^ shift_left;
        let shift_right = value.wrapping_shr(7);
        value ^= shift_right;
        let shift_left = value.wrapping_shl(17);
        value ^= shift_left;

        // update accumulators
        sum = sum.wrapping_add(value);
        state = value;
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(sum)
}
