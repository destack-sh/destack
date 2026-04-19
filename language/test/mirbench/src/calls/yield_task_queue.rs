use super::super::common::mix_result;
use super::super::{Program, ProgramRunner, scale_axis_range};
use destack_vm::Value;

/// Return a constant resume value of two.
fn resume_two(_: &[Value], _: usize, _: Value) -> Value {
    Value::int64(2)
}

declare_program_with_runner! {
    /// Process a task queue and yield each result.
    pub const YIELD_TASK_QUEUE,
    name: "yield_task_queue",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/yield_task_queue.mir")),
    entry: "yield_task_queue",
    expected: || yield_task_queue(50_000),
    default_args: |_interp| vec![Value::int64(50_000)],
    tags: &["calls", "yield", "managed"],
    scales: &[
        scale_axis_range("iter", 0, 50000, 200000, 800000, true, 1, 1000000),
    ],
    runner: ProgramRunner::Coroutine { resume_value: resume_two },
}

/// Compute the expected value for the yield task queue benchmark.
fn yield_task_queue(iterations: i64) -> Value {
    // init buffer
    let mut buffer = vec![0i64; iterations as usize];

    // fill buffer
    for (index, slot) in buffer.iter_mut().enumerate() {
        let idx = index as i64;
        *slot = idx.wrapping_mul(5).wrapping_add(7);
    }

    // process queue
    let mut acc = 0i64;
    for (index, value) in buffer.into_iter().enumerate() {
        let idx = index as i64;
        let parsed = value.wrapping_add(acc).wrapping_add(7);
        let executed = parsed.wrapping_add(idx).wrapping_add(3);
        acc = acc.wrapping_add(executed).wrapping_add(2);
    }

    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
