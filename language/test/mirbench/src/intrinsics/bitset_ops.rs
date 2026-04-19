use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Bitset like updates using populationCount and rotate intrinsics.
    pub const BITSET_OPS,
    name: "bitset_ops",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/bitset_ops.mir")),
    entry: "bitset_ops",
    expected: || bitset_ops(10_000, 424242),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(424242)],
    tags: &["intrinsics", "bitset"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the bitset ops benchmark.
fn bitset_ops(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut state = seed;
    let mut mirror = seed ^ 2654435769;
    let mut acc = 0i64;

    // run bitset loop
    while index < iterations {
        // apply bitset operations
        let shift = (index & 63) as u32;
        let mask = 1i64.wrapping_shl(shift);
        let toggled = state ^ mask;
        let count = (toggled as u64).count_ones() as i64;
        let rotated = (toggled as u64).rotate_right(count as u32) as i64;
        let mirror_toggled = mirror ^ rotated;
        let mirror_count = (mirror_toggled as u64).count_ones() as i64;
        let mirror_rotated = (mirror_toggled as u64).rotate_left(mirror_count as u32) as i64;

        // update accumulators
        acc = acc.wrapping_add(count).wrapping_add(mirror_count);
        state = rotated;
        mirror = mirror_rotated;
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(state ^ mirror ^ acc)
}
