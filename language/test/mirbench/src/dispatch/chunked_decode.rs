use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const CHUNK_HASH_MUL: i64 = 1103515245;
const CHUNK_HASH_ADD: i64 = 12345;
const CHUNK_TOKEN_MASK: i64 = 15;
const CHUNK_TOKEN_DIGIT_MAX: i64 = 9;
const CHUNK_TOKEN_CR: i64 = 10;
const CHUNK_TOKEN_LF: i64 = 11;

const CHUNK_STATE_SIZE: i64 = 0;
const CHUNK_STATE_LF: i64 = 1;
const CHUNK_STATE_DATA: i64 = 2;
const CHUNK_STATE_CR: i64 = 3;
const CHUNK_STATE_END: i64 = 4;

declare_program! {
    /// Decode a pseudo chunked transfer stream.
    pub const CHUNKED_DECODE,
    name: "chunked_decode",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/chunked_decode.mir")),
    entry: "chunked_decode",
    expected: || chunked_decode(60_000),
    default_args: |_interp| vec![Value::int64(60_000)],
    tags: &["dispatch", "http", "chunked"],
    scales: &[
        scale_axis("len", 0, 60000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the chunked decode benchmark.
fn chunked_decode(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut state = CHUNK_STATE_SIZE;
    let mut size = 0i64;
    let mut remaining = 0i64;
    let mut chunks = 0i64;

    // run decode loop
    while index < length {
        let value = index
            .wrapping_mul(CHUNK_HASH_MUL)
            .wrapping_add(CHUNK_HASH_ADD);
        let token = (value >> 16) & CHUNK_TOKEN_MASK;

        match state {
            CHUNK_STATE_SIZE => {
                if token <= CHUNK_TOKEN_DIGIT_MAX {
                    size = size.wrapping_mul(10).wrapping_add(token);
                } else if token == CHUNK_TOKEN_CR {
                    state = CHUNK_STATE_LF;
                }
            }
            CHUNK_STATE_LF => {
                if token == CHUNK_TOKEN_LF {
                    remaining = size;
                    size = 0;
                    state = CHUNK_STATE_DATA;
                }
            }
            CHUNK_STATE_DATA => {
                if remaining > 0 {
                    acc = acc.wrapping_add(token);
                    remaining = remaining.wrapping_sub(1);
                } else {
                    state = CHUNK_STATE_CR;
                }
            }
            CHUNK_STATE_CR => {
                if token == CHUNK_TOKEN_CR {
                    state = CHUNK_STATE_END;
                }
            }
            _ => {
                if token == CHUNK_TOKEN_LF {
                    chunks = chunks.wrapping_add(1);
                    state = CHUNK_STATE_SIZE;
                }
            }
        }

        index = index.wrapping_add(1);
    }

    // return result
    acc = acc
        .wrapping_add(size)
        .wrapping_add(remaining)
        .wrapping_add(chunks);
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
