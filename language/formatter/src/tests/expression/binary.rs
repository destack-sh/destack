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

/// Mixed bitwise precedence should add grouping parentheses for the lower precedence parent.
#[test]
fn test_format_binary_expression_keeps_mixed_bitwise_precedence_explicit() {
    assert_format_program!(
        r#"flags & mask | other
"#,
        r#"(flags & mask) | other;
"#,
        destack_source::FileType::Destack,
    );
}

/// Object property values should own the outer indent for long logical chains.
#[test]
fn test_format_logical_expression_in_object_property_breaks_after_colon() {
    assert_format_program!(
        r#"const value = {
  field: leftHandSideIsVeryLongAndKeepsGoingAndGoingAndGoing || anotherVeryLongThingThatKeepsGoingAndGoingAndGoing || thirdVeryLongThingThatKeepsGoingAndGoingAndGoing
}
"#,
        r#"const value = {
    field:
        leftHandSideIsVeryLongAndKeepsGoingAndGoingAndGoing ||
        anotherVeryLongThingThatKeepsGoingAndGoingAndGoing ||
        thirdVeryLongThingThatKeepsGoingAndGoingAndGoing,
};
"#,
        destack_source::FileType::TypeScript,
    );
}

/// Class field initializers should own the outer indent for long logical chains.
#[test]
fn test_format_logical_expression_in_class_field_initializer_breaks_after_equals() {
    assert_format_program!(
        r#"class Example {
  field = leftHandSideIsVeryLongAndKeepsGoingAndGoingAndGoing || anotherVeryLongThingThatKeepsGoingAndGoingAndGoing || thirdVeryLongThingThatKeepsGoingAndGoingAndGoing
}
"#,
        r#"class Example {
    field =
        leftHandSideIsVeryLongAndKeepsGoingAndGoingAndGoing ||
        anotherVeryLongThingThatKeepsGoingAndGoingAndGoing ||
        thirdVeryLongThingThatKeepsGoingAndGoingAndGoing;
}
"#,
        destack_source::FileType::TypeScript,
    );
}

/// Ternary tests should keep the normal logical-chain tail indent.
#[test]
fn test_format_logical_expression_in_ternary_test_indents_tail_operands() {
    assert_format_program!(
        r#"const value = (firstLongOperandThatForcesTheLogicalChainToBreak === null || secondLongOperandThatForcesTheLogicalChainToBreak === void 0 || thirdLongOperandThatForcesTheLogicalChainToBreak === null ? void 0 : fallbackValue)
"#,
        r#"const value =
    firstLongOperandThatForcesTheLogicalChainToBreak === null ||
    secondLongOperandThatForcesTheLogicalChainToBreak === void 0 ||
    thirdLongOperandThatForcesTheLogicalChainToBreak === null
        ? void 0
        : fallbackValue;
"#,
        destack_source::FileType::TypeScript,
    );
}
