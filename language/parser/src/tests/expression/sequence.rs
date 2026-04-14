use crate::tests::*;
use crate::{assert_expression_path, assert_node};
use destack_ast::*;
use destack_source::LanguageType;

/// Comma in parentheses preserves the explicit sequence grouping.
#[test]
fn test_parse_sequence_expression() {
    let options = LanguageType::JavaScript;
    let mut test = TestParser::new_with_options("(a, b, c)", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // (a, b, c)
    assert_node!(parser.tree, expr_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::SequenceExpression { expressions } => {
            assert_eq!(expressions.len(), 3);

            // a
            assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");

            // b
            assert_expression_path!(parser, parser.tree.get(expressions[1]), "b");

            // c
            assert_expression_path!(parser, parser.tree.get(expressions[2]), "c");
        });
    });
}

/// Comma operator parses as sequence expression in JS/TS.
#[test]
fn test_parse_sequence_expression_without_parens() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options("a, b", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a, b
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);

        // a
        assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");

        // b
        assert_expression_path!(parser, parser.tree.get(expressions[1]), "b");
    });
}

/// Sequence expressions should parse inside lambda block bodies in TS.
#[test]
fn test_parse_sequence_expression_in_lambda_block_body() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options(
        "() => { (lastIndex = history.state?.index), (lastY = scrollY), (lastX = scrollX); }",
        options,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // () => { ... }
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body: Some(body), .. }) => {
            // { ... }
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                let expressions = block_expression_ids(block);
                assert_eq!(expressions.len(), 1);

                // ((lastIndex = ...), (lastY = ...), (lastX = ...));
                assert_node!(parser.tree, expressions[0], Expression::SequenceExpression { expressions } => {
                    assert_eq!(expressions.len(), 3);
                    assert_node!(parser.tree, expressions[0], Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Assign { .. });
                    });
                    assert_node!(parser.tree, expressions[1], Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Assign { .. });
                    });
                    assert_node!(parser.tree, expressions[2], Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Assign { .. });
                    });
                });
            });
        });
    });
}

/// Parse sequence expression statements inside object literal method bodies in JavaScript.
#[test]
fn test_parse_javascript_object_method_body_sequence_expression_statement() {
    let options = LanguageType::JavaScript;
    let mut test = TestParser::new_with_options(
        "objectType({ definition (t) { t.callA(), t.callB(), t.callC() } })",
        options,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // objectType({ definition(t) { ... } })
    assert_node!(parser.tree, expr_id, Expression::Call { dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value: object_id, .. } => {
            assert_node!(parser.tree, *object_id, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], Property::Method { body, .. } => {
                    assert_node!(parser.tree, body.expect("expected object method body"), Expression::Block(block_id) => {
                        let block = parser.tree.get(*block_id);
                        let expressions = block_expression_ids(block);
                        assert_eq!(expressions.len(), 1);
                        assert_node!(parser.tree, expressions[0], Expression::SequenceExpression { expressions } => {
                            assert_eq!(expressions.len(), 3);
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_sequence_expression_with_ternary_tail() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options("a && (b = 1, c = 2), d ? e : f", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // a && (b = 1, c = 2), d ? e : f
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::And);
        });
        assert_node!(parser.tree, expressions[1], Expression::If { kind, .. } => {
            assert_eq!(*kind, IfKind::Ternary);
        });
    });
}

#[test]
fn test_parse_sequence_expression_with_nested_ternary() {
    let options = LanguageType::TypeScript;
    let mut test = TestParser::new_with_options(
        "l === -1 && (s = !1, l = t + 1), a === 46 ? r === -1 ? r = t : n !== 1 && (n = 1) : r !== -1 && (n = -1)",
        options,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // l === -1 && (s = !1, l = t + 1), a === 46 ? r === -1 ? r = t : n !== 1 && (n = 1) : r !== -1 && (n = -1)
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::And);
        });
        assert_node!(parser.tree, expressions[1], Expression::If { kind, .. } => {
            assert_eq!(*kind, IfKind::Ternary);
        });
    });
}

/// Parse async arrow statements that continue into same line comma expressions.
#[test]
fn test_parse_async_arrow_statement_comma_continuation_typescript() {
    let mut test = TestParser::new_with_options("async () => {}, x;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.asynchrony, Asynchrony::Async);
                assert_eq!(signature.kind, FunctionKind::Lambda);
            });
        });
        assert_expression_path!(parser, parser.tree.get(expressions[1]), "x");
    });
}

/// Parse plain arrow statements that continue into same line comma expressions.
#[test]
fn test_parse_arrow_statement_comma_continuation_javascript() {
    let mut test = TestParser::new_with_options("() => 1, 2", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert_node!(parser.tree, body.expect("expected lambda body"), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
        assert_node!(parser.tree, expressions[1], Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
    });
}

/// Sequence expression with unary void.
#[test]
fn test_parse_sequence_expression_with_unary_void() {
    let options = LanguageType::JavaScript;
    let mut test = TestParser::new_with_options("(a, void 0, 1)", options);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // (a, void 0, 1)
    assert_node!(parser.tree, expr_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::SequenceExpression { expressions } => {
            assert_eq!(expressions.len(), 3);

            // a
            assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");

            // void 0
            assert_node!(parser.tree, expressions[1], Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::Void);
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });

            // 1
            assert_node!(parser.tree, expressions[2], Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}
