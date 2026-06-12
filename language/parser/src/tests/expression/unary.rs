use crate::tests::TestParser;
use crate::{assert_expression_path, assert_node, assert_path};
use destack_dir::{
    BinaryOperator, Expression, Mutability, ScalarLiteral, TypeExpression, UnaryOperator,
    VarianceBound,
};
use destack_source::LanguageType;

/// Unary operator spans point at the operator token.
#[test]
fn test_parse_unary_operator_span() {
    let mut test = TestParser::new("-value");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Negate);
        assert_expression_path!(parser, parser.tree.get(*right), "value");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected unary operator span");
    assert_eq!(parser.get_span_str(main_span), "-");
}

#[test]
fn test_parse_unary_postfix_operator_span() {
    let mut test = TestParser::new("value++");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::PostIncrement);
        assert_expression_path!(parser, parser.tree.get(*right), "value");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected unary postfix operator span");
    assert_eq!(parser.get_span_str(main_span), "++");
}

#[test]
fn test_parse_unary_keyword_operators() {
    let mut test = TestParser::new("typeof foo; void 0");
    let mut parser = test.prepare();

    let typeof_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, typeof_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Typeof);
        assert_expression_path!(parser, parser.tree.get(*right), "foo");
    });
    parser.eat_statement_stop().unwrap();

    let void_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, void_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Void);
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
    });
}

#[test]
fn test_parse_parenthesized_unary_exponent_operands() {
    let input = r#"
(void ident) ** 2;
(typeof ident) ** 2;
(-3) ** 2;
(+3) ** 2;
(~3) ** 2;
(!true) ** 2;
"#;
    let mut test = TestParser::new_with_language(input, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 6);
    for expression in expressions {
        assert_node!(parser.tree, expression, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::Exponent);
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Unary { .. });
            });
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });
    }
}

#[test]
fn test_parse_unary_negate_allows_newline_before_operand() {
    let mut test = TestParser::new_with_language("-\n1", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Negate);
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

#[test]
fn test_parse_unary_negate_allows_line_comment_before_operand() {
    let mut test = TestParser::new_with_language("-// comment\n1", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Negate);
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

/// Parse a unary operand with an explicit parenthesized comment wrapper.
#[test]
fn test_parse_unary_negate_preserves_parenthesized_comment_wrapper() {
    let mut test = TestParser::new_with_language("-(/* comment */ 1)", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Negate);
        assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
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
    let mut test = TestParser::new_with_language(
        "await (new Promise<void>(resolve => setTimeout(() => resolve(), delay)))",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // await (new Promise<void>(...))
    assert_node!(parser.tree, expression_id, Expression::Await { expression } => {
        assert_node!(parser.tree, *expression, Expression::Parenthesized { expression: parenthesized_expression } => {
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
    let mut test = TestParser::new("(a++ + ++a) * (b-- - --b)");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
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

        assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
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
    let mut test = TestParser::new("*x");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Dereference);
        assert_expression_path!(parser, parser.tree.get(*right), "x");
    });
}

/// Reject dereference in untyped value mode.
#[test]
fn test_reject_dereference_in_untyped_value_mode() {
    let language = LanguageType::JavaScript;
    let mut test = TestParser::new_with_language("*x", language);
    let mut parser = test.prepare();
    let result = parser.eat_expression(parser.flags);
    assert!(result.is_err());
}

/// Parse a reference expression.
#[test]
fn test_parse_reference_variable() {
    let mut test = TestParser::new("&x");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::BorrowOf { mutability: Some(mutability), variance: None, right } => {
        assert_eq!(*mutability, Mutability::Mutable);
        assert_expression_path!(parser, parser.tree.get(*right), "x");
    });
}

/// Parse a reference to a member call.
#[test]
fn test_parse_reference_member_call() {
    let mut test = TestParser::new("&self.foo()");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
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
    let mut test = TestParser::new("&readonly super T");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::BorrowOf { mutability: Some(mutability), variance, right } => {
        assert_eq!(*mutability, Mutability::Immutable);
        assert_eq!(*variance, Some(VarianceBound::Super));
        assert_expression_path!(parser, parser.tree.get(*right), "T");
    });
}

/// Parse a value expression.
#[test]
fn test_parse_value_expression() {
    let mut test = TestParser::new("^super T");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::MoveOf { mutability, variance, right, .. } => {
        assert_eq!(*mutability, Some(Mutability::Mutable));
        assert_eq!(*variance, Some(VarianceBound::Super));
        assert_expression_path!(parser, parser.tree.get(*right), "T");
    });
}

/// Parse a new constructor call.
#[test]
fn test_parse_new_constructor_call() {
    let mut test = TestParser::new("new Foo()");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::New { ty, arguments } => {
        assert_expression_path!(parser, parser.tree.get(*ty), "Foo");
        assert!(arguments.is_empty());
    });
}

/// Reject delete expressions.
#[test]
fn test_reject_delete_expression() {
    let mut test = TestParser::new("delete foo.bar");
    let mut parser = test.prepare();

    parser.parse();

    test.assert_error_leaves(&parser, &[(None, None, "foo")]);
}
