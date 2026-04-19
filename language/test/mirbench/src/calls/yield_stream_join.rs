use super::super::common::mix_result;
use super::super::{Program, ProgramRunner, scale_axis_range};
use destack_vm::Value;

/// Resume by adding one to the yielded value.
fn resume_plus_one(_: &[Value], _: usize, yielded: Value) -> Value {
    let base = yielded.as_int().unwrap_or(0);
    Value::int64(base + 1)
}

declare_program_with_runner! {
    /// Join two managed streams and yield each result.
    pub const YIELD_STREAM_JOIN,
    name: "yield_stream_join",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/yield_stream_join.mir")),
    entry: "yield_stream_join",
    expected: || yield_stream_join(40_000),
    default_args: |_interp| vec![Value::int64(40_000)],
    tags: &[
        "calls",
        "yield",
        "managed",
    ],
    scales: &[
        scale_axis_range("iter", 0, 40000, 200000, 800000, true, 1, 1000000),
    ],
    runner: ProgramRunner::Coroutine { resume_value: resume_plus_one },
}

/// Compute the expected value for the yield stream join benchmark.
fn yield_stream_join(iterations: i64) -> Value {
    // init buffers
    let mut left = vec![0i64; iterations as usize];
    let mut right = vec![0i64; iterations as usize];

    // fill buffers
    for (index, slot) in left.iter_mut().enumerate() {
        let idx = index as i64;
        *slot = idx.wrapping_mul(3).wrapping_add(1);
    }
    for (index, slot) in right.iter_mut().enumerate() {
        let idx = index as i64;
        *slot = idx.wrapping_mul(5).wrapping_add(2);
    }

    // process stream join
    let mut acc = 0i64;
    for index in 0..iterations {
        let left_value = left[index as usize];
        let right_value = right[index as usize];
        let left_mapped = left_value.wrapping_add(index).wrapping_mul(3);
        let right_mapped = right_value.wrapping_mul(5).wrapping_add(index);
        let joined = left_mapped ^ right_mapped;
        let joined = joined.wrapping_add(7);
        let resume = joined.wrapping_add(1);
        acc = acc.wrapping_add(joined).wrapping_add(resume);
    }

    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
