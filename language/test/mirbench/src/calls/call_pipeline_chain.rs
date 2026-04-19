use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Pipeline of arithmetic stages with chained calls.
    pub const CALL_PIPELINE_CHAIN,
    name: "call_pipeline_chain",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_pipeline_chain.mir")),
    entry: "call_pipeline_chain",
    expected: || call_pipeline_chain(10_000, 42),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(42)],
    tags: &["calls", "call", "pipeline"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Apply pipeline stage 1.
fn pipeline_stage1(value: i64, index: i64) -> i64 {
    // compute stage output
    let added = value.wrapping_add(index);

    // return value
    added.wrapping_mul(3)
}

/// Apply pipeline stage 2.
fn pipeline_stage2(value: i64, index: i64) -> i64 {
    // compute stage output
    let xored = value ^ index;

    // return value
    xored.wrapping_add(5)
}

/// Apply pipeline stage 3.
fn pipeline_stage3(value: i64, index: i64) -> i64 {
    // compute stage output
    let subbed = value.wrapping_sub(index);
    let mulled = subbed.wrapping_mul(2);

    // return value
    mulled.wrapping_add(11)
}

/// Apply pipeline stage 4.
fn pipeline_stage4(value: i64, index: i64) -> i64 {
    // compute stage output
    let merged = value | index;
    let added = merged.wrapping_add(13);
    let shifted = value >> 1;

    // return value
    added ^ shifted
}

/// Apply pipeline stage 5.
fn pipeline_stage5(value: i64, index: i64) -> i64 {
    // compute stage output
    let masked = value & index;
    let added = masked.wrapping_add(17);

    // return value
    added.wrapping_shl(1)
}

/// Apply pipeline stage 6.
fn pipeline_stage6(value: i64, index: i64) -> i64 {
    // compute stage output
    let added = value.wrapping_add(19);

    // return value
    added ^ index
}

/// Apply pipeline stage 7.
fn pipeline_stage7(value: i64, index: i64) -> i64 {
    // compute stage output
    let shifted = value.wrapping_shl(1);
    let xored = shifted ^ index;

    // return value
    xored.wrapping_add(23)
}

/// Apply pipeline stage 8.
fn pipeline_stage8(value: i64, index: i64) -> i64 {
    // compute stage output
    let mulled = value.wrapping_mul(7);
    let subbed = mulled.wrapping_sub(index);

    // return value
    subbed ^ 5
}

/// Compute the expected value for the pipeline chain benchmark.
fn call_pipeline_chain(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = seed;

    // run pipeline loop
    while index < iterations {
        // apply stage chain
        let stage1 = pipeline_stage1(acc, index);
        let stage2 = pipeline_stage2(stage1, index);
        let stage3 = pipeline_stage3(stage2, index);
        let stage4 = pipeline_stage4(stage3, index);
        let stage5 = pipeline_stage5(stage4, index);
        let stage6 = pipeline_stage6(stage5, index);
        let stage7 = pipeline_stage7(stage6, index);
        let stage8 = pipeline_stage8(stage7, index);

        // advance state
        acc = stage8;
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(acc)
}
