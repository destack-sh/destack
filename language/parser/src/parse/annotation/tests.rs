use destack_ast::{
    Annotation, AnnotationPosition, Argument, BinaryOperator, BlockContext, BlockFormat, Comment,
    CommentDirective, CommentStyle, Declaration, Decorator, Doc, DocStyle, Expression, LocalNodeId,
    TriviaRef,
};
use destack_source::LanguageType;

use crate::{Parser, TestParser, assert_expression_path, assert_node, assert_path, assert_string};

fn parse_source(source: &str, language: LanguageType) -> (Parser, Vec<LocalNodeId<Expression>>) {
    let mut test = TestParser::new_with_options(source, language);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    (parser, expressions)
}

fn parse_block_source(
    source: &str,
    language: LanguageType,
) -> (Parser, LocalNodeId<destack_ast::Block>) {
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
    destack_ast::normalize_comment_payload(source).into_owned()
}

fn doc_text(parser: &Parser, doc_id: LocalNodeId<Doc>) -> String {
    parser
        .strings
        .get(parser.tree.get(doc_id).string)
        .to_string()
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

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
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
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
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
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
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
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
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
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
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
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
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
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
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
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
}

#[test]
fn test_multiline_block_comment_between_as_and_const_emits_unowned_seam_trivia() {
    let (parser, expressions) =
        parse_source("1 as /*\nblock-comment\n*/ const", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), "\nblock-comment\n");
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
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
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
    assert_eq!(trivia.target_node, None);
}

#[test]
fn test_comment_after_if_head_emits_unowned_seam_trivia() {
    let (parser, expressions) =
        parse_source("if (ready) // if-head\nrun()", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 1);
    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(comment_text(&parser, trivia.comment), "if-head");
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
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
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
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
    assert_eq!(first_trivia.target_node, None);
    assert_eq!(first_trivia.position, AnnotationPosition::BlockInfix);

    let second_trivia = parser.tree.comment_trivia()[1];
    assert_eq!(
        comment_text(&parser, second_trivia.comment),
        "comment before else"
    );
    assert_eq!(second_trivia.target_node, None);
    assert_eq!(second_trivia.position, AnnotationPosition::BlockInfix);
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
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
}

#[test]
fn test_multiline_trailing_block_comment_on_eat_block_entrypoint_emits_unowned_seam_trivia() {
    let (parser, _block_id) = parse_block_source(
        "{\n    const X = 1 /* some comment\n    * over multiple lines yo       */\n}",
        LanguageType::TypeScript,
    );

    assert_eq!(parser.tree.comment_trivia().len(), 1);

    let trivia = parser.tree.comment_trivia()[0];
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
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
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
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
    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
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
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
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
    let block_expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, block_expression_id, Expression::Block(block_id) => {
        assert_eq!(parser.tree.get(*block_id).expressions.len(), 1);

        let declaration_expression_id =
            parser.unwrap_statement_expression(parser.tree.get(*block_id).expressions[0]);
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
fn test_decorator_on_statement_moves_to_statement_wrapper() {
    let (parser, expressions) = parse_source("@memo\nrun()", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Statement(expression_id) => {
        let statement_annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(statement_annotations.len(), 1);
        assert_node!(parser.tree, statement_annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "memo");
            });
        });

        let value_annotations = parser.tree.get_annotations(expression_id.id);
        assert!(value_annotations.is_empty());
    });
}

#[test]
fn test_decorator_attaches_to_parameter() {
    let (parser, expressions) = parse_source(
        "function demo(@guard value: number): void {}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
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
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
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

    assert_eq!(trivia.target_node, None);
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
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
    assert_eq!(trivia.position, AnnotationPosition::BlockInfix);
    assert_eq!(trivia.target_node, None);
}

#[test]
fn test_comments_around_decorator_chain_emit_unowned_seam_trivia() {
    let (parser, expressions) = parse_source(
        "{\n    // comment before entity\n    @entity\n    // comment after entity\n    // comment before foo\n    @foo(1, 2, 3)\n    // comment after foo\n    struct Entity {}\n}",
        LanguageType::Destack,
    );

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.tree.comment_trivia().len(), 4);

    let first = parser.tree.comment_trivia()[0];
    let second = parser.tree.comment_trivia()[1];
    let third = parser.tree.comment_trivia()[2];
    let fourth = parser.tree.comment_trivia()[3];

    assert_eq!(first.position, AnnotationPosition::BlockInfix);
    assert_eq!(second.position, AnnotationPosition::BlockInfix);
    assert_eq!(third.position, AnnotationPosition::BlockInfix);
    assert_eq!(fourth.position, AnnotationPosition::BlockInfix);
    assert_eq!(first.target_node, None);
    assert_eq!(second.target_node, None);
    assert_eq!(third.target_node, None);
    assert_eq!(fourth.target_node, None);
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

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
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
