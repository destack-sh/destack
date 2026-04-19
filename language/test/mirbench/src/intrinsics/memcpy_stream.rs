use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Memcpy style intrinsic on managed arrays.
    pub const MEMCPY_STREAM,
    name: "memcpy_stream",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/memcpy_stream.mir")),
    entry: "memcpy_stream",
    expected: || memcpy_stream(10_000, 128),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(128)],
    tags: &["intrinsics", "memcpy"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
        scale_axis("len", 1, 128, 512, 2048, false),
    ],
}

/// Compute the expected value for the memcpy stream benchmark.
fn memcpy_stream(iterations: i64, length: i64) -> Value {
    // init state
    let _len = clamp_min(length, 1);
    let mut index = 0i64;
    let mut acc = 0i64;

    // run copy loop
    while index < iterations {
        // record copied value
        acc = acc.wrapping_add(index);
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(acc)
}
