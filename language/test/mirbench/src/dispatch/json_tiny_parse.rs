use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const JSON_HASH_MUL: i64 = 1103515245;
const JSON_HASH_ADD: i64 = 12345;
const JSON_TOKEN_MASK: i64 = 15;

const JSON_TOKEN_LBRACE: i64 = 0;
const JSON_TOKEN_RBRACE: i64 = 1;
const JSON_TOKEN_LBRACKET: i64 = 2;
const JSON_TOKEN_RBRACKET: i64 = 3;
const JSON_TOKEN_QUOTE: i64 = 4;
const JSON_TOKEN_ESCAPE: i64 = 5;
const JSON_TOKEN_DIGIT_MIN: i64 = 6;
const JSON_TOKEN_DIGIT_MAX: i64 = 9;
const JSON_TOKEN_COMMA: i64 = 10;
const JSON_TOKEN_COLON: i64 = 11;
const JSON_TOKEN_SPACE: i64 = 12;

declare_program! {
    /// Parse a tiny JSON-like stream with strings, numbers, and nesting.
    pub const JSON_TINY_PARSE,
    name: "json_tiny_parse",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/json_tiny_parse.mir")),
    entry: "json_tiny_parse",
    expected: || json_tiny_parse(40_000),
    default_args: |_interp| vec![Value::int64(40_000)],
    tags: &["dispatch", "json", "parser"],
    scales: &[
        scale_axis("len", 0, 40000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the tiny JSON parser benchmark.
fn json_tiny_parse(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut depth = 0i64;
    let mut in_string = 0i64;
    let mut escape = 0i64;
    let mut number = 0i64;

    // run parse loop
    while index < length {
        let value = index
            .wrapping_mul(JSON_HASH_MUL)
            .wrapping_add(JSON_HASH_ADD);
        let token = value & JSON_TOKEN_MASK;

        if in_string == 1 {
            if escape == 1 {
                acc = acc.wrapping_add(token);
                escape = 0;
            } else if token == JSON_TOKEN_ESCAPE {
                acc = acc.wrapping_add(1);
                escape = 1;
            } else if token == JSON_TOKEN_QUOTE {
                acc = acc.wrapping_add(2);
                in_string = 0;
            } else {
                acc = acc.wrapping_add(token);
            }
        } else if (JSON_TOKEN_DIGIT_MIN..=JSON_TOKEN_DIGIT_MAX).contains(&token) {
            let digit = token.wrapping_sub(JSON_TOKEN_DIGIT_MIN);
            number = number.wrapping_mul(10).wrapping_add(digit);
        } else {
            if number != 0 {
                acc = acc.wrapping_add(number);
                number = 0;
            }

            match token {
                JSON_TOKEN_LBRACE | JSON_TOKEN_LBRACKET => {
                    depth = depth.wrapping_add(1);
                    acc = acc.wrapping_add(depth);
                }
                JSON_TOKEN_RBRACE | JSON_TOKEN_RBRACKET => {
                    if depth > 0 {
                        depth = depth.wrapping_sub(1);
                    }
                    acc = acc.wrapping_add(depth);
                }
                JSON_TOKEN_QUOTE => {
                    acc = acc.wrapping_add(1);
                    in_string = 1;
                }
                JSON_TOKEN_COMMA | JSON_TOKEN_COLON | JSON_TOKEN_SPACE => {}
                _ => {
                    acc = acc ^ token;
                }
            }
        }

        index = index.wrapping_add(1);
    }

    // return result
    acc = acc
        .wrapping_add(depth)
        .wrapping_add(in_string)
        .wrapping_add(escape)
        .wrapping_add(number);
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
