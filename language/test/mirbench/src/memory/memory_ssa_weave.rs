use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Interleaved loads and stores across branches for memory ssa.
    pub const MEMORY_SSA_WEAVE,
    name: "memory_ssa_weave",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/memory_ssa_weave.mir")),
    entry: "memory_ssa_weave",
    expected: || memory_ssa_weave(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["memory", "ssa", "store"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the memory ssa weave benchmark.
fn memory_ssa_weave(iterations: i64) -> Value {
    // init state
    let mut slot = Box::new(0i64);
    let mut index = 0i64;
    let mut acc = 0i64;

    // run loop
    while index < iterations {
        // simulate branch mutation
        if (index & 1) == 0 {
            let value = *slot;
            *slot = value.wrapping_add(index);
        } else {
            let value = *slot;
            *slot = value.wrapping_add(3);
        }
        acc = acc.wrapping_add(*slot);

        // advance cursor
        index = index.wrapping_add(1);
    }

    // return result
    let mixed = mix_result(acc.wrapping_add(*slot), iterations);
    Value::int64(mixed)
}
