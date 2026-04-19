use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Compact a sparse buffer into a dense one.
    pub const BUFFER_COMPACT,
    name: "buffer_compact",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/buffer_compact.mir")),
    entry: "buffer_compact",
    expected: || buffer_compact(200_000),
    default_args: |_interp| vec![Value::int64(200_000)],
    tags: &["memory", "compact"],
    scales: &[
        scale_axis("len", 0, 200000, 800000, 2000000, true),
    ],
}

/// Compute the expected value for the buffer compact benchmark.
fn buffer_compact(length: i64) -> Value {
    // init buffers
    let mut src = vec![0i64; length as usize];
    let mut dst = vec![0i64; length as usize];

    // fill input
    for (index, slot) in src.iter_mut().enumerate() {
        let idx = index as i64;
        let value = (idx.wrapping_mul(5).wrapping_add(7)) & 15;
        *slot = if value < 4 { 0 } else { value };
    }

    // compact output
    let mut write = 0i64;
    let mut acc = 0i64;
    for value in src {
        if value != 0 {
            dst[write as usize] = value;
            write = write.wrapping_add(1);
            acc = acc.wrapping_add(value);
        }
    }

    let mixed = mix_result(write.wrapping_add(acc), length);
    Value::int64(mixed)
}
