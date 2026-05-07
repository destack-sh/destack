use crate::tests::*;
use crate::{assert_expression_path, assert_node};
use destack_ast::*;
use destack_source::LanguageType;

/// Comma in parentheses preserves the explicit sequence grouping.
#[test]
fn test_parse_sequence_expression() {
    let language = LanguageType::JavaScript;
    let mut test = TestParser::new_with_language("(a, b, c)", language);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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

/// Comma operator parses as a sequence expression.
#[test]
fn test_parse_sequence_expression_without_parens() {
    let language = LanguageType::TypeScript;
    let mut test = TestParser::new_with_language("a, b", language);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    // a, b
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);

        // a
        assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");

        // b
        assert_expression_path!(parser, parser.tree.get(expressions[1]), "b");
    });
}

/// Sequence expressions should parse inside lambda block bodies.
#[test]
fn test_parse_sequence_expression_in_lambda_block_body() {
    let language = LanguageType::TypeScript;
    let mut test = TestParser::new_with_language(
        "() => { (lastIndex = history.state?.index), (lastY = scrollY), (lastX = scrollX); }",
        language,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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
fn test_parse_object_method_body_sequence_expression_statement() {
    let language = LanguageType::JavaScript;
    let mut test = TestParser::new_with_language(
        "objectType({ definition (t) { t.callA(), t.callB(), t.callC() } })",
        language,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // objectType({ definition(t) { ... } })
    assert_node!(parser.tree, expr_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value: object_id, .. } => {
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
    let language = LanguageType::TypeScript;
    let mut test = TestParser::new_with_language("a && (b = 1, c = 2), d ? e : f", language);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // a && (b = 1, c = 2), d ? e : f
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::And);
        });
        assert_node!(parser.tree, expressions[1], Expression::If { form, .. } => {
            assert_eq!(*form, IfForm::Ternary);
        });
    });
}

#[test]
fn test_parse_sequence_expression_with_nested_ternary() {
    let language = LanguageType::TypeScript;
    let mut test = TestParser::new_with_language(
        "l === -1 && (s = !1, l = t + 1), a === 46 ? r === -1 ? r = t : n !== 1 && (n = 1) : r !== -1 && (n = -1)",
        language,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // l === -1 && (s = !1, l = t + 1), a === 46 ? r === -1 ? r = t : n !== 1 && (n = 1) : r !== -1 && (n = -1)
    assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::And);
        });
        assert_node!(parser.tree, expressions[1], Expression::If { form, .. } => {
            assert_eq!(*form, IfForm::Ternary);
        });
    });
}

/// Parse async arrow statements that continue into same line comma expressions.
#[test]
fn test_parse_async_arrow_statement_comma_continuation() {
    let mut test = TestParser::new_with_language("async () => {}, x;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.asynchrony, Asynchrony::Async);
                assert_eq!(signature.form, FunctionForm::Lambda);
            });
        });
        assert_expression_path!(parser, parser.tree.get(expressions[1]), "x");
    });
}

/// Parse plain arrow statements that continue into same line comma expressions.
#[test]
fn test_parse_arrow_statement_comma_continuation() {
    let mut test = TestParser::new_with_language("() => 1, 2", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
                assert_node!(parser.tree, body.expect("expected lambda body"), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
        assert_node!(parser.tree, expressions[1], Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
    });
}

/// Sequence expression with unary void.
#[test]
fn test_parse_sequence_expression_with_unary_void() {
    let language = LanguageType::JavaScript;
    let mut test = TestParser::new_with_language("(a, void 0, 1)", language);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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
