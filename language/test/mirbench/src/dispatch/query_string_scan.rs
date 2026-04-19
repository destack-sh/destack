use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const QUERY_HASH_MUL: i64 = 1664525;
const QUERY_HASH_ADD: i64 = 1013904223;
const QUERY_HASH_MOD: i64 = 6;

const QUERY_TOKEN_AMP: i64 = 0;
const QUERY_TOKEN_EQ: i64 = 1;

const QUERY_MODE_KEY: i64 = 0;
const QUERY_MODE_VALUE: i64 = 1;

declare_program! {
    /// Scan pseudo query strings with key/value separators.
    pub const QUERY_STRING_SCAN,
    name: "query_string_scan",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/query_string_scan.mir")),
    entry: "query_string_scan",
    expected: || query_string_scan(60_000),
    default_args: |_interp| vec![Value::int64(60_000)],
    tags: &["dispatch", "query", "parser"],
    scales: &[
        scale_axis("len", 0, 60000, 200000, 800000, true),
    ],
}

/// Compute the expected value for the query string scan benchmark.
fn query_string_scan(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut key_len = 0i64;
    let mut val_len = 0i64;
    let mut mode = QUERY_MODE_KEY;
    let mut pairs = 0i64;

    // scan pseudo query stream
    while index < length {
        let hash = index
            .wrapping_mul(QUERY_HASH_MUL)
            .wrapping_add(QUERY_HASH_ADD);
        let token = hash % QUERY_HASH_MOD;

        if token == QUERY_TOKEN_AMP {
            if mode == QUERY_MODE_KEY {
                if key_len != 0 {
                    acc = acc.wrapping_add(key_len);
                    pairs = pairs.wrapping_add(1);
                    key_len = 0;
                    val_len = 0;
                    mode = QUERY_MODE_KEY;
                }
            } else {
                acc = acc.wrapping_add(key_len).wrapping_add(val_len);
                pairs = pairs.wrapping_add(1);
                key_len = 0;
                val_len = 0;
                mode = QUERY_MODE_KEY;
            }
        } else if token == QUERY_TOKEN_EQ {
            if mode == QUERY_MODE_KEY {
                acc = acc.wrapping_add(1);
                mode = QUERY_MODE_VALUE;
            } else {
                val_len = val_len.wrapping_add(1);
            }
        } else if mode == QUERY_MODE_KEY {
            key_len = key_len.wrapping_add(1);
            acc = acc.wrapping_add(token);
        } else {
            val_len = val_len.wrapping_add(1);
            acc = acc.wrapping_add(token);
        }

        index = index.wrapping_add(1);
    }

    // finalize trailing key/value
    if mode == QUERY_MODE_KEY {
        if key_len != 0 {
            acc = acc.wrapping_add(key_len);
            pairs = pairs.wrapping_add(1);
        }
    } else {
        acc = acc.wrapping_add(key_len).wrapping_add(val_len);
        pairs = pairs.wrapping_add(1);
    }

    // return result
    acc = acc.wrapping_add(pairs);
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
