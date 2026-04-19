use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Arena style copy across managed arrays.
    pub const ARENA_COPY,
    name: "arena_copy",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/arena_copy.mir")),
    entry: "arena_copy",
    expected: || arena_copy(1000, 2),
    default_args: |_interp| vec![Value::int64(1000), Value::int64(2)],
    tags: &["memory", "arena"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}

/// Compute the expected value for the arena copy benchmark.
fn arena_copy(count: i64, multiplier: i64) -> Value {
    // compute array length
    let length = count.wrapping_mul(clamp_min(multiplier, 1));

    // compute sum over source values
    let mut sum = 0i64;
    for index in 0..length {
        let value = index.wrapping_mul(3).wrapping_add(7);
        sum = sum.wrapping_add(value);
    }

    // return value
    Value::int64(sum)
}
