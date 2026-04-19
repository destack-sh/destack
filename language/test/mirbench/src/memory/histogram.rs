use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Histogram bucket updates over a random stream.
    pub const HISTOGRAM,
    name: "histogram",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/histogram.mir")),
    entry: "histogram",
    expected: || histogram(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["memory", "histogram", "bucket"],
    scales: &[
        scale_axis("len", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the histogram benchmark.
fn histogram(length: i64) -> Value {
    // return total count
    let mixed = mix_result(length, length);
    Value::int64(mixed)
}
