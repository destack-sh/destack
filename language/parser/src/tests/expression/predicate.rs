use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_string};
use destack_ast::*;
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
    parser.eat_newline().unwrap();

    let expr_id = parser.eat_expression(parser.options).unwrap();

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
fn test_parse_type_predicate_in_before_block_context() {
    let mut test = TestParser::new_with_options(
        "module is DynamicModule { value: true }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .with_options(parser.options.in_type().in_before_block(), |parser| {
            parser.eat_expression(parser.options)
        })
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Predicate { asserts, subject, target } => {
            assert!(!asserts);
            assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("module")));
            assert_expression_path!(parser, parser.tree.get(target.unwrap()), "DynamicModule");
        });
    });
}

/// Parse a predicate return type with a parenthesized union target.
#[test]
fn test_parse_arrow_return_type_predicate_with_parenthesized_union_target() {
    let mut test = TestParser::new_with_options(
        "(item): item is (IChatRequestViewModel | IChatResponseViewModel) => isRequestVM(item)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

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
    let mut test = TestParser::new_with_options(
        "(p: unknown): p is (TentativeBoundary & { inner: CharacterPrediction }) => p instanceof TentativeBoundary",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

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
    let mut test = TestParser::new_with_options(
        "const isSystemOverride = (override: ConfigOverrideRule): override is SystemConfigOverrideRule => { return '__systemRef' in override && typeof override.__systemRef === 'string'; }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

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
