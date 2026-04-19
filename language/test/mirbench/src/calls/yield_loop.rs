use super::super::{Program, ProgramRunner, scale_axis_range};
use destack_vm::Value;

/// Return a constant resume value of one.
fn resume_one(_: &[Value], _: usize, _: Value) -> Value {
    Value::int64(1)
}

declare_program_with_runner! {
    /// Yield in a loop and accumulate resume values.
    pub const YIELD_LOOP,
    name: "yield_loop",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/yield_loop.mir")),
    entry: "yield_loop",
    expected: || Value::int64(1000),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["calls", "yield", "coroutine"],
    scales: &[
        scale_axis_range("iter", 0, 1000, 20000, 100000, true, 1, 200000),
    ],
    runner: ProgramRunner::Coroutine { resume_value: resume_one },
}
