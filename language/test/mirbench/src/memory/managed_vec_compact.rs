use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const COMPACT_HASH_MUL: i64 = 1664525;
const COMPACT_HASH_ADD: i64 = 1013904223;
const COMPACT_MASK: i64 = 1023;

declare_program! {
    /// Compact a managed vector by copying selected values.
    pub const MANAGED_VEC_COMPACT,
    name: "managed_vec_compact",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/managed_vec_compact.mir")),
    entry: "managed_vec_compact",
    expected: || managed_vec_compact(80_000),
    default_args: |_interp| vec![Value::int64(80_000)],
    tags: &["memory", "managed", "compact"],
    scales: &[
        scale_axis("len", 0, 80000, 250000, 1000000, true),
    ],
}

/// Compute the expected value for the managed vector compact benchmark.
fn managed_vec_compact(length: i64) -> Value {
    // init buffers
    let mut input = vec![0i64; length as usize];
    let mut output = vec![0i64; length as usize];

    // fill input
    for (index, slot) in input.iter_mut().enumerate() {
        let idx = index as i64;
        let value = idx
            .wrapping_mul(COMPACT_HASH_MUL)
            .wrapping_add(COMPACT_HASH_ADD)
            & COMPACT_MASK;
        *slot = value;
    }

    // compact values
    let mut write = 0i64;
    let mut acc = 0i64;
    for value in input {
        if value & 3 == 0 {
            let out = value.wrapping_add(write);
            output[write as usize] = out;
            acc = acc.wrapping_add(out);
            write = write.wrapping_add(1);
        } else {
            acc = acc.wrapping_add(value);
        }
    }

    let mixed = mix_result(acc.wrapping_add(write), length);
    Value::int64(mixed)
}
