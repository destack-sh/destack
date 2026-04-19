use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// HTTP-style router with method and route switches.
    pub const HTTP_ROUTER,
    name: "http_router",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/http_router.mir")),
    entry: "http_router",
    expected: || http_router(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "router", "http"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the http router benchmark.
fn http_router(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // run router loop
    while index < iterations {
        let method = index & 1;
        let route = index
            .wrapping_mul(1103515245)
            .wrapping_add(12345)
            .wrapping_rem(4);

        if method == 0 {
            acc = match route {
                0 => acc.wrapping_add(index),
                1 => acc.wrapping_add(3),
                2 => acc.wrapping_add(index.wrapping_mul(2)),
                _ => acc.wrapping_add(7),
            };
        } else {
            acc = match route {
                0 => acc.wrapping_add(index ^ 5),
                1 => acc.wrapping_add(index.wrapping_add(11)),
                2 => acc.wrapping_add(index.wrapping_sub(1)),
                _ => acc.wrapping_add(9),
            };
        }

        index = index.wrapping_add(1);
    }

    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
