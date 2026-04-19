use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Stack and heap alias patterns across address spaces.
    pub const ALIAS_ESCAPE,
    name: "alias_escape",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/alias_escape.mir")),
    entry: "alias_escape",
    expected: || alias_escape(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["memory", "alias", "addrspace"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Pair of values for alias escape tracking.
struct Pair {
    left: i64,
    right: i64,
}

/// Compute the expected value for the alias escape benchmark.
fn alias_escape(iterations: i64) -> Value {
    // init state
    let mut stack_pair = Pair { left: 1, right: 2 };
    let mut heap_pair = Box::new(Pair { left: 2, right: 1 });
    let mut index = 0i64;
    let mut acc = 0i64;

    // run loop
    while index < iterations {
        // simulate loop body
        let combined = stack_pair.left.wrapping_add(heap_pair.right);
        stack_pair.right = combined.wrapping_add(index);
        heap_pair.left = combined;
        acc = acc.wrapping_add(combined);

        // advance cursor
        index = index.wrapping_add(1);
    }

    // return result
    let result = stack_pair
        .left
        .wrapping_add(stack_pair.right)
        .wrapping_add(heap_pair.left)
        .wrapping_add(acc);
    let mixed = mix_result(result, iterations);
    Value::int64(mixed)
}
