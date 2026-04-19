use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Nested array allocations with inner fill and sum passes.
    pub const NESTED_ARRAY_SUM,
    name: "nested_array_sum",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/nested_array_sum.mir")),
    entry: "nested_array_sum",
    expected: || nested_array_sum(64, 16),
    default_args: |_interp| vec![Value::int64(64), Value::int64(16)],
    tags: &["memory", "array"],
    scales: &[
        scale_axis("outer", 0, 64, 256, 1024, true),
        scale_axis("inner", 1, 16, 64, 256, false),
    ],
}

/// Compute the expected value for the nested array sum benchmark.
fn nested_array_sum(outer_len: i64, inner_len: i64) -> Value {
    // init state
    let outer_len = clamp_min(outer_len, 1);
    let inner_len = clamp_min(inner_len, 1);
    let mut outer = 0i64;
    let mut sum = 0i64;

    // run nested loops
    while outer < outer_len {
        // compute inner loop
        let mut inner = 0i64;
        while inner < inner_len {
            let scaled_outer = outer.wrapping_mul(31);
            let scaled_inner = inner.wrapping_mul(7);
            let value = scaled_outer.wrapping_add(scaled_inner);
            sum = sum.wrapping_add(value);
            inner = inner.wrapping_add(1);
        }

        // advance outer loop
        outer = outer.wrapping_add(1);
    }

    // return value
    Value::int64(sum)
}
