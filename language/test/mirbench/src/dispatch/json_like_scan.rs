use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Json like scanning with nested depth tracking.
    pub const JSON_LIKE_SCAN,
    name: "json_like_scan",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/json_like_scan.mir")),
    entry: "json_like_scan",
    expected: || json_like_scan(10_000, 1024),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(1024)],
    tags: &["dispatch", "json"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the json scan benchmark.
fn json_like_scan(iterations: i64, stream_len: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut depth = 0i64;
    let mut tokens = 0i64;
    let mut in_string = 0i64;
    let stream_len = clamp_min(stream_len, 1);

    // run scan loop
    while index < iterations {
        // compute kind bucket
        let position = index.wrapping_rem(stream_len);
        let kind = index.wrapping_mul(3).wrapping_add(position).wrapping_rem(8);

        // apply scanner rules
        if in_string == 1 {
            if kind == 4 {
                in_string = 0;
            }
        } else {
            match kind {
                0 | 2 => {
                    depth = depth.wrapping_add(1);
                }
                1 | 3 => {
                    if depth > 0 {
                        depth = depth.wrapping_sub(1);
                    }
                }
                4 => {
                    in_string = 1;
                }
                5 | 6 => {
                    tokens = tokens.wrapping_add(1);
                }
                _ => {}
            }
        }

        // advance counter
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(tokens.wrapping_add(depth).wrapping_add(in_string))
}
