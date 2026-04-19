use super::super::{Program, ProgramRunner, scale_axis_range};
use destack_vm::Value;

/// Resolve awaitable values by incrementing them.
fn resume_from_awaitable(_: &[Value], _: usize, yielded: Value) -> Value {
    // derive a deterministic resolved value
    let base = yielded.as_int().unwrap_or(0);
    Value::int64(base + 1)
}

declare_program_with_runner! {
    /// Async-style state machine with await suspension points.
    pub const ASYNC_STATE_MACHINE,
    name: "async_state_machine",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/async_state_machine.mir")),
    entry: "async_entry",
    expected: || Value::int64(3_500_500),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["calls", "async", "coroutine"],
    scales: &[
        scale_axis_range("iter", 0, 1000, 10000, 50000, true, 1, 100000),
    ],
    runner: ProgramRunner::Coroutine {
        resume_value: resume_from_awaitable,
    },
}
