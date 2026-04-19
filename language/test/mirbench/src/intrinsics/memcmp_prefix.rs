use super::super::common::{clamp_min, memcmp_bytes, mix_result};
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Prefix comparisons using the memcmp intrinsic.
    pub const MEMCMP_PREFIX,
    name: "memcmp_prefix",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/memcmp_prefix.mir")),
    entry: "memcmp_prefix",
    expected: || memcmp_prefix(2048),
    default_args: |_interp| vec![Value::int64(2048)],
    tags: &["intrinsics", "memcmp"],
    scales: &[
        scale_axis("len", 0, 2048, 8192, 32768, true),
    ],
}

/// Compute the expected value for the memcmp prefix benchmark.
fn memcmp_prefix(length: i64) -> Value {
    // clamp length to avoid empty buffers
    let length = clamp_min(length, 1);
    let length_usize = length as usize;

    // initialize buffers
    let mut left = vec![0u8; length_usize];
    let mut right = vec![0u8; length_usize];

    // fill buffers
    let mut index = 0usize;
    while index < length_usize {
        let value = (index as i64).wrapping_mul(3).wrapping_add(1) as u8;
        left[index] = value;
        if (index & 3) == 0 {
            right[index] = value.wrapping_add(1);
        } else {
            right[index] = value;
        }
        index += 1;
    }

    // compare prefixes
    let mut acc = 0i64;
    index = 0;
    while index < length_usize {
        let left_slice = &left[index..];
        let right_slice = &right[index..];
        let cmp = memcmp_bytes(left_slice, right_slice) as i64;
        acc = acc.wrapping_add(cmp);
        index += 1;
    }

    // return value
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
