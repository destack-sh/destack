use crate::tests::TestParser;
use crate::{assert_expression_path, assert_node, assert_path};
use destack_dir::{
    BinaryOperator, Expression, Mutability, NodeType, ScalarLiteral, TokenType, TypeExpression,
    UnaryOperator, VarianceBound,
};

/// Unary operator spans point at the operator token.
#[test]
fn test_parse_unary_operator_span() {
    let test = TestParser::new("-value");
    let mut parser = test.prepare();
    let expr_id = parser.parse_expression(Default::default()).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Negate);
        assert_expression_path!(parser, parser.tree.get(*right), "value");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected unary operator span");
    assert_eq!(parser.span_str(main_span), "-");
}

#[test]
fn test_parse_unary_postfix_operator_span() {
    let test = TestParser::new("value++");
    let mut parser = test.prepare();
    let expr_id = parser.parse_expression(Default::default()).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::PostIncrement);
        assert_expression_path!(parser, parser.tree.get(*right), "value");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected unary postfix operator span");
    assert_eq!(parser.span_str(main_span), "++");
}

/// Reject the JavaScript runtime `typeof` operator.
#[test]
fn test_reject_runtime_typeof_expression() {
    let test = TestParser::new("typeof value");
    let mut parser = test.prepare();

    let expressions = parser.parse();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::Identifier),
            None,
            "typeof",
        )],
    );
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Error);
}

/// Reject the JavaScript runtime `void` operator.
#[test]
fn test_reject_runtime_void_expression() {
    let test = TestParser::new("void value");
    let mut parser = test.prepare();

    let expressions = parser.parse();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::Identifier),
            None,
            "void",
        )],
    );
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Error);
}

#[test]
fn test_parse_parenthesized_unary_exponent_operands() {
    let input = r#"
(-3) ** 2;
(+3) ** 2;
(~3) ** 2;
(!true) ** 2;
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 4);
    for expression in expressions {
        assert_node!(parser.tree, expression, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::Exponent);
            crate::assert_parenthesized!(parser.tree, *left, expression => {
                assert_node!(parser.tree, *expression, Expression::Unary { .. });
            });
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });
    }
}

#[test]
fn test_parse_unary_negate_allows_newline_before_operand() {
    let test = TestParser::new("-\n1");
    let mut parser = test.prepare();
    let expression_id = parser.parse_expression(Default::default()).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Negate);
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

#[test]
fn test_parse_unary_negate_allows_line_comment_before_operand() {
    let test = TestParser::new("-// comment\n1");
    let mut parser = test.prepare();
    let expression_id = parser.parse_expression(Default::default()).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Negate);
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

/// Parse a unary operand with an explicit parenthesized comment wrapper.
#[test]
fn test_parse_unary_negate_preserves_parenthesized_comment_wrapper() {
    let test = TestParser::new("-(/* comment */ 1)");
    let mut parser = test.prepare();
    let expression_id = parser.parse_expression(Default::default()).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Negate);
        crate::assert_parenthesized!(parser.tree, *right, expression => {
            assert_node!(
                parser.tree,
                *expression,
                Expression::ScalarLiteral(ScalarLiteral::Integer(1))
            );
        });
    });
}

/// Parse await parenthesized `new` with generic receiver and `void` type argument.
#[test]
fn test_parse_await_parenthesized_new_expression_with_void_type_argument() {
    let test =
        TestParser::new("await (new Promise<void>(resolve => setTimeout(() => resolve(), delay)))");
    let mut parser = test.prepare();
    let expression_id = parser.parse_expression(Default::default()).unwrap();

    // await (new Promise<void>(...))
    assert_node!(parser.tree, expression_id, Expression::Await { expression } => {
        crate::assert_parenthesized!(parser.tree, *expression, parenthesized_expression => {
            assert_node!(parser.tree, *parenthesized_expression, Expression::New { ty, arguments } => {
                assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "Promise");
                    assert_eq!(generic_arguments.len(), 1);
                });
                assert_eq!(arguments.len(), 1);
            });
        });
    });
}

/// Parse mixed prefix and postfix increment and decrement operations.
#[test]
fn test_parse_mixed_prefix_and_postfix_increment_decrement() {
    let test = TestParser::new("(a++ + ++a) * (b-- - --b)");
    let mut parser = test.prepare();
    let expr_id = parser.parse_expression(Default::default()).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        crate::assert_parenthesized!(parser.tree, *left, expression => {
            assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, ..} => {
                assert_node!(parser.tree, *left, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::PostIncrement);
                    assert_expression_path!(parser, parser.tree.get(*right), "a");
                });
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::PreIncrement);
                    assert_expression_path!(parser, parser.tree.get(*right), "a");
                });
            });
        });

        assert_eq!(*operator, BinaryOperator::Multiply);

        crate::assert_parenthesized!(parser.tree, *right, expression => {
            assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, ..} => {
                assert_node!(parser.tree, *left, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::PostDecrement);
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                });
                assert_eq!(*operator, BinaryOperator::Subtract);
                assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::PreDecrement);
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                });
            });
        });
    });
}

/// Parse a dereference expression.
#[test]
fn test_parse_dereference_variable() {
    let test = TestParser::new("*x");
    let mut parser = test.prepare();
    let expr_id = parser.parse_expression(Default::default()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Dereference);
        assert_expression_path!(parser, parser.tree.get(*right), "x");
    });
}
#[test]
fn test_parse_reference_variable() {
    let test = TestParser::new("&x");
    let mut parser = test.prepare();
    let expr_id = parser.parse_expression(Default::default()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::BorrowOf { mutability: Some(mutability), variance: None, right } => {
        assert_eq!(*mutability, Mutability::Mutable);
        assert_expression_path!(parser, parser.tree.get(*right), "x");
    });
}

/// Parse compact nested reference expressions.
#[test]
fn test_parse_reference_chain_compact() {
    let test = TestParser::new("&&value");
    let mut parser = test.prepare();
    let expr_id = parser.parse_expression(Default::default()).unwrap();

    assert_node!(parser.tree, expr_id, Expression::BorrowOf { mutability, variance, right } => {
        assert_eq!(*mutability, Some(Mutability::Mutable));
        assert_eq!(*variance, None);
        assert_node!(parser.tree, *right, Expression::BorrowOf { mutability, variance, right } => {
            assert_eq!(*mutability, Some(Mutability::Mutable));
            assert_eq!(*variance, None);
            assert_expression_path!(parser, parser.tree.get(*right), "value");
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse a reference to a member call.
#[test]
fn test_parse_reference_member_call() {
    let test = TestParser::new("&self.foo()");
    let mut parser = test.prepare();
    let expr_id = parser.parse_expression(Default::default()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::BorrowOf { mutability: Some(Mutability::Mutable), variance: None, right } => {
        assert_node!(parser.tree, *right, Expression::Call { left, generic_arguments: _, arguments, .. } => {
            assert!(arguments.is_empty());
            assert_expression_path!(parser, parser.tree.get(*left), "self.foo");
        });
    });
}

/// Parse a bound reference expression.
#[test]
fn test_parse_bound_reference_expression() {
    let test = TestParser::new("&readonly super T");
    let mut parser = test.prepare();
    let expr_id = parser.parse_expression(Default::default()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::BorrowOf { mutability: Some(mutability), variance, right } => {
        assert_eq!(*mutability, Mutability::Immutable);
        assert_eq!(*variance, Some(VarianceBound::Super));
        assert_expression_path!(parser, parser.tree.get(*right), "T");
    });
}

/// Reject the retired move prefix in expression position.
#[test]
fn test_reject_move_prefix_expression() {
    let test = TestParser::new("^value");
    let mut parser = test.prepare();

    assert!(
        parser.parse_expression(Default::default()).is_err(),
        "move prefix should not parse"
    );
}

/// Parse a new constructor call.
#[test]
fn test_parse_new_constructor_call() {
    let test = TestParser::new("new Foo()");
    let mut parser = test.prepare();
    let expr_id = parser.parse_expression(Default::default()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::New { ty, arguments } => {
        assert_expression_path!(parser, parser.tree.get(*ty), "Foo");
        assert!(arguments.is_empty());
    });
}

/// Recover unsupported delete expressions as one damaged statement.
#[test]
fn test_recover_delete_expression() {
    let test = TestParser::new("delete foo.bar");
    let mut parser = test.prepare();

    let expressions = parser.parse();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::Identifier),
            None,
            "delete",
        )],
    );
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Error);
}
