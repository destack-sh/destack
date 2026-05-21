use crate::tests::*;
use crate::{assert_expression_path, assert_name, assert_node, assert_string};
use destack_dir::*;
use destack_source::LanguageType;

/// Parse a type asserts expression.
#[test]
fn test_parse_type_unary_postfix_asserts_expression() {
    let mut test = TestParser::new(
        r"
function isStringy(value: any): asserts value is string {
    // ...
}
",
    );
    let mut parser = test.prepare();

    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // function isStringy(value: any): asserts value is string { .. }
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { name, signature, .. }) => {
            // isStringy
            assert_string!(parser, name.unwrap().string(), "isStringy");
            assert_eq!(signature.parameters.len(), 1);

            // value: any
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "value");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Any);
                });
            });

            // asserts value is string
            assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Predicate { asserts, subject, target } => {
                assert!(*asserts);
                assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("value")));
                assert_node!(parser.tree, target.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });

            let predicate_id = signature.return_type.unwrap();
            let main_span = parser
                .tree
                .get_main_span(predicate_id)
                .expect("expected predicate main span");
            assert_eq!(parser.get_span_str(main_span), "value");
        });
    });
}

#[test]
fn test_reject_type_predicate_in_plain_type_before_block_context() {
    let mut test = TestParser::new_with_language(
        "module is DynamicModule { value: true }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let error = parser
        .with_flags(parser.flags.in_type().in_before_block(), |parser| {
            parser.eat_type_expression()
        })
        .unwrap_err();
    let (span, node_type, expected) = error.leaf_content();

    assert_eq!(
        (node_type, expected, parser.get_span_str(span)),
        (None, None, "is")
    );
}

/// Reject TypeScript `asserts` predicate subjects after a line break.
#[test]
fn test_reject_asserts_type_predicate_subject_on_new_line() {
    let mut test = TestParser::new_with_language(
        r"
function assertFoo(value: unknown): asserts
value is Foo {
    return;
}
",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    parser.parse();

    test.assert_error_leaves(
        &parser,
        &[(None, None, "is"), (None, None, "Foo"), (None, None, "{")],
    );
}

/// Parse a predicate return type whose subject is also a contextual type literal.
#[test]
fn test_parse_return_type_predicate_with_object_subject() {
    let mut test = TestParser::new_with_language(
        r#"function isAnyArrayBuffer(object: unknown): object is ArrayBufferLike;"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    // function isAnyArrayBuffer(object: unknown): object is ArrayBufferLike
    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { name, export, is_ambient, signature, body }) => {
            assert_name!(parser, name.expect("expected function name"), "isAnyArrayBuffer");
            assert!(export.is_none());
            assert_eq!(*is_ambient, false);
            assert!(!signature.is_abstract);
            assert!(!signature.is_override);
            assert_eq!(signature.asynchrony, Asynchrony::Sync);
            assert!(!signature.is_generator);
            assert!(signature.role.is_none());
            assert_eq!(signature.form, FunctionForm::Function);
            assert!(signature.generic_parameters.is_empty());
            assert!(signature.where_clauses.is_empty());
            assert!(signature.this_parameter.is_none());
            assert_eq!(signature.parameters.len(), 1);
            assert!(body.is_none());

            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, visibility, is_readonly, is_optional, declared_type, default, .. } => {
                assert_string!(parser, *name, "object");
                assert!(visibility.is_none());
                assert!(!*is_readonly);
                assert!(!*is_optional);
                assert!(default.is_none());
                assert_node!(parser.tree, declared_type.expect("expected parameter type"), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Unknown);
                });
            });

            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Predicate { asserts, subject, target } => {
                assert!(!*asserts);
                assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("object")));
                assert_expression_path!(parser, parser.tree.get(target.expect("expected predicate target")), "ArrayBufferLike");
            });
        });
    });
}

/// Parse a predicate return type with a parenthesized union target.
#[test]
fn test_parse_arrow_return_type_predicate_with_parenthesized_union_target() {
    let mut test = TestParser::new_with_language(
        "(item): item is (IChatRequestViewModel | IChatResponseViewModel) => isRequestVM(item)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Predicate { asserts, subject, target } => {
                assert!(!*asserts);
                assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("item")));
                assert_node!(parser.tree, target.expect("expected predicate target"), TypeExpression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, TypeExpression::Union { elements } => {
                        assert_eq!(elements.len(), 2);
                    });
                });
            });
        });
    });
}

/// Parse a predicate return type with a parenthesized intersection target.
#[test]
fn test_parse_arrow_return_type_predicate_with_parenthesized_intersection_target() {
    let mut test = TestParser::new_with_language(
        "(p: unknown): p is (TentativeBoundary & { inner: CharacterPrediction }) => p instanceof TentativeBoundary",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Predicate { asserts, subject, target } => {
                assert!(!*asserts);
                assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("p")));
                assert_node!(parser.tree, target.expect("expected predicate target"), TypeExpression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, TypeExpression::Intersection { elements } => {
                        assert_eq!(elements.len(), 2);
                    });
                });
            });
        });
    });
}

/// Parse arrow predicate return types whose subject is a contextual keyword name.
#[test]
fn test_parse_arrow_return_type_predicate_with_keyword_subject_override() {
    let mut test = TestParser::new_with_language(
        "const isSystemOverride = (override: ConfigOverrideRule): override is SystemConfigOverrideRule => { return '__systemRef' in override && typeof override.__systemRef === 'string'; }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // const isSystemOverride = (...) : override is SystemConfigOverrideRule => { ... }
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            assert_node!(parser.tree, value.expect("expected initializer"), Expression::Declaration(function_id) => {
                assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Predicate { asserts, subject, target } => {
                        assert!(!*asserts);
                        assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("override")));
                        assert_expression_path!(parser, parser.tree.get(target.expect("expected type target")), "SystemConfigOverrideRule");
                    });
                });
            });
        });
    });
}
