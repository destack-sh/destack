use crate::tests::*;
use crate::{Parser, ParserSettings, assert_comment, assert_expression_path, assert_node};
use destack_ast::*;
use destack_source::{LanguageType, NodeSpanBoundary, NodeSpanType};

#[test]
fn test_parse_type_union_line_comment_on_rhs_separator_owner() {
    let source = "type Value = First | // union-line\nSecond | Third";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let _right_element_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 3);
                    elements[1]
                }
                node => panic!("expected union type, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 3);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "First");
                assert_expression_path!(parser, parser.tree.get(elements[1]), "Second");
                assert_expression_path!(parser, parser.tree.get(elements[2]), "Third");

                let annotations = parser.tree.get_decorators(elements[0].id);
                assert!(annotations.is_empty());
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "union-line");
    assert_eq!(
        parser.tree.comments()[0].position,
        CommentPosition::Trailing
    );
    assert_eq!(parser.tree.comments()[0].attached_to, 0);
}

#[test]
fn test_parse_type_intersection_line_comment_on_rhs_separator_owner() {
    let source = "type Value = First & // intersection-line\nSecond";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Intersection { elements } => {
                assert_eq!(elements.len(), 2);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "First");
                assert_expression_path!(parser, parser.tree.get(elements[1]), "Second");

                let annotations = parser.tree.get_decorators(elements[0].id);
                assert!(annotations.is_empty());
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "intersection-line");
    assert_eq!(
        parser.tree.comments()[0].position,
        CommentPosition::Trailing
    );
    assert_eq!(parser.tree.comments()[0].attached_to, 0);
}

#[test]
fn test_parse_type_reference_prefix_decorator_on_owner() {
    let source = r#"type Value = @addrspace("shared") &Buffer"#;
    let mut test = TestParser::new_with_options(source, LanguageType::Destack);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ReferenceOf { target_type, .. } => {
                assert_expression_path!(parser, parser.tree.get(*target_type), "Buffer");

                let annotations = parser.tree.get_decorators(value.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Decorator { position, .. } => {
                    assert_eq!(*position, DecoratorPosition::BlockPrefix);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_union_line_comment_on_leading_separator_owner() {
    let source = "type Value = | // leading-union\nFirst | Second";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    // `type Value = | ...`, grab the union node
    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let _first_element_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    elements[0]
                }
                node => panic!("expected union type, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "First");
                assert_expression_path!(parser, parser.tree.get(elements[1]), "Second");

                // `First | Second`, the semantic union starts at the first arm
                let union_span = parser.tree.get_span(*value);
                assert_eq!(parser.get_span_str(union_span), "First | Second");

                // `| // leading-union\n`, the explicit prefix stays on the union
                let leading_span = parser
                    .tree
                    .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::Leading))
                    .expect("missing leading union container span");
                assert_eq!(
                    parser.get_span_str(leading_span),
                    "| // leading-union\n",
                );

                // `|`, the leading operator is addressable separately
                let leading_operator_span = parser
                    .tree
                    .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator))
                    .expect("missing leading union operator span");
                assert_eq!(parser.get_span_str(leading_operator_span), "|");

                let head_span = parser
                    .tree
                    .get_head_span(*value)
                    .expect("missing leading union head span");
                assert_eq!(parser.get_span_str(head_span), "First");
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "leading-union");
    assert_eq!(
        parser.tree.comments()[0].position,
        CommentPosition::Trailing
    );
    assert_eq!(parser.tree.comments()[0].attached_to, 0);
}

#[test]
fn test_parse_type_union_block_comment_on_leading_separator_owner() {
    let source = "type Value = | /* leading-union */ First | Second";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let first_element_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    let leading_span = parser
                        .tree
                        .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::Leading));
                    assert_eq!(
                        leading_span.map(|span| parser.get_span_str(span)),
                        Some("| /* leading-union */ ")
                    );
                    elements[0]
                }
                node => panic!("expected union type, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " leading-union");
    assert_eq!(parser.tree.comments()[0].position, CommentPosition::Leading);
    assert_eq!(
        parser.get_span_str(parser.tree.get_span(first_element_id)),
        "First"
    );
    assert_eq!(
        parser.tree.comments()[0].attached_to,
        parser.tree.get_span(first_element_id).start
    );
}

#[test]
fn test_parse_type_union_doc_comment_on_leading_separator_owner() {
    let source = "type Value = | /** leading-union */ First | Second";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let first_element_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    let leading_span = parser
                        .tree
                        .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::Leading));
                    assert_eq!(
                        leading_span.map(|span| parser.get_span_str(span)),
                        Some("| /** leading-union */ ")
                    );
                    elements[0]
                }
                node => panic!("expected union type, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " leading-union");
    assert_eq!(parser.tree.comments()[0].position, CommentPosition::Leading);
    assert_eq!(
        parser.get_span_str(parser.tree.get_span(first_element_id)),
        "First"
    );
    assert_eq!(
        parser.tree.comments()[0].attached_to,
        parser.tree.get_span(first_element_id).start
    );
}

#[test]
fn test_parse_type_union_multiline_doc_comment_on_leading_separator_owner() {
    let source = "type Value = | /**\n * leading-union\n */ First | Second";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let first_element_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    elements[0]
                }
                node => panic!("expected union type, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_eq!(parser.tree.comments().len(), 1);
    assert_eq!(parser.tree.comments()[0].kind, CommentKind::MultiLineBlock);
    assert_eq!(parser.tree.comments()[0].position, CommentPosition::Leading);
    assert_eq!(
        parser.get_span_str(parser.tree.get_span(first_element_id)),
        "First"
    );
    assert_eq!(
        parser.tree.comments()[0].attached_to,
        parser.tree.get_span(first_element_id).start
    );
}

#[test]
fn test_parse_type_union_multiline_doc_comment_before_first_arm_line() {
    let source = "type Value =\n  | /**\n   * leading-union\n   */\n  First | Second";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let first_element_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    elements[0]
                }
                node => panic!("expected union type, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_eq!(parser.tree.comments().len(), 1);
    assert_eq!(parser.tree.comments()[0].kind, CommentKind::MultiLineBlock);
    assert_eq!(
        parser.tree.comments()[0].position,
        CommentPosition::Trailing,
        "the lexer keeps this comment token-local, and the formatter must still surface it before the first arm",
    );

    assert_eq!(
        normalize_comment_payload(parser.get_span_str(parser.tree.comments()[0].span)).trim(),
        "leading-union"
    );
    assert_eq!(
        parser.get_span_str(parser.tree.get_span(first_element_id)),
        "First"
    );
}

#[test]
fn test_parse_type_union_doc_comment_before_leading_separator_owner() {
    let source = "type Value = (/** leading-union */ | First | Second)";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let union_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Parenthesized { expression } => *expression,
                node => panic!("expected parenthesized type, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_node!(parser.tree, union_id, TypeExpression::Union { elements } => {
        assert_eq!(elements.len(), 2);
    });
    // `| First`, the comment before the separator still targets the separator
    let leading_operator_span = parser
        .tree
        .get_side_span(
            union_id,
            NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator),
        )
        .expect("missing leading union operator span");

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " leading-union");
    assert_eq!(parser.tree.comments()[0].position, CommentPosition::Leading);
    assert_eq!(
        parser.tree.comments()[0].attached_to,
        leading_operator_span.start,
    );
}

#[test]
fn test_parse_type_union_single_arm_with_leading_separator() {
    let source = "type Value = | First";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 1);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "First");
            });
        });
    });
}

#[test]
fn test_parse_type_comment_after_open_parenthesis_attaches_to_inner_leading() {
    let mut test =
        TestParser::new_with_options("type Value = (/* keep */ string)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let inner_type_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Parenthesized { expression } => *expression,
                node => panic!("expected parenthesized type, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " keep");
    assert_eq!(parser.tree.comments()[0].position, CommentPosition::Leading);
    assert_eq!(
        parser.tree.comments()[0].attached_to,
        parser.tree.get_span(inner_type_id).start,
    );
}

#[test]
fn test_parse_type_alias_doc_comment_before_leading_separator_owner() {
    let source = "type Value = /** leading-union */ | First | Second";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let union_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => *value,
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_node!(parser.tree, union_id, TypeExpression::Union { elements } => {
        assert_eq!(elements.len(), 2);
    });
    // `| First`, the alias comment before the separator targets the separator
    let leading_operator_span = parser
        .tree
        .get_side_span(
            union_id,
            NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator),
        )
        .expect("missing leading union operator span");

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " leading-union");
    assert_eq!(parser.tree.comments()[0].position, CommentPosition::Leading);
    assert_eq!(
        parser.tree.comments()[0].attached_to,
        leading_operator_span.start,
    );
}

#[test]
fn test_parse_type_union_line_comment_before_operator_on_left_arm_owner() {
    let mut test = TestParser::new_with_options(
        "type Value = First // left-union\n| Second",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let _left_element_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    elements[0]
                }
                node => panic!("expected union type, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "First");

                let annotations = parser.tree.get_decorators(elements[0].id);
                assert!(annotations.is_empty());
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "left-union");
    assert_eq!(
        parser.tree.comments()[0].position,
        CommentPosition::Trailing
    );
    assert_eq!(parser.tree.comments()[0].attached_to, 0);
}

#[test]
fn test_parse_declarator_type_comment_on_declared_type_leading_owner() {
    let mut test =
        TestParser::new_with_options("let value: /* anno */ string", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let declared_type_id = match parser.tree.get(expression_id) {
        Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);

            match parser.tree.get(declarators[0]) {
                Declarator {
                    ty: Some(declared_type),
                    ..
                } => *declared_type,
                node => panic!("expected declarator with type, got {node:?}"),
            }
        }
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " anno");
    assert_eq!(parser.tree.comments()[0].position, CommentPosition::Leading);
    assert_eq!(
        parser.tree.comments()[0].attached_to,
        parser.tree.get_span(declared_type_id).start,
    );
}

#[test]
fn test_parse_type_argument_comment_on_argument_leading_owner() {
    let mut test =
        TestParser::new_with_options("type Box = Foo</* a */ string>", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let argument_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Reference {
                    generic_arguments, ..
                } => {
                    assert_eq!(generic_arguments.len(), 1);
                    generic_arguments[0]
                }
                node => panic!("expected type reference, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " a");
    assert_eq!(parser.tree.comments()[0].position, CommentPosition::Leading);
    assert_eq!(
        parser.tree.comments()[0].attached_to,
        parser.tree.get_span(argument_id).start,
    );
}

#[test]
fn test_parse_type_argument_line_comment_on_argument_leading_owner() {
    let mut test = TestParser::new_with_options(
        "type Box = Foo<\n  // a\n  string>",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    let argument_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Type(TypeDeclaration { value, .. }) => match parser.tree.get(*value) {
                TypeExpression::Reference {
                    generic_arguments, ..
                } => {
                    assert_eq!(generic_arguments.len(), 1);
                    generic_arguments[0]
                }
                node => panic!("expected type reference, got {node:?}"),
            },
            node => panic!("expected type declaration, got {node:?}"),
        },
        node => panic!("expected declaration expression, got {node:?}"),
    };

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "a");
    assert_eq!(parser.tree.comments()[0].position, CommentPosition::Leading);
    assert_eq!(
        parser.tree.comments()[0].attached_to,
        parser.tree.get_span(argument_id).start,
    );
}

#[test]
fn test_parse_type_union_line_comment_between_members_after_leading_separator() {
    let mut test = TestParser::new_with_options(
        "type A6 = /*1*/\n| A\n// A comment to force break\n| B;",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "A");
                assert_expression_path!(parser, parser.tree.get(elements[1]), "B");
            });
        });
    });
}

#[test]
fn test_parse_type_intersection_line_comment_before_operator_on_left_arm_owner() {
    let mut test = TestParser::new_with_options(
        "type Value = First // left-intersection\n& Second",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Intersection { elements } => {
                assert_eq!(elements.len(), 2);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "First");

                let annotations = parser.tree.get_decorators(elements[0].id);
                assert!(annotations.is_empty());
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "left-intersection");
}

#[test]
fn test_parse_type_intersection_line_comment_on_leading_separator_owner() {
    let source = "type Value = & // leading-intersection\nFirst & Second";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Intersection { elements } => {
                assert_eq!(elements.len(), 2);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "First");
                assert_expression_path!(parser, parser.tree.get(elements[1]), "Second");

                // `First & Second`, the semantic intersection starts at the first arm
                let intersection_span = parser.tree.get_span(*value);
                assert_eq!(parser.get_span_str(intersection_span), "First & Second");

                // `& // leading-intersection\n`, the explicit prefix stays on the intersection
                let leading_span = parser
                    .tree
                    .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::Leading))
                    .expect("missing leading intersection container span");
                assert_eq!(
                    parser.get_span_str(leading_span),
                    "& // leading-intersection\n",
                );

                let head_span = parser
                    .tree
                    .get_head_span(*value)
                    .expect("missing leading intersection head span");
                assert_eq!(parser.get_span_str(head_span), "First");
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "leading-intersection");
    assert_eq!(
        parser.tree.comments()[0].position,
        CommentPosition::Trailing
    );
    assert_eq!(parser.tree.comments()[0].attached_to, 0);
}

#[test]
fn test_parse_type_union_object_arm_trailing_comments_stay_on_each_arm_owner() {
    let source = "type Mixed = null // null-arm\n| {\n  y: number;\n  z: string;\n} // object-arm\n| void // void-arm\n;";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 3);
                assert_node!(parser.tree, elements[0], TypeExpression::ScalarLiteral { value } => {
                    assert_eq!(*value, ScalarLiteral::Null);
                });
                assert_node!(parser.tree, elements[1], TypeExpression::Object { .. });
                assert_node!(parser.tree, elements[2], TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Void);
                });

                let null_annotations = parser.tree.get_decorators(elements[0].id);
                assert!(null_annotations.is_empty());

                let object_annotations = parser.tree.get_decorators(elements[1].id);
                assert!(object_annotations.is_empty());

                let void_annotations = parser.tree.get_decorators(elements[2].id);
                assert!(void_annotations.is_empty());

                let object_arm_source = parser.get_span_str(parser.tree.get_span(elements[1]));
                let void_arm_source = parser.get_span_str(parser.tree.get_span(elements[2]));

                assert_eq!(object_arm_source, "{\n  y: number;\n  z: string;\n}");
                assert_eq!(void_arm_source, "void");
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 3);
    assert_comment!(parser, 0, CommentKind::Line, "null-arm");
    assert_comment!(parser, 1, CommentKind::Line, "object-arm");
    assert_comment!(parser, 2, CommentKind::Line, "void-arm");
    assert_eq!(
        parser.get_span_str(parser.tree.comments()[1].span),
        "// object-arm"
    );
    assert_eq!(
        parser.get_span_str(parser.tree.comments()[2].span),
        "// void-arm"
    );
}

#[test]
fn test_parse_type_union_last_arm_span_stops_before_trailing_line_comment() {
    let source = "type Value = First | Second // second-tail\n;";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);

                let last_arm_span = parser.tree.get_span(elements[1]);
                let union_span = parser.tree.get_span(*value);

                assert_eq!(parser.get_span_str(last_arm_span), "Second");
                assert_eq!(parser.get_span_str(union_span), "First | Second");
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "second-tail");
    assert_eq!(
        parser.get_span_str(parser.tree.comments()[0].span),
        "// second-tail"
    );
}

#[test]
fn test_parse_type_union_last_arm_span_stops_before_trailing_line_comment_without_semicolon() {
    let source = "type Value =\n  | A\n  | B // last-union\n";
    let mut test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);

                let last_arm_span = parser.tree.get_span(elements[1]);
                let union_span = parser.tree.get_span(*value);

                assert_eq!(parser.get_span_str(last_arm_span), "B");
                assert_eq!(parser.get_span_str(union_span), "A\n  | B");
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "last-union");
    assert_eq!(
        parser.get_span_str(parser.tree.comments()[0].span),
        "// last-union"
    );
}

#[test]
fn test_parse_without_parenthesized_wrappers_trims_type_union_last_arm() {
    let source = "type Value = First | Second // second-tail\n;";
    let test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = Parser::lex_file_with_settings(
        test.file.clone(),
        test.language,
        ParserSettings {
            preserve_parenthesized_wrappers: false,
            ..ParserSettings::default()
        },
    );
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);

                let last_arm_span = parser.tree.get_span(elements[1]);
                let union_span = parser.tree.get_span(*value);

                assert_eq!(parser.get_span_str(last_arm_span), "Second");
                assert_eq!(parser.get_span_str(union_span), "First | Second");
            });
        });
    });
}

#[test]
fn test_parse_without_parenthesized_wrappers_keeps_inner_type_span() {
    let source = "type Box = (/* keep */ string);";
    let test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = Parser::lex_file_with_settings(
        test.file.clone(),
        test.language,
        ParserSettings {
            preserve_parenthesized_wrappers: false,
            ..ParserSettings::default()
        },
    );
    let expressions = parser.parse();
    parser.attach_comments();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            let value_span = parser.tree.get_span(*value);
            let leading_span = parser
                .tree
                .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::Leading))
                .expect("missing type leading span");
            let comment = parser.tree.comments()[0];

            assert_eq!(parser.get_span_str(value_span), "string");
            assert_eq!(parser.get_span_str(leading_span), "/* keep */ ");
            assert_eq!(comment.attached_to, value_span.start);
        });
    });
}

#[test]
fn test_parse_without_parenthesized_wrappers_keeps_leading_union_chain_head() {
    let source = "type Value = | (A | B);";
    let test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = Parser::lex_file_with_settings(
        test.file.clone(),
        test.language,
        ParserSettings {
            preserve_parenthesized_wrappers: false,
            ..ParserSettings::default()
        },
    );
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 1);

                let outer_span = parser.tree.get_span(*value);
                let outer_head_span = parser
                    .tree
                    .get_head_span(*value)
                    .expect("missing outer union head span");
                let leading_span = parser
                    .tree
                    .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::Leading))
                    .expect("missing leading union container span");
                let leading_operator_span = parser
                    .tree
                    .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator))
                    .expect("missing leading union operator span");

                assert_eq!(parser.get_span_str(outer_span), "A | B");
                assert_eq!(parser.get_span_str(outer_head_span), "A");
                assert_eq!(parser.get_span_str(leading_span), "| (");
                assert_eq!(parser.get_span_str(leading_operator_span), "|");

                assert_node!(parser.tree, elements[0], TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_expression_path!(parser, parser.tree.get(elements[0]), "A");
                    assert_expression_path!(parser, parser.tree.get(elements[1]), "B");
                });
            });
        });
    });
}

#[test]
fn test_parse_without_parenthesized_wrappers_keeps_leading_intersection_chain_head() {
    let source = "type Value = & (A & B);";
    let test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = Parser::lex_file_with_settings(
        test.file.clone(),
        test.language,
        ParserSettings {
            preserve_parenthesized_wrappers: false,
            ..ParserSettings::default()
        },
    );
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Intersection { elements } => {
                assert_eq!(elements.len(), 1);

                let outer_span = parser.tree.get_span(*value);
                let outer_head_span = parser
                    .tree
                    .get_head_span(*value)
                    .expect("missing outer intersection head span");
                let leading_span = parser
                    .tree
                    .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::Leading))
                    .expect("missing leading intersection container span");
                let leading_operator_span = parser
                    .tree
                    .get_side_span(*value, NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator))
                    .expect("missing leading intersection operator span");

                assert_eq!(parser.get_span_str(outer_span), "A & B");
                assert_eq!(parser.get_span_str(outer_head_span), "A");
                assert_eq!(parser.get_span_str(leading_span), "& (");
                assert_eq!(parser.get_span_str(leading_operator_span), "&");

                assert_node!(parser.tree, elements[0], TypeExpression::Intersection { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_expression_path!(parser, parser.tree.get(elements[0]), "A");
                    assert_expression_path!(parser, parser.tree.get(elements[1]), "B");
                });
            });
        });
    });
}

#[test]
fn test_parse_union_doc_block_comment_attaches_to_first_union_arm() {
    let mut test = TestParser::new_with_options(
        "export type Value = /** union-doc\n */\n| { ok: true }\n| { ok: false; value: bigint | null };",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { export, value, .. }) => {
            assert!(export.is_some());
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);

                let value_annotations = parser.tree.get_decorators(value.id);
                assert!(value_annotations.is_empty());
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    let comment = parser.tree.comments()[0];
    assert_eq!(
        normalize_comment_payload(parser.get_span_str(comment.span)),
        "union-doc\n"
    );
}

/// Record mapped remap and value separator ownership and template head spans.
#[test]
fn test_parse_type_mapped_expression_records_separator_and_template_head_spans() {
    let mut test = TestParser::new_with_options(
        "type Paths<T> = {\n  [K in keyof T as // remap-note\n    `get${Capitalize<K & string>}`]: // value-note\n    () => T[K]\n}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, value: mapped_value, .. } => {
                let key_remap = parameter.key_remap.expect("missing key remap");
                let source_type_span = parser.tree.get_span(parameter.source_type);
                let key_remap_span = parser.tree.get_span(key_remap);
                let key_remap_head_span = parser
                    .tree
                    .get_head_span(key_remap)
                    .expect("missing remap template head span");

                assert_eq!(parser.get_span_str(source_type_span), "keyof T");
                assert_eq!(parser.get_span_str(key_remap_span), "`get${Capitalize<K & string>}`");
                assert_eq!(parser.get_span_str(key_remap_head_span), "Capitalize");

                assert_node!(parser.tree, key_remap, TypeExpression::TemplateLiteral { .. } => {
                    let comment = parser
                        .tree
                        .comments()
                        .iter()
                        .copied()
                        .find(|comment| comment.is_line())
                        .expect("missing remap comment");

                    let _ = key_remap;
                    assert_eq!(parser.get_span_str(comment.span), "// remap-note");
                    assert_eq!(
                        normalize_comment_payload(parser.get_span_str(comment.span)),
                        "remap-note",
                    );
                });

                let _value_comment = parser
                    .tree
                    .comments()
                    .iter()
                    .copied()
                    .find(|comment| normalize_comment_payload(parser.get_span_str(comment.span)) == "value-note")
                    .expect("missing value comment");

                let _ = mapped_value;
            });
        });
    });
}

/// Keep mapped remap block comments inside the `as` boundary range.
#[test]
fn test_parse_type_mapped_expression_records_remap_block_comment_boundary() {
    let mut test = TestParser::new_with_options(
        "type Paths<T> = {\n  [K in keyof T as /* remap-note */\n    `get${Capitalize<K & string>}`]: () => T[K]\n}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, .. } => {
                let key_remap = parameter.key_remap.expect("missing key remap");
                let key_remap_span = parser.tree.get_span(key_remap);
                let key_remap_token_start = parser
                    .tree
                    .get_main_span(key_remap)
                    .map(|span| span.start)
                    .unwrap_or(key_remap_span.start);
                let comment = parser
                    .tree
                    .comments()
                    .iter()
                    .copied()
                    .find(|comment| comment.is_block())
                    .expect("missing remap block comment");
                let previous_token = parser
                    .tokens()
                    .iter()
                    .rev()
                    .find(|token| {
                        token.span.end <= key_remap_span.start
                            && !matches!(
                                token.token.ty,
                                TokenType::Newline
                                    | TokenType::Whitespace
                                    | TokenType::LineComment
                                    | TokenType::BlockComment
                                    | TokenType::DocLineComment
                                    | TokenType::DocBlockComment
                                    | TokenType::End
                            )
                    })
                    .copied()
                    .expect("missing token before remap");

                assert_eq!(
                    normalize_comment_payload(parser.get_span_str(comment.span)).trim(),
                    "remap-note"
                );
                assert_eq!(parser.get_span_str(previous_token.span), "as");
                assert!(comment.span.start >= previous_token.span.end);
                assert!(comment.span.end <= key_remap_token_start);
            });
        });
    });
}

/// Record mapped field trailing ownership after the value terminator.
#[test]
fn test_parse_type_mapped_expression_records_trailing_comment_owner() {
    let mut test = TestParser::new_with_options(
        "type Paths<T> = {\n  [K in keyof T as Capitalize<K & string>]: () => T[K]; // remap-note\n}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { .. } => {
                let comment = parser
                    .tree
                    .comments()
                    .iter()
                    .copied()
                    .find(|comment| comment.is_line())
                    .expect("missing remap comment");

                let _ = value;
                assert_eq!(normalize_comment_payload(parser.get_span_str(comment.span)), "remap-note");
            });
        });
    });
}

#[test]
fn test_parse_without_parenthesized_wrappers_trims_mapped_union_last_arm() {
    let source =
        "type Value<T> = {\n  [K in keyof T]:\n    | T[K] // arm-a\n    | undefined // arm-b\n}";
    let test = TestParser::new_with_options(source, LanguageType::TypeScript);
    let mut parser = Parser::lex_file_with_settings(
        test.file.clone(),
        test.language,
        ParserSettings {
            preserve_parenthesized_wrappers: false,
            ..ParserSettings::default()
        },
    );
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_labelled_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { value: mapped_value, .. } => {
                assert_node!(parser.tree, *mapped_value, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);

                    let last_arm_span = parser.tree.get_span(elements[1]);
                    let union_span = parser.tree.get_span(*mapped_value);

                    assert_eq!(parser.get_span_str(last_arm_span), "undefined");
                    assert_eq!(parser.get_span_str(union_span), "T[K] // arm-a\n    | undefined");
                });
            });
        });
    });
}

/// Keep template interpolation trailing line comments as raw trailing trivia.
#[test]
fn test_parse_type_template_interpolation_records_trailing_line_comment_boundary() {
    let mut test = TestParser::new_with_options(
        "type Paths<T> = {\n  [K in keyof T as `get${Capitalize<\n    K & string\n  > // remap-note\n  }`]: () => T[K]\n}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, .. } => {
                let key_remap = parameter.key_remap.expect("missing key remap");

                assert_node!(parser.tree, key_remap, TypeExpression::TemplateLiteral { spans, .. } => {
                    let interpolation_type = spans[0];
                    let interpolation_span = parser.tree.get_span(interpolation_type);
                    let template_span = parser.tree.get_span(key_remap);
                    let comment = parser
                        .tree
                        .comments()
                        .iter()
                        .copied()
                        .find(|comment| normalize_comment_payload(parser.get_span_str(comment.span)) == "remap-note")
                        .expect("missing remap comment");

                    assert_eq!(comment.position, CommentPosition::Trailing);
                    assert!(comment.span.start >= interpolation_span.end);
                    assert!(comment.span.end <= template_span.end);
                });
            });
        });
    });
}
