use super::super::{Program, scale_axis};
use destack_heap::Value;

/// Default iteration count for raw drops.
const DEFAULT_ITERS: i64 = 1000;

/// Compute the expected raw drop output.
fn expected_raw_alloc_drop(iterations: i64) -> i64 {
    // seed the accumulator
    let mut total = 0i64;

    // walk the iteration count
    let mut index = 0i64;
    while index < iterations {
        total = total.wrapping_add(index);
        index += 1;
    }

    // return the accumulator
    total
}

declare_program! {
    /// Raw allocations that end with drop.
    pub const RAW_ALLOC_DROP,
    name: "raw_alloc_drop",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/raw_alloc_drop.mir")),
    entry: "raw_alloc_drop",
    expected: || Value::int64(expected_raw_alloc_drop(DEFAULT_ITERS)),
    default_args: |_interp| vec![Value::int64(DEFAULT_ITERS)],
    tags: &["memory", "alloc", "drop"],
    scales: &[
        scale_axis("iter", 0, DEFAULT_ITERS, 10000, 100000, true),
    ],
}
