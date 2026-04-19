use super::super::common::mix_result;
use super::super::{Program, ProgramRunner, scale_axis_range};
use destack_vm::Value;

/// Return a constant resume value of one.
fn resume_one(_: &[Value], _: usize, _: Value) -> Value {
    Value::int64(1)
}

declare_program_with_runner! {
    /// Yield values from a managed buffer and accumulate resumes.
    pub const YIELD_BUFFER_SCAN,
    name: "yield_buffer_scan",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/yield_buffer_scan.mir")),
    entry: "yield_buffer_scan",
    expected: || yield_buffer_scan(1000),
    default_args: |_interp| vec![Value::int64(1000)],
    tags: &["calls", "yield", "managed"],
    scales: &[
        scale_axis_range("iter", 0, 1000, 20000, 100000, true, 1, 200000),
    ],
    runner: ProgramRunner::Coroutine { resume_value: resume_one },
}

/// Compute the expected value for the yield buffer scan benchmark.
fn yield_buffer_scan(iterations: i64) -> Value {
    // init buffer
    let mut buffer = vec![0i64; iterations as usize];

    // fill buffer
    for (index, slot) in buffer.iter_mut().enumerate() {
        let idx = index as i64;
        *slot = idx.wrapping_mul(3).wrapping_add(1);
    }

    // run loop
    let mut acc = 0i64;
    for value in buffer {
        acc = acc.wrapping_add(value).wrapping_add(1);
    }

    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
