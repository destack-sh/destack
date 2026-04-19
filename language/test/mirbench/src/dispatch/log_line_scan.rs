use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Scan log lines and count digits and spans.
    pub const LOG_LINE_SCAN,
    name: "log_line_scan",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/log_line_scan.mir")),
    entry: "log_line_scan",
    expected: || log_line_scan(250_000),
    default_args: |_interp| vec![Value::int64(250_000)],
    tags: &["dispatch", "scan"],
    scales: &[
        scale_axis("len", 0, 250000, 1000000, 5000000, true),
    ],
}

/// Compute the expected value for the log line scan benchmark.
fn log_line_scan(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut digits = 0i64;
    let mut spans = 0i64;
    let mut in_span = 0i64;
    let mut acc = 0i64;

    // scan tokens
    while index < length {
        let value = index.wrapping_mul(1664525).wrapping_add(1013904223) & 15;
        if value <= 9 {
            digits = digits.wrapping_add(1);
            acc = acc.wrapping_add(value);
            in_span = 1;
        } else if value == 10 {
            if in_span != 0 {
                spans = spans.wrapping_add(1);
                in_span = 0;
            }
        } else {
            acc = acc ^ value;
            in_span = 1;
        }

        index = index.wrapping_add(1);
    }

    // finalize trailing span
    if in_span != 0 {
        spans = spans.wrapping_add(1);
    }

    let mixed = mix_result(digits.wrapping_add(spans).wrapping_add(acc), length);
    Value::int64(mixed)
}
