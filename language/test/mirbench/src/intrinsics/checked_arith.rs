use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Checked arithmetic intrinsics with overflow detection.
    pub const CHECKED_ARITH,
    name: "checked_arith",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/checked_arith.mir")),
    entry: "checked_arith",
    expected: || checked_arith(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["intrinsics"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the checked arithmetic benchmark.
fn checked_arith(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut value = 1i64;
    let mut overflow_count = 0i64;

    // run checked loop
    while index < iterations {
        // apply overflowing add
        let (sum, overflow) = value.overflowing_add(1);
        let delta = if overflow { 1i64 } else { 0i64 };
        overflow_count = overflow_count.wrapping_add(delta);

        // advance state
        value = sum.wrapping_rem(127);
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(overflow_count)
}
