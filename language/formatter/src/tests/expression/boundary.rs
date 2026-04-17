use crate::{
    DestackFormatOptions, ParenthesizedExpressionView, TestFormatter,
    assert_format_program_roundtrip_with_file_type,
};
use destack_ast::{CommentKind, Expression};
use destack_parser::ParserOptions;
use destack_source::FileType;

/// Computed member boundary comments should stay on the receiver.
#[test]
fn test_format_computed_member_boundary_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = source /* before-index */ [key]\n",
        "const value = source /* before-index */[key];\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Template interpolation member comments should stay before the member continuation.
#[test]
fn test_format_template_member_boundary_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = `${source /* member-note */ .name}`\n",
        "const value = `${source /* member-note */.name}`;\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Unary boundary block comments should stay inside the grouped operand.
#[test]
fn test_format_unary_negative_boundary_block_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = -/* unary-note */ 1\n",
        "const value = -(/* unary-note */ 1);\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Explicit parenthesized scalar comment wrappers should stay inline.
#[test]
fn test_format_parenthesized_scalar_boundary_block_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = -(/* unary-note */ 1)\n",
        "const value = -(/* unary-note */ 1);\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Explicit parenthesized scalar line comment wrappers should stay multiline.
#[test]
fn test_format_parenthesized_scalar_boundary_line_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = -(\n    // unary-line-note\n    1\n)\n",
        "const value = -(\n    // unary-line-note\n    1\n);\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Unary boundary line comments should keep the multiline grouped operand.
#[test]
fn test_format_unary_negative_boundary_line_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = -// unary-line-note\n1\n",
        "const value = -(\n    // unary-line-note\n    1\n);\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Unary operand line comments should stay visible to the formatter context.
#[test]
fn test_unary_negative_boundary_line_comment_stays_on_operand_owner() {
    let (formatter, expression_id) = TestFormatter::parse_with_file_type(
        "const value = -// unary-line-note\n1\n",
        FileType::TypeScript,
        |parser| parser.eat_expression(ParserOptions::default()),
    )
    .expect("parse unary declarator initializer");
    let context = formatter.context(DestackFormatOptions::default_with_line_width(100));

    let Expression::Let { declarators, .. } = context.tree.get(expression_id) else {
        panic!("expected let expression");
    };
    let value_id = context
        .tree
        .get(declarators[0])
        .value
        .expect("expected value");
    let Expression::Unary { right, .. } = context.tree.get(value_id) else {
        panic!("expected unary value");
    };
    let right_prefix_comments = context.raw_prefix_comments_for(*right);
    assert_eq!(right_prefix_comments.len(), 1);
    assert_eq!(right_prefix_comments[0].kind, CommentKind::Line);
}

/// Preserved parenthesized wrappers should expose leading inner line comments.
#[test]
fn test_parenthesized_scalar_line_comment_view_finds_leading_inner_comment() {
    let (formatter, expression_id) = TestFormatter::parse_with_file_type(
        "(\n    // unary-line-note\n    1\n)\n",
        FileType::TypeScript,
        |parser| parser.eat_expression(ParserOptions::default()),
    )
    .expect("parse parenthesized scalar");
    let context = formatter.context(DestackFormatOptions::default_with_line_width(100));
    let view = ParenthesizedExpressionView::from_node(&context, expression_id)
        .expect("expected parenthesized view");

    let leading_inner_comments = view.leading_inner_comments();
    assert_eq!(leading_inner_comments.len(), 1);
    assert_eq!(leading_inner_comments[0].kind, CommentKind::Line);
}

/// Preserved parenthesized wrappers should expose trailing inner block comments.
#[test]
fn test_parenthesized_scalar_block_comment_view_finds_trailing_inner_comment() {
    let (formatter, expression_id) =
        TestFormatter::parse_with_file_type("(1 /* keep */)\n", FileType::TypeScript, |parser| {
            parser.eat_expression(ParserOptions::default())
        })
        .expect("parse parenthesized scalar");
    let context = formatter.context(DestackFormatOptions::default_with_line_width(100));
    let view = ParenthesizedExpressionView::from_node(&context, expression_id)
        .expect("expected parenthesized view");

    let trailing_inner_comments = view.trailing_inner_comments();
    assert_eq!(trailing_inner_comments.len(), 1);
    assert_eq!(
        trailing_inner_comments[0].kind,
        CommentKind::SingleLineBlock
    );
}

/// Label boundary line comments should stay above the labeled statement.
#[test]
fn test_format_label_boundary_line_comments() {
    assert_format_program_roundtrip_with_file_type(
        "start: // label-tail\nwhile (true) {\n  break start\n}\n",
        "// label-tail\nstart: while (true) {\n    break start;\n}\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Ternary block comments before the alternate should stay with the alternate branch.
#[test]
fn test_format_ternary_alternate_block_boundary_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const x = condition ? /* then */ valueA : /* else */ valueB\n",
        "const x = condition ? /* then */ valueA : /* else */ valueB;\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Ternary line comments before the alternate should stay on the then branch boundary.
#[test]
fn test_format_ternary_alternate_line_boundary_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const x = condition ? valueA : // else-note\nvalueB\n",
        "const x = condition\n    ? valueA // else-note\n    : valueB;\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}
