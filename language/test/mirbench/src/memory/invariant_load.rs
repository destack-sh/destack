use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Sum a loop-invariant stack load.
    pub const INVARIANT_LOAD,
    name: "invariant_load",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/invariant_load.mir")),
    entry: "invariant_load",
    expected: || invariant_load(10),
    default_args: |_interp| vec![Value::int64(10)],
    tags: &["memory", "loop", "invariant"],
    scales: &[
        scale_axis("iter", 0, 10, 100, 1000, true),
    ],
}

/// Compute the expected value for the invariant load benchmark.
fn invariant_load(iterations: i64) -> Value {
    // init state
    let value = 7i64;
    let mut index = 0i64;
    let mut total = 0i64;

    // run loop
    while index < iterations {
        total = total.wrapping_add(value);
        index = index.wrapping_add(1);
    }

    // return result
    let mixed = mix_result(total, iterations);
    Value::int64(mixed)
}
