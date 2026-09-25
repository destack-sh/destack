use crate::tests::TestParser;
use crate::{
    ExpressionPosition, ExpressionStop, assert_expression_path, assert_name, assert_node,
    assert_string,
};
use tspp_dir::{
    Argument, AssignOperator, AssignPattern, AssignPatternField, BinaryOperator, Expression,
    IfForm, Literal, LocalNodeId, PostfixPosition, TypeExpression, TypeLiteral, UnaryOperator,
};

/// Assert one assign pattern is an expression path.
fn assert_assign_pattern_path(
    parser: &crate::Parser,
    pattern_id: LocalNodeId<AssignPattern>,
    expected: &str,
) {
    assert_expression_path!(parser, parser.tree.get(pattern_id), expected);
}

/// Assert one assign pattern is a defaulted expression path.
fn assert_defaulted_assign_pattern(
    parser: &crate::Parser,
    pattern_id: LocalNodeId<AssignPattern>,
    expected_pattern: &str,
    expected_value: &str,
) {
    assert_node!(parser.tree, pattern_id, AssignPattern::Default { pattern, value } => {
        assert_assign_pattern_path(parser, *pattern, expected_pattern);
        assert_eq!(parser.span_str(parser.tree.get_span(*pattern)), expected_pattern);
        assert_expression_path!(parser, parser.tree.get(*value), expected_value);
    });
}

/// Build one deeply nested assignment sequence pattern.
fn nested_assignment_sequence_source(depth: usize) -> String {
    let mut source = String::new();

    // open assignment patterns
    for _ in 0..depth {
        source.push('[');
    }

    source.push_str("value");

    // close assignment patterns
    for _ in 0..depth {
        source.push(']');
    }

    source.push_str(" = input");

    source
}

/// Build one long right-associative assignment expression.
fn long_assignment_chain_source(depth: usize) -> String {
    let mut source = String::new();

    for index in 0..depth {
        source.push_str(&format!("value{index} = "));
    }

    source.push_str("finalValue");

    source
}

fn assert_assign_expression(parser: &crate::Parser, expression: LocalNodeId<Expression>) {
    assert_node!(parser.tree, expression, Expression::Assign { .. });
}

#[test]
fn test_parse_assignment_destructuring_targets() {
    let input = r#"
foo += bar = b ??= 3;
foo -= bar;
foo = [bar] = baz;
(foo = bar);
[foo, bar] = baz;
[foo, bar = "default", ...rest] = baz;
[,,,foo,bar] = baz;
({ bar, baz } = {});
({ bar: [baz = "baz"], foo = "foo", ...rest } = {});
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 9);
    for expression in expressions {
        assert_assign_expression(&parser, expression);
    }
}

/// Parse a deeply nested destructuring assignment without overflowing the parser stack.
#[test]
fn test_parse_deeply_nested_assignment_sequence_pattern() {
    let source = nested_assignment_sequence_source(1024);
    let test = TestParser::new(&source);
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::Assign { left, .. } => {
        assert_node!(parser.tree, *left, AssignPattern::Sequence { fields } => {
            assert_eq!(fields.len(), 1);
        });
    });
}

#[test]
fn test_parse_assignment_member_targets() {
    let input = r#"
foo += bar = b ??= 3;
a.foo -= bar;
(foo = bar);
(((foo))) = bar;
a["test"] = bar;
a.call().chain().member = x;
++count === 3
a['b'] = c[d] = "test"
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 8);
    assert_node!(parser.tree, expressions[0], Expression::Assign { .. });
    assert_node!(parser.tree, expressions[1], Expression::Assign { .. });
    crate::assert_parenthesized!(parser.tree, expressions[2], expression => {
        assert_node!(parser.tree, *expression, Expression::Assign { .. });
    });
    assert_node!(parser.tree, expressions[3], Expression::Assign { .. });
    assert_node!(parser.tree, expressions[4], Expression::Assign { .. });
    assert_node!(parser.tree, expressions[5], Expression::Assign { .. });
    assert_node!(parser.tree, expressions[6], Expression::Binary { operator, .. } => {
        assert_eq!(*operator, BinaryOperator::EqualStrict);
    });
    assert_node!(parser.tree, expressions[7], Expression::Assign { .. });
}

/// Parse dereferences of arbitrary expressions as assignment places.
#[test]
fn test_parse_dereference_assignment_targets() {
    let test = TestParser::new("*reference() += offset; *this = source");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 2);

    assert_node!(parser.tree, expressions[0], Expression::Assign { left, operator, .. } => {
        assert_eq!(*operator, AssignOperator::AddAssign);
        assert_node!(parser.tree, *left, AssignPattern::Place { expression } => {
            assert_node!(parser.tree, *expression, Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::Dereference);
                assert_node!(parser.tree, *right, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "reference");
                });
            });
        });
    });

    assert_node!(parser.tree, expressions[1], Expression::Assign { left, operator, .. } => {
        assert_eq!(*operator, AssignOperator::Assign);
        assert_node!(parser.tree, *left, AssignPattern::Place { expression } => {
            assert_node!(parser.tree, *expression, Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::Dereference);
                assert_node!(parser.tree, *right, Expression::This);
            });
        });
    });
}

/// Parse a long assignment chain without overflowing the parser stack.
#[test]
fn test_parse_long_assignment_chain() {
    let depth = 2100;
    let source = long_assignment_chain_source(depth);
    let test = TestParser::new(&source);
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    let mut expression_id = expression_id;
    for index in 0..depth {
        let expected = format!("value{index}");

        assert_node!(parser.tree, expression_id, Expression::Assign { left, right, .. } => {
            assert_assign_pattern_path(&parser, *left, &expected);
            expression_id = *right;
        });
    }

    assert_expression_path!(parser, parser.tree.get(expression_id), "finalValue");
}

/// Addition is left associative.
#[test]
fn test_parse_precedence_addition_left_associative() {
    let test = TestParser::new("a + b + c");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new(
        r#"a +
 b +
 c"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("a + b * c");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("a + b * c + d");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("a as number + b");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
                    assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                }
            );
            // b
            assert_expression_path!(parser, parser.tree.get(*right), "b");
        }
    );
}

/// Addition has higher precedence than elementwise or.
#[test]
fn test_parse_precedence_elementwise_vs_addition() {
    let test = TestParser::new("a + b | c + d");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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

/// Elementwise and binds tighter than elementwise or.
#[test]
fn test_parse_precedence_elementwise_and_before_or() {
    let test = TestParser::new("a & b | c");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // a & b | c
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::ElementwiseOr);

            // a & b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseAnd);

                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");

                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );

            // c
            assert_expression_path!(parser, parser.tree.get(*right), "c");
        }
    );
}

/// Elementwise and binds tighter on the right side of elementwise or.
#[test]
fn test_parse_precedence_elementwise_and_before_or_right_side() {
    let test = TestParser::new("a | b & c");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // a | b & c
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::ElementwiseOr);

            // a
            assert_expression_path!(parser, parser.tree.get(*left), "a");

            // b & c
            assert_node!(
                parser.tree,
                *right,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseAnd);

                    // b
                    assert_expression_path!(parser, parser.tree.get(*left), "b");

                    // c
                    assert_expression_path!(parser, parser.tree.get(*right), "c");
                }
            );
        }
    );
}

/// Comparison binds tighter than equality.
#[test]
fn test_parse_precedence_comparison_before_equality() {
    let test = TestParser::new("a < b == c");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // a < b == c
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Equal);

            // a < b
            assert_node!(
                parser.tree,
                *left,
                Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::LessThan);

                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");

                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                }
            );

            // c
            assert_expression_path!(parser, parser.tree.get(*right), "c");
        }
    );
}

/// Comparison has higher precedence than logical and.
#[test]
fn test_parse_precedence_comparison_vs_logical() {
    let test = TestParser::new("a == b && c == d");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("value is string && ready");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // value is string && ready
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::And);

            // value is string
            assert_node!(parser.tree, *left, Expression::Is { value, target_type } => {
                assert_expression_path!(parser, parser.tree.get(*value), "value");
                assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
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
    let test = TestParser::new("value instanceof Box && ready");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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

/// Parse multiline `instanceof` guards in expression position.
#[test]
fn test_parse_newline_before_instanceof_in_expression_position() {
    let test = TestParser::new("call(value\ninstanceof Box)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::InstanceOf { value, target } => {
                assert_expression_path!(parser, parser.tree.get(*value), "value");
                assert_expression_path!(parser, parser.tree.get(*target), "Box");
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Unary prefix has higher precedence than multiplication.
#[test]
fn test_parse_precedence_unary_before_multiply() {
    let test = TestParser::new("-a * b");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("left + right");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::Add);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.span_str(main_span), "+");
}

#[test]
fn test_parse_binary_operator_multichar_span() {
    let test = TestParser::new("left === right");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::EqualStrict);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.span_str(main_span), "===");
}

#[test]
fn test_parse_binary_operator_logical_span() {
    let test = TestParser::new("left && right");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::And);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.span_str(main_span), "&&");
}

#[test]
fn test_parse_binary_operator_coalesce_span() {
    let test = TestParser::new("left ?? right");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::Coalesce);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected binary operator span");
    assert_eq!(parser.span_str(main_span), "??");
}

#[test]
fn test_parse_assign_operator_span() {
    let test = TestParser::new("left += right");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Assign { operator, left, right } => {
        assert_eq!(*operator, AssignOperator::AddAssign);
        assert_expression_path!(parser, parser.tree.get(*left), "left");
        assert_expression_path!(parser, parser.tree.get(*right), "right");
    });

    let main_span = parser
        .tree
        .get_main_span(expr_id)
        .expect("expected assign operator span");
    assert_eq!(parser.span_str(main_span), "+=");
}

/// Parse a computed compound assignment as an indexed place.
#[test]
fn test_parse_computed_assignment() {
    let test = TestParser::new("counter[\"value\"] += 1");
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression, Expression::Assign { operator, left, right } => {
        assert_eq!(*operator, AssignOperator::AddAssign);
        assert_node!(parser.tree, *left, AssignPattern::Place { expression: place } => {
            assert_node!(parser.tree, *place, Expression::Index { position, left, index, is_optional } => {
                assert_eq!(*position, PostfixPosition::Direct);
                assert!(!is_optional);
                assert_expression_path!(parser, parser.tree.get(*left), "counter");

                let index = index.expect("expected index expression");
                assert_node!(parser.tree, index, Expression::Literal(Literal::String(value)) => {
                    assert_string!(parser, *value, "value");
                });
            });
        });
        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(1)));
    });

    let main_span = parser
        .tree
        .get_main_span(expression)
        .expect("expected assign operator span");

    assert_eq!(parser.span_str(main_span), "+=");
}

/// Parse object destructuring assignment defaults as assignment patterns.
#[test]
fn test_parse_object_destructuring_assignment_defaults() {
    let test = TestParser::new("({ x = fallback, y: z = other, [key]: target, ...rest } = value)");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    crate::assert_parenthesized!(parser.tree, expression_id, expression => {
        assert_node!(parser.tree, *expression, Expression::Assign { left, operator, right } => {
            assert_eq!(*operator, AssignOperator::Assign);
            assert_expression_path!(parser, parser.tree.get(*right), "value");

            assert_node!(parser.tree, *left, AssignPattern::Object { fields } => {
                assert_eq!(fields.len(), 4);

                assert_node!(parser.tree, fields[0], AssignPatternField::Named { name, is_shorthand, pattern } => {
                    assert_name!(parser, *name, "x");
                    assert!(*is_shorthand);
                    assert_defaulted_assign_pattern(&parser, *pattern, "x", "fallback");
                });

                assert_node!(parser.tree, fields[1], AssignPatternField::Named { name, is_shorthand, pattern } => {
                    assert_name!(parser, *name, "y");
                    assert!(!*is_shorthand);
                    let main_span = parser
                        .tree
                        .get_main_span(fields[1])
                        .expect("expected assignment key main span");
                    assert_eq!(parser.span_str(main_span), "y");
                    assert_defaulted_assign_pattern(&parser, *pattern, "z", "other");
                });

                assert_node!(parser.tree, fields[2], AssignPatternField::Computed { key, pattern } => {
                    let main_span = parser
                        .tree
                        .get_main_span(fields[2])
                        .expect("expected computed assignment key main span");
                    assert_eq!(parser.span_str(main_span), "[key]");
                    assert_expression_path!(parser, parser.tree.get(*key), "key");
                    assert_assign_pattern_path(&parser, *pattern, "target");
                });

                assert_node!(parser.tree, fields[3], AssignPatternField::Rest { pattern } => {
                    assert_assign_pattern_path(&parser, pattern.expect("expected rest target"), "rest");
                });
            });
        });
    });
}

/// Parse array destructuring assignment defaults as assignment patterns.
#[test]
fn test_parse_array_destructuring_assignment_defaults() {
    let test = TestParser::new("[first, , second = fallback, ...rest] = value");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::Assign);
        assert_expression_path!(parser, parser.tree.get(*right), "value");

        assert_node!(parser.tree, *left, AssignPattern::Sequence { fields } => {
            assert_eq!(fields.len(), 4);

            assert_node!(parser.tree, fields[0], AssignPatternField::Positional { pattern } => {
                assert_assign_pattern_path(&parser, *pattern, "first");
            });

            assert_node!(parser.tree, fields[1], AssignPatternField::Elision);

            assert_node!(parser.tree, fields[2], AssignPatternField::Positional { pattern } => {
                assert_defaulted_assign_pattern(&parser, *pattern, "second", "fallback");
            });

            assert_node!(parser.tree, fields[3], AssignPatternField::Rest { pattern } => {
                assert_assign_pattern_path(&parser, pattern.expect("expected rest target"), "rest");
            });
        });
    });
}

/// Parse tuple destructuring assignment as assignment patterns.
#[test]
fn test_parse_tuple_destructuring_assignment() {
    let test = TestParser::new("(first, second = fallback,) = value");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::Assign);
        assert_expression_path!(parser, parser.tree.get(*right), "value");

        assert_node!(parser.tree, *left, AssignPattern::Tuple { fields } => {
            assert_eq!(fields.len(), 2);

            assert_node!(parser.tree, fields[0], AssignPatternField::Positional { pattern } => {
                assert_assign_pattern_path(&parser, *pattern, "first");
            });

            assert_node!(parser.tree, fields[1], AssignPatternField::Positional { pattern } => {
                assert_defaulted_assign_pattern(&parser, *pattern, "second", "fallback");
            });
        });
    });
}

/// Parse nested destructuring assignment defaults as assignment patterns.
#[test]
fn test_parse_nested_destructuring_assignment_defaults() {
    let test = TestParser::new("({ a: { b = c } = d, e: [f = g], t: (u = v,) } = h)");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    crate::assert_parenthesized!(parser.tree, expression_id, expression => {
        assert_node!(parser.tree, *expression, Expression::Assign { left, operator, right } => {
            assert_eq!(*operator, AssignOperator::Assign);
            assert_expression_path!(parser, parser.tree.get(*right), "h");

            assert_node!(parser.tree, *left, AssignPattern::Object { fields } => {
                assert_eq!(fields.len(), 3);

                assert_node!(parser.tree, fields[0], AssignPatternField::Named { name, is_shorthand, pattern } => {
                    assert_name!(parser, *name, "a");
                    assert!(!*is_shorthand);

                    assert_node!(parser.tree, *pattern, AssignPattern::Default { pattern, value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "d");

                        assert_node!(parser.tree, *pattern, AssignPattern::Object { fields } => {
                            assert_eq!(fields.len(), 1);

                            assert_node!(parser.tree, fields[0], AssignPatternField::Named { name, is_shorthand, pattern } => {
                                assert_name!(parser, *name, "b");
                                assert!(*is_shorthand);
                                assert_defaulted_assign_pattern(&parser, *pattern, "b", "c");
                            });
                        });
                    });
                });

                assert_node!(parser.tree, fields[1], AssignPatternField::Named { name, is_shorthand, pattern } => {
                    assert_name!(parser, *name, "e");
                    assert!(!*is_shorthand);

                    assert_node!(parser.tree, *pattern, AssignPattern::Sequence { fields } => {
                        assert_eq!(fields.len(), 1);

                        assert_node!(parser.tree, fields[0], AssignPatternField::Positional { pattern } => {
                            assert_defaulted_assign_pattern(&parser, *pattern, "f", "g");
                        });
                    });
                });

                assert_node!(parser.tree, fields[2], AssignPatternField::Named { name, is_shorthand, pattern } => {
                    assert_name!(parser, *name, "t");
                    assert!(!*is_shorthand);

                    assert_node!(parser.tree, *pattern, AssignPattern::Tuple { fields } => {
                        assert_eq!(fields.len(), 1);

                        assert_node!(parser.tree, fields[0], AssignPatternField::Positional { pattern } => {
                            assert_defaulted_assign_pattern(&parser, *pattern, "u", "v");
                        });
                    });
                });
            });
        });
    });
}

/// Parse member and index destructuring assignment targets.
#[test]
fn test_parse_destructuring_assignment_member_targets() {
    let test = TestParser::new("({ value: object.property, [key]: target[index] } = source)");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    crate::assert_parenthesized!(parser.tree, expression_id, expression => {
        assert_node!(parser.tree, *expression, Expression::Assign { left, operator, right } => {
            assert_eq!(*operator, AssignOperator::Assign);
            assert_expression_path!(parser, parser.tree.get(*right), "source");

            assert_node!(parser.tree, *left, AssignPattern::Object { fields } => {
                assert_eq!(fields.len(), 2);

                assert_node!(parser.tree, fields[0], AssignPatternField::Named { name, is_shorthand, pattern } => {
                    assert_name!(parser, *name, "value");
                    assert!(!*is_shorthand);
                    assert_node!(parser.tree, *pattern, AssignPattern::Place { expression: value } => {
                        assert_node!(parser.tree, *value, Expression::Member { left, name: Some(name), .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "object");
                            assert_string!(parser, *name, "property");
                        });
                    });
                });

                assert_node!(parser.tree, fields[1], AssignPatternField::Computed { key, pattern } => {
                    assert_expression_path!(parser, parser.tree.get(*key), "key");

                    assert_node!(parser.tree, *pattern, AssignPattern::Place { expression: value } => {
                        assert_node!(parser.tree, *value, Expression::Index { left, index: Some(index), .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "target");
                            assert_expression_path!(parser, parser.tree.get(*index), "index");
                        });
                    });
                });
            });
        });
    });
}

/// Assignment chains bind right associatively.
#[test]
fn test_parse_precedence_assignment_right_associative() {
    let test = TestParser::new("alpha = beta = computeValue()");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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

/// Conditional expressions bind tighter than assignment.
#[test]
fn test_parse_precedence_assignment_rhs_conditional() {
    let test = TestParser::new("files = commit.files ? commit.files.map((file) => file.name) : []");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(
        parser.tree,
        expr_id,
        Expression::Assign { operator, left, right } => {
            assert_eq!(*operator, AssignOperator::Assign);
            assert_expression_path!(parser, parser.tree.get(*left), "files");

            assert_node!(
                parser.tree,
                *right,
                Expression::If {
                    form: IfForm::Ternary,
                    condition,
                    then_expression,
                    else_expression,
                } => {
                    let condition = condition.as_expression().expect("expected expression condition");
                    assert_expression_path!(parser, parser.tree.get(condition), "commit.files");
                    assert_node!(parser.tree, *then_expression, Expression::Call { .. });
                    let else_expression = else_expression.expect("expected ternary else branch");
                    assert_node!(parser.tree, else_expression, Expression::ArrayExpression { elements } => {
                        assert!(elements.is_empty());
                    });
                }
            );
        }
    );
}

/// Postfix call has higher precedence than addition.
#[test]
fn test_parse_precedence_postfix_call_before_add() {
    let test = TestParser::new("a() + b() / c");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("y.sqrt() ?? 0");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
                Expression::Literal(Literal::Integer(0))
            );
        }
    );
}
