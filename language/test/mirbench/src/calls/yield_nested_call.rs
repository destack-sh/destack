use super::super::{Program, ProgramRunner, scale_axis_range};
use destack_vm::Value;

/// Return a constant resume value of one.
fn resume_one(_: &[Value], _: usize, _: Value) -> Value {
    Value::int64(1)
}

declare_program_with_runner! {
    /// Yield within a helper call and accumulate results.
    pub const YIELD_NESTED_CALL,
    name: "yield_nested_call",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/yield_nested_call.mir")),
    entry: "yield_nested_call",
    expected: || Value::int64(500500),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["calls", "yield", "coroutine", "nested"],
    scales: &[
        scale_axis_range("iter", 0, 500, 5000, 20000, true, 1, 50000),
    ],
    runner: ProgramRunner::Coroutine { resume_value: resume_one },
}
