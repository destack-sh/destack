use crate::tests::*;
use crate::{assert_comment, assert_expression_path, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

#[test]
fn test_parse_type_predicate_asserts_target_with_boundary_comment() {
    let source = "type T = asserts value is // predicate-target\nstring";
    let mut test = TestParser::new(source);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
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
    let expr_id = parser.eat_expression(parser.options).unwrap();

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
    let expr_id = parser.eat_expression(parser.options).unwrap();

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

#[test]
fn test_parse_type_unary_postfix_operator_span() {
    let mut test = TestParser::new_with_options("Value as const", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

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
    let expr_id = parser.eat_expression(parser.options).unwrap();

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
    let mut test = TestParser::new_with_options("Value as Other", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

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
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            let binary_id = *value;
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Left");
                });
                assert_node!(parser.tree, *extends_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "Right");
                });
                assert_node!(parser.tree, *then_type, TypeExpression::Missing);
                assert_node!(parser.tree, *else_type, TypeExpression::Missing);
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
fn test_parse_type_binary_satisfies_operator_span() {
    let mut test =
        TestParser::new_with_options("Value satisfies Constraint", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

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
    assert!(parser.eat_expression(parser.options).is_err());
}

#[test]
fn test_parse_type_binary_in_operator_span() {
    let mut test = TestParser::new("type T = Key in Record");
    let mut parser = test.prepare();
    assert!(parser.eat_expression(parser.options).is_err());
}

#[test]
fn test_parse_type_binary_instanceof_operator_span() {
    let mut test = TestParser::new("type T = Value instanceof Other");
    let mut parser = test.prepare();
    assert!(parser.eat_expression(parser.options).is_err());
}
