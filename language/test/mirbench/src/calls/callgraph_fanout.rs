use super::super::common::mix_result;
use super::super::{Program, function_pointer_by_name, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Mixed direct and indirect calls with multiple leaves.
    pub const CALLGRAPH_FANOUT,
    name: "callgraph_fanout",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/callgraph_fanout.mir")),
    entry: "callgraph_fanout",
    expected: || callgraph_fanout(10_000, 7),
    default_args: |interp| {
        // resolve function pointer argument
        let helper = function_pointer_by_name(interp, "fanout_helper");

        vec![Value::int64(10_000), Value::int64(7), helper]
    },
    tags: &["calls", "call", "fanout"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Apply the add leaf.
fn fanout_add(index: i64, acc: i64) -> i64 {
    // compute leaf output
    let added = index.wrapping_add(acc);

    // return value
    added.wrapping_add(1)
}

/// Apply the multiply leaf.
fn fanout_mul(index: i64, acc: i64) -> i64 {
    // compute leaf output
    let scaled = index.wrapping_mul(3);

    // return value
    scaled.wrapping_add(acc)
}

/// Apply the mix leaf.
fn fanout_mix(index: i64, acc: i64) -> i64 {
    // compute leaf output
    let mixed = index ^ acc;

    // return value
    mixed.wrapping_add(7)
}

/// Apply the helper call.
fn fanout_helper(acc: i64, index: i64) -> i64 {
    // compute helper output
    acc.wrapping_add(index)
}

/// Compute the expected value for the callgraph fanout benchmark.
fn callgraph_fanout(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = seed;

    // run dispatch loop
    while index < iterations {
        // select a leaf
        let slot = index.wrapping_rem(3);
        acc = match slot {
            0 => fanout_add(index, acc),
            1 => fanout_mul(index, acc),
            _ => fanout_mix(index, acc),
        };

        // apply helper call
        acc = fanout_helper(acc, index);

        // advance cursor
        index = index.wrapping_add(1);
    }

    // return value
    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
