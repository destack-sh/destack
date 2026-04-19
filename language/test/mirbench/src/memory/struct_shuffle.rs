use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Shuffle struct fields with address-based loads and stores.
    pub const STRUCT_SHUFFLE,
    name: "struct_shuffle",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/struct_shuffle.mir")),
    entry: "struct_shuffle",
    expected: || struct_shuffle(10),
    default_args: |_interp| vec![Value::int64(10)],
    tags: &["memory", "struct", "field", "alias"],
    scales: &[
        scale_axis("iter", 0, 10, 100, 1000, true),
    ],
}

/// Pair of values for struct shuffle updates.
struct Pair {
    left: i64,
    right: i64,
}

/// Compute the expected value for the struct shuffle benchmark.
fn struct_shuffle(iterations: i64) -> Value {
    // init state
    let mut pair = Pair { left: 1, right: 2 };
    let mut index = 0i64;

    // run loop
    while index < iterations {
        let left = pair.left;
        let right = pair.right;
        let updated = left.wrapping_add(right).wrapping_add(index);
        pair.left = updated;
        index = index.wrapping_add(1);
    }

    // return result
    let total = pair.left.wrapping_add(pair.right);
    let mixed = mix_result(total, iterations);
    Value::int64(mixed)
}
