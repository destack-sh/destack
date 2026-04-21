use crate::{
    DestackFormatOptions, assert_format_program_idempotent, assert_format_program_reference_widths,
};
use destack_source::FileType;

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

/// Comments before type-argument calls should survive call formatting.
#[test]
fn test_format_type_argument_call_comments() {
    assert_format_program_reference_widths(
        r#"// Type arguments should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>()
const s = /* comment */ foo<A | B | C>()
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// Type arguments should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>();
const s = /* comment */ foo<A | B | C>();
"#,
            ),
            (
                100,
                r#"// Type arguments should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>();
const s = /* comment */ foo<A | B | C>();
"#,
            ),
        ],
    );
}

/// Hook dependency arrays should stay on their own argument line.
#[test]
fn test_format_hook_dependency_array_layout() {
    assert_format_program_reference_widths(
        r#"const handleFoo = useCallback((...args) => {
  onSubmit(...args);
  onClose();
}, [onSubmit, onClose]);
"#,
        FileType::JavaScript,
        &[
            (
                80,
                r#"const handleFoo = useCallback(
  (...args) => {
    onSubmit(...args);
    onClose();
  },
  [onSubmit, onClose],
);
"#,
            ),
            (
                100,
                r#"const handleFoo = useCallback(
  (...args) => {
    onSubmit(...args);
    onClose();
  },
  [onSubmit, onClose],
);
"#,
            ),
        ],
    );
}

/// Line comments before optional calls stay attached to the full call expression.
#[test]
fn test_format_optional_call_separator_line_comment() {
    assert_format_program_reference_widths(
        r#"const value = target // opt-call
?.()
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"const value = target?.(); // opt-call
"#,
            ),
            (
                100,
                r#"const value = target?.(); // opt-call
"#,
            ),
        ],
    );
}

/// Comments between the callee and `?.` should stay before the optional operator.
#[test]
fn test_format_optional_call_separator_block_comment() {
    assert_format_program_reference_widths(
        r#"alert /* comment */?.("value")
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"alert /* comment */?.("value");
"#,
            ),
            (
                100,
                r#"alert /* comment */?.("value");
"#,
            ),
        ],
    );
}

/// Block comments inside empty optional calls stay inside the argument list.
#[test]
fn test_format_optional_call_empty_argument_comment() {
    assert_format_program_reference_widths(
        r#"const value = call?.(/* argument comment */)
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"const value = call?.(/* argument comment */);
"#,
            ),
            (
                100,
                r#"const value = call?.(/* argument comment */);
"#,
            ),
        ],
    );
}

/// Line comments inside empty call arguments stay inside multiline parentheses.
#[test]
fn test_format_empty_call_line_comment_argument() {
    assert_format_program_reference_widths(
        r#"const value = call(
  // argument line comment
)
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"const value = call(
  // argument line comment
);
"#,
            ),
            (
                100,
                r#"const value = call(
  // argument line comment
);
"#,
            ),
        ],
    );
}

/// Comments between the callee and `(` move into the first argument position.
#[test]
fn test_format_call_callee_separator_comment_before_parentheses() {
    assert_format_program_reference_widths(
        r#"const value = run /* callee-note */ (first, second)
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"const value = run(/* callee-note */ first, second);
"#,
            ),
            (
                100,
                r#"const value = run(/* callee-note */ first, second);
"#,
            ),
        ],
    );
}

/// Member hop comments stay attached to the same chain hop.
#[test]
fn test_format_member_hop_inline_comments() {
    assert_format_program_reference_widths(
        r#"const value = source /* hop-a */ .first() /* hop-b */ .second()
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"const value = source /* hop-a */
  .first() /* hop-b */
  .second();
"#,
            ),
            (
                100,
                r#"const value = source /* hop-a */
  .first() /* hop-b */
  .second();
"#,
            ),
        ],
    );
}

/// Long standalone member expressions should break after the receiver when they cross the width.
#[test]
fn test_format_static_member_breaks_after_receiver() {
    assert_format_program_reference_widths(
        r#"expect(genCode(createVNodeCall(null, "`div`", mockProps))).toMatchInlineSnapshot
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"expect(genCode(createVNodeCall(null, "`div`", mockProps)))
  .toMatchInlineSnapshot;
"#,
            ),
            (
                100,
                r#"expect(genCode(createVNodeCall(null, "`div`", mockProps))).toMatchInlineSnapshot;
"#,
            ),
        ],
    );
}

/// Snapshot matcher calls should follow the shared width split behavior.
#[test]
fn test_format_inline_snapshot_matcher_call_width_behavior() {
    assert_format_program_reference_widths(
        r#"expect(genCode(createVNodeCall(null, "`div`", mockProps)))
  .toMatchInlineSnapshot(`
  `)
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"expect(genCode(createVNodeCall(null, "`div`", mockProps)))
  .toMatchInlineSnapshot(`
  `);
"#,
            ),
            (
                100,
                r#"expect(genCode(createVNodeCall(null, "`div`", mockProps))).toMatchInlineSnapshot(`
  `);
"#,
            ),
        ],
    );
}
