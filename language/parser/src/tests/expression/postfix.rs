use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

/// Parse optional chaining after comment-separated newlines.
#[test]
fn test_parse_optional_chain_after_comment_newlines() {
    let input = "promise\n  .then(noop)\n  // comment\n  // comment\n  ?.catch(noop)";
    let mut test = TestParser::new_with_options(input, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);

    let statement_id = expressions[0];

    assert_node!(parser.tree, statement_id, Expression::Call { left, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "noop");
        });

        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "catch");
            assert_node!(parser.tree, *left, Expression::Maybe { left: maybe_left, position: PostfixPosition::Direct } => {
                assert_node!(parser.tree, *maybe_left, Expression::Call { .. });
            });
        });
    });
}

#[test]
fn test_parse_optional_call_after_question_dot_line_comment_newline() {
    let input = "call?.// comment\n()";
    let mut test = TestParser::new_with_options(input, LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, position, .. } => {
        assert_eq!(*position, PostfixPosition::Indirect);
        assert!(arguments.is_empty());

        assert_node!(parser.tree, *left, Expression::Maybe { left: maybe_left, position } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*maybe_left), "call");
        });
    });
}

/// Parse optional chaining when the member target starts on the next line after ?..
#[test]
fn test_parse_optional_chain_member_after_question_dot_newline() {
    let input = "items?.\nmap(noop)";
    let mut test = TestParser::new_with_options(input, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    // items?.map(noop)
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "noop");
        });

        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "map");
            assert_node!(parser.tree, *left, Expression::Maybe { left: maybe_left, position: PostfixPosition::Direct } => {
                assert_expression_path!(parser, parser.tree.get(*maybe_left), "items");
            });
        });
    });
}

/// Parse chained optional members with newline-delimited segments.
#[test]
fn test_parse_optional_chain_chained_members_after_question_dot_newline() {
    let input = "permissions?.\nconcat(first).\nconcat(second)";
    let mut test = TestParser::new_with_options(input, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "second");
        });

        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "concat");
            assert_node!(parser.tree, *left, Expression::Call { left, arguments, .. } => {
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "first");
                });

                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "concat");
                    assert_node!(parser.tree, *left, Expression::Maybe { left: maybe_left, position: PostfixPosition::Direct } => {
                        assert_expression_path!(parser, parser.tree.get(*maybe_left), "permissions");
                    });
                });
            });
        });
    });
}

/// Parse an arrow function parameter named `accessor`.
#[test]
fn test_parse_arrow_parameter_accessor_name() {
    let mut test = TestParser::new_with_options(
        "(accessor: ServicesAccessor) => accessor.get()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
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
    let mut test = TestParser::new_with_options(
        "(foo.bar as Baz)\n(foo.bar as any)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // (foo.bar as Baz) (foo.bar as any)
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_eq!(arguments.len(), 1);

        // (foo.bar as Baz)
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            // foo.bar as Baz
            assert_node!(parser.tree, *expression, Expression::As { expression, target_type } => {
                // foo.bar
                assert_expression_path!(parser, parser.tree.get(*expression), "foo.bar");
                // Baz
                assert_expression_path!(parser, parser.tree.get(*target_type), "Baz");
            });
        });

        // (foo.bar as any)
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            // foo.bar as any
            assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                // foo.bar
                assert_expression_path!(parser, parser.tree.get(*expression), "foo.bar");
                // any
                assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Any);
                });
            });
        });
    });
}
