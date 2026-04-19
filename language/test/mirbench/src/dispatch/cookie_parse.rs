use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const COOKIE_HASH_MUL: i64 = 1664525;
const COOKIE_HASH_ADD: i64 = 1013904223;
const COOKIE_TOKEN_MASK: i64 = 7;

const COOKIE_MODE_KEY: i64 = 0;
const COOKIE_MODE_VALUE: i64 = 1;
const COOKIE_MODE_SPACE: i64 = 2;

const COOKIE_TOKEN_EQ: i64 = 0;
const COOKIE_TOKEN_SEMI: i64 = 1;
const COOKIE_TOKEN_SPACE: i64 = 2;

declare_program! {
    /// Parse a cookie header stream with key value segments.
    pub const COOKIE_PARSE,
    name: "cookie_parse",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/cookie_parse.mir")),
    entry: "cookie_parse",
    expected: || cookie_parse(90_000),
    default_args: |_interp| vec![Value::int64(90_000)],
    tags: &["dispatch", "cookie", "parse"],
    scales: &[
        scale_axis("len", 0, 90000, 250000, 1000000, true),
    ],
}

/// Compute the expected value for the cookie parse benchmark.
fn cookie_parse(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut key_len = 0i64;
    let mut val_len = 0i64;
    let mut pairs = 0i64;
    let mut mode = COOKIE_MODE_KEY;
    let mut acc = 0i64;

    // parse token stream
    while index < length {
        let token = index
            .wrapping_mul(COOKIE_HASH_MUL)
            .wrapping_add(COOKIE_HASH_ADD)
            .wrapping_shr(16)
            & COOKIE_TOKEN_MASK;

        if token == COOKIE_TOKEN_EQ {
            if mode == COOKIE_MODE_KEY {
                mode = COOKIE_MODE_VALUE;
            } else {
                acc = acc.wrapping_add(1);
            }
        } else if token == COOKIE_TOKEN_SEMI {
            pairs = pairs.wrapping_add(1);
            acc = acc.wrapping_add(key_len).wrapping_add(val_len);
            key_len = 0;
            val_len = 0;
            mode = COOKIE_MODE_KEY;
        } else if token == COOKIE_TOKEN_SPACE {
            if mode == COOKIE_MODE_KEY {
                mode = COOKIE_MODE_SPACE;
            } else if mode == COOKIE_MODE_VALUE {
                val_len = val_len.wrapping_add(1);
            }
        } else if mode == COOKIE_MODE_VALUE {
            val_len = val_len.wrapping_add(token);
        } else {
            key_len = key_len.wrapping_add(token);
            if mode == COOKIE_MODE_SPACE {
                mode = COOKIE_MODE_KEY;
            }
        }

        index = index.wrapping_add(1);
    }

    // return result
    acc = acc
        .wrapping_add(key_len)
        .wrapping_add(val_len)
        .wrapping_add(pairs)
        .wrapping_add(mode);
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
