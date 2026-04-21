use crate::tests::*;
use crate::{assert_expression_path, assert_node};
use destack_ast::*;
use destack_source::LanguageType;

/// Addition is left associative.
#[test]
fn test_parse_precedence_addition_left_associative() {
    let mut test = TestParser::new("a + b + c");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b + c
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a + b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    assert_eq!(*operator, BinaryOperator::Add);
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );
            // c
            assert_expression_path!(parser, parser.tree.get(*right), "c");
        }
    );
}

/// Infix operators work across lines.
#[test]
fn test_parse_precedence_addition_across_lines() {
    let mut test = TestParser::new(
        r#"a +
 b +
 c"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b + c (across lines)
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a + b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    assert_eq!(*operator, BinaryOperator::Add);
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );
            // c
            assert_expression_path!(parser, parser.tree.get(*right), "c");
        }
    );
}

/// Multiplication has higher precedence than addition.
#[test]
fn test_parse_precedence_multiply_before_addition() {
    let mut test = TestParser::new("a + b * c");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b * c
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a
            assert_expression_path!(parser, parser.tree.get(*left), "a");
            // b * c
            assert_node!(
                parser.tree,
                *right,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Multiply);
                    // b
                    assert_expression_path!(parser, parser.tree.get(*left), "b");
                    // c
                    assert_expression_path!(parser, parser.tree.get(*right), "c");
                }
            );
        }
    );
}

/// Mixed precedence chain with addition and multiplication.
#[test]
fn test_parse_precedence_chain_mixed() {
    let mut test = TestParser::new("a + b * c + d");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b * c + d
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a + b * c
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Add);
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // b * c
                    assert_node!(
                        parser.tree,
                        *right,
                        Expression::Binary { left, operator, right, .. } => {
                            assert_eq!(*operator, BinaryOperator::Multiply);
                            // b
                            assert_expression_path!(parser, parser.tree.get(*left), "b");
                            // c
                            assert_expression_path!(parser, parser.tree.get(*right), "c");
                        }
                    );
                }
            );
            // d
            assert_expression_path!(parser, parser.tree.get(*right), "d");
        }
    );
}

/// Type casts bind to the left side before addition.
#[test]
fn test_parse_precedence_cast_before_addition() {
    let mut test = TestParser::new("a as number + b");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a as number + b
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a as number
            assert_node!(
                parser.tree,
                *left,
                Expression::As { expression, target_type } => {
                    // a
                    assert_expression_path!(parser, parser.tree.get(*expression), "a");
                    // number
                    assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                }
            );
            // b
            assert_expression_path!(parser, parser.tree.get(*right), "b");
        }
    );
}

/// Reject a TypeScript angle bracket type assertion.
/// Addition has higher precedence than elementwise or.
#[test]
fn test_parse_precedence_elementwise_vs_addition() {
    let mut test = TestParser::new("a + b | c + d");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a + b | c + d
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::ElementwiseOr);
            // a + b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Add);
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );
            // c + d
            assert_node!(
                parser.tree,
                *right,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Add);
                    // c
                    assert_expression_path!(parser, parser.tree.get(*left), "c");
                    // d
                    assert_expression_path!(parser, parser.tree.get(*right), "d");
                }
            );
        }
    );
}

/// Comparison has higher precedence than logical and.
#[test]
fn test_parse_precedence_comparison_vs_logical() {
    let mut test = TestParser::new("a == b && c == d");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a == b && c == d
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::And);
            // a == b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Equal);
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );
            // c == d
            assert_node!(
                parser.tree,
                *right,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Equal);
                    // c
                    assert_expression_path!(parser, parser.tree.get(*left), "c");
                    // d
                    assert_expression_path!(parser, parser.tree.get(*right), "d");
                }
            );
        }
    );
}

/// Runtime `is` guards bind before logical and.
#[test]
fn test_parse_precedence_is_before_logical_and() {
    let mut test = TestParser::new("value is string && ready");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // value is string && ready
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::And);

            // value is string
            assert_node!(parser.tree, *left, Expression::Is { value, target_type } => {
                assert_expression_path!(parser, parser.tree.get(*value), "value");
                assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });

            // ready
            assert_expression_path!(parser, parser.tree.get(*right), "ready");
        }
    );

    test.assert_no_errors(&parser);
}

/// Runtime `instanceof` guards bind before logical and.
#[test]
fn test_parse_precedence_instanceof_before_logical_and() {
    let mut test =
        TestParser::new_with_options("value instanceof Box && ready", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // value instanceof Box && ready
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::And);

            // value instanceof Box
            assert_node!(parser.tree, *left, Expression::InstanceOf { value, target } => {
                assert_expression_path!(parser, parser.tree.get(*value), "value");
                assert_expression_path!(parser, parser.tree.get(*target), "Box");
            });

            // ready
            assert_expression_path!(parser, parser.tree.get(*right), "ready");
        }
    );

    test.assert_no_errors(&parser);
}

/// Unary prefix has higher precedence than multiplication.
#[test]
fn test_parse_precedence_unary_before_multiply() {
    let mut test = TestParser::new("-a * b");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // -a * b
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Multiply);
            // -a
            assert_node!(
                parser.tree,
                *left,
                Expression::Unary { right, .. } => {
                    // a
                    assert_expression_path!(parser, parser.tree.get(*right), "a");
                }
            );
            // b
            assert_expression_path!(parser, parser.tree.get(*right), "b");
        }
    );
}

/// Binary operator spans point at the operator token.
#[test]
fn test_parse_binary_operator_span() {
    let mut test = TestParser::new("left + right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::Add);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.get_span_str(main_span), "+");
}

#[test]
fn test_parse_binary_operator_multichar_span() {
    let mut test = TestParser::new("left === right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::EqualStrict);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.get_span_str(main_span), "===");
}

#[test]
fn test_parse_binary_operator_logical_span() {
    let mut test = TestParser::new("left && right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::And);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.get_span_str(main_span), "&&");
}

#[test]
fn test_parse_binary_operator_coalesce_span() {
    let mut test = TestParser::new("left ?? right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::Coalesce);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.get_span_str(main_span), "??");
}

#[test]
fn test_parse_assign_operator_span() {
    let mut test = TestParser::new("left += right");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Assign { operator, left, right } => {
        assert_eq!(*operator, AssignOperator::AddAssign);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected assign operator span");
    assert_eq!(parser.get_span_str(main_span), "+=");
}

/// Reject compound assignment on one destructuring target.
#[test]
fn test_reject_compound_assignment_on_destructuring_target() {
    let mut test = TestParser::new("({ x } += value)");
    let mut parser = test.prepare();

    let result = parser.eat_expression(parser.options);

    assert!(
        result.is_err(),
        "expected compound destructuring assignment to fail"
    );
}

/// Assignment chains bind right associatively.
#[test]
fn test_parse_precedence_assignment_right_associative() {
    let mut test = TestParser::new("alpha = beta = computeValue()");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(
        parser.tree,
        expr_id,
        Expression::Assign { operator, left, right } => {
            assert_eq!(*operator, AssignOperator::Assign);
            assert_expression_path!(parser, parser.tree.get(*left), "alpha");

            assert_node!(
                parser.tree,
                *right,
                Expression::Assign { operator, left, right } => {
                    assert_eq!(*operator, AssignOperator::Assign);
                    assert_expression_path!(parser, parser.tree.get(*left), "beta");
                    assert_node!(parser.tree, *right, Expression::Call { left, arguments, .. } => {
                        assert!(arguments.is_empty());
                        assert_expression_path!(parser, parser.tree.get(*left), "computeValue");
                    });
                }
            );
        }
    );
}

/// Postfix call has higher precedence than addition.
#[test]
fn test_parse_precedence_postfix_call_before_add() {
    let mut test = TestParser::new("a() + b() / c");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // a() + b() / c
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
            // a()
            assert_node!(
                parser.tree,
                *left,
                Expression::Call { left, .. } => {
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                }
            );
            // b() / c
            assert_node!(
                parser.tree,
                *right,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::Divide);
                    // b()
                    assert_node!(
                        parser.tree,
                        *left,
                        Expression::Call { left, .. } => {
                            // b
                            assert_expression_path!(parser, parser.tree.get(*left), "b");
                        }
                    );
                    // c
                    assert_expression_path!(parser, parser.tree.get(*right), "c");
                }
            );
        }
    );
}

/// Combine postfix member access and call with coalesce.
#[test]
fn test_parse_precedence_postfix_call_before_coalesce() {
    let mut test = TestParser::new("y.sqrt() ?? 0");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    // y.sqrt() ?? 0
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Coalesce);
            // y.sqrt()
            assert_node!(parser.tree, *left, Expression::Call { left, .. } => {
                // y.sqrt
                assert_expression_path!(parser, parser.tree.get(*left), "y.sqrt");
            });
            // 0
            assert_node!(
                parser.tree,
                *right,
                Expression::ScalarLiteral(ScalarLiteral::Integer(0))
            );
        }
    );
}
