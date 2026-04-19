use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Float math intrinsics including sqrt, sin, and cos.
    pub const MATH_FLOAT,
    name: "math_float",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/intrinsics/math_float.mir")),
    entry: "math_float",
    expected: || math_float(10_000, 0.0, 0.01),
    default_args: |_interp| {
        vec![
            Value::int64(10_000),
            Value::float64(0.0),
            Value::float64(0.01),
        ]
    },
    tags: &["intrinsics", "float", "math"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the math float benchmark.
fn math_float(iterations: i64, start: f64, step: f64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = start;
    let mut angle = step;

    // run math loop
    while index < iterations {
        // compute trig sum
        let _root = angle.sqrt();
        let sin = angle.sin();
        let cos = angle.cos();
        let sum = sin * sin + cos * cos;

        // advance state
        acc += sum;
        angle += step;
        index = index.wrapping_add(1);
    }

    // return value
    Value::float64(acc)
}
