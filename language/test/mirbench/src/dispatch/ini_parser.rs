use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const INI_HASH_MUL: i64 = 1103515245;
const INI_HASH_ADD: i64 = 12345;
const INI_TOKEN_MASK: i64 = 15;

const INI_STATE_IDLE: i64 = 0;
const INI_STATE_SECTION: i64 = 1;
const INI_STATE_KEY: i64 = 2;
const INI_STATE_VALUE: i64 = 3;
const INI_STATE_COMMENT: i64 = 4;

const INI_TOKEN_NEWLINE: i64 = 0;
const INI_TOKEN_LBRACKET: i64 = 1;
const INI_TOKEN_RBRACKET: i64 = 2;
const INI_TOKEN_EQUALS: i64 = 3;
const INI_TOKEN_COMMENT: i64 = 4;
const INI_TOKEN_WHITESPACE: i64 = 13;

declare_program! {
    /// Parse a pseudo INI stream with sections, keys, and values.
    pub const INI_PARSER,
    name: "ini_parser",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/ini_parser.mir")),
    entry: "ini_parser",
    expected: || ini_parser(60_000),
    default_args: |_interp| vec![Value::int64(60_000)],
    tags: &["dispatch", "ini", "parser"],
    scales: &[
        scale_axis("len", 0, 60000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the INI parser benchmark.
fn ini_parser(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut state = INI_STATE_IDLE;
    let mut sections = 0i64;
    let mut keys = 0i64;
    let mut values = 0i64;
    let mut comments = 0i64;

    // run parse loop
    while index < length {
        let value = index.wrapping_mul(INI_HASH_MUL).wrapping_add(INI_HASH_ADD);
        let token = value & INI_TOKEN_MASK;

        if state == INI_STATE_COMMENT {
            if token == INI_TOKEN_NEWLINE {
                comments = comments.wrapping_add(1);
                state = INI_STATE_IDLE;
            }
        } else {
            match token {
                INI_TOKEN_NEWLINE => {
                    if state == INI_STATE_VALUE {
                        values = values.wrapping_add(1);
                    } else if state == INI_STATE_KEY {
                        keys = keys.wrapping_add(1);
                    } else if state == INI_STATE_SECTION {
                        sections = sections.wrapping_add(1);
                    }
                    state = INI_STATE_IDLE;
                }
                INI_TOKEN_LBRACKET => {
                    if state == INI_STATE_IDLE {
                        state = INI_STATE_SECTION;
                    }
                }
                INI_TOKEN_RBRACKET => {
                    if state == INI_STATE_SECTION {
                        sections = sections.wrapping_add(1);
                        state = INI_STATE_IDLE;
                    }
                }
                INI_TOKEN_EQUALS => {
                    if state == INI_STATE_KEY {
                        state = INI_STATE_VALUE;
                    }
                }
                INI_TOKEN_COMMENT => {
                    state = INI_STATE_COMMENT;
                }
                INI_TOKEN_WHITESPACE => {}
                _ => {
                    acc = acc.wrapping_add(token);
                    if state == INI_STATE_IDLE {
                        state = INI_STATE_KEY;
                    }
                }
            }
        }

        index = index.wrapping_add(1);
    }

    // return result
    acc = acc
        .wrapping_add(sections)
        .wrapping_add(keys)
        .wrapping_add(values)
        .wrapping_add(comments)
        .wrapping_add(state);
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
