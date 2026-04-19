use super::super::{Program, scale_axis};
use destack_vm::Value;

/// Default iteration count for the local accumulator.
const DEFAULT_ITERS: i64 = 1000;

/// Compute the expected accumulator output.
fn expected_local_accumulate(iterations: i64) -> i64 {
    // seed the local values
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

    // return the final accumulator
    current
}

declare_program! {
    /// Loop that updates locals through local.get/local.set and assume.
    pub const LOCAL_ACCUMULATE,
    name: "local_accumulate",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/local_accumulate.mir")),
    entry: "local_accumulate",
    expected: || Value::int64(expected_local_accumulate(DEFAULT_ITERS)),
    default_args: |_interp| vec![Value::int64(DEFAULT_ITERS)],
    tags: &["memory", "local"],
    scales: &[
        scale_axis("iter", 0, DEFAULT_ITERS, 10000, 100000, true),
    ],
}
