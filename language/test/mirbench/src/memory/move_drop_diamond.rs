use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Alternating managed allocations with per-branch updates.
    pub const MOVE_DROP_DIAMOND,
    name: "move_drop_diamond",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/move_drop_diamond.mir")),
    entry: "move_drop_diamond",
    expected: || move_drop_diamond(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["memory", "move", "drop"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the move drop diamond benchmark.
fn move_drop_diamond(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // run loop
    while index < iterations {
        // simulate branch selection
        let value = if (index & 1) == 0 {
            Box::new(index)
        } else {
            Box::new(index.wrapping_add(1))
        };
        acc = acc.wrapping_add(*value);

        // advance cursor
        index = index.wrapping_add(1);
    }

    // return result
    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
