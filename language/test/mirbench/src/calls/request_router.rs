use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const ROUTE_HASH_MUL: i64 = 1103515245;
const ROUTE_HASH_ADD: i64 = 12345;
const ROUTE_MASK: i64 = 3;

declare_program! {
    /// Dispatch request handlers with a route switch.
    pub const REQUEST_ROUTER,
    name: "request_router",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/calls/request_router.mir")),
    entry: "request_router",
    expected: || request_router(100_000),
    default_args: |_interp| vec![Value::int64(100_000)],
    tags: &["calls", "dispatch", "router"],
    scales: &[
        scale_axis("ops", 0, 100000, 500000, 1000000, true),
    ],
}

/// Compute the expected value for the request router benchmark.
fn request_router(iterations: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;

    // run routing loop
    while index < iterations {
        let hash = index
            .wrapping_mul(ROUTE_HASH_MUL)
            .wrapping_add(ROUTE_HASH_ADD);
        let route = hash & ROUTE_MASK;

        let result = match route {
            0 => handle_get(index, acc),
            1 => handle_post(index, acc),
            2 => handle_put(index, acc),
            _ => handle_delete(index, acc),
        };

        acc = acc.wrapping_add(result);
        index = index.wrapping_add(1);
    }

    let mixed = mix_result(acc, iterations);
    Value::int64(mixed)
}
/// Handle a GET style request.
fn handle_get(index: i64, acc: i64) -> i64 {
    // update hash
    let value = index.wrapping_add(acc);
    let scaled = value.wrapping_mul(3);
    scaled.wrapping_add(7)
}

/// Handle a POST style request.
fn handle_post(index: i64, acc: i64) -> i64 {
    // update hash
    let mixed = index ^ acc;
    let shifted = mixed.wrapping_add(11) << 1;
    shifted
}

/// Handle a PUT style request.
fn handle_put(index: i64, acc: i64) -> i64 {
    // update hash
    let diff = acc.wrapping_sub(index);
    let scaled = diff.wrapping_add(5).wrapping_mul(2);
    scaled
}

/// Handle a DELETE style request.
fn handle_delete(index: i64, acc: i64) -> i64 {
    // update hash
    let shifted = index << 1;
    let mixed = shifted ^ acc;
    mixed.wrapping_add(13)
}
