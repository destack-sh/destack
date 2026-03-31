use crate::{
    DestackFormatOptions, assert_format, assert_format_program, assert_format_program_idempotent,
    assert_format_program_reference_widths,
};
use destack_source::FileType;

/// Simple named tree arguments should stay stable.
#[test]
fn test_format_argument_named() {
    assert_format!(
        r#"x: 1"#,
        r#"x: 1"#,
        |p| p.eat_tree_argument(),
        DestackFormatOptions::default()
    );
}

/// React-hook separator comment clusters should stay idempotent.
#[test]
fn test_format_react_hook_separator_comment_cluster_is_idempotent() {
    assert_format_program_idempotent!(
        r#"useEffect(
  () => {
    console.log("some code", props.foo);
  }

  ,
  // We need to disable the eslint warning here,
  // because of some complicated reason.
  // eslint-disable line react-hooks/exhaustive-deps
  []
)"#,
        FileType::JavaScriptXml,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Multiline JSX arguments should force expanded multi-argument call layout.
#[test]
fn test_format_multiline_jsx_argument_forces_expanded_call_layout() {
    assert_format_program!(
        r#"const view = fn(bar, <div>
  <span />
</div>)
"#,
        r#"const view = fn(
  bar,
  <div>
    <span />
  </div>,
);
"#,
        FileType::JavaScriptXml,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Comments before type-argument calls should survive speculative call formatting.
#[test]
fn test_format_type_argument_call_comment_seams() {
    assert_format_program_reference_widths(
        r#"// Type arguments with speculative formatting should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>()
const s = /* comment */ foo<A | B | C>()
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// Type arguments with speculative formatting should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>();
const s = /* comment */ foo<A | B | C>();
"#,
            ),
            (
                100,
                r#"// Type arguments with speculative formatting should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>();
const s = /* comment */ foo<A | B | C>();
"#,
            ),
        ],
    );
}
