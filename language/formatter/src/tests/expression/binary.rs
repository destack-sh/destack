use crate::{assert_format_program, assert_format_program_reference_widths};
use tspp_source::FileType;

/// Logical chains should place each operator before its continuation operand.
#[test]
fn test_format_logical_chain_indents_tail_operands() {
    assert_format_program_reference_widths(
        r#"a && b && c && d
"#,
        FileType::Tspp,
        &[(
            12,
            r#"a
  && b
  && c
  && d;
"#,
        )],
    );
}

/// Multiline operands should keep the operator on their continuation line.
#[test]
fn test_format_logical_expression_breaks_before_multiline_operand() {
    assert_format_program!(
        r#"isAfterStart && (match (this.endBound()) {
    { kind: "included", value: bound } => *value <= *bound
    { kind: "excluded", value: bound } => *value < *bound
    { kind: "unbounded" } => true
})
"#,
        r#"isAfterStart
    && (match (this.endBound()) {
        { kind: "included", value: bound } => *value <= *bound
        { kind: "excluded", value: bound } => *value < *bound
        { kind: "unbounded" } => true
    });
"#,
        FileType::Tspp,
    );
}

/// Multiline operands should keep the operator on their continuation line in block tails.
#[test]
fn test_format_logical_block_tail_breaks_before_multiline_operand() {
    assert_format_program!(
        r#"function contains(): boolean {
    isAfterStart && (match (this.endBound()) {
        { kind: "included", value: bound } => *value <= *bound
        { kind: "excluded", value: bound } => *value < *bound
        { kind: "unbounded" } => true
    })
}
"#,
        r#"function contains(): boolean {
    isAfterStart
        && (match (this.endBound()) {
            { kind: "included", value: bound } => *value <= *bound
            { kind: "excluded", value: bound } => *value < *bound
            { kind: "unbounded" } => true
        })
}
"#,
        FileType::Tspp,
    );
}

/// Arithmetic chains should place each operator before its continuation operand.
#[test]
fn test_format_arithmetic_chain_uses_leading_operators() {
    assert_format_program_reference_widths(
        r#"const determinant = this.x.x * (this.y.y * zw - this.y.z * yw + this.y.w * yz) - this.x.y * (this.y.x * zw - this.y.z * xw + this.y.w * xz) + this.x.z * (this.y.x * yw - this.y.y * xw + this.y.w * xy) - this.x.w * (this.y.x * yz - this.y.y * xz + this.y.z * xy)
"#,
        FileType::Tspp,
        &[(
            100,
            r#"const determinant = this.x.x * (this.y.y * zw - this.y.z * yw + this.y.w * yz)
  - this.x.y * (this.y.x * zw - this.y.z * xw + this.y.w * xz)
  + this.x.z * (this.y.x * yw - this.y.y * xw + this.y.w * xy)
  - this.x.w * (this.y.x * yz - this.y.y * xz + this.y.z * xy);
"#,
        )],
    );
}

/// Coalescing expressions should keep the fallback with its leading operator.
#[test]
fn test_format_coalescing_expression_uses_leading_operator() {
    assert_format_program_reference_widths(
        r#"return getContextValue<VeryLongContextValueName>(currentContext, variable.identifier) ?? panic("missing context variable")
"#,
        FileType::Tspp,
        &[(
            80,
            r#"return getContextValue<VeryLongContextValueName>(
  currentContext,
  variable.identifier,
)
  ?? panic("missing context variable");
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
        FileType::Tspp,
    );
}

/// Same-precedence operands should preserve associativity with minimal parentheses.
#[test]
fn test_format_binary_expression_parenthesizes_only_non_associative_positions() {
    assert_format_program!(
        r#"(a / b) * c;
a / (b * c);
a ** (b ** c);
(a ** b) ** c;
"#,
        r#"a / b * c;
a / (b * c);
a ** b ** c;
(a ** b) ** c;
"#,
        FileType::Tspp,
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
        FileType::Tspp,
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
        leftHandSideIsVeryLongAndKeepsGoingAndGoingAndGoing
            || anotherVeryLongThingThatKeepsGoingAndGoingAndGoing
            || thirdVeryLongThingThatKeepsGoingAndGoingAndGoing,
};
"#,
        FileType::Tspp,
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
        leftHandSideIsVeryLongAndKeepsGoingAndGoingAndGoing
            || anotherVeryLongThingThatKeepsGoingAndGoingAndGoing
            || thirdVeryLongThingThatKeepsGoingAndGoingAndGoing;
}
"#,
        FileType::Tspp,
    );
}

/// Ternary assignments should keep their test beside the assignment operator.
#[test]
fn test_format_logical_expression_in_ternary_assignment_stays_beside_equals() {
    assert_format_program!(
        r#"const value = (firstLongOperandThatForcesTheLogicalChainToBreak === null || secondLongOperandThatForcesTheLogicalChainToBreak === undefined || thirdLongOperandThatForcesTheLogicalChainToBreak === null ? undefined : fallbackValue)
"#,
        r#"const value = firstLongOperandThatForcesTheLogicalChainToBreak === null
    || secondLongOperandThatForcesTheLogicalChainToBreak === undefined
    || thirdLongOperandThatForcesTheLogicalChainToBreak === null
    ? undefined
    : fallbackValue;
"#,
        FileType::Tspp,
    );
}
