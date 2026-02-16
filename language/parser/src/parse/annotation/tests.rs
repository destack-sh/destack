use destack_ast::{
    Annotation, AnnotationPosition, Argument, BlockFormat, Comment, CommentDirective, CommentStyle,
    Declaration, Decorator, Doc, DocStyle, Expression, LocalNodeId, TriviaRef,
};
use destack_source::LanguageType;

use crate::{Parser, TestParser, assert_expression_path, assert_node, assert_path, assert_string};

fn parse_source(source: &str, language: LanguageType) -> (Parser, Vec<LocalNodeId<Expression>>) {
    let mut test = TestParser::new_with_options(source, language);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    (parser, expressions)
}

fn comment_text(parser: &Parser, comment_id: LocalNodeId<Comment>) -> String {
    parser
        .strings
        .get(parser.tree.get(comment_id).string)
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

    let mut doc_annotations = Vec::new();
    for annotation_id in parser.tree.iter_nodes::<Annotation>() {
        if let Annotation::Doc { node, position } = parser.tree.get(annotation_id) {
            doc_annotations.push((*node, *position));
        }
    }

    assert_eq!(doc_annotations.len(), 1);
    assert_eq!(doc_annotations[0].1, AnnotationPosition::LinePrefix);
    assert_node!(parser.tree, doc_annotations[0].0, Doc { string, style } => {
        assert_eq!(*style, DocStyle::Slash);
        assert_string!(parser, *string, "docs");
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

    let has_doc_annotation = parser
        .tree
        .iter_nodes::<Annotation>()
        .any(|annotation_id| matches!(parser.tree.get(annotation_id), Annotation::Doc { .. }));
    assert!(!has_doc_annotation);
}

#[test]
fn test_decorator_attaches_to_function_declaration() {
    let (parser, expressions) = parse_source("@memo\nfunction f() {}", LanguageType::TypeScript);

    assert_eq!(expressions.len(), 1);

    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "memo");
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
fn test_docs_and_decorators_remain_semantic_annotations() {
    let (parser, expressions) = parse_source(
        "/** docs */\n@memo\nfunction f() {}",
        LanguageType::TypeScript,
    );

    assert_eq!(expressions.len(), 1);
    let mut documentation_nodes = Vec::new();
    let mut decorator_nodes = Vec::new();
    for annotation_id in parser.tree.iter_nodes::<Annotation>() {
        match parser.tree.get(annotation_id) {
            Annotation::Doc { node, .. } => {
                documentation_nodes.push(*node);
            }
            Annotation::Decorator { node, .. } => {
                decorator_nodes.push(*node);
            }
        }
    }

    assert_eq!(documentation_nodes.len(), 1);
    assert_eq!(decorator_nodes.len(), 1);

    assert_node!(parser.tree, documentation_nodes[0], Doc { string, style } => {
        assert_eq!(*style, DocStyle::Star);
        assert_string!(parser, *string, " docs");
    });
    assert_node!(parser.tree, decorator_nodes[0], Decorator { expression } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "memo");
    });

    assert_eq!(parser.tree.comment_trivia().len(), 0);
}
