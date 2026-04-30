use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

#[test]
fn test_parse_conditional_type_alias_with_generics() {
    let mut test = TestParser::new_with_language(
        "type FindMyWayVersion<RawServer extends RawServerBase> = RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);

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

#[test]
fn test_parse_type_alias_records_generic_parameter_container_span() {
    let mut test =
        TestParser::new_with_language("type Box<T> = T", LanguageType::TypeScriptDeclaration);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        let generic_parameter_span = parser
            .tree
            .get_side_span(*declaration_id, NodeSpanType::Region(NodeSpanRegion::GenericParameters))
            .expect("missing type alias generic parameter span");

        assert_eq!(parser.get_span_str(generic_parameter_span), "<T>");
    });
}

/// Parse conditional type aliases with object infer constraints after a multiline extends.
#[test]
fn test_parse_type_declaration_conditional_object_infer_after_newline() {
    let mut test = TestParser::new_with_language(
        r#"type ImplicitArrayBuffer<T extends WithImplicitCoercion<ArrayBufferLike>> = T extends
    { valueOf(): infer V extends ArrayBufferLike } ? V : T"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { extends_type, then_type, else_type, .. } => {
                assert_node!(parser.tree, *extends_type, TypeExpression::Object { .. });
                assert_expression_path!(parser, parser.tree.get(*then_type), "V");
                assert_expression_path!(parser, parser.tree.get(*else_type), "T");
            });
        });
    });
}

/// Parse generic parameter defaults that end before a shifted type close.
#[test]
fn test_parse_type_declaration_generic_default_before_shifted_close() {
    let mut test = TestParser::new_with_language(
        r#"export type TuplifyUnion<Union, LastElement = LastOf<Union>> =
    IsNever<Union> extends true ? [] : [...TuplifyUnion<Exclude<Union, LastElement>>, LastElement]"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { generic_parameters, value, .. }) => {
            assert_eq!(generic_parameters.len(), 2);
            assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { default: Some(default), .. } => {
                assert_node!(parser.tree, *default, TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "LastOf");
                    assert_eq!(generic_arguments.len(), 1);
                });
            });
            assert_node!(parser.tree, *value, TypeExpression::Conditional { .. });
        });
    });
}

#[test]
fn test_parse_namespace_conditional_type_alias() {
    let mut test = TestParser::new_with_language(
        "declare namespace fastify { type FindMyWayVersion<RawServer extends RawServerBase> = RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2 }",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);

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

#[test]
fn test_parse_conditional_type_alias_with_generics_through_expression_entry() {
    let mut test = TestParser::new_with_language(
        "type FindMyWayVersion<RawServer extends RawServerBase> = RawServer extends http.Server ? HTTPVersion.V1 : HTTPVersion.V2",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

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

/// Parse `type` expressions with indexed generic arguments without forcing an alias head.
#[test]
fn test_parse_type_expression_with_indexed_generic_argument_after_type_keyword() {
    let mut test = TestParser::new("type Foo<T[number]>");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // type Foo<T[number]>
    assert_node!(parser.tree, expression_id, Expression::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "Foo");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "T");
                        assert_node!(parser.tree, *index, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                    });
            });
        });
    });

    test.assert_no_errors(&parser);
}
