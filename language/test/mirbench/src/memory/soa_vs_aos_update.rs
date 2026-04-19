use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Update struct of arrays and array of structs layouts.
    pub const SOA_VS_AOS_UPDATE,
    name: "soa_vs_aos_update",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/soa_vs_aos_update.mir")),
    entry: "soa_vs_aos_update",
    expected: || soa_vs_aos_update(1000, 2),
    default_args: |_interp| vec![Value::int64(1000), Value::int64(2)],
    tags: &["memory", "layout"],
    scales: &[
        scale_axis("iter", 0, 1000, 10000, 100000, true),
    ],
}

/// Compute the expected value for the soa vs aos benchmark.
fn soa_vs_aos_update(count: i64, multiplier: i64) -> Value {
    // compute effective count
    let count = count.wrapping_mul(clamp_min(multiplier, 1));

    // fill value arrays
    let mut soa_x = Vec::with_capacity(count as usize);
    let mut soa_y = Vec::with_capacity(count as usize);
    let mut aos_x = Vec::with_capacity(count as usize);
    let mut aos_y = Vec::with_capacity(count as usize);

    for index in 0..count {
        let x = index.wrapping_add(1);
        let y = x.wrapping_mul(2);
        soa_x.push(x);
        soa_y.push(y);
        aos_x.push(x);
        aos_y.push(y);
    }

    // sum both layouts
    let mut total = 0i64;
    for index in 0..count {
        let soa_sum = soa_x[index as usize].wrapping_add(soa_y[index as usize]);
        let aos_sum = aos_x[index as usize].wrapping_add(aos_y[index as usize]);
        total = total.wrapping_add(soa_sum.wrapping_add(aos_sum));
    }

    // return value
    Value::int64(total)
}
