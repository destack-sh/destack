use super::super::common::mix_result;
use super::super::{Program, ProgramRunner, scale_axis_range};
use destack_vm::Value;

const FILTER_HASH_MUL: i64 = 1103515245;
const FILTER_HASH_ADD: i64 = 12345;
const FILTER_TOKEN_MASK: i64 = 255;

/// Return a constant resume value of three.
fn resume_three(_: &[Value], _: usize, _: Value) -> Value {
    Value::int64(3)
}

declare_program_with_runner! {
    /// Filter a stream and yield even entries.
    pub const YIELD_BATCH_FILTER,
    name: "yield_batch_filter",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/yield_batch_filter.mir")),
    entry: "yield_batch_filter",
    expected: || yield_batch_filter(60_000),
    default_args: |_interp| vec![Value::int64(60_000)],
    tags: &["calls", "yield", "managed", "filter"],
    scales: &[
        scale_axis_range("iter", 0, 60000, 200000, 800000, true, 1, 1000000),
    ],
    runner: ProgramRunner::Coroutine { resume_value: resume_three },
}

/// Compute the expected value for the yield batch filter benchmark.
fn yield_batch_filter(iterations: i64) -> Value {
    // init buffer
    let mut buffer = vec![0i64; iterations as usize];

    // fill buffer
    for (index, slot) in buffer.iter_mut().enumerate() {
        let idx = index as i64;
        let value = idx
            .wrapping_mul(FILTER_HASH_MUL)
            .wrapping_add(FILTER_HASH_ADD)
            .wrapping_shr(16)
            & FILTER_TOKEN_MASK;
        *slot = value;
    }

    // process stream
    let mut acc = 0i64;
    let mut count = 0i64;
    for value in buffer {
        if value & 1 == 0 {
            let out = value.wrapping_add(acc).wrapping_add(count);
            acc = acc.wrapping_add(out).wrapping_add(3);
            count = count.wrapping_add(1);
        } else {
            acc = acc.wrapping_add(value);
        }
    }

    let mixed = mix_result(acc.wrapping_add(count), iterations);
    Value::int64(mixed)
}
