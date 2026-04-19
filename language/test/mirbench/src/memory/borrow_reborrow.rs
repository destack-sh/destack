use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Borrowed references passed through a call in a tight loop.
    pub const BORROW_REBORROW,
    name: "borrow_reborrow",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/borrow_reborrow.mir")),
    entry: "borrow_reborrow",
    expected: || borrow_reborrow(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["memory", "borrow", "lifetime"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Return the borrowed reference unchanged.
fn borrow_passthrough(value: &i64) -> &i64 {
    // forward borrow
    value
}

/// Compute the expected value for the borrow reborrow benchmark.
fn borrow_reborrow(iterations: i64) -> Value {
    // init state
    let pair = Box::new((iterations, 0i64));
    let mut index = 0i64;
    let mut acc = 0i64;

    // run loop
    while index < iterations {
        // simulate loop body
        let slot = borrow_passthrough(&pair.0);
        acc = acc.wrapping_add(*slot).wrapping_add(index);

        // advance cursor
        index = index.wrapping_add(1);
    }

    // return result
    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
