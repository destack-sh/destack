use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const CSV_HASH_MUL: i64 = 1664525;
const CSV_HASH_ADD: i64 = 1013904223;
const CSV_HASH_MOD: i64 = 7;

const CSV_TOKEN_COMMA: i64 = 0;
const CSV_TOKEN_QUOTE: i64 = 1;
const CSV_TOKEN_NEWLINE: i64 = 2;

declare_program! {
    /// Scan pseudo CSV data with a quote-aware state machine.
    pub const CSV_SCAN,
    name: "csv_scan",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/csv_scan.mir")),
    entry: "csv_scan",
    expected: || csv_scan(75_000),
    default_args: |_interp| vec![Value::int64(75_000)],
    tags: &["dispatch", "csv", "parser"],
    scales: &[
        scale_axis("len", 0, 75000, 250000, 1000000, true),
    ],
}

/// Compute the expected value for the csv scan benchmark.
fn csv_scan(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut field_len = 0i64;
    let mut fields = 0i64;
    let mut lines = 0i64;
    let mut in_quote = 0i64;

    // scan pseudo csv stream
    while index < length {
        let hash = index.wrapping_mul(CSV_HASH_MUL).wrapping_add(CSV_HASH_ADD);
        let token = hash % CSV_HASH_MOD;

        if token == CSV_TOKEN_QUOTE {
            in_quote = in_quote ^ 1;
            acc = acc.wrapping_add(1);
        } else if token == CSV_TOKEN_COMMA {
            if in_quote == 0 {
                fields = fields.wrapping_add(1);
                acc = acc.wrapping_add(field_len);
                field_len = 0;
            } else {
                field_len = field_len.wrapping_add(1);
            }
        } else if token == CSV_TOKEN_NEWLINE {
            if in_quote == 0 {
                lines = lines.wrapping_add(1);
                fields = fields.wrapping_add(1);
                acc = acc.wrapping_add(field_len);
                field_len = 0;
            } else {
                field_len = field_len.wrapping_add(1);
            }
        } else {
            field_len = field_len.wrapping_add(1);
            acc = acc.wrapping_add(token);
        }

        index = index.wrapping_add(1);
    }

    // flush trailing field
    if field_len != 0 {
        fields = fields.wrapping_add(1);
        acc = acc.wrapping_add(field_len);
    }

    // return result
    acc = acc.wrapping_add(fields).wrapping_add(lines);
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
