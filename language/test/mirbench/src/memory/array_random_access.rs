use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Random access updates over a managed array.
    pub const ARRAY_RANDOM_ACCESS,
    name: "array_random_access",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/array_random_access.mir")),
    entry: "array_random_access",
    expected: || array_random_access(10_000, 2048),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(2048)],
    tags: &["memory", "array"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
        scale_axis("len", 1, 2048, 8192, 32768, false),
    ],
}

/// Compute the expected value for the random access array benchmark.
fn array_random_access(iterations: i64, length: i64) -> Value {
    // init state
    let len = clamp_min(length, 1);
    let mut data = vec![0i64; len as usize];
    let mut index = 0i64;

    // initialize array contents
    while index < len {
        let slot = index as usize;
        data[slot] = index;
        index = index.wrapping_add(1);
    }

    // run random access loop
    let mut index = 0i64;
    let mut acc = 0i64;
    while index < iterations {
        // compute random slot
        let mixed = index.wrapping_mul(1103515245).wrapping_add(12345);
        let slot = mixed.wrapping_rem(len) as usize;

        // update slot and accumulator
        let value = data[slot] ^ mixed;
        data[slot] = value;
        acc = acc.wrapping_add(value);

        // advance counter
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(acc)
}
