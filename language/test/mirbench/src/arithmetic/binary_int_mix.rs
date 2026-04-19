use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Mixed integer operations including add, sub, mul, div, mod, shifts, and bitwise.
    pub const BINARY_INT_MIX,
    name: "binary_int_mix",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/binary_int_mix.mir")),
    entry: "binary_int_mix",
    expected: || binary_int_mix(10_000, 12345),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["arithmetic", "binary"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the binary integer mix benchmark.
fn binary_int_mix(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = seed;

    // run mix loop
    while index < iterations {
        // mix arithmetic and bit operations
        let added = acc.wrapping_add(index);
        let subbed = added.wrapping_sub(1);
        let mulled = subbed.wrapping_mul(1);
        let divided = mulled.wrapping_div(7);
        let rem = divided.wrapping_rem(13);
        let shifted_left = rem.wrapping_shl(3);
        let shifted_right = shifted_left.wrapping_shr(1);
        let masked = shifted_right & 255;
        let xored = masked ^ 170;
        let merged = xored | 85;

        // advance state
        acc = merged;
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(acc)
}
