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
const TEMPLATE_TOKEN_SECTION_START: i64 = 2;
const TEMPLATE_TOKEN_SECTION_END: i64 = 3;
const TEMPLATE_TOKEN_ESCAPE: i64 = 4;

declare_program! {
    /// Render a tiny template stream with sections and variables.
    pub const TEMPLATE_RENDER,
    name: "template_render",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/template_render.mir")),
    entry: "template_render",
    expected: || template_render(70_000),
    default_args: |_interp| vec![Value::int64(70_000)],
    tags: &["dispatch", "template", "render"],
    scales: &[
        scale_axis("len", 0, 70000, 200000, 1000000, true),
    ],
}

/// Compute the expected value for the template render benchmark.
fn template_render(length: i64) -> Value {
    // init state
    let mut index = 0i64;
    let mut output = 0i64;
    let mut mode = TEMPLATE_MODE_TEXT;
    let mut depth = 0i64;
    let mut vars = 0i64;

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
            } else {
                output = output.wrapping_add(1);
            }
        } else {
            match token {
                TEMPLATE_TOKEN_VAR => {
                    vars = vars.wrapping_add(1);
                    let bump = depth.wrapping_add(3);
                    output = output.wrapping_add(bump);
                }
                TEMPLATE_TOKEN_SECTION_START => {
                    depth = depth.wrapping_add(1);
                    output = output.wrapping_add(2);
                }
                TEMPLATE_TOKEN_SECTION_END => {
                    if depth > 0 {
                        depth = depth.wrapping_sub(1);
                    }
                    output = output.wrapping_add(1);
                }
                TEMPLATE_TOKEN_ESCAPE => {
                    output = output.wrapping_add(1);
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
        .wrapping_add(vars)
        .wrapping_add(mode);
    let mixed = mix_result(output, length);
    Value::int64(mixed)
}
