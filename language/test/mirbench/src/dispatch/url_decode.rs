use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const URL_HASH_MUL: i64 = 1103515245;
const URL_HASH_ADD: i64 = 12345;
const URL_TOKEN_MASK: i64 = 31;
const URL_TOKEN_PERCENT: i64 = 16;
const URL_TOKEN_PLUS: i64 = 17;
const URL_HEX_MASK: i64 = 15;
const URL_HEX_SHIFT: i64 = 16;

declare_program! {
    /// Decode a pseudo URL stream with percent and plus handling.
    pub const URL_DECODE,
    name: "url_decode",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/url_decode.mir")),
    entry: "url_decode",
    expected: || url_decode(60_000),
    default_args: |_interp| vec![Value::int64(60_000)],
    tags: &["dispatch", "url", "parser"],
    scales: &[
        scale_axis("len", 0, 60000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the URL decode benchmark.
fn url_decode(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut pending = 0i64;
    let mut first_hex = 0i64;

    // run decode loop
    while index < length {
        let value = index.wrapping_mul(URL_HASH_MUL).wrapping_add(URL_HASH_ADD);
        let token = value & URL_TOKEN_MASK;

        if pending == 0 {
            if token == URL_TOKEN_PERCENT {
                pending = 1;
            } else if token == URL_TOKEN_PLUS {
                acc = acc.wrapping_add(32);
            } else {
                acc = acc.wrapping_add(token);
            }
        } else if pending == 1 {
            first_hex = token & URL_HEX_MASK;
            pending = 2;
        } else {
            let low = token & URL_HEX_MASK;
            let byte = first_hex.wrapping_mul(URL_HEX_SHIFT).wrapping_add(low);
            acc = acc.wrapping_add(byte);
            pending = 0;
        }

        index = index.wrapping_add(1);
    }

    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
