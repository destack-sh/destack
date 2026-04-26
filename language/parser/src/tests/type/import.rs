use crate::tests::*;
use crate::{assert_node, assert_path, assert_string};
use destack_ast::*;

#[test]
fn test_parse_type_import_expression_with_generic_arguments() {
    let mut test = TestParser::new("type T = import(\"mod\").Type<string, number>");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = import("mod").Type<string, number>
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Import { target, arguments, qualifier, generic_arguments } => {
                assert_node!(parser.tree, *target, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                    assert_string!(parser, *string_id, "mod");
                });
                assert!(arguments.is_empty());
                assert_path!(parser, qualifier.as_ref().unwrap(), "Type");
                assert_eq!(generic_arguments.len(), 2);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::String);
                        });
                });
                assert_node!(parser.tree, generic_arguments[1], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                });
            });
        });
    });
}

/// Parse type import expressions with attributes.
#[test]
fn test_parse_type_import_expression_with_attributes() {
    let mut test =
        TestParser::new("type T = import(\"vite\", { with: { \"resolution-mode\": \"import\" } })");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Import { target, arguments, .. } => {
                assert_node!(parser.tree, *target, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                    assert_string!(parser, *string_id, "vite");
                });
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::ObjectExpression { .. });
                });
            });
        });
    });
}

/// Parse type import expressions with non-string first arguments.
#[test]
fn test_parse_type_import_expression_with_non_string_target() {
    let mut test = TestParser::new("type T = import(1)");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Import { target, arguments, .. } => {
                assert_node!(parser.tree, *target, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                assert!(arguments.is_empty());
            });
        });
    });
}

/// Parse type import expressions with trailing commas.
#[test]
fn test_parse_type_import_expression_with_trailing_comma() {
    let mut test = TestParser::new("type T = import(\"vite\",)");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Import { target, arguments, .. } => {
                assert_node!(parser.tree, *target, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                    assert_string!(parser, *string_id, "vite");
                });
                assert!(arguments.is_empty());
            });
        });
    });
}

#[test]
fn test_parse_type_import_expression_missing_target() {
    // type T = import()
    let mut test = TestParser::new("type T = import()");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = import()
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Import { target, arguments, qualifier, generic_arguments } => {
                assert_node!(parser.tree, *target, Expression::Missing);
                assert!(arguments.is_empty());
                assert!(qualifier.is_none());
                assert!(generic_arguments.is_empty());
            });
        });
    });
}

#[test]
fn test_parse_type_import_expression_replaces_error_target_slot() {
    // type T = import(, "fallback")
    let mut test = TestParser::new("type T = import(, \"fallback\")");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = import(, "fallback")
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Import { target, arguments, qualifier, generic_arguments } => {
                assert_node!(parser.tree, *target, Expression::Missing);
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "fallback");
                    });
                });
                assert!(qualifier.is_none());
                assert!(generic_arguments.is_empty());
            });
        });
    });
}

#[test]
fn test_parse_type_import_expression_missing_close_parenthesis_with_member_target() {
    // type T = import("mod".Type
    let mut test = TestParser::new("type T = import(\"mod\".Type");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::TypeExpression), None, "")]);

    // type T = import("mod".Type
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Import { target, arguments, qualifier, generic_arguments } => {
                assert_node!(parser.tree, *target, Expression::Member { left, name } => {
                    assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "mod");
                    });
                    assert_string!(parser, *name, "Type");
                });
                assert!(arguments.is_empty());
                assert!(qualifier.is_none());
                assert!(generic_arguments.is_empty());
            });
        });
    });
}

#[test]
fn test_parse_type_import_expression() {
    let mut test = TestParser::new("type T = import(\"mod\").Type");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = import("mod").Type
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Import { target, arguments, qualifier, generic_arguments } => {
                assert_node!(parser.tree, *target, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                    assert_string!(parser, *string_id, "mod");
                });
                assert!(arguments.is_empty());
                assert_path!(parser, qualifier.as_ref().unwrap(), "Type");
                assert!(generic_arguments.is_empty());
            });
        });
    });
}

#[test]
fn test_parse_type_import_span() {
    let mut test = TestParser::new(r#"type T = import("foo").Bar"#);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            let import_id = *value;
            assert_node!(parser.tree, *value, TypeExpression::Import { target, arguments, .. } => {
                assert_node!(parser.tree, *target, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                    assert_string!(parser, *string_id, "foo");
                });
                assert!(arguments.is_empty());
            });

            let main_span = parser
                .tree
                .get_main_span(import_id)
                .expect("expected import main span");
            assert_eq!(parser.get_span_str(main_span), "\"foo\"");
        });
    });
}
