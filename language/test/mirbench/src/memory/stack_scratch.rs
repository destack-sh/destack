use super::super::{Program, scale_axis};
use destack_heap::Value;

/// Default iteration count for stack scratch space.
const DEFAULT_ITERS: i64 = 1000;

/// Compute the expected stack scratch output.
fn expected_stack_scratch(iterations: i64) -> i64 {
    // seed the stack values
    let mut previous = 0i64;
    let mut current = 1i64;

    // walk the iteration count
    let mut index = 0i64;
    while index < iterations {
        let next = previous.wrapping_add(current);
        previous = current;
        current = next;
        index += 1;
    }

    // return the final scratch value
    current
}

declare_program! {
    /// Stack allocations with drop cleanup.
    pub const STACK_SCRATCH,
    name: "stack_scratch",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/stack_scratch.mir")),
    entry: "stack_scratch",
    expected: || Value::int64(expected_stack_scratch(DEFAULT_ITERS)),
    default_args: |_interp| vec![Value::int64(DEFAULT_ITERS)],
    tags: &["memory", "stack"],
    scales: &[
        scale_axis("iter", 0, DEFAULT_ITERS, 10000, 100000, true),
    ],
}
