use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Float arithmetic with add, sub, mul, and div loop.
    pub const BINARY_FLOAT,
    name: "binary_float",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/binary_float.mir")),
    entry: "binary_float",
    expected: || binary_float(10_000, 1.0, 0.5, 2.0),
    default_args: |_interp| {
        vec![
            Value::int64(10_000),
            Value::float64(1.0),
            Value::float64(0.5),
            Value::float64(2.0),
        ]
    },
    tags: &["arithmetic", "binary", "float"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the binary float benchmark.
fn binary_float(iterations: i64, start: f64, add: f64, mul: f64) -> Value {
    // init state
    let mut index = 0i64;
    let mut value = start;

    // run float loop
    while index < iterations {
        // apply float operations
        let added = value + add;
        let scaled = added * mul;
        let subbed = scaled - add;
        let divided = subbed / mul;

        // advance state
        value = divided;
        index = index.wrapping_add(1);
    }

    // return value
    Value::float64(value)
}
