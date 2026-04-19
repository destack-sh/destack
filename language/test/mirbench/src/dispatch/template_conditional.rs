use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

const TEMPLATE_HASH_MUL: i64 = 1103515245;
const TEMPLATE_HASH_ADD: i64 = 12345;
const TEMPLATE_TOKEN_MASK: i64 = 7;

const TEMPLATE_MODE_TEXT: i64 = 0;
const TEMPLATE_MODE_TAG: i64 = 1;

const TEMPLATE_TOKEN_TAG_START: i64 = 0;
const TEMPLATE_TOKEN_VAR: i64 = 1;
const TEMPLATE_TOKEN_IF: i64 = 2;
const TEMPLATE_TOKEN_ELSE: i64 = 3;
const TEMPLATE_TOKEN_END: i64 = 4;
const TEMPLATE_TOKEN_ESCAPE: i64 = 5;

declare_program! {
    /// Render a template stream with conditional sections.
    pub const TEMPLATE_CONDITIONAL,
    name: "template_conditional",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/template_conditional.mir")),
    entry: "template_conditional",
    expected: || template_conditional(70_000),
    default_args: |_interp| vec![Value::int64(70_000)],
    tags: &["dispatch", "template", "conditional"],
    scales: &[
        scale_axis("len", 0, 70000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the template conditional benchmark.
fn template_conditional(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut output = 0i64;
    let mut mode = TEMPLATE_MODE_TEXT;
    let mut depth = 0i64;
    let mut enabled = 1i64;

    // run render loop
    while index < length {
        let value = index
            .wrapping_mul(TEMPLATE_HASH_MUL)
            .wrapping_add(TEMPLATE_HASH_ADD);
        let token = value & TEMPLATE_TOKEN_MASK;

        if mode == TEMPLATE_MODE_TEXT {
            if token == TEMPLATE_TOKEN_TAG_START {
                output = output.wrapping_add(1);
                mode = TEMPLATE_MODE_TAG;
            } else if enabled == 1 {
                output = output.wrapping_add(1);
            }
        } else {
            match token {
                TEMPLATE_TOKEN_VAR => {
                    if enabled == 1 {
                        output = output.wrapping_add(3);
                    }
                }
                TEMPLATE_TOKEN_IF => {
                    depth = depth.wrapping_add(1);
                    output = output.wrapping_add(1);
                    enabled = if depth % 2 == 0 { 1 } else { 0 };
                }
                TEMPLATE_TOKEN_ELSE => {
                    output = output.wrapping_add(1);
                    enabled = 1 - enabled;
                }
                TEMPLATE_TOKEN_END => {
                    output = output.wrapping_add(1);
                    if depth > 0 {
                        depth = depth.wrapping_sub(1);
                    }
                    enabled = 1;
                }
                TEMPLATE_TOKEN_ESCAPE => {
                    if enabled == 1 {
                        output = output.wrapping_add(1);
                    }
                }
                _ => {}
            }

            mode = TEMPLATE_MODE_TEXT;
        }

        index = index.wrapping_add(1);
    }

    // return result
    output = output
        .wrapping_add(depth)
        .wrapping_add(enabled)
        .wrapping_add(mode);
    let mixed = mix_result(output, length);
    Value::int64(mixed)
}
