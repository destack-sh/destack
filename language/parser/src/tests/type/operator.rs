use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::tests::TestParser;
use crate::{
    TypePosition, TypeStop, assert_expression_path, assert_node, assert_path, assert_string,
};
use tspp_dir::{
    Declaration, Expression, GenericArgument, TypeDeclaration, TypeExpression, TypeLiteral,
};

#[test]
fn test_parse_type_infer_span() {
    let test = TestParser::new("type T = infer Value");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            let infer_id = *value;
            assert_node!(parser.tree, *value, TypeExpression::Infer { name, constraint, .. } => {
                assert_string!(parser, name.expect("expected infer name"), "Value");
                assert!(constraint.is_none());
            });

            let main_span = parser
                .tree
                .get_main_span(infer_id)
                .expect("expected infer main span");
            assert_eq!(parser.span_str(main_span), "Value");
        });
    });
}

#[test]
fn test_parse_type_unary_prefix_operator_span() {
    let test = TestParser::new("type T = keyof Value");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
            assert_eq!(parser.span_str(main_span), "keyof");
        });
    });
}

#[test]
fn test_parse_static_value_call_type_expression() {
    let test = TestParser::new("type T = runtime.sizeOf<Header>()");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::StaticValue { expression } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, generic_arguments, arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "runtime.sizeOf");
                    let main_span = parser
                        .tree
                        .get_main_span(*left)
                        .expect("expected promoted call target main span");
                    assert_eq!(parser.span_str(main_span), "sizeOf");
                    assert_node!(parser.tree, *left, Expression::Member { left, .. } => {
                        let main_span = parser
                            .tree
                            .get_main_span(*left)
                            .expect("expected promoted call root main span");
                        assert_eq!(parser.span_str(main_span), "runtime");
                    });
                    assert_eq!(generic_arguments.len(), 1);
                    assert!(arguments.is_empty());

                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "Header");
                    });
                });
            });
        });
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_type_not_operator_span() {
    let test = TestParser::new("type T = !Unpin");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
            assert_eq!(parser.span_str(main_span), "!");
        });
    });
}

#[test]
fn test_parse_readonly_type_operator_precedence() {
    let test = TestParser::new("type T = readonly string[] | undefined");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TypeExpression::Readonly { target_type } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Array { element } => {
                        assert_node!(parser.tree, *element, TypeExpression::Keyword { value } => {
                            assert_eq!(*value, TypeLiteral::String);
                        });
                    });
                });
                assert_node!(parser.tree, elements[1], TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Undefined);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_unary_postfix_operator_span() {
    let test = TestParser::new("Value as const");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    let unary_id = expr_id;
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "Value");
        assert_node!(parser.tree, *target_type, TypeExpression::Const);
    });

    let main_span = parser
        .tree
        .get_main_span(unary_id)
        .expect("expected type unary postfix operator span");
    assert_eq!(parser.span_str(main_span), "as const");
}

#[test]
fn test_parse_type_binary_operator_span() {
    let test = TestParser::new("Value as Other");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
    assert_eq!(parser.span_str(main_span), "as");
}

#[test]
fn test_parse_type_binary_extends_operator_span() {
    let test = TestParser::new("type T = Left extends Right");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
            assert_eq!(parser.span_str(main_span), "extends");
        });
    });
}

#[test]
fn test_parse_type_binary_satisfies_operator_span() {
    let test = TestParser::new("Value satisfies Constraint");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
    assert_eq!(parser.span_str(main_span), "satisfies");
}

#[test]
fn test_parse_type_binary_implements_operator_span() {
    let test = TestParser::new("type T = Value implements Trait");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
fn test_parse_type_expression_stops_before_in() {
    let test = TestParser::new("Key in Record");
    let mut parser = test.prepare();
    let type_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();

    assert_expression_path!(parser, parser.tree.get(type_id), "Key");

    let next_span = parser.peek_token_span().span;
    assert_eq!(parser.span_str(next_span), "in");
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_type_expression_stops_before_instanceof() {
    let test = TestParser::new("Value instanceof Other");
    let mut parser = test.prepare();
    let type_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();

    // Value
    assert_node!(parser.tree, type_id, TypeExpression::Reference { path, generic_arguments } => {
        assert!(generic_arguments.is_empty());
        assert_path!(parser, *path, "Value");
    });

    // leftover token: instanceof
    let next_span = parser.peek_token_span().span;
    assert_eq!(parser.span_str(next_span), "instanceof");

    // no implicit recovery
    test.assert_no_errors(&parser);
}
