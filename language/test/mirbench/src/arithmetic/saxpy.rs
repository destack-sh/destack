use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// SAXPY: scale and add vectors, then reduce.
    pub const SAXPY,
    name: "saxpy",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/saxpy.mir")),
    entry: "saxpy",
    expected: || saxpy(4096),
    default_args: |_interp| vec![Value::int64(4096)],
    tags: &["arithmetic", "vector"],
    scales: &[
        scale_axis("len", 0, 4096, 16384, 65536, true),
    ],
}

/// Compute the expected value for the saxpy benchmark.
fn saxpy(length: i64) -> Value {
    // init buffers
    let mut x = vec![0i64; length as usize];
    let mut y = vec![0i64; length as usize];

    // fill buffers
    for (index, (x_slot, y_slot)) in x.iter_mut().zip(y.iter_mut()).enumerate() {
        let idx = index as i64;
        *x_slot = idx.wrapping_mul(2).wrapping_add(1);
        *y_slot = idx.wrapping_mul(3).wrapping_add(2);
    }

    // apply saxpy
    let mut acc = 0i64;
    for (x_value, y_slot) in x.iter().zip(y.iter_mut()) {
        let value = x_value.wrapping_mul(7).wrapping_add(*y_slot);
        *y_slot = value;
        acc = acc.wrapping_add(value);
    }

    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
