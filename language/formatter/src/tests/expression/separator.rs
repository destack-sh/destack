use crate::format::expression::expression_needs_parentheses_in_parent;
use crate::{DestackFormatOptions, TestFormatter, assert_format_program_roundtrip_with_file_type};
use destack_ast::{
    CommentKind, CommentPosition, Declaration, Declarator, Expression, TypeExpression,
};
use destack_source::FileType;

/// Computed member separator comments should stay on the receiver.
#[test]
fn test_format_computed_member_separator_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = source /* before-index */ [key]\n",
        "const value = source /* before-index */[key];\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Template interpolation member comments should stay before the member continuation.
#[test]
fn test_format_template_member_separator_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = `${source /* member-note */ .name}`\n",
        "const value = `${source /* member-note */.name}`;\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Unary separator block comments should stay inside the grouped operand.
#[test]
fn test_format_unary_negative_separator_block_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = -/* unary-note */ 1\n",
        "const value = -(/* unary-note */ 1);\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Unary separator comments should stay visible before the operand token.
#[test]
fn test_unary_negative_separator_comment_attaches_before_operand_token() {
    let input = "-/* unary-note */ 1";
    let comment_start = input.find("/* unary-note */").unwrap() as u32;
    let comment_end = comment_start + "/* unary-note */".len() as u32;
    let literal_start = input.find('1').unwrap() as u32;
    let (test, expression_id) =
        TestFormatter::parse_with_file_type(input, FileType::TypeScript, |parser| {
            parser.eat_expression(Default::default())
        })
        .unwrap();
    let context = test.context(DestackFormatOptions::default_with_line_width(100));

    let Expression::Unary { right, .. } = context.tree.get(expression_id) else {
        panic!("expected unary expression");
    };

    let operand_token_start = context.expression_token_start(*right);
    let operand_leading_comments = context.comments().comments_before(operand_token_start);

    assert_eq!(operand_token_start, literal_start);
    assert_eq!(operand_leading_comments.len(), 1);
    assert_eq!(operand_leading_comments[0].span.start, comment_start);
    assert_eq!(operand_leading_comments[0].span.end, comment_end);
    assert!(context.comments().has_comment_before(operand_token_start));
}

/// Unary separator comments inside initializers should stay visible before the operand token.
#[test]
fn test_unary_negative_initializer_separator_comment_attaches_before_operand_token() {
    let input = "const value = -/* unary-note */ 1";
    let comment_start = input.find("/* unary-note */").unwrap() as u32;
    let comment_end = comment_start + "/* unary-note */".len() as u32;
    let literal_start = input.rfind('1').unwrap() as u32;
    let (test, expression_id) =
        TestFormatter::parse_with_file_type(input, FileType::TypeScript, |parser| {
            parser.eat_expression(Default::default())
        })
        .unwrap();
    let context = test.context(DestackFormatOptions::default_with_line_width(100));

    let Expression::Let { declarators, .. } = context.tree.get(expression_id) else {
        panic!("expected let expression");
    };
    let Declarator { value, .. } = context.tree.get(declarators[0]);
    let value_id = value.expect("expected initializer");
    let Expression::Unary { right, .. } = context.tree.get(value_id) else {
        panic!("expected unary initializer");
    };

    let operand_token_start = context.expression_token_start(*right);
    let operand_leading_comments = context.comments().comments_before(operand_token_start);

    assert_eq!(operand_token_start, literal_start);
    assert_eq!(operand_leading_comments.len(), 1);
    assert_eq!(operand_leading_comments[0].span.start, comment_start);
    assert_eq!(operand_leading_comments[0].span.end, comment_end);
    assert!(context.comments().has_comment_before(operand_token_start));
}

/// Standalone unary expressions should group separator comments inside the operand wrapper.
#[test]
fn test_format_unary_negative_expression_separator_block_comments() {
    let (test, expression_id) = TestFormatter::parse_with_file_type(
        "-/* unary-note */ 1",
        FileType::TypeScript,
        |parser| parser.eat_expression(Default::default()),
    )
    .unwrap();

    let formatted = test.format(
        &expression_id,
        DestackFormatOptions::default_with_line_width(100),
    );

    assert_eq!(formatted, "-(/* unary-note */ 1)");
}

/// Explicit parenthesized scalar comment wrappers should stay inline.
#[test]
fn test_format_parenthesized_scalar_separator_block_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = -(/* unary-note */ 1)\n",
        "const value = -(/* unary-note */ 1);\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Explicit parenthesized scalar line comment wrappers should stay multiline.
#[test]
fn test_format_parenthesized_scalar_separator_line_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = -(\n    // unary-line-note\n    1\n)\n",
        "const value = -(\n    // unary-line-note\n    1\n);\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Mixed inline block and line comments should stay together before the wrapped value.
#[test]
fn test_format_parenthesized_scalar_separator_mixed_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = (/* keep */ // comment\n    a as any) + 1\n",
        "const value =\n    /* keep */ // comment\n    (a as any) + 1;\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Mixed comments after `(` should stay visible as one leading slice for the inner assertion.
#[test]
fn test_parenthesized_scalar_separator_mixed_comments_attach_as_inner_leading_slice() {
    let input = "(/* keep */ // comment\n    a as any) + 1";
    let block_start = 1;
    let block_end = 11;
    let line_start = 12;
    let line_end = 22;
    let inner_start = 27;
    let (test, expression_id) =
        TestFormatter::parse_with_file_type(input, FileType::TypeScript, |parser| {
            parser.eat_expression(Default::default())
        })
        .unwrap();
    let context = test.context(DestackFormatOptions::default_with_line_width(100));

    let Expression::Binary { left, .. } = context.tree.get(expression_id) else {
        panic!("expected binary expression");
    };

    if !matches!(context.tree.get(*left), Expression::As { .. }) {
        panic!("expected assertion left expression");
    };

    let inner_expression_id = *left;
    let token_start = context.expression_token_start(inner_expression_id);
    let leading_comments = context.comments().comments_before(token_start);

    assert_eq!(leading_comments.len(), 2);
    assert_eq!(token_start, inner_start);

    assert_eq!(leading_comments[0].kind, CommentKind::SingleLineBlock);
    assert_eq!(leading_comments[0].position, CommentPosition::Leading);
    assert_eq!(leading_comments[0].attached_to, inner_start);
    assert_eq!(leading_comments[0].span.start, block_start);
    assert_eq!(leading_comments[0].span.end, block_end);
    assert!(!leading_comments[0].preceded_by_newline());
    assert!(!leading_comments[0].followed_by_newline());

    assert_eq!(leading_comments[1].kind, CommentKind::Line);
    assert_eq!(leading_comments[1].position, CommentPosition::Leading);
    assert_eq!(leading_comments[1].attached_to, inner_start);
    assert_eq!(leading_comments[1].span.start, line_start);
    assert_eq!(leading_comments[1].span.end, line_end);
    assert!(!leading_comments[1].preceded_by_newline());
    assert!(leading_comments[1].followed_by_newline());

    assert!(expression_needs_parentheses_in_parent(
        &context,
        inner_expression_id
    ));
}

/// The inner assertion should keep mixed leading comments on one line before derived parentheses.
#[test]
fn test_format_inner_assertion_with_parenthesized_scalar_separator_mixed_comments() {
    let input = "(/* keep */ // comment\n    a as any) + 1";
    let (test, expression_id) =
        TestFormatter::parse_with_file_type(input, FileType::TypeScript, |parser| {
            parser.eat_expression(Default::default())
        })
        .unwrap();
    let context = test.context(DestackFormatOptions::default_with_line_width(100));

    let inner_expression_id = match context.tree.get(expression_id) {
        Expression::Binary { left, .. } => *left,
        _ => panic!("expected binary expression"),
    };

    let formatted = test.format(
        &inner_expression_id,
        DestackFormatOptions::default_with_line_width(100),
    );

    assert_eq!(formatted, "/* keep */ // comment\n(a as any)");
}

/// Unary separator line comments should keep the multiline grouped operand.
#[test]
fn test_format_unary_negative_separator_line_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = -// unary-line-note\n1\n",
        "const value = -(\n    // unary-line-note\n    1\n);\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Label separator line comments should stay above the labeled statement.
#[test]
fn test_format_label_separator_line_comments() {
    assert_format_program_roundtrip_with_file_type(
        "start: // label-tail\nwhile (true) {\n  break start\n}\n",
        "// label-tail\nstart: while (true) {\n    break start;\n}\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Ternary block comments before the alternate should stay with the alternate branch.
#[test]
fn test_format_ternary_alternate_block_separator_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const x = condition ? /* then */ valueA : /* else */ valueB\n",
        "const x = condition ? /* then */ valueA : /* else */ valueB;\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Ternary line comments before the alternate should stay on the then-branch separator.
#[test]
fn test_format_ternary_alternate_line_separator_comments() {
    assert_format_program_roundtrip_with_file_type(
        "const x = condition ? valueA : // else-note\nvalueB\n",
        "const x = condition\n    ? valueA // else-note\n    : valueB;\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Tagged template generic arguments should stay between the tag and template literal.
#[test]
fn test_format_tagged_template_expression_preserves_generic_arguments() {
    assert_format_program_roundtrip_with_file_type(
        "const value = sql<Type>`select * from t`\n",
        "const value = sql<Type>`select * from t`;\n",
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Template remap comments should stay owned by the template container, not the generic argument.
#[test]
fn test_type_template_remap_comment_stays_outside_generic_argument_ownership() {
    let input = "type Paths<T> = {\n  [K in keyof T as // remap-note\n    `get${Capitalize<K & string>}`]: () => T[K]\n}";
    let (test, expression_id) =
        TestFormatter::parse_with_file_type(input, FileType::TypeScript, |parser| {
            parser.eat_expression(Default::default())
        })
        .unwrap();
    let context = test.context(DestackFormatOptions::default_with_line_width(100));

    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        panic!("expected type declaration");
    };
    let Declaration::Type(declaration) = context.tree.get(*declaration_id) else {
        panic!("expected type declaration");
    };
    let TypeExpression::Mapped { parameter, .. } = context.tree.get(declaration.value) else {
        panic!("expected mapped type");
    };
    let key_remap = parameter.key_remap.expect("expected key remap");
    let TypeExpression::TemplateLiteral { spans, .. } = context.tree.get(key_remap) else {
        panic!("expected template literal remap");
    };
    let interpolation_type = spans[0];
    let TypeExpression::Reference {
        generic_arguments, ..
    } = context.tree.get(interpolation_type)
    else {
        panic!("expected reference interpolation type");
    };
    let generic_argument_leading_comments =
        context.comments_after_previous_non_trivia_token_for(generic_arguments[0]);
    let formatted_expression = test.format(
        &expression_id,
        DestackFormatOptions::default_with_line_width(100),
    );

    assert!(
        generic_argument_leading_comments.is_empty(),
        "generic argument should not claim the remap comment"
    );
    assert_eq!(
        formatted_expression,
        "type Paths<T> = {\n    [K in keyof T as `get${Capitalize<K & string>}`]: () => T[K]; // remap-note\n};"
    );
}
