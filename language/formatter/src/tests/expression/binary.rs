use crate::{assert_format_program, assert_format_program_reference_widths};

/// Logical chains should indent continuation operands under the head.
#[test]
fn test_format_logical_chain_indents_tail_operands() {
    assert_format_program_reference_widths(
        r#"a && b && c && d
"#,
        destack_source::FileType::Destack,
        &[(
            12,
            r#"a &&
  b &&
  c &&
  d;
"#,
        )],
    );
}

/// Binary operands should drop redundant grouped wrappers when precedence already preserves meaning.
#[test]
fn test_format_binary_expression_drops_redundant_grouping_parentheses() {
    assert_format_program!(
        r#"(a + b * c) && (d - e / f)
"#,
        r#"a + b * c && d - e / f;
"#,
        destack_source::FileType::Destack,
    );
}

/// Mixed bitwise precedence should not gain redundant grouping parentheses.
#[test]
fn test_format_binary_expression_keeps_mixed_bitwise_precedence_without_extra_grouping() {
    assert_format_program!(
        r#"flags & mask | other
"#,
        r#"flags & mask | other;
"#,
        destack_source::FileType::Destack,
    );
}

/// Short logical rhs values can stay inline when the whole cast-headed expression still fits.
#[test]
fn test_format_logical_expression_keeps_short_tail_after_multiline_type_cast_head() {
    assert_format_program_reference_widths(
        r#"const sourcemap =
  /** @type {'inline' | 'hidden' | 'sourcemap'} */ (
      process.env.WORKER_MODE
    ) || sourcemap
"#,
        destack_source::FileType::TypeScript,
        &[(
            100,
            r#"const sourcemap =
  /** @type {'inline' | 'hidden' | 'sourcemap'} */ (process.env.WORKER_MODE) || sourcemap;
"#,
        )],
    );
}
