use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Scrub buffers with memset and memcpy intrinsics.
    pub const MEMSET_SCRUB,
    name: "memset_scrub",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/memset_scrub.mir")),
    entry: "memset_scrub",
    expected: || memset_scrub(10_000, 256),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(256)],
    tags: &["intrinsics", "memset", "memcpy"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
        scale_axis("len", 1, 256, 1024, 4096, false),
    ],
}

/// Compute the expected value for the memset scrub benchmark.
fn memset_scrub(iterations: i64, length: i64) -> Value {
    // init buffers
    let length = length.max(0);
    let length_usize = length as usize;
    let mut buffer_a = vec![0i64; length_usize];
    let mut buffer_b = vec![0i64; length_usize];

    // run scrub loop
    let mut index = 0i64;
    let mut acc = 0i64;
    while index < iterations {
        // scrub buffers
        if (index & 1) == 0 {
            buffer_a.fill(index);
        } else {
            buffer_b.fill(index.wrapping_add(1));
            buffer_a.copy_from_slice(&buffer_b);
        }

        acc = acc.wrapping_add(index);
        index = index.wrapping_add(1);
    }

    let mixed = mix_result(acc, iterations ^ length);
    Value::int64(mixed)
}
