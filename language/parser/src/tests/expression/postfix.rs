use crate::tests::TestParser;
use crate::{
    ExpressionPosition, ExpressionStop, assert_expression_path, assert_node, assert_string,
};
use tspp_dir::{
    Argument, BinaryOperator, Declaration, Declarator, Expression, FunctionDeclaration, IfForm,
    Literal, LocalNodeId, Parameter, PostfixPosition, TypeExpression, TypeLiteral,
};

/// Assert one direct maybe expression wrapping a call.
fn assert_direct_maybe_call(
    parser: &crate::Parser,
    expression_id: LocalNodeId<Expression>,
    expected_callee: &str,
) {
    assert_node!(parser.tree, expression_id, Expression::Maybe { left, position } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_node!(parser.tree, *left, Expression::Call { left, arguments, .. } => {
            assert!(arguments.is_empty());
            assert_expression_path!(parser, parser.tree.get(*left), expected_callee);
        });
    });
}

/// Parse optional chaining after comment-separated newlines.
#[test]
fn test_parse_optional_chain_after_comment_newlines() {
    let input = "promise\n  .then(noop)\n  // comment\n  // comment\n  ?.catch(noop)";
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);

    let statement_id = expressions[0];

    assert_node!(parser.tree, statement_id, Expression::Chain { expression } => {
        assert_node!(parser.tree, *expression, Expression::Call { left, arguments, .. } => {
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "noop");
            });

            assert_node!(parser.tree, *left, Expression::Member { left, name, is_optional, .. } => {
                assert_string!(parser, *name, "catch");
                assert!(*is_optional);
                assert_node!(parser.tree, *left, Expression::Call { .. });
            });
        });
    });
}

/// Parse direct `?` before arithmetic continuation.
#[test]
fn test_parse_direct_maybe_before_arithmetic() {
    let test = TestParser::new("encode()? + 1");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::Add);
        assert_direct_maybe_call(&parser, *left, "encode");
        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(1)));
    });
}

/// Parse direct `?` before logical continuation.
#[test]
fn test_parse_direct_maybe_before_logical() {
    let test = TestParser::new("encode()? && ready");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::And);
        assert_direct_maybe_call(&parser, *left, "encode");
        assert_expression_path!(parser, parser.tree.get(*right), "ready");
    });
}

/// Parse direct `?` before an `as` assertion continuation.
#[test]
fn test_parse_direct_maybe_before_as() {
    let test = TestParser::new("encode()? as string");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::As { expression, target_type } => {
        assert_direct_maybe_call(&parser, *expression, "encode");
        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::String);
        });
    });
}

/// Parse direct `?` before a type assertion continuation.
#[test]
fn test_parse_direct_maybe_before_satisfies() {
    let test = TestParser::new("encode(value)? satisfies string");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Satisfies { expression, target_type } => {
        assert_node!(parser.tree, *expression, Expression::Maybe { left, position } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_node!(parser.tree, *left, Expression::Call { .. });
        });

        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::String);
        });
    });
}

/// Parse an unspaced `?.` after a call as an optional chain member.
#[test]
fn test_parse_direct_maybe_before_member() {
    let test = TestParser::new("encode()?.field");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::Chain { expression } => {
        assert_node!(parser.tree, *expression, Expression::Member { left, name, is_optional, .. } => {
            assert_string!(parser, *name, "field");
            assert!(*is_optional);
            assert_node!(parser.tree, *left, Expression::Call { left, arguments, .. } => {
                assert!(arguments.is_empty());
                assert_expression_path!(parser, parser.tree.get(*left), "encode");
            });
        });
    });
}

/// Parse direct `?` before index continuation.
#[test]
fn test_parse_direct_maybe_before_index() {
    let test = TestParser::new("encode()?[0]");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::Index { left, index, position, .. } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_direct_maybe_call(&parser, *left, "encode");
        assert_node!(parser.tree, index.expect("expected index"), Expression::Literal(Literal::Integer(0)));
    });
}

/// Parse a detached question mark after an identifier condition as a ternary.
#[test]
fn test_parse_identifier_question_expression_as_ternary() {
    let test = TestParser::new("a ? b : c");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_expression_path!(parser, parser.tree.get(condition_id), "a");
        assert_expression_path!(parser, parser.tree.get(*then_expression), "b");
        assert_expression_path!(parser, parser.tree.get(else_expression.expect("expected else expression")), "c");
    });
}

/// Parse direct `?` after a generic call with an escaped string argument.
#[test]
fn test_parse_direct_maybe_after_generic_call_with_escaped_string() {
    let test = TestParser::new(
        r#"const config = decode<ServerConfig>("{\"host\":\"127.0.0.1\",\"port\":8080,\"secure\":true}")?;"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Maybe { left, position } => {
                assert_eq!(*position, PostfixPosition::Direct);
                assert_node!(parser.tree, *left, Expression::Call { .. });
            });
        });
    });
}

/// Parse direct `?` after a multiline generic call.
#[test]
fn test_parse_direct_maybe_after_multiline_generic_call() {
    let test = TestParser::new(
        r#"const config = decode<AppConfig>(
    "{\"database\":{\"url\":\"postgres://local\"}",
)?;"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
        assert_node!(parser.tree, declarators[0], Declarator { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Maybe { left, position } => {
                assert_eq!(*position, PostfixPosition::Direct);
                assert_node!(parser.tree, *left, Expression::Call { .. });
            });
        });
    });
}

/// Parse direct `?` after a qualified method call.
#[test]
fn test_parse_direct_maybe_after_qualified_call() {
    let test = TestParser::new("JSON.parse(text)?;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Maybe { left, position } => {
        assert_eq!(*position, PostfixPosition::Direct);
        assert_node!(parser.tree, *left, Expression::Call { .. });
    });
}

/// Parse direct `?` after a qualified method call before a type assertion.
#[test]
fn test_parse_direct_maybe_after_qualified_call_before_satisfies() {
    let test = TestParser::new("JSON.stringify(value)? satisfies string;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Satisfies { expression, target_type } => {
        assert_node!(parser.tree, *expression, Expression::Maybe { left, position } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_node!(parser.tree, *left, Expression::Call { .. });
        });

        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::String);
        });
    });
}

#[test]
fn test_parse_optional_call_after_question_dot_line_comment_newline() {
    let input = "call?.// comment\n()";
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Chain { expression } => {
        assert_node!(parser.tree, *expression, Expression::Call { left, arguments, position, is_optional, .. } => {
            assert_eq!(*position, PostfixPosition::Indirect);
            assert!(*is_optional);
            assert!(arguments.is_empty());
            assert_expression_path!(parser, parser.tree.get(*left), "call");
        });
    });
}

/// Parse optional chaining when the member target starts on the next line after ?..
#[test]
fn test_parse_optional_chain_member_after_question_dot_newline() {
    let input = "items?.\nmap(noop)";
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    // items?.map(noop)
    assert_node!(parser.tree, expression_id, Expression::Chain { expression } => {
        assert_node!(parser.tree, *expression, Expression::Call { left, arguments, .. } => {
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "noop");
            });

            assert_node!(parser.tree, *left, Expression::Member { left, name, is_optional, .. } => {
                assert_string!(parser, *name, "map");
                assert!(*is_optional);
                assert_expression_path!(parser, parser.tree.get(*left), "items");
            });
        });
    });
}

/// Parse chained optional members with newline-delimited segments.
#[test]
fn test_parse_optional_chain_chained_members_after_question_dot_newline() {
    let input = "permissions?.\nconcat(first).\nconcat(second)";
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Chain { expression } => {
        assert_node!(parser.tree, *expression, Expression::Call { left, arguments, .. } => {
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "second");
            });

            assert_node!(parser.tree, *left, Expression::Member { left, name, is_optional, .. } => {
                assert_string!(parser, *name, "concat");
                assert!(!*is_optional);
                assert_node!(parser.tree, *left, Expression::Call { left, arguments, .. } => {
                    assert_eq!(arguments.len(), 1);
                    assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "first");
                    });

                    assert_node!(parser.tree, *left, Expression::Member { left, name, is_optional, .. } => {
                        assert_string!(parser, *name, "concat");
                        assert!(*is_optional);
                        assert_expression_path!(parser, parser.tree.get(*left), "permissions");
                    });
                });
            });
        });
    });
}

/// Parse an arrow function parameter named `accessor`.
#[test]
fn test_parse_arrow_parameter_accessor_name() {
    let test = TestParser::new("(accessor: ServicesAccessor) => accessor.get()");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "accessor");
                assert_expression_path!(parser, parser.tree.get(declared_type.unwrap()), "ServicesAccessor");
            });
        });
    });
}

/// Parse newline-separated parenthesized assertion starters as a continued call.
#[test]
fn test_parse_statement_newline_before_parenthesized_assertion_continues_call() {
    let test = TestParser::new("(foo.bar as Baz)\n(foo.bar as unknown)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // (foo.bar as Baz) (foo.bar as unknown)
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_eq!(arguments.len(), 1);

        // (foo.bar as Baz)
        crate::assert_parenthesized!(parser.tree, *left, expression => {
            // foo.bar as Baz
            assert_node!(parser.tree, *expression, Expression::As { expression, target_type } => {
                // foo.bar
                assert_expression_path!(parser, parser.tree.get(*expression), "foo.bar");
                // Baz
                assert_expression_path!(parser, parser.tree.get(*target_type), "Baz");
            });
        });

        // (foo.bar as unknown)
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            // foo.bar as unknown
            assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                // foo.bar
                assert_expression_path!(parser, parser.tree.get(*expression), "foo.bar");
                // unknown
                assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::Unknown);
                });
            });
        });
    });
}
