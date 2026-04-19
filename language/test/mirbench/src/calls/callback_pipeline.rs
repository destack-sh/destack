use super::super::common::mix_result;
use super::super::{Program, function_pointer_by_name, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Call pipeline with an indirect callback stage.
    pub const CALLBACK_PIPELINE,
    name: "callback_pipeline",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/callback_pipeline.mir")),
    entry: "callback_pipeline",
    expected: || callback_pipeline(10_000),
    default_args: |interp| {
        // resolve callback argument
        let callback = function_pointer_by_name(interp, "stage_mix");

        vec![Value::int64(10_000), callback]
    },
    tags: &["calls", "callback", "indirect"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Apply the add stage.
fn stage_add(value: i64) -> i64 {
    // apply add stage
    value.wrapping_add(2)
}

/// Apply the multiply stage.
fn stage_mul(value: i64) -> i64 {
    // apply multiply stage
    value.wrapping_mul(3).wrapping_add(1)
}

/// Apply the mix stage.
fn stage_mix(value: i64) -> i64 {
    // apply mix stage
    (value ^ 7).wrapping_add(5)
}

/// Compute the expected value for the callback pipeline benchmark.
fn callback_pipeline(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // run loop
    while index < iterations {
        // apply pipeline stages
        let stage0 = stage_add(index);
        let stage1 = stage_mix(stage0);
        let stage2 = stage_mul(stage1);
        acc = acc.wrapping_add(stage2);

        // advance cursor
        index = index.wrapping_add(1);
    }

    // return value
    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
