use crate::tests::*;
use crate::{assert_comment, assert_expression_path, assert_node, assert_path, assert_string};
use destack_dir::*;
use destack_source::LanguageType;

#[test]
fn test_reject_angle_type_assertion_expression() {
    let mut test = TestParser::new_with_language("<any>value", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let result = parser.eat_expression(parser.flags);
    assert!(result.is_err());
}

/// Reject a const assertion in angle bracket form.
#[test]
fn test_reject_angle_const_assertion_expression() {
    let mut test = TestParser::new_with_language("<const>[1, 2, 3]", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let result = parser.eat_expression(parser.flags);
    assert!(result.is_err());
}

/// Type casts bind to the full addition expression on the left.
#[test]
fn test_parse_precedence_cast_after_addition() {
    let mut test = TestParser::new("a + b as number");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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
        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Number);
        });
    });
}

/// Type casts bind to the left side before multiplication.
#[test]
fn test_parse_precedence_cast_before_multiply() {
    let mut test = TestParser::new("a as boolean * b");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // a as boolean * b
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::Multiply);

        // a as boolean
        assert_node!(parser.tree, *left, Expression::As { expression, target_type } => {
            // a
            assert_expression_path!(parser, parser.tree.get(*expression), "a");

            // boolean
            assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
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
    let mut test = TestParser::new("a * b as boolean");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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
        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Boolean);
        });
    });
}

/// Type casts bind to the full comparison expression on the left.
#[test]
fn test_parse_precedence_cast_after_comparison() {
    let mut test = TestParser::new("a >= b as number");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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
        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Number);
        });
    });
}

/// Parse parenthesized casts on the right side of comparisons.
#[test]
fn test_parse_parenthesized_cast_in_comparison_right_side() {
    let mut test = TestParser::new("i >= (this.length as number)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // i >= (this.length as number)
    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right, .. } => {
        assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
        assert_expression_path!(parser, parser.tree.get(*left), "i");

        // (this.length as number)
        assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::As { expression, target_type } => {
                // this.length
                assert_node!(parser.tree, *expression, Expression::Member { left, name, .. } => {
                    assert_node!(parser.tree, *left, Expression::This);
                    assert_string!(parser, *name, "length");
                });

                // number
                assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
            });
        });
    });
}

/// Parse logical-or expressions that compare against a parenthesized cast.
#[test]
fn test_parse_logical_or_with_parenthesized_cast_comparison() {
    let mut test = TestParser::new("i < 0 || i >= (this.length as number)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

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
            assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::As { .. } => {});
            });
        });
    });
}

/// Parse if statements with parenthesized cast comparisons in the condition.
#[test]
fn test_parse_if_condition_with_parenthesized_cast_comparison() {
    let mut test =
        TestParser::new("if (i < 0 || i >= (this.length as number)) {\n  undefined!;\n}");
    let mut parser = test.prepare();
    let _ = parser.parse();

    // if (i < 0 || i >= (this.length as number)) { ... }
    let has_parse_error = parser
        .diagnostics
        .to_vec()
        .into_iter()
        .any(|diagnostic| diagnostic.code.starts_with("EP"));
    assert!(!has_parse_error);
}

/// Parse type prefix operators and infer bindings.
#[test]
fn test_parse_type_unary_prefix_expression() {
    let mut test = TestParser::new("keyof typeof infer Value");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // keyof typeof infer Value
    assert_node!(parser.tree, expr_id, Expression::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::KeyOf { target_type } => {
            assert_node!(parser.tree, *target_type, TypeExpression::TypeOfValue { value } => {
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
    let mut test = TestParser::new("Value as const");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // Value as const
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "Value");
        assert_node!(parser.tree, *target_type, TypeExpression::Const);
    });
}

/// Parse `value /* assert-tail */ as const` without mutating wrapped annotations.
#[test]
fn test_parse_as_const_keeps_wrapped_expression_plain() {
    let mut test = TestParser::new("value /* assert-tail */ as const");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

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
    let mut test = TestParser::new("value as const");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::As { expression, target_type } => {
        assert_node!(parser.tree, *target_type, TypeExpression::Const);
        let head_span = parser
            .tree
            .get_head_span(expression_id)
            .expect("missing as const head span");
        let value_head_span = parser.expression_head_span(*expression);

        assert_eq!(head_span, value_head_span);
        assert_eq!(parser.get_span_str(head_span), "value");
    });
}

/// Parse type unary postfix as comptime operation.
#[test]
fn test_parse_type_unary_postfix_as_comptime_expression() {
    let mut test = TestParser::new("Value as comptime");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // Value as comptime
    assert_node!(parser.tree, expr_id, Expression::Comptime { body } => {
        assert_expression_path!(parser, parser.tree.get(*body), "Value");
    });
}

/// Parse comparisons against members on an identifier named `as`.
#[test]
fn test_parse_comparison_with_as_identifier_member_access() {
    for language in [LanguageType::TypeScript, LanguageType::Destack] {
        let mut test = TestParser::new_with_language("i > as.length", language);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.flags).unwrap();

        // i > as.length
        assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::GreaterThan);
            assert_expression_path!(parser, parser.tree.get(*left), "i");
            assert_expression_path!(parser, parser.tree.get(*right), "as.length");
        });
    }
}

/// Parse comparisons against members on an identifier named `satisfies`.
#[test]
fn test_parse_comparison_with_satisfies_identifier_member_access() {
    for language in [LanguageType::TypeScript, LanguageType::Destack] {
        let mut test = TestParser::new_with_language("i > satisfies.length", language);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.flags).unwrap();

        // i > satisfies.length
        assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::GreaterThan);
            assert_expression_path!(parser, parser.tree.get(*left), "i");
            assert_expression_path!(parser, parser.tree.get(*right), "satisfies.length");
        });
    }
}

/// Parse comparisons against optional members on an identifier named `as`.
#[test]
fn test_parse_comparison_with_as_identifier_optional_member_access() {
    for language in [LanguageType::TypeScript, LanguageType::Destack] {
        let mut test = TestParser::new_with_language("i > as?.length", language);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression(parser.flags).unwrap();

        // i > as?.length
        assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::GreaterThan);
            assert_expression_path!(parser, parser.tree.get(*left), "i");
            assert_node!(parser.tree, *right, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "length");
                assert_node!(parser.tree, *left, Expression::Maybe { left, position: PostfixPosition::Direct } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "as");
                });
            });
        });
    }
}

/// Parse typed arrow bodies that reference a parameter named `as`.
#[test]
fn test_parse_typed_arrow_body_with_as_parameter_member_access() {
    let mut test = TestParser::new_with_language(
        "(as: Array<number>) => i > as.length",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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
                            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
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
    let source = r#"<A>(i: number, a: A) =>
  (as: Array<A>): Option<NonEmptyArray<A>> =>
    i < 0 || i > as.length ? _.none : _.some(unsafeInsertAt(i, a, as))"#;
    let mut test = TestParser::new_with_language(source, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // <A>(i: number, a: A) =>
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.parameters.len(), 2);

            // (i: number, a: A)
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                assert_string!(parser, *name, "i");
                assert_node!(parser.tree, *declared_type, TypeExpression::Literal { value } => {
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
                        let condition_id = match condition {
                            IfCondition::Expression { condition } => *condition,
                            IfCondition::Let { .. } => panic!("expected expression condition"),
                        };
                        assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
                            assert_eq!(*operator, BinaryOperator::Or);
                            assert_node!(parser.tree, *left, Expression::Binary { left, operator, right } => {
                                assert_eq!(*operator, BinaryOperator::LessThan);
                                assert_expression_path!(parser, parser.tree.get(*left), "i");
                                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
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
    let mut test = TestParser::new_with_language("async as any", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "async");
        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Any);
        });
    });
}

/// Parse casts with a missing type target.
#[test]
fn test_parse_as_cast_missing_type_target() {
    let mut test = TestParser::new_with_language("value as", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.get_span_str(parser.errors[0].leaf_span()), "");

    // value as
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "value");
        assert_node!(parser.tree, *target_type, TypeExpression::Missing);
    });
}

/// Parse satisfies expressions with a missing type target.
#[test]
fn test_parse_satisfies_missing_type_target() {
    let mut test = TestParser::new_with_language("value satisfies", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.get_span_str(parser.errors[0].leaf_span()), "");

    // value satisfies
    assert_node!(parser.tree, expr_id, Expression::Satisfies { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "value");
        assert_node!(parser.tree, *target_type, TypeExpression::Missing);
    });
}

/// Reject assertion operators that start on a new line.
#[test]
fn test_reject_newline_before_assertion_operator() {
    for language in [LanguageType::TypeScript, LanguageType::Destack] {
        for operator in ["as", "satisfies"] {
            let source = format!("call(value\n{operator} number)");
            let mut test = TestParser::new_with_language(&source, language);
            let mut parser = test.prepare();

            parser.parse();

            test.assert_error_leaves(
                &parser,
                &[
                    (Some(NodeType::Expression), None, operator),
                    (None, None, "number"),
                    (None, None, ")"),
                ],
            );
        }
    }
}

/// Parse multiline cast rhs unions after an own-line boundary comment.
#[test]
fn test_parse_cast_rhs_leading_union_after_own_line_comment() {
    let mut test = TestParser::new_with_language(
        "functionArg = a as\n  // comment\n  TSESTree.ArrowFunctionExpression\n  | TSESTree.ArrowFunctionExpression\n  | TSESTree.FunctionExpression\n  | undefined",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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

/// Parse multiline satisfies rhs intersections with a leading separator.
#[test]
fn test_parse_satisfies_rhs_leading_intersection() {
    let mut test = TestParser::new_with_language(
        "value satisfies\n  & Foo\n  & Bar",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // value satisfies & Foo & Bar
    assert_node!(parser.tree, expr_id, Expression::Satisfies { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "value");
        assert_node!(parser.tree, *target_type, TypeExpression::Intersection { elements } => {
            assert_eq!(elements.len(), 2);
            assert_expression_path!(parser, parser.tree.get(elements[0]), "Foo");
            assert_expression_path!(parser, parser.tree.get(elements[1]), "Bar");
        });
    });
}

/// Parse `as` rhs comments without synthetic separator ownership.
#[test]
fn test_parse_as_rhs_with_line_comment() {
    for language in [LanguageType::TypeScript, LanguageType::Destack] {
        let mut test = TestParser::new_with_language("value as // as-tail\nnumber", language);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.flags).unwrap();
        parser.attach_comments();

        assert_node!(parser.tree, expression_id, Expression::As { expression, target_type } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "value");
            assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Number);
            });
        });

        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "as-tail");
    }
}

/// Parse `satisfies` rhs comments without synthetic separator ownership.
#[test]
fn test_parse_satisfies_rhs_with_line_comment() {
    for language in [LanguageType::TypeScript, LanguageType::Destack] {
        let mut test = TestParser::new_with_language("value satisfies // sat-tail\nFoo", language);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.flags).unwrap();
        parser.attach_comments();

        assert_node!(parser.tree, expression_id, Expression::Satisfies { expression, target_type } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "value");
            assert_expression_path!(parser, parser.tree.get(*target_type), "Foo");
        });

        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "sat-tail");
    }
}

/// Parse async arrows with a parameter named `as`.
#[test]
fn test_parse_async_arrow_with_as_parameter() {
    let mut test = TestParser::new_with_language("async as => {}", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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

/// Parse async arrows with a newline before a return type annotation.
#[test]
fn test_parse_async_arrow_with_newline_before_return_type() {
    let mut test = TestParser::new_with_language("async (f)\n: t => { }", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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
    let mut test = TestParser::new_with_language("type as string", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "type");
        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::String);
        });
    });
}

#[test]
fn test_parse_module_identifier_as_cast_expression() {
    let mut test =
        TestParser::new_with_language("module as DynamicModule", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "module");
        assert_expression_path!(parser, parser.tree.get(*target_type), "DynamicModule");
    });
}

#[test]
fn test_parse_namespace_identifier_as_cast_call_argument() {
    let mut test = TestParser::new_with_language(
        "render(cloned, rootContainer, namespace as ElementNamespace)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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
    let mut test = TestParser::new_with_language(
        "Object.keys(touchedFields) as Array<keyof typeof touchedFields>",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

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
                        assert_node!(parser.tree, *target_type, TypeExpression::TypeOfValue { value } => {
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
    let mut test = TestParser::new_with_language(
        "value as Flag extends true ? Selected : Rejected",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // value as Flag extends true ? Selected : Rejected
    assert_node!(parser.tree, expr_id, Expression::As { expression, target_type } => {
        assert_expression_path!(parser, parser.tree.get(*expression), "value");

        // Flag extends true ? Selected : Rejected
        assert_node!(parser.tree, *target_type, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Flag");
            assert_node!(parser.tree, *extends_type, TypeExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::Boolean(true));
            });
            assert_expression_path!(parser, parser.tree.get(*then_type), "Selected");
            assert_expression_path!(parser, parser.tree.get(*else_type), "Rejected");
        });
    });
}

/// Parse ternary expressions after an `as` cast.
#[test]
fn test_parse_cast_followed_by_ternary_expression() {
    let mut test = TestParser::new_with_language(
        "perFileCache === resolvedModuleNames as unknown ? resolved : fallback",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // perFileCache === (resolvedModuleNames as unknown) ? resolved : fallback
    assert_node!(parser.tree, expr_id, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        assert_node!(condition, IfCondition::Expression { condition } => {
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::EqualStrict);
                assert_expression_path!(parser, parser.tree.get(*left), "perFileCache");
                assert_node!(parser.tree, *right, Expression::As { expression, target_type } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "resolvedModuleNames");
                    assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Unknown);
                    });
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
    let mut test = TestParser::new_with_language(
        "value satisfies SomeType ? yes : no",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        assert_node!(condition, IfCondition::Expression { condition } => {
            assert_node!(parser.tree, *condition, Expression::Satisfies { expression, target_type } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "value");
                assert_expression_path!(parser, parser.tree.get(*target_type), "SomeType");
            });
        });
        assert_expression_path!(parser, parser.tree.get(*then_expression), "yes");
        assert_expression_path!(parser, parser.tree.get(else_expression.expect("expected else expression")), "no");
    });
}

#[test]
fn test_parse_parenthesized_cast_followed_by_flat_map_call() {
    let mut test = TestParser::new_with_language(
        "(Object.keys(touchedFields) as Array<keyof typeof touchedFields>).flatMap((topLevelKey) => topLevelKey)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Call { left, arguments, .. } => {
        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "flatMap");
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
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
    let mut test = TestParser::new_with_language(
        "type as = 0;\ntype satisfies = 0;",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);

    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "as");
            assert_node!(parser.tree, *value, TypeExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::Integer(0));
            });
        });
    });

    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, value, .. }) => {
            assert_string!(parser, name.string(), "satisfies");
            assert_node!(parser.tree, *value, TypeExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::Integer(0));
            });
        });
    });
}

/// Reject angle bracket assertions in disallow ambiguous mode.
#[test]
fn test_reject_type_assertion_when_disallow_ambiguous_tree_literal() {
    let mut test = TestParser::new_with_language("<T>x", LanguageType::TypeScript);
    let mut parser = test.prepare();
    parser.flags.set_disallow_ambiguous_tree_literal(true);

    let result = parser.eat_expression(parser.flags);
    assert!(result.is_err());
}

/// Reject ambiguous generic arrows in disallow ambiguous mode.
#[test]
fn test_reject_generic_arrow_when_disallow_ambiguous_tree_literal() {
    let mut test = TestParser::new_with_language("<T>() => 1", LanguageType::TypeScript);
    let mut parser = test.prepare();
    parser.flags.set_disallow_ambiguous_tree_literal(true);

    let result = parser.eat_expression(parser.flags);
    assert!(result.is_err());
}

/// Parse `new` calls with generic receivers and const assertion arguments.
#[test]
fn test_parse_new_expression_with_generic_receiver_and_const_assertion_argument() {
    let mut test = TestParser::new_with_language(
        "new Set<keyof A | keyof B>([\"connect\"] as const)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::New { left, generic_arguments, arguments } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Set");
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

/// Reject angle bracket assertions in `new` receivers.
#[test]
fn test_reject_type_assertion_in_new_receiver() {
    let mut test = TestParser::new_with_language("new <any>Test2();", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let result = parser.eat_expression(parser.flags);
    assert!(result.is_err());
}

/// Reject unparenthesized cast assignment targets.
#[test]
fn test_reject_unparenthesized_cast_assignment_target() {
    let mut test = TestParser::new_with_language("value as number = 2", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let result = parser.eat_expression(parser.flags);
    assert!(result.is_err());
}

/// Reject unparenthesized satisfies assignment targets.
#[test]
fn test_reject_unparenthesized_satisfies_assignment_target() {
    let mut test =
        TestParser::new_with_language("value satisfies number = 2", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let result = parser.eat_expression(parser.flags);
    assert!(result.is_err());
}

/// Parse parenthesized cast assignment targets.
#[test]
fn test_parse_parenthesized_cast_assignment_target() {
    let mut test = TestParser::new_with_language("(value as number) = 2", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::Assign);

        assert_node!(parser.tree, *left, AssignPattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "value");
                assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
            });
        });

        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
    });
}

/// Parse parenthesized satisfies assignment targets.
#[test]
fn test_parse_parenthesized_satisfies_assignment_target() {
    let mut test =
        TestParser::new_with_language("(value satisfies number) = 2", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::Assign);

        assert_node!(parser.tree, *left, AssignPattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Satisfies { expression, target_type } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "value");
                assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
            });
        });

        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
    });
}
