use destack_ast::{
    Annotation, AnnotationPosition, Argument, BinaryOperator, Block, BlockContext, BlockFormat,
    Comment, CommentKind, Declaration, Decorator, Expression, LocalNodeId, TokenType,
    normalize_comment_payload,
};
use destack_source::LanguageType;

use crate::{Parser, TestParser, assert_comment, assert_expression_path, assert_node};

fn parse_source(source: &str, language: LanguageType) -> (Parser, Vec<LocalNodeId<Expression>>) {
    let mut test = TestParser::new_with_options(source, language);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    (parser, expressions)
}

fn parse_block_source(source: &str, language: LanguageType) -> (Parser, LocalNodeId<Block>) {
    let mut test = TestParser::new_with_options(source, language);
    let mut parser = test.prepare();
    let block_id = parser
        .eat_block(BlockContext::Expression)
        .expect("expected block expression in test source");
    parser.attach_comments();
    (parser, block_id)
}

fn comment_text(parser: &Parser, comment: Comment) -> String {
    let source = parser.get_span_str(comment.span);
    normalize_comment_payload(source).into_owned()
}

fn previous_boundary_token_type(parser: &Parser, comment: Comment) -> Option<TokenType> {
    parser
        .tokens()
        .iter()
        .rev()
        .find(|token| {
            token.span.end <= comment.span.start
                && !matches!(token.token.ty, TokenType::Newline | TokenType::End)
        })
        .copied()
        .map(|token| token.token.ty)
}

fn next_boundary_token_type(parser: &Parser, comment: Comment) -> Option<TokenType> {
    if comment.is_leading() && comment.attached_to != 0 {
        return parser
            .tokens()
            .iter()
            .find(|token| token.span.start == comment.attached_to)
            .copied()
            .map(|token| token.token.ty);
    }

    parser
        .tokens()
        .iter()
        .find(|token| {
            token.span.start >= comment.span.end
                && !matches!(token.token.ty, TokenType::Newline | TokenType::End)
        })
        .copied()
        .map(|token| token.token.ty)
}

fn assert_comment_boundary_tokens(
    parser: &Parser,
    comment: Comment,
    token_before: Option<TokenType>,
    token_after: Option<TokenType>,
) {
    assert_eq!(previous_boundary_token_type(parser, comment), token_before);
    assert_eq!(next_boundary_token_type(parser, comment), token_after);
}

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

fn comments(parser: &Parser) -> &[Comment] {
    parser.tree.comments()
}

#[test]
fn test_parse_runs_attach_comments_for_comments() {
    let (parser, expressions) = parse_source("// lead\nvalue", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comments().len(), 1);

    let comment = parser.tree.comments()[0];
    assert_eq!(comment_text(&parser, comment), "lead");

    let expression_annotations = parser.tree.get_annotations(expressions[0].id);
    assert!(expression_annotations.is_empty());
}

#[test]
fn test_parse_without_trivia_leaves_comments_empty_until_attach() {
    let mut test = TestParser::new_with_options("// lead\nvalue\n\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expressions = parser.parse_without_trivia();
    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.tree.comments().len(), 0);

    parser.attach_comments();
    assert_eq!(parser.tree.comments().len(), 1);
}

#[test]
fn test_attach_comments_on_direct_entrypoint_emits_output() {
    let mut test = TestParser::new_with_options("// lead\nvalue\n\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.tree.comments().len(), 0);

    parser.attach_comments();

    assert_eq!(parser.tree.comments().len(), 1);
}

#[test]
fn test_attach_comments_is_idempotent() {
    let mut test = TestParser::new_with_options("// lead\nvalue\n\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expressions = parser.parse_without_trivia();
    assert_eq!(expressions.len(), 2);

    parser.attach_comments();
    let first_comment_count = parser.tree.comments().len();

    parser.attach_comments();
    assert_eq!(parser.tree.comments().len(), first_comment_count);
}

#[test]
fn test_comment_only_file_gets_stub_expression_and_trivia() {
    let (parser, expressions) = parse_source("// only", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Stub);

    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert!(annotations.is_empty());

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

    assert_eq!(expressions.len(), 5);
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
fn test_comment_trivia_normalizes_payload_and_style() {
    let (parser, expressions) =
        parse_source("// line\n/* block */\nvalue", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 2);

    let first = comments(&parser)[0];
    assert_eq!(comment_text(&parser, first), "line");
    assert_eq!(first.kind, CommentKind::Line);

    let second = comments(&parser)[1];
    assert_eq!(comment_text(&parser, second), " block");
    assert_eq!(second.kind, CommentKind::SingleLineBlock);
}

#[test]
fn test_doc_comments_attach_semantically_and_skip_raw_comments() {
    let (parser, expressions) = parse_source("/// docs\nfunction f() {}", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        let expression_annotations = parser.tree.get_annotations(expression_id.id);
        assert!(expression_annotations.is_empty());

        let declaration_annotations = parser.tree.get_annotations(declaration_id.id);
        assert!(declaration_annotations.is_empty());

        let comment = comments(&parser)[0];
        assert!(comment.is_leading());
        assert_eq!(comment_text(&parser, comment), "docs");
        assert_eq!(comment.attached_to, parser.tree.get_span(*declaration_id).start);
    });
}

#[test]
fn test_doc_comment_attaches_to_parameter() {
    let (parser, expressions) = parse_source(
        "function demo(/** parameter-doc */ value: number): void {}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            assert_eq!(signature.dynamic_parameters.len(), 1);
            let parameter_id = signature.dynamic_parameters[0];
            let annotations = parser.tree.get_annotations(parameter_id.id);
            assert!(annotations.is_empty());

            assert_eq!(comments(&parser).len(), 1);
            let comment = comments(&parser)[0];
            assert!(comment.is_leading());
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

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);

                let left_annotations = parser.tree.get_annotations(left.id);
                assert!(left_annotations.is_empty());

                assert_eq!(comments(&parser).len(), 1);
                let comment = comments(&parser)[0];
                assert!(comment.is_trailing());
                assert!(parser.get_span_str(comment.span).starts_with("/**"));
                assert!(comment_text(&parser, comment).contains("keep-doc"));
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value: _, .. } => {
            let comment = comments(&parser)[0];
            assert_eq!(comment_text(&parser, comment), " keep");
            assert!(comment.is_leading());
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
fn test_comment_after_open_parenthesis_attaches_to_parenthesized_wrapper() {
    let (parser, expressions) = parse_source("(/* keep */ value)", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Parenthesized { expression } => {
        let comment = comments(&parser)[0];
        assert_eq!(comment_text(&parser, comment), " keep");
        assert_ne!(expression_id.id, expression.id);
        assert!(comment.is_leading());
        assert_comment_boundary_tokens(
            &parser,
            comment,
            Some(TokenType::OpenParenthesis),
            Some(TokenType::Identifier),
        );
        assert_comment_newline_shape(comment, false, false);
    });
}

#[test]
fn test_doc_comment_attaches_to_call_argument() {
    let (parser, expressions) =
        parse_source("run(/** argument-doc */ value)", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Call { dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        let argument_id = dynamic_arguments[0];
        let argument_annotations = parser.tree.get_annotations(argument_id.id);
        assert!(argument_annotations.is_empty());

        assert_eq!(comments(&parser).len(), 1);
        let comment = comments(&parser)[0];
        assert!(comment.is_leading());
        assert_eq!(comment_text(&parser, comment), " argument-doc");
        assert_comment_boundary_tokens(
            &parser,
            comment,
            Some(TokenType::OpenParenthesis),
            Some(TokenType::Identifier),
        );

        assert_node!(parser.tree, argument_id, Argument::Positional { value, .. } => {
            let value_annotations = parser.tree.get_annotations(value.id);
            assert!(value_annotations.is_empty());
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}

#[test]
fn test_comment_between_export_and_declaration_head_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "export // seam\nasync function f() {}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "seam");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, true);
}

#[test]
fn test_comment_after_satisfies_keyword_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "value satisfies // seam\nRecord<A, B>",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "seam");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, true);
}

#[test]
fn test_comment_before_as_keyword_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "const value = source /* seam */ as number",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), " seam");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, false);
}

#[test]
fn test_comment_after_as_keyword_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "const value = source as // seam\nnumber",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);
    let comment = comments(&parser)[0];
    assert_eq!(comment_text(&parser, comment), "seam");
    assert_comment_boundary_tokens(
        &parser,
        comment,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(comment, false, true);
}

#[test]
fn test_multiline_block_comment_between_as_and_const_emits_unowned_seam_trivia() {
    let (parser, expressions) =
        parse_source("1 as /*\nblock-comment\n*/ const", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
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
fn test_variable_trailing_marker_comment_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "declare const PAGE_PATH: string\n  //<- keep-marker\n;(()=>{})()",
        LanguageType::TypeScript,
    );

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert!(
        !expressions.is_empty(),
        "expected at least one parsed expression"
    );
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
fn test_comment_after_if_head_emits_unowned_seam_trivia() {
    let (parser, expressions) =
        parse_source("if (ready) // if-head\nrun()", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
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
fn test_comment_between_ternary_then_and_colon_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "const result = cond ? left /* left-note */ : right",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let comment = comments(&parser)[0];
    assert!(comment.is_leading());
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let comment = comments(&parser)[0];
    assert!(comment.is_leading());
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
fn test_comments_between_if_chain_branches_emit_unowned_seam_trivia() {
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 2);

    let first_trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, first_trivia), "comment before cond2");
    assert_comment_boundary_tokens(
        &parser,
        first_trivia,
        Some(TokenType::CloseBrace),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(first_trivia, true, true);

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
fn test_multiline_trailing_block_comment_inside_block_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "{\n    const X = 1 /* some comment\n    * over multiple lines yo       */\n}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
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
fn test_multiline_trailing_block_comment_on_eat_block_entrypoint_emits_unowned_seam_trivia() {
    let (parser, _block_id) = parse_block_source(
        "{\n    const X = 1 /* some comment\n    * over multiple lines yo       */\n}",
        LanguageType::TypeScript,
    );

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
fn test_multiline_trailing_block_comment_on_eat_block_entrypoint_destack_emits_unowned_seam_trivia()
{
    let (parser, _block_id) = parse_block_source(
        "{\n    const X = 1 /* some comment\n    * over multiple lines yo       */\n}",
        LanguageType::Destack,
    );

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

    assert_eq!(expressions.len(), 1);
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
fn test_doc_and_decorator_attach_to_function_declaration_in_source_order() {
    let (parser, expressions) = parse_source(
        "/** docs */\n@memo\nfunction f() {}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);
    assert_eq!(parser.tree.iter_nodes::<Decorator>().count(), 1);

    let doc_comment = comments(&parser)[0];
    assert!(doc_comment.is_leading());
    assert_eq!(comment_text(&parser, doc_comment), " docs");

    let decorator_annotation = parser
        .tree
        .iter_nodes::<Annotation>()
        .find(|annotation_id| {
            matches!(
                parser.tree.get(*annotation_id),
                Annotation::Decorator { .. }
            )
        })
        .expect("missing decorator annotation");
    assert_node!(parser.tree, decorator_annotation, Annotation::Decorator { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "memo");
        });
    });
}

#[test]
fn test_empty_doc_block_comment_falls_back_to_raw_comments() {
    let (parser, expressions) = parse_source("/**/\nvalue", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let comment = comments(&parser)[0];
    assert!(comment.is_block());
}

#[test]
fn test_decorator_attaches_to_function_declaration() {
    let (parser, expressions) = parse_source("@memo\nfunction f() {}", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        let annotations = parser.tree.get_annotations(declaration_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "memo");
            });
        });
    });
}

#[test]
fn test_decorator_attaches_to_struct_declaration_inside_block() {
    let (parser, expressions) = parse_source(
        "{\n    @memo\n    struct Entity {}\n}",
        LanguageType::Destack,
    );

    assert_eq!(expressions.len(), 1);
    let block_expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, block_expression_id, Expression::Block(block_id) => {
        let block = parser.tree.get(*block_id);
        assert_eq!(block.leading_expressions.len(), 1);
        assert!(block.tail_expression.is_none());

        let declaration_expression_id =
            parser.unwrap_labelled_expression(block.leading_expressions[0]);
        assert_node!(parser.tree, declaration_expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Struct { .. });

            let declaration_annotations = parser.tree.get_annotations(declaration_id.id);
            assert_eq!(declaration_annotations.len(), 1);
            assert_node!(parser.tree, declaration_annotations[0], Annotation::Decorator { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "memo");
                });
            });

            let declaration_expression_annotations =
                parser.tree.get_annotations(declaration_expression_id.id);
            assert!(declaration_expression_annotations.is_empty());
        });
    });
}

#[test]
fn test_decorator_on_expression_attaches_directly_without_wrapper() {
    let (parser, expressions) = parse_source("@memo\nrun()", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Call { .. } => {
        let annotations = parser.tree.get_annotations(expression_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "memo");
            });
        });
    });
}

#[test]
fn test_decorator_attaches_to_parameter() {
    let (parser, expressions) = parse_source(
        "function demo(@guard value: number): void {}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            assert_eq!(signature.dynamic_parameters.len(), 1);
            let parameter_id = signature.dynamic_parameters[0];
            let annotations = parser.tree.get_annotations(parameter_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, .. } => {
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "guard");
                });
            });
        });
    });
}

#[test]
fn test_decorator_attaches_to_call_argument() {
    let (parser, expressions) = parse_source("run(@memo value)", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Call { dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        let argument_id = dynamic_arguments[0];
        let annotations = parser.tree.get_annotations(argument_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, .. } => {
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "memo");
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

    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.tree.comments().len(), 1);

    let first_annotations = parser.tree.get_annotations(expressions[0].id);
    let second_annotations = parser.tree.get_annotations(expressions[1].id);
    assert!(first_annotations.is_empty());
    assert!(second_annotations.is_empty());
}

#[test]
fn test_comment_inside_function_body_attaches_to_block_infix() {
    let (parser, expressions) =
        parse_source("function foo() { /* empty */ }", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    assert!(trivia.is_leading());
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    assert!(trivia.is_leading());
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let trivia = comments(&parser)[0];
    assert_eq!(comment_text(&parser, trivia), " a");
    assert!(trivia.is_leading());
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Maybe),
        Some(TokenType::Colon),
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 4);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
            assert_eq!(members.len(), 1);
            let member_id = members[0];
            let member_annotations = parser.tree.get_annotations(member_id.id);
            assert_eq!(member_annotations.len(), 2);

            assert_node!(parser.tree, member_annotations[0], Annotation::Decorator { node, .. } => {
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "entity");
                });
            });

            assert_node!(parser.tree, member_annotations[1], Annotation::Decorator { node, .. } => {
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "foo");
                        assert_eq!(dynamic_arguments.len(), 3);
                    });
                });
            });

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

    assert_eq!(expressions.len(), 2);
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

    assert_eq!(expressions.len(), 2);
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let comment = comments(&parser)[0];
    assert!(comment.is_leading());
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
fn test_comment_before_postfix_static_arguments_attaches_forward() {
    let (parser, expressions) =
        parse_source("call /* marker */ <string>(1)", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let comment = comments(&parser)[0];
    assert!(comment.is_leading());
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

    assert_eq!(expressions.len(), 2);
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
fn test_if_shell_trailing_line_comments_preserve_raw_if_boundaries() {
    let (parser, expressions) = parse_source(
        r#"if (base.endsWith(".js") || base === `/worker-entries`); // for dev
if (base.endsWith(".js") || base === `/worker-entries`) base = ""; // for dev
if (base.endsWith(".js") || base === `/worker-entries`) a; // for dev"#,
        LanguageType::JavaScript,
    );

    assert_eq!(expressions.len(), 3);
    assert_eq!(comments(&parser).len(), 3);

    for (index, expression_id) in expressions.iter().copied().enumerate() {
        let expression_id = parser.unwrap_labelled_expression(expression_id);
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 2);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::ArrayExpression { elements } => {
        assert_eq!(elements.len(), 2);

        let first_trivia = comments(&parser)[0];
        assert_comment_boundary_tokens(
            &parser,
            first_trivia,
            Some(TokenType::OpenBracket),
            Some(TokenType::Literal),
        );
        assert_comment_newline_shape(first_trivia, true, true);
        assert_comment!(parser, 0, CommentKind::Line, "first");

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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::ArrayExpression { elements } => {
        assert_eq!(elements.len(), 2);

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

    assert_eq!(expressions.len(), 1);
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
            let _body_id = body.expect("missing lambda body");
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
            let _body_id = body.expect("missing lambda body");
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

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.iter_nodes::<Decorator>().count(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let comment = comments(&parser)[0];
    assert!(comment.is_leading());
    assert_eq!(comment_text(&parser, comment), " docs");
}

#[test]
fn test_doc_comment_attaches_to_class_extends_expression() {
    let (parser, expressions) = parse_source(
        "class Box extends /** @type {{new (): Base}} */ (baseFactory()) {}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(comments(&parser).len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, .. } => {
            let extends_types = heritage.extends_types.as_ref().expect("missing extends");
            assert_eq!(extends_types.len(), 1);
            let extends_id = extends_types[0];
            let annotations = parser.tree.get_annotations(extends_id.id);
            assert!(annotations.is_empty());

            let comment = comments(&parser)[0];
            assert!(comment.is_leading());
            assert_comment_boundary_tokens(
                &parser,
                comment,
                Some(TokenType::Identifier),
                Some(TokenType::OpenParenthesis),
            );
            assert!(parser.get_span_str(comment.span).starts_with("/**"));
        });
    });
}
