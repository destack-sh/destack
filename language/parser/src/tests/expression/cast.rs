use crate::tests::TestParser;
use crate::{
    ExpressionPosition, ExpressionStop, assert_comment, assert_expression_path, assert_node,
    assert_path, assert_string, assert_value_expression_path,
};
use tspp_dir::{
    Argument, AssignOperator, AssignPattern, Asynchrony, BinaryOperator, CommentKind, Declaration,
    Expression, FunctionDeclaration, FunctionForm, GenericArgument, IfForm, Literal, NodeType,
    Parameter, TokenType, TypeDeclaration, TypeExpression, TypeLiteral,
};
use tspp_source::DiagnosticSeverity;

/// Type casts bind to the full addition expression on the left.
#[test]
fn test_parse_precedence_cast_after_addition() {
    let test = TestParser::new("a + b as number");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // a + b as number
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        // a + b
        assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);

            // a
            assert_expression_path!(parser, parser.tree.get(*left), "a");

            // b
            assert_expression_path!(parser, parser.tree.get(*right), "b");
        });

        // number
        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Number);
        });
    });
}

/// Type casts bind to the left side before multiplication.
#[test]
fn test_parse_precedence_cast_before_multiply() {
    let test = TestParser::new("a as boolean * b");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // a as boolean * b
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::Multiply);

        // a as boolean
        assert_node!(parser.tree, *left, Expression::As { expression, target_type } => {
            // a
            assert_expression_path!(parser, parser.tree.get(*expression), "a");

            // boolean
            assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Boolean);
            });
        });

        // b
        assert_expression_path!(parser, parser.tree.get(*right), "b");
    });
}

/// Type casts bind to the full multiplication expression on the left.
#[test]
fn test_parse_precedence_cast_after_multiply() {
    let test = TestParser::new("a * b as boolean");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // a * b as boolean
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        // a * b
        assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::Multiply);

            // a
            assert_expression_path!(parser, parser.tree.get(*left), "a");

            // b
            assert_expression_path!(parser, parser.tree.get(*right), "b");
        });

        // boolean
        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Boolean);
        });
    });
}

/// Type casts bind to the full comparison expression on the left.
#[test]
fn test_parse_precedence_cast_after_comparison() {
    let test = TestParser::new("a >= b as number");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // a >= b as number
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        // a >= b
        assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);

            // a
            assert_expression_path!(parser, parser.tree.get(*left), "a");

            // b
            assert_expression_path!(parser, parser.tree.get(*right), "b");
        });

        // number
        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Number);
        });
    });
}

/// Parse parenthesized casts on the right side of comparisons.
#[test]
fn test_parse_parenthesized_cast_in_comparison_right_side() {
    let test = TestParser::new("i >= (this.length as number)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // i >= (this.length as number)
    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
        assert_expression_path!(parser, parser.tree.get(*left), "i");

        // (this.length as number)
        crate::assert_parenthesized!(parser.tree, *right, expression => {
            assert_node!(parser.tree, *expression, Expression::As { expression, target_type } => {
                // this.length
                assert_node!(parser.tree, *expression, Expression::Member { left, name, .. } => {
                    assert_node!(parser.tree, *left, Expression::This);
                    assert_string!(parser, *name, "length");
                });

                // number
                assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
            });
        });
    });
}

/// Parse logical-or expressions that compare against a parenthesized cast.
#[test]
fn test_parse_logical_or_with_parenthesized_cast_comparison() {
    let test = TestParser::new("i < 0 || i >= (this.length as number)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // i < 0 || i >= (this.length as number)
    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::Or);

        // i < 0
        assert_node!(parser.tree, *left, Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::LessThan);
        });

        // i >= (this.length as number)
        assert_node!(parser.tree, *right, Expression::Binary { operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
            crate::assert_parenthesized!(parser.tree, *right, expression => {
                assert_node!(parser.tree, *expression, Expression::As { .. } => {});
            });
        });
    });
}

/// Parse if statements with parenthesized cast comparisons in the condition.
#[test]
fn test_parse_if_condition_with_parenthesized_cast_comparison() {
    let test = TestParser::new("if (i < 0 || i >= (this.length as number)) {\n  undefined!;\n}");
    let mut parser = test.prepare();
    let _ = parser.parse_in_place();

    // if (i < 0 || i >= (this.length as number)) { ... }
    let has_parse_error = parser
        .diagnostics()
        .has_diagnostics_of_severity(DiagnosticSeverity::Error);
    assert!(!has_parse_error);
}

/// Parse type prefix operators and infer bindings.
#[test]
fn test_parse_type_unary_prefix_expression() {
    let test = TestParser::new("keyof typeof infer Value");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // keyof typeof infer Value
    assert_node!(parser.tree, expr_id, Expression::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::KeyOf { target_type } => {
            assert_node!(parser.tree, *target_type, TypeExpression::TypeOf { value } => {
                assert_node!(parser.tree, *value, Expression::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Infer { name, constraint, .. } => {
                        assert_string!(parser, name.expect("expected infer name"), "Value");
                        assert!(constraint.is_none());
                    });
                });
            });
        });
    });
}

/// Parse type unary postfix as const operation.
#[test]
fn test_parse_type_unary_postfix_as_const_expression() {
    let test = TestParser::new("Value as const");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // Value as const
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "Value");
        assert_node!(parser.tree, *target_type, TypeExpression::Const);
    });
}

/// Parse `value /* assert-tail */ as const` without mutating wrapped annotations.
#[test]
fn test_parse_as_const_keeps_wrapped_expression_plain() {
    let test = TestParser::new("value /* assert-tail */ as const");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "value");
        assert_node!(parser.tree, *target_type, TypeExpression::Const);
        let annotations = parser.tree.get_decorators(expression.id);
        assert!(annotations.is_empty());
    });
}

/// Parse `value as const` with a transparent head span.
#[test]
fn test_parse_as_const_records_semantic_head_span() {
    let test = TestParser::new("value as const");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::As { expression, target_type } => {
        assert_node!(parser.tree, *target_type, TypeExpression::Const);
        let head_range = parser
            .tree
            .get_head_range(expression_id)
            .expect("missing as const head range");
        let value_head_range = parser.expression_head_range(*expression);

        assert_eq!(head_range, value_head_range);
        assert_eq!(parser.range_str(head_range), "value");
    });
}

/// Parse comparisons against members on an identifier named `as`.
#[test]
fn test_parse_comparison_with_as_identifier_member_access() {
    let test = TestParser::new("i > as.length");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // i > as.length
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThan);
        assert_expression_path!(parser, parser.tree.get(*left), "i");
        assert_expression_path!(parser, parser.tree.get(*right), "as.length");
    });
}

/// Parse comparisons against members on an identifier named `satisfies`.
#[test]
fn test_parse_comparison_with_satisfies_identifier_member_access() {
    let test = TestParser::new("i > satisfies.length");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // i > satisfies.length
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThan);
        assert_expression_path!(parser, parser.tree.get(*left), "i");
        assert_expression_path!(parser, parser.tree.get(*right), "satisfies.length");
    });
}

/// Parse comparisons against optional members on an identifier named `as`.
#[test]
fn test_parse_comparison_with_as_identifier_optional_member_access() {
    let test = TestParser::new("i > as?.length");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // i > as?.length
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThan);
        assert_expression_path!(parser, parser.tree.get(*left), "i");
        assert_node!(parser.tree, *right, Expression::Chain { expression } => {
            assert_node!(parser.tree, *expression, Expression::Member { left, name, is_optional } => {
                assert_string!(parser, *name, "length");
                assert!(*is_optional);
                assert_expression_path!(parser, parser.tree.get(*left), "as");
            });
        });
    });
}

/// Parse typed arrow bodies that reference a parameter named `as`.
#[test]
fn test_parse_typed_arrow_body_with_as_parameter_member_access() {
    let test = TestParser::new("(as: Array<number>) => i > as.length");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // (as: Array<number>) => i > as.length
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.parameters.len(), 1);

            // (as: Array<number>)
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                assert_string!(parser, *name, "as");
                assert_node!(parser.tree, *declared_type, TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "Array");
                    assert_eq!(generic_arguments.len(), 1);
                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                            assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                                assert_eq!(*value, TypeLiteral::Number);
                            });
                    });
                });
            });

            // i > as.length
            assert_node!(parser.tree, body.expect("expected body"), Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                assert_expression_path!(parser, parser.tree.get(*left), "i");
                assert_expression_path!(parser, parser.tree.get(*right), "as.length");
            });
        });
    });
}

/// Parse nested typed arrows with an `as` parameter used in a ternary condition.
#[test]
fn test_parse_typed_arrow_with_as_parameter_in_ternary_condition() {
    let source = r#"<A,>(i: number, a: A) =>
  (as: Array<A>): Option<NonEmptyArray<A>> =>
    i < 0 || i > as.length ? _.none : _.some(unsafeInsertAt(i, a, as))"#;
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // <A,>(i: number, a: A) =>
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.parameters.len(), 2);

            // (i: number, a: A)
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                assert_string!(parser, *name, "i");
                assert_node!(parser.tree, *declared_type, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
            });
            assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                assert_string!(parser, *name, "a");
                assert_expression_path!(parser, parser.tree.get(*declared_type), "A");
            });

            // (as: Array<A>): Option<NonEmptyArray<A>> =>
            assert_node!(parser.tree, body.expect("expected body"), Expression::Declaration(inner_declaration_id) => {
                assert_node!(parser.tree, *inner_declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_eq!(signature.parameters.len(), 1);
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                        assert_string!(parser, *name, "as");
                    });

                    // i < 0 || i > as.length ? _.none : _.some(...)
                    assert_node!(parser.tree, body.expect("expected body"), Expression::If { condition, then_expression, else_expression, .. } => {
                        let condition_id = condition.as_expression().expect("expected expression condition");
                        assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
                            assert_eq!(*operator, BinaryOperator::Or);
                            assert_node!(parser.tree, *left, Expression::Binary { left, operator, right } => {
                                assert_eq!(*operator, BinaryOperator::LessThan);
                                assert_expression_path!(parser, parser.tree.get(*left), "i");
                                assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(0)));
                            });
                            assert_node!(parser.tree, *right, Expression::Binary { left, operator, right } => {
                                assert_eq!(*operator, BinaryOperator::GreaterThan);
                                assert_expression_path!(parser, parser.tree.get(*left), "i");
                                assert_expression_path!(parser, parser.tree.get(*right), "as.length");
                            });
                        });

                        assert_expression_path!(parser, parser.tree.get(*then_expression), "_.none");
                        assert_node!(parser.tree, else_expression.expect("expected else expression"), Expression::Call { left, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "_.some");
                        });
                    });
                });
            });
        });
    });
}

/// Parse async identifiers with `as` casts.
#[test]
fn test_parse_async_as_cast() {
    let test = TestParser::new("async as unknown");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "async");
        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Unknown);
        });
    });
}

/// Parse casts with a missing type target.
#[test]
fn test_parse_as_cast_missing_type_target() {
    let test = TestParser::new("value as");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.range_str(parser.errors[0].range()), "");

    // value as
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "value");
        assert_node!(parser.tree, *target_type, TypeExpression::Missing);
    });
}

/// Parse satisfies expressions with a missing type target.
#[test]
fn test_parse_satisfies_missing_type_target() {
    let test = TestParser::new("value satisfies");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.range_str(parser.errors[0].range()), "");

    // value satisfies
    assert_node!(parser.tree, expr_id, Expression::Satisfies { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "value");
        assert_node!(parser.tree, *target_type, TypeExpression::Missing);
    });
}

/// Recover assertion operators that start on a new line.
#[test]
fn test_recover_newline_before_assertion_operator() {
    for operator in ["as", "satisfies"] {
        let source = format!("call(value\n{operator} number)");
        let test = TestParser::new(&source);
        let mut parser = test.prepare();

        parser.parse_in_place();

        test.assert_errors(
            &parser,
            &[
                (
                    Some(NodeType::Expression),
                    Some(TokenType::Identifier),
                    Some(TokenType::CloseParenthesis),
                    operator,
                ),
                (None, Some(TokenType::CloseParenthesis), None, ")"),
            ],
        );
    }
}

/// Parse multiline cast rhs unions after an own-line boundary comment.
#[test]
fn test_parse_cast_rhs_leading_union_after_own_line_comment() {
    let test = TestParser::new(
        "functionArg = a as\n  // comment\n  TSESTree.ArrowFunctionExpression\n  | TSESTree.ArrowFunctionExpression\n  | TSESTree.FunctionExpression\n  | undefined",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // functionArg = a as // comment Some.Namespace.Type | ...
    assert_node!(parser.tree, expr_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::Assign);
        assert_expression_path!(parser, parser.tree.get(*left), "functionArg");

        // a as ...
        assert_node!(parser.tree, *right, Expression::As { expression, target_type } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "a");

            // Some.Namespace.Type | ...
            assert_node!(parser.tree, *target_type, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 4);
            });
        });
    });
}
#[test]
fn test_parse_as_rhs_with_line_comment() {
    let test = TestParser::new("value as // as-tail\nnumber");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, expression_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "value");
        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Number);
        });
    });

    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "as-tail");
}

/// Parse `satisfies` rhs comments without synthetic separator ownership.
#[test]
fn test_parse_satisfies_rhs_with_line_comment() {
    let test = TestParser::new("value satisfies // sat-tail\nFoo");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, expression_id, Expression::Satisfies { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "value");
        assert_expression_path!(parser, parser.tree.get(*target_type), "Foo");
    });

    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "sat-tail");
}

/// Parse async arrows with a parameter named `as`.
#[test]
fn test_parse_async_arrow_with_as_parameter() {
    let test = TestParser::new("async as => {}");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.asynchrony, Asynchrony::Async);
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "as");
            });
            assert!(body.is_some());
        });
    });
}

/// Recover cast tails as malformed parameters without abandoning their arrow functions.
#[test]
fn test_recover_arrow_parameter_cast_tails() {
    let cases = [
        ("(a as T) => {};", "as"),
        ("async (a satisfies T) => {};", "satisfies"),
    ];

    for (source, error_text) in cases {
        let source = format!("{source}\nconst stable = 1;");
        let test = TestParser::new(&source);
        let mut parser = test.prepare();
        let expressions = parser.parse_in_place();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                assert_eq!(signature.parameters.len(), 1);
                assert_node!(parser.tree, signature.parameters[0], Parameter::Error);
                assert!(body.is_some());
            });
        });
        assert_node!(parser.tree, expressions[1], Expression::Let { .. });
        test.assert_errors(
            &parser,
            &[(
                Some(NodeType::Parameter),
                Some(TokenType::Identifier),
                None,
                error_text,
            )],
        );
    }
}

/// Parse async arrows with a newline before a return type annotation.
#[test]
fn test_parse_async_arrow_with_newline_before_return_type() {
    let test = TestParser::new("async (f)\n: t => { }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.asynchrony, Asynchrony::Async);
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "f");
            });
            assert_expression_path!(parser, parser.tree.get(signature.return_type.expect("expected return type")), "t");
            assert!(body.is_some());
        });
    });
}

#[test]
fn test_parse_type_keyword_as_cast_expression() {
    let test = TestParser::new("type as string");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "type");
        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::String);
        });
    });
}

#[test]
fn test_parse_module_identifier_as_cast_expression() {
    let test = TestParser::new("module as DynamicModule");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "module");
        assert_expression_path!(parser, parser.tree.get(*target_type), "DynamicModule");
    });
}

#[test]
fn test_parse_namespace_identifier_as_cast_call_argument() {
    let test = TestParser::new("render(cloned, rootContainer, namespace as ElementNamespace)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "render");
        assert_eq!(arguments.len(), 3);
        assert_node!(parser.tree, arguments[2], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "namespace");
                assert_expression_path!(parser, parser.tree.get(*target_type), "ElementNamespace");
            });
        });
    });
}

#[test]
fn test_parse_cast_with_keyof_typeof_type_argument() {
    let test = TestParser::new("Object.keys(touchedFields) as Array<keyof typeof touchedFields>");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_node!(parser.tree, *expression, Expression::Call { left, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Object.keys");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "touchedFields");
            });
        });
        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "Array");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::KeyOf { target_type } => {
                        assert_node!(parser.tree, *target_type, TypeExpression::TypeOf { value } => {
                            assert_expression_path!(parser, parser.tree.get(*value), "touchedFields");
                        });
                    });
            });
        });
    });
}

/// Parse casts whose type target is a conditional type.
#[test]
fn test_parse_cast_with_conditional_type_target() {
    let test = TestParser::new("value as Flag extends true ? Selected : Rejected");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // value as Flag extends true ? Selected : Rejected
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "value");

        // Flag extends true ? Selected : Rejected
        assert_node!(parser.tree, *target_type, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Flag");
            assert_node!(parser.tree, *extends_type, TypeExpression::Literal { value } => {
                assert_eq!(*value, Literal::Boolean(true));
            });
            assert_expression_path!(parser, parser.tree.get(*then_type), "Selected");
            assert_expression_path!(parser, parser.tree.get(*else_type), "Rejected");
        });
    });
}

/// Parse ternary expressions after an `as` cast.
#[test]
fn test_parse_cast_followed_by_ternary_expression() {
    let test =
        TestParser::new("perFileCache === resolvedModuleNames as unknown ? resolved : fallback");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // perFileCache === (resolvedModuleNames as unknown) ? resolved : fallback
    assert_node!(parser.tree, expr_id, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        let condition = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition, Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::EqualStrict);
                assert_expression_path!(parser, parser.tree.get(*left), "perFileCache");
                assert_node!(parser.tree, *right, Expression::As { expression, target_type } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "resolvedModuleNames");
                    assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Unknown);
                    });
                });
        });
        assert_expression_path!(parser, parser.tree.get(*then_expression), "resolved");
        assert_expression_path!(parser, parser.tree.get(else_expression.expect("expected else expression")), "fallback");
    });
}

/// Parse ternary expressions after `satisfies`.
#[test]
fn test_parse_satisfies_followed_by_ternary_expression() {
    let test = TestParser::new("value satisfies SomeType ? yes : no");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        let condition = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition, Expression::Satisfies { expression, target_type } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "value");
                assert_expression_path!(parser, parser.tree.get(*target_type), "SomeType");
        });
        assert_expression_path!(parser, parser.tree.get(*then_expression), "yes");
        assert_expression_path!(parser, parser.tree.get(else_expression.expect("expected else expression")), "no");
    });
}

#[test]
fn test_parse_parenthesized_cast_followed_by_flat_map_call() {
    let test = TestParser::new(
        "(Object.keys(touchedFields) as Array<keyof typeof touchedFields>).flatMap((topLevelKey) => topLevelKey)",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Call { left, arguments, .. } => {
        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "flatMap");
            crate::assert_parenthesized!(parser.tree, *left, expression => {
                assert_node!(parser.tree, *expression, Expression::As { .. } => {});
            });
        });
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                    assert_eq!(signature.parameters.len(), 1);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_alias_named_as_or_satisfies() {
    let test = TestParser::new("type as = 0;\ntype satisfies = 0;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 2);

    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "as");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, Literal::Integer(0));
            });
        });
    });

    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "satisfies");
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, Literal::Integer(0));
            });
        });
    });
}
#[test]
fn test_parse_new_expression_with_generic_receiver_and_const_assertion_argument() {
    let test = TestParser::new("new Set<keyof A | keyof B>([\"connect\"] as const)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, generic_arguments, arguments } => {
        assert_value_expression_path!(parser, parser.tree.get(*left), "Set");
        assert_eq!(generic_arguments.len(), 1);

        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                assert_node!(parser.tree, *target_type, TypeExpression::Const);
                assert_node!(parser.tree, *expression, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 1);
                });
            });
        });
    });
}

/// Report angle bracket assertions in `new` receivers.
#[test]
fn test_report_type_assertion_in_new_receiver() {
    let test = TestParser::new("new <unknown>Test2();");
    let mut parser = test.prepare();
    let error = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), "<");
}

/// Report unparenthesized cast assignment targets.
#[test]
fn test_report_unparenthesized_cast_assignment_target() {
    let test = TestParser::new("value as number = 2");
    let mut parser = test.prepare();
    let error = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), "value as number");
}

/// Report unparenthesized satisfies assignment targets.
#[test]
fn test_report_unparenthesized_satisfies_assignment_target() {
    let test = TestParser::new("value satisfies number = 2");
    let mut parser = test.prepare();
    let error = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), "value satisfies number");
}

/// Parse parenthesized cast assignment targets.
#[test]
fn test_parse_parenthesized_cast_assignment_target() {
    let test = TestParser::new("(value as number) = 2");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::Assign);

        assert_node!(parser.tree, *left, AssignPattern::Place { expression: value } => {
            assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "value");
                assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
            });
        });

        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(2)));
    });
}

/// Parse parenthesized satisfies assignment targets.
#[test]
fn test_parse_parenthesized_satisfies_assignment_target() {
    let test = TestParser::new("(value satisfies number) = 2");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::Assign);

        assert_node!(parser.tree, *left, AssignPattern::Place { expression: value } => {
            assert_node!(parser.tree, *value, Expression::Satisfies { expression, target_type } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "value");
                assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
            });
        });

        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(2)));
    });
}
