use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const PATH_STATE_EMPTY: i64 = 0;
const PATH_STATE_DOT: i64 = 1;
const PATH_STATE_DOTDOT: i64 = 2;
const PATH_STATE_NORMAL: i64 = 3;

const PATH_TOKEN_SLASH: i64 = 0;
const PATH_TOKEN_DOT_A: i64 = 1;
const PATH_TOKEN_DOT_B: i64 = 2;

const PATH_HASH_MUL: i64 = 1103515245;
const PATH_HASH_ADD: i64 = 12345;
const PATH_TOKEN_MOD: i64 = 6;

declare_program! {
    /// Normalize pseudo path segments with a small state machine.
    pub const PATH_NORMALIZE,
    name: "path_normalize",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/path_normalize.mir")),
    entry: "path_normalize",
    expected: || path_normalize(50_000),
    default_args: |_interp| vec![Value::int64(50_000)],
    tags: &["dispatch", "path", "parser"],
    scales: &[
        scale_axis("len", 0, 50000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the path normalization benchmark.
fn path_normalize(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut state = PATH_STATE_EMPTY;
    let mut depth = 0i64;

    // walk pseudo path bytes
    while index < length {
        let hash = index
            .wrapping_mul(PATH_HASH_MUL)
            .wrapping_add(PATH_HASH_ADD);
        let token = hash % PATH_TOKEN_MOD;

        match token {
            PATH_TOKEN_SLASH => {
                match state {
                    PATH_STATE_DOT => {
                        acc = acc.wrapping_add(1);
                    }
                    PATH_STATE_DOTDOT => {
                        if depth > 0 {
                            depth = depth.wrapping_sub(1);
                            acc = acc.wrapping_add(2);
                        } else {
                            acc = acc.wrapping_add(1);
                        }
                    }
                    PATH_STATE_NORMAL => {
                        depth = depth.wrapping_add(1);
                        acc = acc.wrapping_add(3);
                    }
                    _ => {}
                }

                state = PATH_STATE_EMPTY;
            }
            PATH_TOKEN_DOT_A | PATH_TOKEN_DOT_B => {
                if state == PATH_STATE_EMPTY {
                    state = PATH_STATE_DOT;
                } else if state == PATH_STATE_DOT {
                    state = PATH_STATE_DOTDOT;
                } else {
                    state = PATH_STATE_NORMAL;
                }

                acc = acc.wrapping_add(token);
            }
            _ => {
                state = PATH_STATE_NORMAL;
                acc = acc.wrapping_add(token);
            }
        }

        index = index.wrapping_add(1);
    }

    // finalize trailing segment
    match state {
        PATH_STATE_DOT => {
            acc = acc.wrapping_add(1);
        }
        PATH_STATE_DOTDOT => {
            if depth > 0 {
                depth = depth.wrapping_sub(1);
                acc = acc.wrapping_add(2);
            } else {
                acc = acc.wrapping_add(1);
            }
        }
        PATH_STATE_NORMAL => {
            depth = depth.wrapping_add(1);
            acc = acc.wrapping_add(3);
        }
        _ => {}
    }

    // return result
    acc = acc.wrapping_add(depth);
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
