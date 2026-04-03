use destack_ast::{
    Annotation, AnnotationPosition, Argument, BinaryOperator, Block, BlockContext, BlockFormat,
    Comment, CommentDirective, CommentStyle, CommentTrivia, Declaration, Decorator, Doc, DocStyle,
    Expression, LocalNodeId, TokenType, TriviaRef, normalize_comment_payload,
};
use destack_source::LanguageType;

use crate::{
    Parser, TestParser, assert_comment_trivia, assert_expression_path, assert_node, assert_string,
};

const NO_TOKEN_INDEX: u32 = u32::MAX;

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
    parser.attach_trivia();
    (parser, block_id)
}

fn comment_text(parser: &Parser, comment_id: LocalNodeId<Comment>) -> String {
    let source = parser.get_span_str(parser.tree.get_span(comment_id));
    normalize_comment_payload(source).into_owned()
}

fn doc_text(parser: &Parser, doc_id: LocalNodeId<Doc>) -> String {
    parser
        .strings
        .get(parser.tree.get(doc_id).string)
        .to_string()
}

fn boundary_token_type(parser: &Parser, token_index: u32) -> Option<TokenType> {
    if token_index == NO_TOKEN_INDEX {
        return None;
    }

    parser
        .tokens()
        .get(token_index as usize)
        .map(|token| token.token.ty)
}

fn assert_comment_boundary_tokens(
    parser: &Parser,
    trivia: CommentTrivia,
    token_before: Option<TokenType>,
    token_after: Option<TokenType>,
) {
    assert_eq!(
        boundary_token_type(parser, trivia.boundary.token_before),
        token_before
    );
    assert_eq!(
        boundary_token_type(parser, trivia.boundary.token_after),
        token_after
    );
}

fn assert_comment_newline_shape(
    trivia: CommentTrivia,
    has_leading_newline: bool,
    has_trailing_newline: bool,
) {
    assert_eq!(
        trivia.boundary.newlines.has_leading_newline(),
        has_leading_newline
    );
    assert_eq!(
        trivia.boundary.newlines.has_trailing_newline(),
        has_trailing_newline
    );
}

#[test]
fn test_parse_runs_attach_trivia_for_comments() {
    let (parser, expressions) = parse_source("// lead\nvalue", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    assert_eq!(parser.tree.blank_trivia().len(), 0);

    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), "lead");

    let expression_annotations = parser.tree.get_annotations(expressions[0].id);
    assert!(expression_annotations.is_empty());
}

#[test]
fn test_parse_without_trivia_leaves_trivia_empty_until_attach() {
    let mut test = TestParser::new_with_options("// lead\nvalue\n\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expressions = parser.parse_without_trivia();
    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.tree.comment_trivia().len(), 0);
    assert_eq!(parser.tree.blank_trivia().len(), 0);

    parser.attach_trivia();
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    assert_eq!(parser.tree.blank_trivia().len(), 1);
}

#[test]
fn test_attach_trivia_on_direct_entrypoint_emits_output() {
    let mut test = TestParser::new_with_options("// lead\nvalue\n\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.tree.comment_trivia().len(), 0);
    assert_eq!(parser.tree.blank_trivia().len(), 0);

    parser.attach_trivia();

    assert_eq!(parser.tree.comment_trivia().len(), 1);
    assert_eq!(parser.tree.blank_trivia().len(), 1);
}

#[test]
fn test_attach_trivia_is_idempotent() {
    let mut test = TestParser::new_with_options("// lead\nvalue\n\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expressions = parser.parse_without_trivia();
    assert_eq!(expressions.len(), 2);

    parser.attach_trivia();
    let first_comment_count = parser.tree.comment_trivia().len();
    let first_blank_count = parser.tree.blank_trivia().len();

    parser.attach_trivia();
    assert_eq!(parser.tree.comment_trivia().len(), first_comment_count);
    assert_eq!(parser.tree.blank_trivia().len(), first_blank_count);
}

#[test]
fn test_comment_only_file_gets_stub_expression_and_trivia() {
    let (parser, expressions) = parse_source("// only", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Stub);

    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert!(annotations.is_empty());

    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), "only");
}

#[test]
fn test_comment_trivia_directive_classification() {
    let (parser, expressions) = parse_source(
        "// @ts-ignore\na\n/* @__PURE__ */\nb\n// prettier-ignore\nc\n// prettier-ignore-start\nd\n// prettier-ignore-end\ne",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 5);
    assert_eq!(parser.tree.comment_trivia().len(), 5);

    let first = parser.tree.comment_trivia()[0];
    let second = parser.tree.comment_trivia()[1];
    let third = parser.tree.comment_trivia()[2];
    let fourth = parser.tree.comment_trivia()[3];
    let fifth = parser.tree.comment_trivia()[4];

    assert_eq!(comment_text(&parser, first.comment), "@ts-ignore");
    assert_eq!(first.directive, CommentDirective::TypeScript);

    assert_eq!(comment_text(&parser, second.comment), " @__PURE__");
    assert_eq!(second.directive, CommentDirective::Pure);

    assert_eq!(comment_text(&parser, third.comment), "prettier-ignore");
    assert_eq!(third.directive, CommentDirective::FormatIgnore);

    assert_eq!(
        comment_text(&parser, fourth.comment),
        "prettier-ignore-start"
    );
    assert_eq!(fourth.directive, CommentDirective::FormatIgnoreStart);

    assert_eq!(comment_text(&parser, fifth.comment), "prettier-ignore-end");
    assert_eq!(fifth.directive, CommentDirective::FormatIgnoreEnd);
}

#[test]
fn test_comment_trivia_normalizes_payload_and_style() {
    let (parser, expressions) =
        parse_source("// line\n/* block */\nvalue", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 2);

    let first = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, first.comment), "line");
    assert_node!(parser.tree, first.comment, Comment { style, .. } => {
        assert_eq!(*style, CommentStyle::Slash);
    });

    let second = parser.tree.comment_trivia()[1];
    assert_eq!(comment_text(&parser, second.comment), " block");
    assert_node!(parser.tree, second.comment, Comment { style, .. } => {
        assert_eq!(*style, CommentStyle::Star);
    });
}

#[test]
fn test_doc_comments_attach_semantically_and_skip_comment_trivia() {
    let (parser, expressions) = parse_source("/// docs\nfunction f() {}", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 0);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        let expression_annotations = parser.tree.get_annotations(expression_id.id);
        assert!(expression_annotations.is_empty());

        let declaration_annotations = parser.tree.get_annotations(declaration_id.id);
        assert_eq!(declaration_annotations.len(), 1);
        assert_node!(parser.tree, declaration_annotations[0], Annotation::Doc { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePrefix);
            assert_node!(parser.tree, *node, Doc { string, style } => {
                assert_eq!(*style, DocStyle::Slash);
                assert_string!(parser, *string, "docs");
            });
        });
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
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePrefix);
                assert_node!(parser.tree, *node, Doc { string, style } => {
                    assert_eq!(*style, DocStyle::Star);
                    assert_string!(parser, *string, " parameter-doc");
                });
            });
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
        let declaration_annotations = parser.tree.get_annotations(declaration_id.id);
        assert!(declaration_annotations.is_empty());

        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);

                let left_annotations = parser.tree.get_annotations(left.id);
                assert_eq!(left_annotations.len(), 1);
                assert_node!(parser.tree, left_annotations[0], Annotation::Doc { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Doc { style, .. } => {
                        assert_eq!(*style, DocStyle::Star);
                    });

                    let text = doc_text(&parser, *node);
                    assert!(text.contains("keep-doc"));
                });
            });
        });
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
        assert_eq!(argument_annotations.len(), 1);
        assert_node!(parser.tree, argument_annotations[0], Annotation::Doc { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePrefix);
            assert_node!(parser.tree, *node, Doc { string, style } => {
                assert_eq!(*style, DocStyle::Star);
                assert_string!(parser, *string, " argument-doc");
            });
        });

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
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), "seam");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, true);
}

#[test]
fn test_comment_after_satisfies_keyword_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "value satisfies // seam\nRecord<A, B>",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), "seam");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, true);
}

#[test]
fn test_comment_before_as_keyword_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "const value = source /* seam */ as number",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), " seam");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_after_as_keyword_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "const value = source as // seam\nnumber",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), "seam");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, true);
}

#[test]
fn test_multiline_block_comment_between_as_and_const_emits_unowned_seam_trivia() {
    let (parser, expressions) =
        parse_source("1 as /*\nblock-comment\n*/ const", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), "\nblock-comment\n");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Identifier),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, false);
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
    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), "<- keep-marker");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Identifier),
        Some(TokenType::Semicolon),
    );
    assert_comment_newline_shape(trivia, true, true);
}

#[test]
fn test_comment_after_if_head_emits_unowned_seam_trivia() {
    let (parser, expressions) =
        parse_source("if (ready) // if-head\nrun()", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), "if-head");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::CloseParenthesis),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(trivia, false, true);
}

#[test]
fn test_comment_between_ternary_then_and_colon_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "const result = cond ? left /* left-note */ : right",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), " left-note");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Identifier),
        Some(TokenType::Colon),
    );
    assert_comment_newline_shape(trivia, false, false);
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
    assert_eq!(parser.tree.comment_trivia().len(), 2);

    let first_trivia = parser.tree.comment_trivia()[0];
    assert_eq!(
        comment_text(&parser, first_trivia.comment),
        "comment before cond2"
    );
    assert_comment_boundary_tokens(
        &parser,
        first_trivia,
        Some(TokenType::CloseBrace),
        Some(TokenType::Identifier),
    );
    assert_comment_newline_shape(first_trivia, true, true);

    let second_trivia = parser.tree.comment_trivia()[1];
    assert_eq!(
        comment_text(&parser, second_trivia.comment),
        "comment before else"
    );
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
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(
        comment_text(&parser, trivia.comment),
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

    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let trivia = parser.tree.comment_trivia()[0];
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

    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let trivia = parser.tree.comment_trivia()[0];
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
    assert_eq!(parser.tree.comment_trivia().len(), 2);
    let trivia = parser.tree.comment_trivia()[1];
    assert_eq!(comment_text(&parser, trivia.comment), "");
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
    assert_eq!(parser.tree.comment_trivia().len(), 0);
    assert_eq!(parser.tree.iter_nodes::<Doc>().count(), 1);
    assert_eq!(parser.tree.iter_nodes::<Decorator>().count(), 1);

    let doc_annotation = parser
        .tree
        .iter_nodes::<Annotation>()
        .find(|annotation_id| matches!(parser.tree.get(*annotation_id), Annotation::Doc { .. }))
        .expect("missing doc annotation");
    assert_node!(parser.tree, doc_annotation, Annotation::Doc { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePrefix);
        assert_node!(parser.tree, *node, Doc { string, style } => {
            assert_eq!(*style, DocStyle::Star);
            assert_string!(parser, *string, " docs");
        });
    });

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
fn test_empty_doc_block_comment_falls_back_to_comment_trivia() {
    let (parser, expressions) = parse_source("/**/\nvalue", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let trivia = parser.tree.comment_trivia()[0];
    assert_node!(parser.tree, trivia.comment, Comment { style, .. } => {
        assert_eq!(*style, CommentStyle::Star);
    });

    assert_eq!(parser.tree.iter_nodes::<Doc>().count(), 0);
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
fn test_blank_trivia_emits_for_blank_line_runs_only() {
    let (parser, expressions) = parse_source("a\n\nb\nc\n\n\nd", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 4);
    assert_eq!(parser.tree.blank_trivia().len(), 2);

    let first_blank = parser.tree.blank_trivia()[0];
    let second_blank = parser.tree.blank_trivia()[1];

    assert_eq!(parser.tree.get(first_blank.blank).lines, 1);
    assert_eq!(parser.tree.get(second_blank.blank).lines, 2);
}

#[test]
fn test_trivia_refs_preserve_source_order() {
    let (parser, expressions) = parse_source("// first\n\n// second\nx", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 2);
    assert_eq!(parser.tree.blank_trivia().len(), 1);

    let refs = parser.tree.trivia_refs();
    assert_eq!(refs.len(), 3);
    assert_eq!(refs[0], TriviaRef::Comment(0));
    assert_eq!(refs[1], TriviaRef::Comment(1));
    assert_eq!(refs[2], TriviaRef::Blank(0));
}

#[test]
fn test_comments_and_blanks_are_not_semantic_annotations() {
    let (parser, expressions) = parse_source("a // tail\n\nb", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    assert_eq!(parser.tree.blank_trivia().len(), 1);

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
    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), " empty");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::OpenBrace),
        Some(TokenType::CloseBrace),
    );
    assert_comment_newline_shape(trivia, false, false);
}

#[test]
fn test_comment_between_parameter_name_and_type_emits_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "function f(x /* a */ : number) {}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), " a");
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Identifier),
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
    assert_eq!(parser.tree.comment_trivia().len(), 4);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
            assert_eq!(members.len(), 1);

            let trivia = parser.tree.comment_trivia();

            assert_eq!(comment_text(&parser, trivia[0].comment), "comment before entity");
            assert_eq!(boundary_token_type(&parser, trivia[0].boundary.token_after), Some(TokenType::At));
            assert_comment_newline_shape(trivia[0], true, true);

            assert_eq!(comment_text(&parser, trivia[1].comment), "comment after entity");
            assert!(boundary_token_type(&parser, trivia[1].boundary.token_after).is_some());
            assert_comment_newline_shape(trivia[1], true, true);

            assert_eq!(comment_text(&parser, trivia[2].comment), "comment before foo");
            assert!(boundary_token_type(&parser, trivia[2].boundary.token_after).is_some());
            assert_comment_newline_shape(trivia[2], true, true);

            assert_eq!(comment_text(&parser, trivia[3].comment), "comment after foo");
            assert_eq!(
                boundary_token_type(&parser, trivia[3].boundary.token_after),
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
    assert_eq!(parser.tree.comment_trivia().len(), 2);

    let trivia = parser.tree.comment_trivia();

    assert_comment_boundary_tokens(
        &parser,
        trivia[0],
        Some(TokenType::Identifier),
        Some(TokenType::OpenParenthesis),
    );
    assert_comment_newline_shape(trivia[0], false, true);
    assert_comment_trivia!(parser, 0, CommentStyle::Slash, "direct");

    assert_comment_boundary_tokens(
        &parser,
        trivia[1],
        Some(TokenType::Identifier),
        Some(TokenType::Maybe),
    );
    assert_comment_newline_shape(trivia[1], false, true);
    assert_comment_trivia!(parser, 1, CommentStyle::Slash, "optional");
}

#[test]
fn test_empty_call_boundary_block_comments_preserve_raw_call_gap_boundaries() {
    let (parser, expressions) = parse_source(
        r#"call/* direct */()
call/* optional */?.()"#,
        LanguageType::JavaScript,
    );

    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.tree.comment_trivia().len(), 2);

    let trivia = parser.tree.comment_trivia();

    assert_comment_boundary_tokens(
        &parser,
        trivia[0],
        Some(TokenType::Identifier),
        Some(TokenType::OpenParenthesis),
    );
    assert_comment_newline_shape(trivia[0], false, false);
    assert_comment_trivia!(parser, 0, CommentStyle::Star, " direct");

    assert_comment_boundary_tokens(
        &parser,
        trivia[1],
        Some(TokenType::Identifier),
        Some(TokenType::Maybe),
    );
    assert_comment_newline_shape(trivia[1], false, false);
    assert_comment_trivia!(parser, 1, CommentStyle::Star, " optional");
}

#[test]
fn test_statement_trailing_line_comments_preserve_raw_statement_boundaries() {
    let (parser, expressions) = parse_source(
        r#"call(); // direct
call?.(); // optional"#,
        LanguageType::JavaScript,
    );

    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.tree.comment_trivia().len(), 2);

    for (index, trivia) in parser.tree.comment_trivia().iter().copied().enumerate() {
        assert_eq!(
            boundary_token_type(&parser, trivia.boundary.token_before),
            Some(TokenType::Semicolon)
        );
        assert_comment_trivia!(
            parser,
            index,
            CommentStyle::Slash,
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
    assert_eq!(parser.tree.comment_trivia().len(), 3);

    for (index, expression_id) in expressions.iter().copied().enumerate() {
        let expression_id = parser.unwrap_labelled_expression(expression_id);
        assert_node!(parser.tree, expression_id, Expression::If { .. });

        let trivia = parser.tree.comment_trivia()[index];
        assert_eq!(
            boundary_token_type(&parser, trivia.boundary.token_before),
            Some(TokenType::Semicolon)
        );
        assert_comment_trivia!(parser, index, CommentStyle::Slash, "for dev");
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
    assert_eq!(parser.tree.comment_trivia().len(), 2);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::ArrayExpression { elements } => {
        assert_eq!(elements.len(), 2);

        let first_trivia = parser.tree.comment_trivia()[0];
        assert_comment_boundary_tokens(
            &parser,
            first_trivia,
            Some(TokenType::OpenBracket),
            Some(TokenType::Literal),
        );
        assert_comment_newline_shape(first_trivia, true, true);
        assert_comment_trivia!(parser, 0, CommentStyle::Slash, "first");

        let second_trivia = parser.tree.comment_trivia()[1];
        assert_comment_boundary_tokens(
            &parser,
            second_trivia,
            Some(TokenType::Comma),
            Some(TokenType::Literal),
        );
        assert_comment_newline_shape(second_trivia, true, true);
        assert_comment_trivia!(parser, 1, CommentStyle::Slash, "second");
    });
}

#[test]
fn test_inline_separator_comments_preserve_raw_separator_boundaries() {
    let (parser, expressions) = parse_source("[a, /* keep */ b]", LanguageType::JavaScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::ArrayExpression { elements } => {
        assert_eq!(elements.len(), 2);

        let trivia = parser.tree.comment_trivia()[0];
        assert_comment_boundary_tokens(
            &parser,
            trivia,
            Some(TokenType::Comma),
            Some(TokenType::Identifier),
        );
        assert_comment_newline_shape(trivia, false, false);
        assert_comment_trivia!(parser, 0, CommentStyle::Star, " keep");
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
    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let trivia = parser.tree.comment_trivia()[0];
    assert_comment_boundary_tokens(
        &parser,
        trivia,
        Some(TokenType::Literal),
        Some(TokenType::CloseBracket),
    );
    assert_comment_newline_shape(trivia, true, true);
    assert_comment_trivia!(parser, 0, CommentStyle::Slash, "tail");
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
    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
            let _body_id = body.expect("missing lambda body");
            let trivia = parser.tree.comment_trivia()[0];
            assert_comment_boundary_tokens(
                &parser,
                trivia,
                Some(TokenType::ArrowWide),
                Some(TokenType::OpenBracket),
            );
            assert_comment_newline_shape(trivia, true, true);
            assert_comment_trivia!(parser, 0, CommentStyle::Slash, "body");
        });
    });
}

#[test]
fn test_lambda_inline_body_comments_preserve_raw_body_boundaries() {
    let (parser, expressions) = parse_source("() => /* body */ []", LanguageType::JavaScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
            let _body_id = body.expect("missing lambda body");
            let trivia = parser.tree.comment_trivia()[0];
            assert_comment_boundary_tokens(
                &parser,
                trivia,
                Some(TokenType::ArrowWide),
                Some(TokenType::OpenBracket),
            );
            assert_comment_newline_shape(trivia, false, false);
            assert_comment_trivia!(parser, 0, CommentStyle::Star, " body");
        });
    });
}

#[test]
fn test_docs_and_decorators_remain_semantic_annotations() {
    let (parser, expressions) = parse_source(
        "/** docs */\n@memo\nfunction f() {}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.iter_nodes::<Doc>().count(), 1);
    assert_eq!(parser.tree.iter_nodes::<Decorator>().count(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 0);
}

#[test]
fn test_doc_comment_attaches_to_class_extends_expression() {
    let (parser, expressions) = parse_source(
        "class Box extends /** @type {{new (): Base}} */ (baseFactory()) {}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.iter_nodes::<Doc>().count(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, .. } => {
            let extends_types = heritage.extends_types.as_ref().expect("missing extends");
            assert_eq!(extends_types.len(), 1);
            let extends_id = extends_types[0];
            let annotations = parser.tree.get_annotations(extends_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePrefix);
                assert_node!(parser.tree, *node, Doc { style, .. } => {
                    assert_eq!(*style, DocStyle::Star);
                });
            });
        });
    });
}
