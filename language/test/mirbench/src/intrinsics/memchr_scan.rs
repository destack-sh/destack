use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Scan a buffer for a sentinel value.
    pub const MEMCHR_SCAN,
    name: "memchr_scan",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/memchr_scan.mir")),
    entry: "memchr_scan",
    expected: || memchr_scan(100_000),
    default_args: |_interp| vec![Value::int64(100_000)],
    tags: &["intrinsics", "scan"],
    scales: &[
        scale_axis("len", 0, 100000, 500000, 2000000, true),
    ],
}

/// Compute the expected value for the memchr scan benchmark.
fn memchr_scan(length: i64) -> Value {
    // init buffer
    let mut buffer = vec![0i64; length as usize];

    // fill buffer
    for (index, slot) in buffer.iter_mut().enumerate() {
        let idx = index as i64;
        *slot = idx.wrapping_mul(37).wrapping_add(11) & 255;
    }

    // scan for sentinel
    let mut index = 0i64;
    let mut matches = 0i64;
    let mut acc = 0i64;
    while index < length {
        if buffer[index as usize] == 42 {
            matches = matches.wrapping_add(1);
            acc = acc.wrapping_add(index);
        }

        index = index.wrapping_add(1);
    }

    let mixed = mix_result(matches.wrapping_add(acc), length);
    Value::int64(mixed)
}
