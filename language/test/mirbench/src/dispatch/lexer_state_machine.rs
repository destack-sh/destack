use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Lexer style state machine scanning synthetic tokens.
    pub const LEXER_STATE_MACHINE,
    name: "lexer_state_machine",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/lexer_state_machine.mir")),
    entry: "lexer_state_machine",
    expected: || lexer_state_machine(10_000, 4242),
    default_args: |_interp| vec![Value::int64(10_000), Value::int64(4242)],
    tags: &["dispatch", "lexer", "state"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the lexer state machine benchmark.
fn lexer_state_machine(iterations: i64, seed: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut state = 0i64;
    let mut tokens = 0i64;

    // run lexer loop
    while index < iterations {
        // compute next byte
        let mixed = index.wrapping_mul(13).wrapping_add(seed);
        let mut byte = mixed & 127;

        // bias token distribution
        let bias = byte & 3;
        if bias == 0 {
            byte = byte.wrapping_rem(26).wrapping_add(65);
        } else if bias == 1 {
            byte = byte.wrapping_rem(10).wrapping_add(48);
        }

        // advance state machine
        match state {
            0 => {
                let is_alpha = byte >= 65 && byte <= 90;
                if is_alpha {
                    state = 1;
                    tokens = tokens.wrapping_add(1);
                } else {
                    let is_digit = byte >= 48 && byte <= 57;
                    if is_digit {
                        state = 2;
                        tokens = tokens.wrapping_add(1);
                    } else if byte == 34 {
                        state = 3;
                    } else {
                        state = 0;
                    }
                }
            }
            1 => {
                let is_alpha = byte >= 65 && byte <= 90;
                let is_digit = byte >= 48 && byte <= 57;
                if is_alpha || is_digit {
                    state = 1;
                } else {
                    state = 0;
                }
            }
            2 => {
                let is_digit = byte >= 48 && byte <= 57;
                if is_digit {
                    state = 2;
                } else {
                    state = 0;
                }
            }
            _ => {
                if byte == 34 {
                    state = 0;
                } else {
                    state = 3;
                }
            }
        }

        // advance counter
        index = index.wrapping_add(1);
    }

    // return value
    Value::int64(tokens)
}
