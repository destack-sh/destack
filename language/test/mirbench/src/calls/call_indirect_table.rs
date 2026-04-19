use super::super::{Program, function_pointer_by_name, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Indirect calls loaded from a function pointer table.
    pub const CALL_INDIRECT_TABLE,
    name: "call_indirect_table",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_indirect_table.mir")),
    entry: "call_indirect_table",
    expected: || call_indirect_table(10_000, 7),
    default_args: |interp| {
        // resolve function pointer arguments
        let method_add = function_pointer_by_name(interp, "method_add");
        let method_mul = function_pointer_by_name(interp, "method_mul");
        let method_mix = function_pointer_by_name(interp, "method_mix");

        // assemble default argument list
        vec![
            Value::int64(10_000),
            Value::int64(7),
            method_add,
            method_mul,
            method_mix,
        ]
    },
    tags: &["calls", "call", "indirect"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Apply vtable method add.
fn vtable_method_add(self_value: i64, acc: i64) -> i64 {
    // compute method output
    let added = self_value.wrapping_add(acc);

    // return value
    added.wrapping_add(1)
}

/// Apply vtable method multiply.
fn vtable_method_mul(self_value: i64, acc: i64) -> i64 {
    // compute method output
    let mulled = self_value.wrapping_mul(3);

    // return value
    mulled.wrapping_add(acc)
}

/// Apply vtable method mix.
fn vtable_method_mix(self_value: i64, acc: i64) -> i64 {
    // compute method output
    let xored = self_value ^ acc;

    // return value
    xored.wrapping_add(7)
}

/// Compute the expected value for the indirect table call benchmark.
fn call_indirect_table(iterations: i64, self_value: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // run dispatch loop
    while index < iterations {
        // pick target from table
        let mixed = index.wrapping_mul(1103515245).wrapping_add(12345);
        let slot = mixed.wrapping_rem(3);

        // dispatch call
        match slot {
            0 => {
                acc = vtable_method_add(self_value, acc);
            }
            1 => {
                acc = vtable_method_mul(self_value, acc);
            }
            _ => {
                acc = vtable_method_mix(self_value, acc);
            }
        }

        // advance counter
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(acc)
}
