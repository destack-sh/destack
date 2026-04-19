use super::super::common::{clamp_min, memcmp_bytes, mix_result};
use super::super::{Program, scale_axis};
use destack_vm::Value;

const CHUNK_LEN: usize = 16;

declare_program! {
    /// Chunked memcmp comparisons over two buffers.
    pub const MEMCMP_CHUNKS,
    name: "memcmp_chunks",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/memcmp_chunks.mir")),
    entry: "memcmp_chunks",
    expected: || memcmp_chunks(10_000, 256),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(256)],
    tags: &["intrinsics", "memcmp"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
        scale_axis("len", 1, 256, 1024, 4096, false),
    ],
}

/// Compute the expected value for the memcmp chunks benchmark.
fn memcmp_chunks(iterations: i64, length: i64) -> Value {
    // clamp length to avoid empty buffers
    let length = clamp_min(length, CHUNK_LEN as i64);
    let length_usize = length as usize;

    // build buffers
    let mut left = vec![0u8; length_usize];
    let mut right = vec![0u8; length_usize];
    for index in 0..length_usize {
        let value = (index as i64).wrapping_mul(3).wrapping_add(1) as u8;
        left[index] = value;
        if (index & 7) == 0 {
            right[index] = value.wrapping_add(1);
        } else {
            right[index] = value;
        }
    }

    // compare chunks and accumulate
    let mut acc = 0i64;
    let mut offset = 0usize;
    while offset < length_usize {
        let remaining = length_usize - offset;
        let chunk_len = CHUNK_LEN.min(remaining);
        let end = offset + chunk_len;
        let cmp = memcmp_bytes(&left[offset..end], &right[offset..end]);
        acc = acc.wrapping_add(cmp as i64);
        offset += chunk_len;
    }

    // scale by iterations
    let mut iter = 0i64;
    let mut total = 0i64;
    while iter < iterations {
        total = total.wrapping_add(acc);
        iter = iter.wrapping_add(1);
    }

    let mixed = mix_result(total, iterations ^ length);
    Value::int64(mixed)
}
