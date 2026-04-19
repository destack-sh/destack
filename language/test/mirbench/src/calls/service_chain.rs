use super::super::common::mix_result;
use super::super::{Program, function_pointer_by_name, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Decode-transform-encode service pipeline with an indirect stage.
    pub const SERVICE_CHAIN,
    name: "service_chain",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/service_chain.mir")),
    entry: "service_chain",
    expected: || service_chain(10_000),
    default_args: |interp| {
        // resolve transform callback
        let transform = function_pointer_by_name(interp, "svc_transform");

        vec![Value::int64(10_000), transform]
    },
    tags: &["calls", "service", "pipeline"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Apply the decode stage.
fn svc_decode(value: i64) -> i64 {
    // compute decoded value
    value.wrapping_mul(3).wrapping_add(1)
}

/// Apply the transform stage.
fn svc_transform(value: i64) -> i64 {
    // compute transformed value
    (value ^ 7).wrapping_add(5)
}

/// Apply the encode stage.
fn svc_encode(value: i64) -> i64 {
    // compute encoded value
    value.wrapping_mul(2).wrapping_add(9)
}

/// Compute the expected value for the service chain benchmark.
fn service_chain(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // run pipeline loop
    while index < iterations {
        let decoded = svc_decode(index);
        let transformed = svc_transform(decoded);
        let encoded = svc_encode(transformed);
        acc = acc.wrapping_add(encoded);
        index = index.wrapping_add(1);
    }

    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
