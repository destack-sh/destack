use super::super::common::{clamp_min, mix_result};
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Ring buffer updates with wraparound indexing.
    pub const RING_BUFFER,
    name: "ring_buffer",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/ring_buffer.mir")),
    entry: "ring_buffer",
    expected: || ring_buffer(1024),
    default_args: |_interp| vec![Value::int64(1024)],
    tags: &["memory", "ring", "buffer"],
    scales: &[
        scale_axis("len", 0, 1024, 8192, 65536, true),
    ],
}

/// Compute the expected value for the ring buffer benchmark.
fn ring_buffer(length: i64) -> Value {
    // clamp length to avoid zero-sized buffers
    let length = clamp_min(length, 1);
    let length_usize = length as usize;

    // init buffer
    let mut buffer = Vec::with_capacity(length_usize);
    let mut index = 0usize;
    while index < length_usize {
        buffer.push(index as i64);
        index += 1;
    }

    // update ring buffer
    let mut acc = 0i64;
    index = 0;
    while index < length_usize {
        let slot = index % length_usize;
        let value = buffer[slot].wrapping_add(index as i64);
        buffer[slot] = value;
        acc = acc.wrapping_add(value);
        index += 1;
    }

    // return value
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
