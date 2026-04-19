use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const PIPELINE_HASH_MUL: i64 = 1103515245;
const PIPELINE_HASH_ADD: i64 = 12345;

const PIPELINE_EXEC_MASK: i64 = 1;

declare_program! {
    /// Pipeline a task through parse, auth, optional exec, and logging stages.
    pub const TASK_PIPELINE,
    name: "task_pipeline",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/task_pipeline.mir")),
    entry: "task_pipeline",
    expected: || task_pipeline(120_000),
    default_args: |_interp| vec![Value::int64(120_000)],
    tags: &["calls", "pipeline", "task"],
    scales: &[
        scale_axis("ops", 0, 120000, 600000, 1200000, true),
    ],
}

/// Compute the expected value for the task pipeline benchmark.
fn task_pipeline(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut log_slots = [0i64; 64];

    // run pipeline loop
    while index < iterations {
        let hash = index
            .wrapping_mul(PIPELINE_HASH_MUL)
            .wrapping_add(PIPELINE_HASH_ADD);
        let route = hash & PIPELINE_EXEC_MASK;

        let parsed = stage_parse(index, acc);
        let authed = stage_auth(parsed, index);
        let executed = if route == 0 {
            stage_exec(authed, index)
        } else {
            authed
        };
        let logged = stage_log(executed, acc);
        let slot = (index & 63) as usize;
        log_slots[slot] = logged;

        acc = acc.wrapping_add(logged);
        index = index.wrapping_add(1);
    }

    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
/// Parse a task payload.
fn stage_parse(index: i64, acc: i64) -> i64 {
    // compute parse score
    let value = index.wrapping_add(acc);
    value.wrapping_add(7)
}

/// Authorize a task payload.
fn stage_auth(value: i64, index: i64) -> i64 {
    // mix auth token
    let summed = value.wrapping_add(index);
    summed.wrapping_add(11)
}

/// Execute a task payload.
fn stage_exec(value: i64, index: i64) -> i64 {
    // apply execution cost
    let summed = value.wrapping_add(index);
    summed.wrapping_add(5)
}

/// Log a task payload.
fn stage_log(value: i64, acc: i64) -> i64 {
    // compute log hash
    let summed = value.wrapping_add(acc);
    summed.wrapping_add(13)
}
