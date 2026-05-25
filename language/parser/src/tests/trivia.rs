use std::sync::Arc;

use destack_core::StringPool;
use destack_dir::{
    Argument, Block, BlockContext, BlockForm, ClassDeclaration, Comment, CommentContent,
    CommentKind, CommentPosition, Declaration, Declarator, Decorator, DecoratorPosition,
    Expression, FunctionDeclaration, LocalNodeId, Member, Parameter, Property, StructDeclaration,
    TokenType, TypeDeclaration, TypeExpression, normalize_comment_payload,
};
use destack_source::LanguageType;

use crate::{
    Lexer, Parser, ParserOptions, ParserTriviaMode, TestParser, assert_comment,
    assert_expression_path, assert_node,
};

/// Parse one whole source string and return the resulting root expressions.
fn parse_source(source: &str, language: LanguageType) -> (Parser, Vec<LocalNodeId<Expression>>) {
    let mut test = TestParser::new_with_language(source, language);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    (parser, expressions)
}

/// Parse one whole source string with one trivia mode.
fn parse_source_with_trivia_mode(
    source: &str,
    language: LanguageType,
    trivia_mode: ParserTriviaMode,
) -> (Parser, Vec<LocalNodeId<Expression>>) {
    let test = TestParser::new_with_language(source, language);
    let mut parser = Parser::lex_file_with_options(
        test.file.clone(),
        language,
        ParserOptions {
            trivia_mode,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    );
    let expressions = parser.parse();

    (parser, expressions)
}

/// Parse one block expression source and attach comments after the direct entrypoint.
fn parse_block_source(source: &str, language: LanguageType) -> (Parser, LocalNodeId<Block>) {
    let mut test = TestParser::new_with_language(source, language);
    let mut parser = test.prepare();
    let block_id = parser
        .eat_block(BlockContext::Expression)
        .expect("expected block expression in test source");
    parser.attach_comments();
    (parser, block_id)
}

/// Parse one direct property entrypoint and attach comments after parsing.
fn parse_property_source(
    source: &str,
    language: LanguageType,
    is_in_variant: bool,
) -> (Parser, LocalNodeId<Property>) {
    let mut test = TestParser::new_with_language(source, language);
    let mut parser = test.prepare();
    parser.flags = parser.flags.with_variant(is_in_variant);
    let property_id = parser
        .eat_property()
        .expect("expected property in test source");
    parser.attach_comments();
    (parser, property_id)
}

/// Return the normalized payload text for one raw comment.
fn comment_text(parser: &Parser, comment: Comment) -> String {
    let source = parser.get_span_str(comment.span);
    normalize_comment_payload(source).into_owned()
}

/// Return the nearest non-trivia token before one comment boundary.
fn previous_boundary_token_type(parser: &Parser, comment: Comment) -> Option<TokenType> {
    parser
        .tokens()
        .iter()
        .rev()
        .find(|token| {
            token.span.end <= comment.span.start && !matches!(token.token.ty(), TokenType::End)
        })
        .copied()
        .map(|token| token.token.ty())
}

/// Return the nearest non-trivia token after one comment boundary.
fn next_boundary_token_type(parser: &Parser, comment: Comment) -> Option<TokenType> {
    parser
        .tokens()
        .iter()
        .find(|token| {
            token.span.start >= comment.span.end && !matches!(token.token.ty(), TokenType::End)
        })
        .copied()
        .map(|token| token.token.ty())
}

/// Assert the non-trivia token kinds on both sides of one comment boundary.
fn assert_comment_boundary_tokens(
    parser: &Parser,
    comment: Comment,
    token_before: Option<TokenType>,
    token_after: Option<TokenType>,
) {
    assert_eq!(previous_boundary_token_type(parser, comment), token_before);
    assert_eq!(next_boundary_token_type(parser, comment), token_after);
}

/// Assert the stored newline flags for one comment.
fn assert_comment_newline_shape(
    comment: Comment,
    has_leading_newline: bool,
    has_trailing_newline: bool,
) {
    assert_eq!(comment.newlines.has_leading_newline(), has_leading_newline);
    assert_eq!(
        comment.newlines.has_trailing_newline(),
        has_trailing_newline
    );
}

/// Return the raw comments stored on the parsed tree.
fn comments(parser: &Parser) -> &[Comment] {
    parser.tree.comments()
}

#[test]
fn test_parse_runs_attach_comments_for_comments() {
    let (parser, expressions) = parse_source("// lead\nvalue", LanguageType::TypeScript);

    // `value`
    assert_eq!(expressions.len(), 1);
    let expression_annotations = parser.tree.get_decorators(expressions[0].id);
    assert!(expression_annotations.is_empty());

    // `// lead`
    assert_eq!(parser.tree.comments().len(), 1);
    let comment = parser.tree.comments()[0];
    assert_eq!(comment_text(&parser, comment), "lead");
}

#[test]
fn test_parse_statement_span_stops_before_following_blank_line_comment() {
    let source = r#"if (!process.stdout.isTTY) process.stdout._handle?.setBlocking?.(true);

// Call the Rust CLI first
const mode = runCli();
"#;
    let (parser, expressions) = parse_source(source, LanguageType::TypeScript);

    assert_eq!(expressions.len(), 2);

    let first_span = parser.tree.get_span(expressions[0]);
    let comment_start = source.find("// Call").expect("expected comment") as u32;

    assert!(
        first_span.end < comment_start,
        "first statement span should end before following comment: {first_span:?}"
    );
}

#[test]
fn test_parse_without_attaching_comments_leaves_comments_empty_until_attach() {
    let mut test =
        TestParser::new_with_language("// lead\nvalue\n\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();

    // `value`, `next`
    let expressions = parser.parse_without_attaching_comments();
    assert_eq!(expressions.len(), 2);

    // `// lead`
    assert_eq!(parser.tree.comments().len(), 0);

    // `// lead`
    parser.attach_comments();
    assert_eq!(parser.tree.comments().len(), 1);
}

#[test]
fn test_parse_ignore_trivia_mode_drops_comments() {
    let source = "/// docs\n/*! legal */\n// raw\nvalue";
    let (parser, expressions) =
        parse_source_with_trivia_mode(source, LanguageType::TypeScript, ParserTriviaMode::Ignore);

    // `value`
    assert_eq!(expressions.len(), 1);

    // no retained comments
    assert!(parser.tree.comments().is_empty());
}

#[test]
fn test_parse_documentation_trivia_mode_keeps_structured_comments() {
    let source = "/// docs\n// raw\n/*! legal */\n/* ordinary */\n/** block */\nvalue";
    let (parser, expressions) = parse_source_with_trivia_mode(
        source,
        LanguageType::TypeScript,
        ParserTriviaMode::Documentation,
    );

    // `value`
    assert_eq!(expressions.len(), 1);

    // structured comments only
    assert_eq!(parser.tree.comments().len(), 3);
    assert_eq!(comment_text(&parser, parser.tree.comments()[0]), "docs");
    assert_eq!(parser.tree.comments()[0].content, CommentContent::Jsdoc);
    assert_eq!(parser.tree.comments()[1].content, CommentContent::Legal);
    assert_eq!(comment_text(&parser, parser.tree.comments()[2]), " block");
    assert_eq!(parser.tree.comments()[2].content, CommentContent::Jsdoc);
}

#[test]
fn test_lex_documentation_trivia_mode_skips_side_tokens() {
    let test = TestParser::new_with_language("/// docs\n// raw\nvalue", LanguageType::TypeScript);
    let result = Lexer::lex_with_options(
        test.file.clone(),
        LanguageType::TypeScript,
        ParserTriviaMode::Documentation,
    );

    // comments are retained without formatter side tokens
    assert_eq!(result.comments.len(), 1);
    assert!(result.side_tokens.is_empty());
}

#[test]
fn test_attach_comments_on_direct_entrypoint_emits_output() {
    let mut test =
        TestParser::new_with_language("// lead\nvalue\n\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();

    // `value`, `next`
    let expressions = parser.eat_block_body(BlockForm::Implicit).unwrap();
    assert_eq!(expressions.len(), 2);

    // `// lead`
    assert_eq!(parser.tree.comments().len(), 0);

    // `// lead`
    parser.attach_comments();
    assert_eq!(parser.tree.comments().len(), 1);
}

#[test]
fn test_attach_comments_is_idempotent() {
    let mut test =
        TestParser::new_with_language("// lead\nvalue\n\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();

    // `value`, `next`
    let expressions = parser.parse_without_attaching_comments();
    assert_eq!(expressions.len(), 2);

    // first `// lead`
    parser.attach_comments();
    let first_comment_count = parser.tree.comments().len();

    // second `// lead`
    parser.attach_comments();
    assert_eq!(parser.tree.comments().len(), first_comment_count);
}

#[test]
fn test_attach_comments_keeps_one_comment_after_restore_and_reparse() {
    let mut test = TestParser::new_with_language("a // note\nb", LanguageType::TypeScript);
    let mut parser = test.prepare();

    // speculative lookahead across the comment
    let mark = parser.checkpoint();
    let mark_node_id = parser.tree.next_id();
    let next_span = parser.next_token().span;
    assert_eq!(parser.get_span_str(next_span), "b");

    // restore and consume the same boundary again
    parser.restore(mark, mark_node_id);
    parser.eat().expect("expected first token");
    parser.eat().expect("expected second token");

    // attach one raw comment
    parser.attach_comments();
    assert_eq!(parser.tree.comments().len(), 1);
    let comment = parser.tree.comments()[0];
    assert_eq!(comment_text(&parser, comment), "note");
    assert_eq!(comment.position, CommentPosition::Trailing);
}

#[test]
fn test_comment_only_file_gets_stub_expression_and_trivia() {
    let (parser, expressions) = parse_source("// only", LanguageType::TypeScript);

    // stub for `// only`
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Stub);
    let expression_span = parser.tree.get_span(expressions[0]);
    assert_eq!(expression_span.start, parser.file.len);
    assert_eq!(expression_span.end, parser.file.len);
    let annotations = parser.tree.get_decorators(expressions[0].id);
    assert!(annotations.is_empty());

    // `// only`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "only");
}

#[test]
fn test_comment_trivia_keeps_directive_comments_raw() {
    let (parser, expressions) = parse_source(
        "// @ts-ignore\na\n/* @__PURE__ */\nb\n// prettier-ignore\nc\n// prettier-ignore-start\nd\n// prettier-ignore-end\ne",
        LanguageType::TypeScript,
    );

    // `a`, `b`, `c`, `d`, `e`
    assert_eq!(expressions.len(), 5);

    // `// @ts-ignore`, `/* @__PURE__ */`, `// prettier-ignore`,
    // `// prettier-ignore-start`, `// prettier-ignore-end`
    assert_eq!(comments(&parser).len(), 5);
    let first = comments(&parser)[0];
    let second = comments(&parser)[1];
    let third = comments(&parser)[2];
    let fourth = comments(&parser)[3];
    let fifth = comments(&parser)[4];

    assert_eq!(comment_text(&parser, first), "@ts-ignore");
    assert_eq!(comment_text(&parser, second), " @__PURE__");
    assert_eq!(comment_text(&parser, third), "prettier-ignore");
    assert_eq!(comment_text(&parser, fourth), "prettier-ignore-start");
    assert_eq!(comment_text(&parser, fifth), "prettier-ignore-end");
}

#[test]
fn test_comment_trivia_keeps_empty_line_comments() {
    let (parser, expressions) = parse_source(
        r#"function func() {
  /******/ "use strict" //
  /******/ b;
}"#,
        LanguageType::TypeScript,
    );

    // `function func() { ... }`
    assert_eq!(expressions.len(), 1);

    // `/**/`, `//`, `/**/`
    assert_eq!(comments(&parser).len(), 3);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, "***");
    assert_comment!(parser, 1, CommentKind::Line, "");
    assert_comment!(parser, 2, CommentKind::SingleLineBlock, "***");
}

#[test]
fn test_comment_trivia_normalizes_payload_and_style() {
    let (parser, expressions) =
        parse_source("// line\n/* block */\nvalue", LanguageType::TypeScript);

    // `value`
    assert_eq!(expressions.len(), 1);

    // `// line`
    assert_eq!(comments(&parser).len(), 2);
    let first = comments(&parser)[0];
    assert_eq!(comment_text(&parser, first), "line");
    assert_eq!(first.kind, CommentKind::Line);

    // `/* block */`
    let second = comments(&parser)[1];
    assert_eq!(comment_text(&parser, second), " block");
    assert_eq!(second.kind, CommentKind::SingleLineBlock);
}

#[test]
fn test_comment_trivia_classifies_annotation_content() {
    let source = r#"
/*! keep */
/** docs */
/** @license keep */
/* #__PURE__ */
/* webpackChunkName: "main" */
/* @vite-ignore */
value
"#;
    let (parser, expressions) = parse_source(source, LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 6);

    let expected = [
        CommentContent::Legal,
        CommentContent::Jsdoc,
        CommentContent::JsdocLegal,
        CommentContent::None,
        CommentContent::None,
        CommentContent::None,
    ];

    for (comment, expected_content) in comments(&parser).iter().zip(expected) {
        assert_eq!(comment.content, expected_content);
        assert_eq!(comment.position, CommentPosition::Leading);
    }
}

#[test]
fn test_doc_comments_attach_semantically_and_skip_raw_comments() {
    let (parser, expressions) = parse_source("/// docs\nfunction f() {}", LanguageType::TypeScript);

    // `function f() {}`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        // `function f() {}`
        let expression_annotations = parser.tree.get_decorators(expression_id.id);
        assert!(expression_annotations.is_empty());
        let declaration_annotations = parser.tree.get_decorators(declaration_id.id);
        assert!(declaration_annotations.is_empty());

        // `/// docs`
        assert_eq!(comments(&parser).len(), 1);
        let comment = comments(&parser)[0];
        assert_eq!(comment_text(&parser, comment), "docs");
        assert_eq!(comment.position, CommentPosition::Leading);
        assert_eq!(comment.attached_to, parser.tree.get_span(*declaration_id).start);
    });
}

#[test]
fn test_doc_comment_attaches_to_parameter() {
    let (parser, expressions) = parse_source(
        "function demo(/** parameter-doc */ value: number): void {}",
        LanguageType::TypeScript,
    );

    // `function demo(...)`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            // `value: number`
            assert_eq!(signature.parameters.len(), 1);
            let parameter_id = signature.parameters[0];
            let annotations = parser.tree.get_decorators(parameter_id.id);
            assert!(annotations.is_empty());

            // `/** parameter-doc */`
            assert_eq!(comments(&parser).len(), 1);
            let comment = comments(&parser)[0];
            assert_eq!(comment_text(&parser, comment), " parameter-doc");
            assert_comment_boundary_tokens(
                &parser,
                comment,
                Some(TokenType::OpenParenthesis),
                Some(TokenType::Identifier),
            );
        });
    });
}

#[test]
fn test_doc_comment_after_type_assignment_attaches_to_type_value() {
    let (parser, expressions) = parse_source(
        "export type Value = /** keep-doc\n */\n| { ok: true }\n| { ok: false; value: bigint | null };",
        LanguageType::TypeScript,
    );

    // `export type Value = ...`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            // `| { ok: true } | ...`
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);

                // `| { ok: true }`
                let left_annotations = parser.tree.get_decorators(elements[0].id);
                assert!(left_annotations.is_empty());

                // `/** keep-doc */`
                assert_eq!(comments(&parser).len(), 1);
                let comment = comments(&parser)[0];
                assert!(parser.get_span_str(comment.span).starts_with("/**"));
                assert!(comment_text(&parser, comment).contains("keep-doc"));
                let _ = value;
                assert_comment_boundary_tokens(
                    &parser,
                    comment,
                    Some(TokenType::Assign),
                    Some(TokenType::ElementwiseOr),
                );
            });
        });
    });
}

#[test]
fn test_comment_after_type_assignment_attaches_to_type_value() {
    let (parser, expressions) = parse_source(
        "export type Value = /* keep */ number;",
        LanguageType::TypeScript,
    );

    // `export type Value = ...`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            // `/* keep */`
            assert_eq!(comments(&parser).len(), 1);
            let comment = comments(&parser)[0];
            assert_eq!(comment_text(&parser, comment), " keep");
            let _ = value;
            assert_comment_boundary_tokens(
                &parser,
                comment,
                Some(TokenType::Assign),
                Some(TokenType::Identifier),
            );
        });
    });
}

#[test]
fn test_comment_after_open_parenthesis_attaches_to_inner_expression_leading() {
    let (parser, expressions) = parse_source("(/* keep */ value)", LanguageType::TypeScript);

    // `(/* keep */ value)`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Parenthesized { expression } => {
        // `/* keep */`
        assert_eq!(comments(&parser).len(), 1);
        let comment = comments(&parser)[0];
        assert_eq!(comment_text(&parser, comment), " keep");
        // `value`
        assert_ne!(expression_id.id, expression.id);
        assert_comment_boundary_tokens(
            &parser,
            comment,
            Some(TokenType::OpenParenthesis),
            Some(TokenType::Identifier),
        );
        assert_comment_newline_shape(comment, false, false);
        assert_eq!(comment.position, CommentPosition::Leading);
        assert_eq!(comment.attached_to, parser.tree.get_span(*expression).start);
    });
}

#[test]
fn test_comment_before_close_parenthesis_attaches_to_inner_expression_trailing() {
    let (parser, expressions) = parse_source("(value /* keep */)", LanguageType::TypeScript);

    // `(value /* keep */)`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Parenthesized { expression: _ } => {
        // `/* keep */`
        assert_eq!(comments(&parser).len(), 1);
        let comment = comments(&parser)[0];
        assert_eq!(comment_text(&parser, comment), " keep");
        // `value`
        assert_comment_boundary_tokens(
            &parser,
            comment,
            Some(TokenType::Identifier),
            Some(TokenType::CloseParenthesis),
        );
        assert_comment_newline_shape(comment, false, false);
    });
}

#[test]
fn test_line_comment_between_unary_prefix_and_operand_attaches_to_operand_leading() {
    let (parser, expressions) = parse_source("-// unary-line-note\n1", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Unary { right: _, .. } => {
        assert_eq!(comments(&parser).len(), 1);
        let comment = comments(&parser)[0];
        assert_eq!(comment_text(&parser, comment), "unary-line-note");
        assert_comment_boundary_tokens(
            &parser,
            comment,
            Some(TokenType::Subtract),
            Some(TokenType::Literal),
        );
    });
}

#[test]
fn test_line_comment_between_unary_prefix_and_operand_in_initializer_attaches_to_operand_leading() {
    let (parser, expressions) = parse_source(
        "const value = -// unary-line-note\n1",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value = value.expect("expected initializer");

            assert_node!(parser.tree, value, Expression::Unary { right: _, .. } => {
                assert_eq!(comments(&parser).len(), 1);
                let comment = comments(&parser)[0];
                assert_eq!(comment_text(&parser, comment), "unary-line-note");
                assert_comment_boundary_tokens(
                    &parser,
                    comment,
                    Some(TokenType::Subtract),
                    Some(TokenType::Literal),
                );
            });
        });
    });
}

#[test]
fn test_block_comments_between_ternary_branches_attach_to_separator_owners() {
    let (parser, expressions) = parse_source(
        "cond ? /* then */ left : /* else */ right",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::If { then_expression, else_expression, .. } => {
        let else_expression_id = else_expression.expect("expected ternary else branch");

        assert_eq!(comments(&parser).len(), 2);

        let then_comment = comments(&parser)[0];
        assert_eq!(comment_text(&parser, then_comment), " then");
        let _ = then_expression;

        let else_comment = comments(&parser)[1];
        assert_eq!(comment_text(&parser, else_comment), " else");
        let _ = else_expression_id;
    });
}

#[test]
fn test_doc_comment_attaches_to_call_argument() {
    let (parser, expressions) =
        parse_source("run(/** argument-doc */ value)", LanguageType::TypeScript);

    // `run(...)`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Call { arguments, .. } => {
        // `value`
        assert_eq!(arguments.len(), 1);
        let argument_id = arguments[0];
        let argument_annotations = parser.tree.get_decorators(argument_id.id);
        assert!(argument_annotations.is_empty());

        // `/** argument-doc */`
        assert_eq!(comments(&parser).len(), 1);
        let comment = comments(&parser)[0];
        assert_eq!(comment_text(&parser, comment), " argument-doc");
        assert_comment_boundary_tokens(
            &parser,
            comment,
            Some(TokenType::OpenParenthesis),
            Some(TokenType::Identifier),
        );

        // `value`
        assert_node!(parser.tree, argument_id, Argument::Positional { value, .. } => {
            let value_annotations = parser.tree.get_decorators(value.id);
            assert!(value_annotations.is_empty());
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}

#[test]
fn test_comment_between_export_and_declaration_head_emits_unowned_boundary_trivia() {
    let (parser, expressions) = parse_source(
        "export // boundary\nasync function f() {}",
        LanguageType::TypeScript,
    );

    // `async function f() {}`
    assert_eq!(expressions.len(), 1);

    // `// boundary`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "boundary");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, true);
}

#[test]
fn test_comment_after_satisfies_keyword_emits_unowned_boundary_trivia() {
    let (parser, expressions) = parse_source(
        "value satisfies // boundary\nRecord<A, B>",
        LanguageType::TypeScript,
    );

    // `value satisfies Record<A, B>`
    assert_eq!(expressions.len(), 1);

    // `// boundary`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "boundary");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, true);
}

#[test]
fn test_comment_before_as_keyword_emits_unowned_boundary_trivia() {
    let (parser, expressions) = parse_source(
        "const value = source /* boundary */ as number",
        LanguageType::TypeScript,
    );

    // `const value = source as number`
    assert_eq!(expressions.len(), 1);

    // `/* boundary */`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), " boundary");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, false);
}

#[test]
fn test_comment_after_as_keyword_emits_unowned_boundary_trivia() {
    let (parser, expressions) = parse_source(
        "const value = source as // boundary\nnumber",
        LanguageType::TypeScript,
    );

    // `const value = source as number`
    assert_eq!(expressions.len(), 1);

    // `// boundary`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "boundary");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, true);
}

#[test]
fn test_multiline_block_comment_between_as_and_const_emits_unowned_boundary_trivia() {
    let (parser, expressions) =
        parse_source("1 as /*\nblock-comment\n*/ const", LanguageType::TypeScript);

    // `1 as const`
    assert_eq!(expressions.len(), 1);

    // `/* block-comment */`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "\nblock-comment\n");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, false);
}

#[test]
fn test_variable_trailing_marker_comment_emits_unowned_boundary_trivia() {
    let (parser, expressions) = parse_source(
        "declare const PAGE_PATH: string\n  //<- keep-marker\n;(()=>{})()",
        LanguageType::TypeScript,
    );

    // `declare const PAGE_PATH: string`, `;(()=>{})()`
    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert!(
        !expressions.is_empty(),
        "expected at least one parsed expression"
    );

    // `//<- keep-marker`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "<- keep-marker");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Semicolon),
    );
    assert_comment_newline_shape(comment, true, true);
}

#[test]
fn test_comment_after_if_head_emits_unowned_boundary_trivia() {
    let (parser, expressions) =
        parse_source("if (ready) // if-head\nrun()", LanguageType::TypeScript);

    // `if (ready) run()`
    assert_eq!(expressions.len(), 1);

    // `// if-head`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "if-head");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::CloseParenthesis),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, true);
}

#[test]
fn test_comment_between_ternary_then_and_colon_emits_unowned_boundary_trivia() {
    let (parser, expressions) = parse_source(
        "const result = cond ? left /* left-note */ : right",
        LanguageType::TypeScript,
    );

    // `const result = cond ? left : right`
    assert_eq!(expressions.len(), 1);

    // `/* left-note */`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), " left-note");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Colon),
    );
    assert_comment_newline_shape(comment, false, false);
}

#[test]
fn test_comment_before_ternary_question_attaches_to_question_boundary() {
    let (parser, expressions) = parse_source(
        "const result = cond /* cond-note */ ? left : right",
        LanguageType::TypeScript,
    );

    // `const result = cond ? left : right`
    assert_eq!(expressions.len(), 1);

    // `/* cond-note */`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), " cond-note");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Maybe),
    );
    assert_comment_newline_shape(comment, false, false);
}

#[test]
fn test_comment_before_less_than_comparison_attaches_to_operator_boundary() {
    let (parser, expressions) = parse_source(
        "const result = left /* marker */ < right",
        LanguageType::TypeScript,
    );

    // `const result = left < right`
    assert_eq!(expressions.len(), 1);

    // `/* marker */`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), " marker");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::LessThan),
    );
    assert_comment_newline_shape(comment, false, false);
}

#[test]
fn test_comments_between_if_chain_branches_emit_unowned_boundary_trivia() {
    let (parser, expressions) = parse_source(
        r#"if (cond1) {
    const X = 1;
}
// comment before cond2
else if (cond2) {
    const Y = 2;
}
// comment before else
else {
    const Z = 3;
}"#,
        LanguageType::TypeScript,
    );

    // `if (...) { ... } else if (...) { ... } else { ... }`
    assert_eq!(expressions.len(), 1);

    // `// comment before cond2`, `// comment before else`
    assert_eq!(comments(&parser).len(), 2);

    // `// comment before cond2`
    let first_trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, first_trivia), "comment before cond2");
    assert_comment_boundary_tokens(
        &parser,
        first_trivia,
        Some(TokenType::CloseBrace),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(first_trivia, true, true);

    // `// comment before else`
    let second_trivia = comments(&parser)[1];
    assert_eq!(comment_text(&parser, second_trivia), "comment before else");
    assert_comment_boundary_tokens(
        &parser,
        second_trivia,
        Some(TokenType::CloseBrace),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(second_trivia, true, true);
}

#[test]
fn test_multiline_trailing_block_comment_inside_block_emits_unowned_boundary_trivia() {
    let (parser, expressions) = parse_source(
        "{\n    const X = 1 /* some comment\n    * over multiple lines yo       */\n}",
        LanguageType::TypeScript,
    );

    // `{ const X = 1 }`
    assert_eq!(expressions.len(), 1);

    // `/* some comment ... */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(
        comment_text(&parser, trivia),
        "some comment\nover multiple lines yo"
    );
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Literal),
        Some(TokenType::CloseBrace),
    );
    assert_comment_newline_shape(trivia, false, true);
}

#[test]
fn test_multiline_trailing_block_comment_on_eat_block_entrypoint_emits_unowned_boundary_trivia() {
    let (parser, _block_id) = parse_block_source(
        "{\n    const X = 1 /* some comment\n    * over multiple lines yo       */\n}",
        LanguageType::TypeScript,
    );

    // `/* some comment ... */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Literal),
        Some(TokenType::CloseBrace),
    );
    assert_comment_newline_shape(trivia, false, true);
}

#[test]
fn test_multiline_trailing_block_comment_on_eat_block_entrypoint_emits_unowned_boundary_trivia_in_value_block_mode()
 {
    let (parser, _block_id) = parse_block_source(
        "{\n    const X = 1 /* some comment\n    * over multiple lines yo       */\n}",
        LanguageType::Destack,
    );

    // `/* some comment ... */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Literal),
        Some(TokenType::CloseBrace),
    );
    assert_comment_newline_shape(trivia, false, true);
}

#[test]
fn test_comment_inside_empty_lambda_block_attaches_to_block_infix() {
    let (parser, expressions) = parse_source(
        "call(/* comment */\n  () => {\n    //\n  }\n)",
        LanguageType::TypeScript,
    );

    // `call(() => {})`
    assert_eq!(expressions.len(), 1);

    // `//`
    assert_eq!(comments(&parser).len(), 2);
    let trivia = comments(&parser)[1];
    assert_eq!(comment_text(&parser, trivia), "");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::OpenBrace),
        Some(TokenType::CloseBrace),
    );
    assert_comment_newline_shape(trivia, true, true);
}

#[test]
fn test_line_comment_after_block_opener_stays_trailing() {
    let (parser, expressions) =
        parse_source("{ // block-note\n  value\n}", LanguageType::TypeScript);

    // `{ value }`
    assert_eq!(expressions.len(), 1);

    // `// block-note`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), "block-note");
    assert_eq!(trivia.position, CommentPosition::Trailing);
    assert_eq!(trivia.attached_to, 0);
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::OpenBrace),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, true);
}

#[test]
fn test_line_comment_after_object_literal_opener_stays_trailing() {
    let (parser, expressions) = parse_source(
        "({ // object-note\n  value: 1\n})",
        LanguageType::TypeScript,
    );

    // `({ value: 1 })`
    assert_eq!(expressions.len(), 1);

    // `// object-note`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), "object-note");
    assert_eq!(trivia.position, CommentPosition::Trailing);
    assert_eq!(trivia.attached_to, 0);
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::OpenBrace),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, true);
}

#[test]
fn test_doc_and_decorator_attach_to_function_declaration_in_source_order() {
    let (parser, expressions) = parse_source(
        "/** docs */\n@memo\nfunction f() {}",
        LanguageType::TypeScript,
    );

    // `function f() {}`
    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.iter_nodes::<Decorator>().count(), 1);

    // `/** docs */`
    assert_eq!(comments(&parser).len(), 1);
    let doc_comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, doc_comment), " docs");

    // `@memo`
    let decorator_annotation = parser
        .tree
        .iter_nodes::<Decorator>()
        .next()
        .expect("missing decorator annotation");
    assert_node!(parser.tree, decorator_annotation, Decorator { expression: node, position } => {
        assert_eq!(*position, DecoratorPosition::BlockPrefix);
        assert_expression_path!(parser, parser.tree.get(*node), "memo");
    });
}

#[test]
fn test_empty_doc_block_comment_falls_back_to_raw_comments() {
    let (parser, expressions) = parse_source("/**/\nvalue", LanguageType::TypeScript);

    // `value`
    assert_eq!(expressions.len(), 1);

    // `/**/`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert!(comment.is_block());
}

#[test]
fn test_decorator_attaches_to_function_declaration() {
    let (parser, expressions) = parse_source("@memo\nfunction f() {}", LanguageType::TypeScript);

    // `function f() {}`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        // `@memo`
        let annotations = parser.tree.get_decorators(declaration_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Decorator { expression: node, position } => {
            assert_eq!(*position, DecoratorPosition::BlockPrefix);
            assert_expression_path!(parser, parser.tree.get(*node), "memo");
        });
    });
}

#[test]
fn test_decorator_attaches_to_struct_declaration_inside_block() {
    let (parser, expressions) = parse_source(
        "{\n    @memo\n    struct Entity {}\n}",
        LanguageType::Destack,
    );

    // `{ struct Entity {} }`
    assert_eq!(expressions.len(), 1);
    let block_expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, block_expression_id, Expression::Block(block_id) => {
        // `struct Entity {}`
        let block = parser.tree.get(*block_id);
        assert_eq!(block.leading_expressions.len(), 1);
        assert!(block.tail_expression.is_none());

        let declaration_expression_id =
            parser.unwrap_label_expression(block.leading_expressions[0]);
        assert_node!(parser.tree, declaration_expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Struct(StructDeclaration { .. }));

            // `@memo`
            let declaration_annotations = parser.tree.get_decorators(declaration_id.id);
            assert_eq!(declaration_annotations.len(), 1);
            assert_node!(parser.tree, declaration_annotations[0], Decorator { expression: node, position } => {
                assert_eq!(*position, DecoratorPosition::BlockPrefix);
                assert_expression_path!(parser, parser.tree.get(*node), "memo");
            });

            // `struct Entity {}`
            let declaration_expression_annotations =
                parser.tree.get_decorators(declaration_expression_id.id);
            assert!(declaration_expression_annotations.is_empty());
        });
    });
}

#[test]
fn test_decorator_on_expression_attaches_directly_without_wrapper() {
    let (parser, expressions) = parse_source("@memo\nrun()", LanguageType::TypeScript);

    // `run()`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Call { .. } => {
        // `@memo`
        let annotations = parser.tree.get_decorators(expression_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Decorator { expression: node, position } => {
            assert_eq!(*position, DecoratorPosition::BlockPrefix);
            assert_expression_path!(parser, parser.tree.get(*node), "memo");
        });
    });
}

#[test]
fn test_decorator_attaches_to_parameter() {
    let (parser, expressions) = parse_source(
        "function demo(@guard value: number): void {}",
        LanguageType::TypeScript,
    );

    // `function demo(...)`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            // `@guard value: number`
            assert_eq!(signature.parameters.len(), 1);
            let parameter_id = signature.parameters[0];
            let annotations = parser.tree.get_decorators(parameter_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Decorator { expression: node, .. } => {
                assert_expression_path!(parser, parser.tree.get(*node), "guard");
            });
        });
    });
}

#[test]
fn test_decorator_attaches_to_call_argument() {
    let (parser, expressions) = parse_source("run(@memo value)", LanguageType::TypeScript);

    // `run(...)`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Call { arguments, .. } => {
        // `@memo value`
        assert_eq!(arguments.len(), 1);
        let argument_id = arguments[0];
        let annotations = parser.tree.get_decorators(argument_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Decorator { expression: node, .. } => {
            assert_expression_path!(parser, parser.tree.get(*node), "memo");
        });

        // `value`
        assert_node!(parser.tree, argument_id, Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}

#[test]
fn test_keyword_decorator_attaches_to_call_argument() {
    let (parser, expressions) = parse_source("run(@if(true) value)", LanguageType::Destack);

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        let argument_id = arguments[0];
        let annotations = parser.tree.get_decorators(argument_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Decorator { expression, .. } => {
            assert_node!(parser.tree, *expression, Expression::Call { left, arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "if");
                assert_eq!(arguments.len(), 1);
            });
        });

        assert_node!(parser.tree, argument_id, Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}

#[test]
fn test_comments_and_blanks_are_not_semantic_annotations() {
    let (parser, expressions) = parse_source("a // tail\n\nb", LanguageType::TypeScript);

    // `a`, `b`
    assert_eq!(expressions.len(), 2);

    // `// tail`
    assert_eq!(parser.tree.comments().len(), 1);

    // `a`, `b`
    let first_annotations = parser.tree.get_decorators(expressions[0].id);
    let second_annotations = parser.tree.get_decorators(expressions[1].id);
    assert!(first_annotations.is_empty());
    assert!(second_annotations.is_empty());
}

#[test]
fn test_comment_inside_function_body_attaches_to_block_infix() {
    let (parser, expressions) =
        parse_source("function foo() { /* empty */ }", LanguageType::TypeScript);

    // `function foo() { ... }`
    assert_eq!(expressions.len(), 1);

    // `/* empty */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " empty");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::OpenBrace),
        Some(TokenType::CloseBrace),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_between_parameter_name_and_type_attaches_to_type_boundary() {
    let (parser, expressions) = parse_source(
        "function f(x /* a */ : number) {}",
        LanguageType::TypeScript,
    );

    // `function f(...) {}`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    let _parameter_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            signature.parameters[0]
        })
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Identifier),
        Some(TokenType::Colon),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_between_parameter_pattern_and_type_attaches_to_type_boundary() {
    let (parser, expressions) = parse_source(
        "function f({ value } /* a */ : Box) {}",
        LanguageType::TypeScript,
    );

    // `function f(...) {}`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    let _parameter_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            signature.parameters[0]
        })
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::CloseBrace),
        Some(TokenType::Colon),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_after_optional_parameter_marker_attaches_to_type_boundary() {
    let (parser, expressions) = parse_source(
        "function f(x? /* a */ : number) {}",
        LanguageType::TypeScript,
    );

    // `function f(...) {}`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    let _parameter_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            signature.parameters[0]
        })
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Maybe),
        Some(TokenType::Colon),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_after_parameter_colon_attaches_to_type_boundary() {
    let (parser, expressions) =
        parse_source("function f(x: /* a */ number) {}", LanguageType::TypeScript);

    // `function f(...) {}`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    let parameter_type = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { declared_type: Some(ty), .. } => {
                *ty
            })
        })
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    let _ = parameter_type;
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Colon),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_after_type_conditional_question_attaches_to_then_separator() {
    let (parser, expressions) = parse_source(
        "type T = A extends B ? // then-note\nC : D",
        LanguageType::TypeScript,
    );

    // `type T = A extends B ? C : D`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    let then_type = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { then_type, .. } => {
                *then_type
            })
        })
    });

    // `// then-note`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "then-note");
    let _ = then_type;
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Maybe),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, true);
}

#[test]
fn test_comment_after_type_conditional_colon_attaches_to_else_separator() {
    let (parser, expressions) = parse_source(
        "type T = A extends B ? C : // else-note\nD",
        LanguageType::TypeScript,
    );

    // `type T = A extends B ? C : D`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    let else_type = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { else_type, .. } => {
                *else_type
            })
        })
    });

    // `// else-note`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "else-note");
    let _ = else_type;
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Colon),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, true);
}

#[test]
fn test_comment_after_function_return_type_colon_attaches_to_return_type_boundary() {
    let (parser, expressions) =
        parse_source("function f(): /* a */ number {}", LanguageType::TypeScript);

    // `function f(): number {}`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    let return_type = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            signature.return_type.expect("expected return type")
        })
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    let _ = return_type;
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Colon),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_after_member_field_colon_attaches_to_field_type_boundary() {
    let (parser, expressions) = parse_source(
        "class Box { value: /* a */ number }",
        LanguageType::TypeScript,
    );

    // `class Box { value: number }`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    let field_type = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            assert_node!(parser.tree, members[0], Member::Field { declared_type: Some(value), .. } => {
                *value
            })
        })
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    let _ = field_type;
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Colon),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_after_optional_member_marker_attaches_to_field_type_boundary() {
    let (parser, expressions) = parse_source(
        "class Box { value? /* a */ : number }",
        LanguageType::TypeScript,
    );

    // `class Box { value?: number }`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    let _field_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            members[0]
        })
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Maybe),
        Some(TokenType::Colon),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_after_member_return_type_colon_attaches_to_return_type_boundary() {
    let (parser, expressions) = parse_source(
        "class Box { method(): /* a */ number {} }",
        LanguageType::TypeScript,
    );

    // `class Box { method(): number {} }`
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_label_expression(expressions[0]);
    let return_type = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                signature.return_type.expect("expected return type")
            })
        })
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    let _ = return_type;
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Colon),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_after_property_field_colon_attaches_to_field_type_boundary() {
    let (parser, property_id) =
        parse_property_source("value: /* a */ number", LanguageType::TypeScript, true);

    // `value: number`
    let field_type = assert_node!(parser.tree, property_id, Property::Field { value, .. } => {
        *value
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    let _ = field_type;
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Colon),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_after_optional_property_marker_attaches_to_field_type_boundary() {
    let (parser, property_id) =
        parse_property_source("value? /* a */ : number", LanguageType::TypeScript, true);

    // `value?: number`
    let _field_type = assert_node!(parser.tree, property_id, Property::Field { value, .. } => {
        *value
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Maybe),
        Some(TokenType::Colon),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_after_property_return_type_colon_attaches_to_return_type_boundary() {
    let (parser, property_id) = parse_property_source(
        "method(): /* a */ number {}",
        LanguageType::TypeScript,
        false,
    );

    // `method(): number {}`
    let return_type = assert_node!(parser.tree, property_id, Property::Method { signature, .. } => {
        signature.return_type.expect("expected return type")
    });

    // `/* a */`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    let _ = return_type;
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Colon),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comments_around_member_decorator_chain_remain_raw_trivia() {
    let (parser, expressions) = parse_source(
        r#"class Box {
    // comment before entity
    @entity
    // comment after entity
    // comment before foo
    @foo(1, 2, 3)
    // comment after foo
    method() {}
}"#,
        LanguageType::TypeScript,
    );

    // `class Box { ... }`
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            // `@entity`, `@foo(1, 2, 3)`
            assert_eq!(members.len(), 1);
            let member_id = members[0];
            let member_annotations = parser.tree.get_decorators(member_id.id);
            assert_eq!(member_annotations.len(), 2);

            assert_node!(parser.tree, member_annotations[0], Decorator { expression: node, .. } => {
                assert_expression_path!(parser, parser.tree.get(*node), "entity");
            });

            assert_node!(parser.tree, member_annotations[1], Decorator { expression: node, .. } => {
                    assert_node!(parser.tree, *node, Expression::Call { left, arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "foo");
                        assert_eq!(arguments.len(), 3);
                    });
            });

            // `// comment before entity`, `// comment after entity`,
            // `// comment before foo`, `// comment after foo`
            assert_eq!(comments(&parser).len(), 4);
            let trivia = comments(&parser);

            assert_eq!(comment_text(&parser, trivia[0]), "comment before entity");
            assert_eq!(next_boundary_token_type(&parser, trivia[0]), Some(TokenType::At));
            assert_comment_newline_shape(trivia[0], true, true);

            assert_eq!(comment_text(&parser, trivia[1]), "comment after entity");
            assert!(next_boundary_token_type(&parser, trivia[1]).is_some());
            assert_comment_newline_shape(trivia[1], true, true);

            assert_eq!(comment_text(&parser, trivia[2]), "comment before foo");
            assert!(next_boundary_token_type(&parser, trivia[2]).is_some());
            assert_comment_newline_shape(trivia[2], true, true);

            assert_eq!(comment_text(&parser, trivia[3]), "comment after foo");
            assert_eq!(
                next_boundary_token_type(&parser, trivia[3]),
                Some(TokenType::Identifier)
            );
            assert_comment_newline_shape(trivia[3], true, true);
        });
    });
}

#[test]
fn test_empty_call_boundary_line_comments_preserve_raw_call_gap_boundaries() {
    let (parser, expressions) = parse_source(
        r#"call // direct
()

call // optional
?.()"#,
        LanguageType::JavaScript,
    );

    // `call()`, `call?.()`
    assert_eq!(expressions.len(), 2);

    // `// direct`, `// optional`
    assert_eq!(comments(&parser).len(), 2);

    let trivia = comments(&parser);

    assert_comment_boundary_tokens(
        &parser,
        trivia[0],
        Some(TokenType::Identifier),
        Some(TokenType::OpenParenthesis),
    );
    assert_comment_newline_shape(trivia[0], false, true);
    assert_comment!(parser, 0, CommentKind::Line, "direct");

    assert_comment_boundary_tokens(
        &parser,
        trivia[1],
        Some(TokenType::Identifier),
        Some(TokenType::Maybe),
    );
    assert_comment_newline_shape(trivia[1], false, true);
    assert_comment!(parser, 1, CommentKind::Line, "optional");
}

#[test]
fn test_empty_call_boundary_block_comments_preserve_raw_call_gap_boundaries() {
    let (parser, expressions) = parse_source(
        r#"call/* direct */()
call/* optional */?.()"#,
        LanguageType::JavaScript,
    );

    // `call()`, `call?.()`
    assert_eq!(expressions.len(), 2);

    // `/* direct */`, `/* optional */`
    assert_eq!(comments(&parser).len(), 2);

    let trivia = comments(&parser);

    assert_comment_boundary_tokens(
        &parser,
        trivia[0],
        Some(TokenType::Identifier),
        Some(TokenType::OpenParenthesis),
    );
    assert_comment_newline_shape(trivia[0], false, false);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " direct");

    assert_comment_boundary_tokens(
        &parser,
        trivia[1],
        Some(TokenType::Identifier),
        Some(TokenType::Maybe),
    );
    assert_comment_newline_shape(trivia[1], false, false);
    assert_comment!(parser, 1, CommentKind::SingleLineBlock, " optional");
}

#[test]
fn test_comment_before_optional_chain_question_attaches_forward() {
    let (parser, expressions) = parse_source("call /* optional */ ?.()", LanguageType::JavaScript);

    // `call?.()`
    assert_eq!(expressions.len(), 1);

    // `/* optional */`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), " optional");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Maybe),
    );
    assert_comment_newline_shape(comment, false, false);
}

#[test]
fn test_comment_before_postfix_generic_arguments_attaches_forward() {
    let (parser, expressions) =
        parse_source("call /* marker */ <string>(1)", LanguageType::TypeScript);

    // `call<string>(1)`
    assert_eq!(expressions.len(), 1);

    // `/* marker */`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), " marker");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::LessThan),
    );
    assert_comment_newline_shape(comment, false, false);
}

#[test]
fn test_statement_trailing_line_comments_preserve_raw_statement_boundaries() {
    let (parser, expressions) = parse_source(
        r#"call(); // direct
call?.(); // optional"#,
        LanguageType::JavaScript,
    );

    // `call();`, `call?.();`
    assert_eq!(expressions.len(), 2);

    // `// direct`, `// optional`
    assert_eq!(comments(&parser).len(), 2);

    for (index, trivia) in comments(&parser).iter().copied().enumerate() {
        assert_eq!(
            previous_boundary_token_type(&parser, trivia),
            Some(TokenType::Semicolon)
        );
        assert_comment!(
            parser,
            index,
            CommentKind::Line,
            if index == 0 { "direct" } else { "optional" }
        );
    }
}

#[test]
fn test_if_statement_trailing_line_comments_preserve_raw_if_boundaries() {
    let (parser, expressions) = parse_source(
        r#"if (base.endsWith(".js") || base === `/worker-entries`); // for dev
if (base.endsWith(".js") || base === `/worker-entries`) base = ""; // for dev
if (base.endsWith(".js") || base === `/worker-entries`) a; // for dev"#,
        LanguageType::JavaScript,
    );

    // `if (...) ;`, `if (...) base = "";`, `if (...) a;`
    assert_eq!(expressions.len(), 3);

    // `// for dev`
    assert_eq!(comments(&parser).len(), 3);

    for (index, expression_id) in expressions.iter().copied().enumerate() {
        let expression_id = parser.unwrap_label_expression(expression_id);
        assert_node!(parser.tree, expression_id, Expression::If { .. });

        let trivia = comments(&parser)[index];
        assert_eq!(
            previous_boundary_token_type(&parser, trivia),
            Some(TokenType::Semicolon)
        );
        assert_comment!(parser, index, CommentKind::Line, "for dev");
    }
}

#[test]
fn test_array_element_prefix_comments_preserve_raw_element_boundaries() {
    let (parser, expressions) = parse_source(
        r#"[
  // first
  1,
  // second
  2,
]"#,
        LanguageType::JavaScript,
    );

    // `[1, 2]`
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::ArrayExpression { elements } => {
        // `1`, `2`
        assert_eq!(elements.len(), 2);

        // `// first`
        assert_eq!(comments(&parser).len(), 2);
        let first_trivia = comments(&parser)[0];
        assert_comment_boundary_tokens(
            &parser,
            first_trivia,
            Some(TokenType::OpenBracket),
            Some(TokenType::Literal),
        );
        assert_comment_newline_shape(first_trivia, true, true);
        assert_comment!(parser, 0, CommentKind::Line, "first");

        // `// second`
        let second_trivia = comments(&parser)[1];
        assert_comment_boundary_tokens(
            &parser,
            second_trivia,
            Some(TokenType::Comma),
            Some(TokenType::Literal),
        );
        assert_comment_newline_shape(second_trivia, true, true);
        assert_comment!(parser, 1, CommentKind::Line, "second");
    });
}

#[test]
fn test_inline_separator_comments_preserve_raw_separator_boundaries() {
    let (parser, expressions) = parse_source("[a, /* keep */ b]", LanguageType::JavaScript);

    // `[a, b]`
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::ArrayExpression { elements } => {
        // `a`, `b`
        assert_eq!(elements.len(), 2);

        // `/* keep */`
        assert_eq!(comments(&parser).len(), 1);
        let trivia = comments(&parser)[0];
        assert_comment_boundary_tokens(
            &parser,
            trivia,
            Some(TokenType::Comma),
            Some(TokenType::Identifier),
        );
        assert_comment_newline_shape(trivia, false, false);
        assert_comment!(parser, 0, CommentKind::SingleLineBlock, " keep");
    });
}

#[test]
fn test_trailing_collection_comments_before_close_remain_unowned() {
    let (parser, expressions) = parse_source(
        r#"[
  1
  // tail
]"#,
        LanguageType::JavaScript,
    );

    // `[1]`
    assert_eq!(expressions.len(), 1);

    // `// tail`
    assert_eq!(comments(&parser).len(), 1);
    let trivia = comments(&parser)[0];
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Literal),
        Some(TokenType::CloseBracket),
    );
    assert_comment_newline_shape(trivia, true, true);
    assert_comment!(parser, 0, CommentKind::Line, "tail");
}

#[test]
fn test_lambda_body_prefix_comments_preserve_raw_body_boundaries() {
    let (parser, expressions) = parse_source(
        r#"() =>
  // body
  []"#,
        LanguageType::JavaScript,
    );

    // `() => []`
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body, .. }) => {
            // `// body`
            let _body_id = body.expect("missing lambda body");
            assert_eq!(comments(&parser).len(), 1);
            let trivia = comments(&parser)[0];
            assert_comment_boundary_tokens(
                &parser,
                trivia,
                Some(TokenType::ArrowWide),
                Some(TokenType::OpenBracket),
            );
            assert_comment_newline_shape(trivia, true, true);
            assert_comment!(parser, 0, CommentKind::Line, "body");
        });
    });
}

#[test]
fn test_lambda_inline_body_comments_preserve_raw_body_boundaries() {
    let (parser, expressions) = parse_source("() => /* body */ []", LanguageType::JavaScript);

    // `() => []`
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body, .. }) => {
            // `/* body */`
            let _body_id = body.expect("missing lambda body");
            assert_eq!(comments(&parser).len(), 1);
            let trivia = comments(&parser)[0];
            assert_comment_boundary_tokens(
                &parser,
                trivia,
                Some(TokenType::ArrowWide),
                Some(TokenType::OpenBracket),
            );
            assert_comment_newline_shape(trivia, false, false);
            assert_comment!(parser, 0, CommentKind::SingleLineBlock, " body");
        });
    });
}

#[test]
fn test_doc_comment_and_decorator_emit_raw_comment_and_decorator_node() {
    let (parser, expressions) = parse_source(
        "/** docs */\n@memo\nfunction f() {}",
        LanguageType::TypeScript,
    );

    // `function f() {}`
    assert_eq!(expressions.len(), 1);

    // `@memo`
    assert_eq!(parser.tree.iter_nodes::<Decorator>().count(), 1);

    // `/** docs */`
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), " docs");
}
