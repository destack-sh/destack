use crate::tests::*;
use crate::{assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

/// Parse declaration-file conditional aliases with generic parameter constraints.
#[test]
fn test_parse_conditional_type_alias_with_generics_in_typescript_declaration() {
    let mut test = TestParser::new_with_options(
        "type FindMyWayVersion<RawServer extends RawServerBase> = RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // type FindMyWayVersion<RawServer extends RawServerBase> = RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, generic_parameters, value, .. }) => {
            assert_string!(parser, name.string(), "FindMyWayVersion");
            assert_eq!(generic_parameters.len(), 1);

            // RawServer extends RawServerBase
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(ty), default, .. } => {
                assert_string!(parser, *name, "RawServer");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "RawServerBase");
                });
            });

            // RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "RawServer");
                });
                assert_node!(parser.tree, *extends_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "http.Server");
                });
                assert_node!(parser.tree, *then_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "HTTPVersion.V1");
                });
                assert_node!(parser.tree, *else_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "HTTPVersion.V2");
                });
            });
        });
    });
}

/// Parse namespace-scoped declaration-file conditional aliases.
#[test]
fn test_parse_namespace_conditional_type_alias_in_typescript_declaration() {
    let mut test = TestParser::new_with_options(
        "declare namespace fastify { type FindMyWayVersion<RawServer extends RawServerBase> = RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2 }",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // declare namespace fastify { type FindMyWayVersion<RawServer extends RawServerBase> = RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2 }
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(namespace_declaration_id) => {
        assert_node!(parser.tree, *namespace_declaration_id, Declaration::Namespace(NamespaceDeclaration { name, expressions, .. }) => {
            assert_string!(parser, name.string(), "fastify");
            assert_eq!(expressions.len(), 1);

            // type FindMyWayVersion<RawServer extends RawServerBase> = RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2
            let namespace_expression_id = parser.unwrap_labelled_expression(expressions[0]);
            assert_node!(parser.tree, namespace_expression_id, Expression::Declaration(type_declaration_id) => {
                assert_node!(parser.tree, *type_declaration_id, Declaration::Type(TypeDeclaration { name, generic_parameters, value, .. }) => {
                    assert_string!(parser, name.string(), "FindMyWayVersion");
                    assert_eq!(generic_parameters.len(), 1);

                    // RawServer extends RawServerBase
                    assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(ty), default, .. } => {
                        assert_string!(parser, *name, "RawServer");
                        assert!(default.is_none());
                        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "RawServerBase");
                        });
                    });

                    // RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2
                    assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                        assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "RawServer");
                        });
                        assert_node!(parser.tree, *extends_type, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "http.Server");
                        });
                        assert_node!(parser.tree, *then_type, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "HTTPVersion.V1");
                        });
                        assert_node!(parser.tree, *else_type, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "HTTPVersion.V2");
                        });
                    });
                });
            });
        });
    });
}

/// Parse declaration-file conditional aliases through expression entry.
#[test]
fn test_parse_conditional_type_alias_with_generics_expression_entry_typescript_declaration() {
    let mut test = TestParser::new_with_options(
        "type FindMyWayVersion<RawServer extends RawServerBase> = RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // type FindMyWayVersion<RawServer extends RawServerBase> = RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, generic_parameters, value, .. }) => {
            assert_string!(parser, name.string(), "FindMyWayVersion");
            assert_eq!(generic_parameters.len(), 1);

            // RawServer extends RawServerBase
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(ty), default, .. } => {
                assert_string!(parser, *name, "RawServer");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "RawServerBase");
                });
            });

            // RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "RawServer");
                });
                assert_node!(parser.tree, *extends_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "http.Server");
                });
                assert_node!(parser.tree, *then_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "HTTPVersion.V1");
                });
                assert_node!(parser.tree, *else_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "HTTPVersion.V2");
                });
            });
        });
    });
}
