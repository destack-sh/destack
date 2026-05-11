use crate::tests::*;
use crate::{assert_comment, assert_expression_path, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

#[test]
fn test_parse_type_predicate_asserts_target_with_boundary_comment() {
    let source = "type T = asserts value is // predicate-target\nstring";
    let mut test = TestParser::new(source);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Predicate { asserts, target, .. } => {
                assert!(*asserts);
                let target = target.expect("expected predicate target");
                assert_node!(parser.tree, target, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });
        });
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "predicate-target");
}

#[test]
fn test_parse_type_infer_span() {
    let mut test = TestParser::new("type T = infer Value");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            let infer_id = *value;
            assert_node!(parser.tree, *value, TypeExpression::Infer { name, constraint } => {
                assert_string!(parser, *name, "Value");
                assert!(constraint.is_none());
            });

            let main_span = parser
                .tree
                .get_main_span(infer_id)
                .expect("expected infer main span");
            assert_eq!(parser.get_span_str(main_span), "Value");
        });
    });
}

#[test]
fn test_parse_type_unary_prefix_operator_span() {
    let mut test = TestParser::new("type T = keyof Value");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            let unary_id = *value;
            assert_node!(parser.tree, *value, TypeExpression::KeyOf { target_type } => {
                assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Value");
                });
            });

            let main_span = parser
                .tree
                .get_main_span(unary_id)
                .expect("expected type unary operator span");
            assert_eq!(parser.get_span_str(main_span), "keyof");
        });
    });
}

fn test_parse_typescript_type_expression_keeps_shared_as_identifier() {
    let mut test = TestParser::new_with_language("shared Value", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let type_id = parser
        .with_flags(parser.flags.in_type(), |parser| {
            parser.eat_type_expression()
        })
        .unwrap();

    assert_expression_path!(parser, parser.tree.get(type_id), "shared");

    let next_span = parser.peek().unwrap().span;
    assert_eq!(parser.get_span_str(next_span), "Value");
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_type_not_operator_span() {
    let mut test = TestParser::new("type T = !Unpin");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            let unary_id = *value;
            assert_node!(parser.tree, *value, TypeExpression::Not { target_type } => {
                assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Unpin");
                });
            });

            let main_span = parser
                .tree
                .get_main_span(unary_id)
                .expect("expected type unary operator span");
            assert_eq!(parser.get_span_str(main_span), "!");
        });
    });
}

#[test]
fn test_parse_readonly_type_operator_precedence() {
    let mut test = TestParser::new_with_language(
        "type T = readonly string[] | undefined",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TypeExpression::Readonly { target_type } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Array { element } => {
                        assert_node!(parser.tree, *element, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::String);
                        });
                    });
                });
                assert_node!(parser.tree, elements[1], TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Undefined);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_unary_postfix_operator_span() {
    let mut test = TestParser::new_with_language("Value as const", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    let unary_id = expr_id;
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "Value");
        assert_node!(parser.tree, *target_type, TypeExpression::Const);
    });

    let main_span = parser
        .tree
        .get_main_span(unary_id)
        .expect("expected type unary postfix operator span");
    assert_eq!(parser.get_span_str(main_span), "as const");
}

#[test]
fn test_parse_type_unary_postfix_as_comptime_operator_span() {
    let mut test = TestParser::new("type T = Value as comptime");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            let unary_id = *value;
            let main_span = parser
                .tree
                .get_main_span(unary_id)
                .expect("expected type unary postfix operator span");
            assert_eq!(parser.get_span_str(main_span), "as comptime");
        });
    });
}

#[test]
fn test_parse_type_binary_operator_span() {
    let mut test = TestParser::new_with_language("Value as Other", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    let binary_id = expr_id;
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "Value");
        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
            assert!(generic_arguments.is_empty());
            assert_path!(parser, *path, "Other");
        });
    });

    let main_span = parser
        .tree
        .get_main_span(binary_id)
        .expect("expected type binary operator span");
    assert_eq!(parser.get_span_str(main_span), "as");
}

#[test]
fn test_parse_type_binary_extends_operator_span() {
    let mut test = TestParser::new("type T = Left extends Right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            let binary_id = *value;
            assert_node!(parser.tree, *value, TypeExpression::Extends { left, right } => {
                assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Left");
                });
                assert_node!(parser.tree, *right, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Right");
                });
            });

            let main_span = parser
                .tree
                .get_main_span(binary_id)
                .expect("expected type binary operator span");
            assert_eq!(parser.get_span_str(main_span), "extends");
        });
    });
}

#[test]
fn test_parse_typescript_extends_type_requires_conditional_branches() {
    let mut test = TestParser::new_with_language("Left extends Right", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let error = parser
        .with_flags(parser.flags.in_type(), |parser| {
            parser.eat_type_expression()
        })
        .unwrap_err();

    let (span, node_type, token_type) = error.leaf_content();
    assert_eq!(node_type, None);
    assert_eq!(token_type, Some(TokenType::Maybe));
    assert_eq!(parser.get_span_str(span), "");
}

#[test]
fn test_parse_type_binary_satisfies_operator_span() {
    let mut test =
        TestParser::new_with_language("Value satisfies Constraint", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    let binary_id = expr_id;
    assert_node!(parser.tree, expr_id, Expression::Satisfies { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "Value");
        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
            assert!(generic_arguments.is_empty());
            assert_path!(parser, *path, "Constraint");
        });
    });

    let main_span = parser
        .tree
        .get_main_span(binary_id)
        .expect("expected type binary operator span");
    assert_eq!(parser.get_span_str(main_span), "satisfies");
}

#[test]
fn test_parse_type_binary_implements_operator_span() {
    let mut test = TestParser::new("type T = Value implements Trait");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Implements { left, right } => {
                assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Value");
                });
                assert_node!(parser.tree, *right, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Trait");
                });
            });
        });
    });
}

#[test]
fn test_parse_typescript_type_expression_stops_before_implements() {
    let mut test =
        TestParser::new_with_language("Value implements Trait", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let type_id = parser
        .with_flags(parser.flags.in_type(), |parser| {
            parser.eat_type_expression()
        })
        .unwrap();

    assert_expression_path!(parser, parser.tree.get(type_id), "Value");

    let next_span = parser.peek().unwrap().span;
    assert_eq!(parser.get_span_str(next_span), "implements");
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_type_binary_in_operator_span() {
    let mut test = TestParser::new(r#"type T = "key" in Record"#);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::In { left, right } => {
                assert_node!(parser.tree, *left, TypeExpression::ScalarLiteral { value } => {
                    let ScalarLiteral::String(string_id) = value else {
                        panic!("expected string literal, got {value:?}");
                    };
                    assert_string!(parser, *string_id, "key");
                });
                assert_node!(parser.tree, *right, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Record");
                });
            });
        });
    });
}

#[test]
fn test_parse_typescript_type_expression_stops_before_in() {
    let mut test = TestParser::new_with_language("Key in Record", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let type_id = parser
        .with_flags(parser.flags.in_type(), |parser| {
            parser.eat_type_expression()
        })
        .unwrap();

    assert_expression_path!(parser, parser.tree.get(type_id), "Key");

    let next_span = parser.peek().unwrap().span;
    assert_eq!(parser.get_span_str(next_span), "in");
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_type_expression_stops_before_instanceof() {
    let mut test = TestParser::new("Value instanceof Other");
    let mut parser = test.prepare();
    let type_id = parser
        .with_flags(parser.flags.in_type(), |parser| {
            parser.eat_type_expression()
        })
        .unwrap();

    // Value
    assert_node!(parser.tree, type_id, TypeExpression::Reference { path, generic_arguments } => {
        assert!(generic_arguments.is_empty());
        assert_path!(parser, *path, "Value");
    });

    // leftover token: instanceof
    let next_span = parser.peek().unwrap().span;
    assert_eq!(parser.get_span_str(next_span), "instanceof");

    // no implicit recovery
    test.assert_no_errors(&parser);
}
