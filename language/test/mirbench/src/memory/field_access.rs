use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Field access on composite values (tuples from overflow intrinsics).
    pub const FIELD_ACCESS,
    name: "field_access",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/field_access.mir")),
    entry: "field_access",
    expected: || field_access(10_000, 1),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["memory", "field"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the field access benchmark.
fn field_access(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut value = seed;

    // run field access loop
    while index < iterations {
        // apply overflow intrinsics
        let (added, _) = value.overflowing_add(value);
        let (mulled, _) = added.overflowing_mul(value);
        let (subbed, _) = mulled.overflowing_sub(value);
        let masked = subbed & 255;

        // advance state
        value = masked;
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(value)
}
