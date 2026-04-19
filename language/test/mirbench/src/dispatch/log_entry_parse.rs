use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const LOG_HASH_MUL: i64 = 1664525;
const LOG_HASH_ADD: i64 = 1013904223;
const LOG_TOKEN_MASK: i64 = 7;

const LOG_STATE_TIMESTAMP: i64 = 0;
const LOG_STATE_LEVEL: i64 = 1;
const LOG_STATE_MESSAGE: i64 = 2;

const LOG_TOKEN_NEWLINE: i64 = 0;
const LOG_TOKEN_DIGIT_MIN: i64 = 1;
const LOG_TOKEN_DIGIT_MAX: i64 = 3;
const LOG_TOKEN_SPACE: i64 = 4;
const LOG_TOKEN_LBRACKET: i64 = 5;
const LOG_TOKEN_RBRACKET: i64 = 6;

declare_program! {
    /// Parse pseudo log entries into timestamp, level, and message fields.
    pub const LOG_ENTRY_PARSE,
    name: "log_entry_parse",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/log_entry_parse.mir")),
    entry: "log_entry_parse",
    expected: || log_entry_parse(80_000),
    default_args: |_interp| vec![Value::int64(80_000)],
    tags: &["dispatch", "log", "parser"],
    scales: &[
        scale_axis("len", 0, 80000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the log entry parser benchmark.
fn log_entry_parse(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut state = LOG_STATE_TIMESTAMP;
    let mut digits = 0i64;
    let mut level_hash = 0i64;
    let mut message_hash = 0i64;
    let mut entries = 0i64;

    // run parse loop
    while index < length {
        let value = index.wrapping_mul(LOG_HASH_MUL).wrapping_add(LOG_HASH_ADD);
        let token = (value >> 16) & LOG_TOKEN_MASK;

        match token {
            LOG_TOKEN_NEWLINE => {
                acc = acc
                    .wrapping_add(digits)
                    .wrapping_add(level_hash)
                    .wrapping_add(message_hash);
                entries = entries.wrapping_add(1);
                digits = 0;
                level_hash = 0;
                message_hash = 0;
                state = LOG_STATE_TIMESTAMP;
            }
            LOG_TOKEN_DIGIT_MIN..=LOG_TOKEN_DIGIT_MAX => {
                if state == LOG_STATE_TIMESTAMP {
                    digits = digits.wrapping_add(1);
                    acc = acc.wrapping_add(token);
                } else if state == LOG_STATE_MESSAGE {
                    message_hash = message_hash.wrapping_add(token);
                }
            }
            LOG_TOKEN_SPACE => {
                if state == LOG_STATE_TIMESTAMP {
                    state = LOG_STATE_LEVEL;
                } else if state == LOG_STATE_LEVEL {
                    state = LOG_STATE_MESSAGE;
                } else {
                    message_hash = message_hash ^ token;
                }
            }
            LOG_TOKEN_LBRACKET => {
                if state == LOG_STATE_LEVEL {
                    level_hash = level_hash.wrapping_add(1);
                }
            }
            LOG_TOKEN_RBRACKET => {
                if state == LOG_STATE_LEVEL {
                    level_hash = level_hash.wrapping_add(2);
                    state = LOG_STATE_MESSAGE;
                }
            }
            _ => {
                if state == LOG_STATE_LEVEL {
                    level_hash = level_hash ^ token;
                } else if state == LOG_STATE_MESSAGE {
                    message_hash = message_hash.wrapping_add(token);
                }
            }
        }

        index = index.wrapping_add(1);
    }

    // return result
    acc = acc
        .wrapping_add(digits)
        .wrapping_add(level_hash)
        .wrapping_add(message_hash)
        .wrapping_add(entries);
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
