use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const HEADER_HASH_MUL: i64 = 1664525;
const HEADER_HASH_ADD: i64 = 1013904223;
const HEADER_TOKEN_MASK: i64 = 15;

const HEADER_STATE_KEY: i64 = 0;
const HEADER_STATE_VALUE: i64 = 1;

const HEADER_TOKEN_NEWLINE: i64 = 0;
const HEADER_TOKEN_COLON: i64 = 1;
const HEADER_TOKEN_SPACE: i64 = 2;

declare_program! {
    /// Parse pseudo HTTP headers into key/value hashes.
    pub const HTTP_HEADER_PARSE,
    name: "http_header_parse",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/http_header_parse.mir")),
    entry: "http_header_parse",
    expected: || http_header_parse(60_000),
    default_args: |_interp| vec![Value::int64(60_000)],
    tags: &["dispatch", "http", "parser"],
    scales: &[
        scale_axis("len", 0, 60000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the HTTP header parser benchmark.
fn http_header_parse(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut key_hash = 0i64;
    let mut value_hash = 0i64;
    let mut state = HEADER_STATE_KEY;
    let mut headers = 0i64;

    // run parse loop
    while index < length {
        let value = index
            .wrapping_mul(HEADER_HASH_MUL)
            .wrapping_add(HEADER_HASH_ADD);
        let token = (value >> 16) & HEADER_TOKEN_MASK;

        match token {
            HEADER_TOKEN_NEWLINE => {
                acc = acc.wrapping_add(key_hash).wrapping_add(value_hash);
                headers = headers.wrapping_add(1);
                key_hash = 0;
                value_hash = 0;
                state = HEADER_STATE_KEY;
            }
            HEADER_TOKEN_COLON => {
                if state == HEADER_STATE_KEY {
                    state = HEADER_STATE_VALUE;
                }
            }
            HEADER_TOKEN_SPACE => {
                if state == HEADER_STATE_KEY {
                    key_hash = key_hash.wrapping_add(1);
                }
            }
            _ => {
                if state == HEADER_STATE_KEY {
                    key_hash = key_hash.wrapping_add(token);
                } else {
                    value_hash = value_hash.wrapping_add(token);
                }
            }
        }

        index = index.wrapping_add(1);
    }

    // return result
    acc = acc
        .wrapping_add(key_hash)
        .wrapping_add(value_hash)
        .wrapping_add(state)
        .wrapping_add(headers);
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
