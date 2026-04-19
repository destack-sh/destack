use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Stateful token parser with a small DFA.
    pub const TOKEN_PARSER,
    name: "token_parser",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/token_parser.mir")),
    entry: "token_parser",
    expected: || token_parser(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "state", "parser"],
    scales: &[
        scale_axis("len", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the token parser benchmark.
fn token_parser(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut acc = 0i64;
    let mut state = 0i64;

    // run parser loop
    while index < length {
        let value = index.wrapping_mul(1103515245).wrapping_add(12345);
        let token = value % 5;

        match state {
            0 => match token {
                0 => {
                    acc = acc.wrapping_add(1);
                    state = 1;
                }
                1 => {
                    acc = acc.wrapping_add(token);
                    state = 2;
                }
                2 => {
                    acc = acc.wrapping_add(index);
                    state = 0;
                }
                3 => {
                    acc = acc ^ token;
                    state = 1;
                }
                4 => {
                    acc = acc.wrapping_add(2);
                    state = 0;
                }
                _ => {
                    acc = acc.wrapping_add(2);
                    state = 0;
                }
            },
            1 => match token {
                0 => {
                    acc = acc.wrapping_add(3);
                    state = 2;
                }
                1 => {
                    acc = acc.wrapping_add(index);
                    state = 1;
                }
                2 => {
                    acc = acc.wrapping_add(token.wrapping_mul(2));
                    state = 0;
                }
                _ => {
                    acc = acc.wrapping_sub(1);
                    state = 1;
                }
            },
            _ => match token {
                0 => {
                    acc = acc.wrapping_add(token);
                    state = 0;
                }
                1 => {
                    acc = acc.wrapping_add(5);
                    state = 2;
                }
                2 => {
                    acc = acc ^ index;
                    state = 1;
                }
                _ => {
                    acc = acc.wrapping_add(2);
                    state = 0;
                }
            },
        }

        index = index.wrapping_add(1);
    }

    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
