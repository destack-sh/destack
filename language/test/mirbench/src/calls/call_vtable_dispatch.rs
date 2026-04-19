use super::super::{Program, function_pointer_by_name, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Vtable like dispatch through indirect calls.
    pub const CALL_VTABLE_DISPATCH,
    name: "call_vtable_dispatch",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/call_vtable_dispatch.mir")),
    entry: "call_vtable_dispatch",
    expected: || call_vtable_dispatch(10_000, 7),
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
    tags: &["calls", "call", "vtable"],
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

/// Compute the expected value for the vtable dispatch benchmark.
fn call_vtable_dispatch(iterations: i64, self_value: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // run vtable loop
    while index < iterations {
        // dispatch based on skewed selector
        let mixed = index.wrapping_mul(1103515245).wrapping_add(12345);
        let bucket = mixed & 255;
        if bucket < 200 {
            acc = vtable_method_add(self_value, acc);
        } else if bucket < 240 {
            acc = vtable_method_mul(self_value, acc);
        } else {
            acc = vtable_method_mix(self_value, acc);
        }

        // advance counter
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(acc)
}
