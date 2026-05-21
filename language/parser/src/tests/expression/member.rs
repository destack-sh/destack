use crate::tests::*;
use crate::{
    assert_comment, assert_expression_path, assert_node, assert_string,
    assert_value_expression_path,
};
use destack_dir::*;
use destack_source::LanguageType;

#[test]
fn test_parse_keyword_and_private_member_expressions() {
    let input = r#"
foo.bar
foo.await
foo.yield
foo.for
foo?.for
foo?.bar
class Test {
  #bar
  test(other) {
    this.#bar;
    this?.#bar;
    other.#bar;
    other?.#bar;
  }
}
"#;
    let mut test = TestParser::new_with_language(input, LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 7);
    for expression in &expressions[..6] {
        assert!(
            matches!(
                parser.tree.get(*expression),
                Expression::Member { .. }
                    | Expression::PrivateMember { .. }
                    | Expression::Maybe { .. }
            ),
            "expected member expression, got {:?}",
            parser.tree.get(*expression)
        );
    }
    assert_node!(parser.tree, expressions[6], Expression::Declaration(_));
}

/// Parse a dotted value reference as a runtime member chain.
#[test]
fn test_parse_member_expression_as_member_chain() {
    let mut test = TestParser::new_with_language("foo.bar", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name } => {
        assert_string!(parser, *name, "bar");
        assert_node!(parser.tree, *left, Expression::Identifier { name } => {
            assert_string!(parser, *name, "foo");
        });
    });
    assert_value_expression_path!(parser, parser.tree.get(expression_id), "foo.bar");
}

/// Parse contextual type keyword heads in Destack member chains.
#[test]
fn test_parse_destack_contextual_type_keyword_member_expression() {
    let input = r#"
keyof.nested.ok satisfies string;
readonly.nested.ok satisfies number;
shared?.nested.ok satisfies boolean;
"#;
    let mut test = TestParser::new_with_language(input, LanguageType::Destack);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 3);
    assert_node!(parser.tree, expressions[0], Expression::Satisfies { expression, target_type } => {
        assert_value_expression_path!(parser, parser.tree.get(*expression), "keyof.nested.ok");
        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::String);
        });
    });
    assert_node!(parser.tree, expressions[1], Expression::Satisfies { expression, target_type } => {
        assert_value_expression_path!(parser, parser.tree.get(*expression), "readonly.nested.ok");
        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Number);
        });
    });
    assert_node!(parser.tree, expressions[2], Expression::Satisfies { expression, target_type } => {
        assert_node!(parser.tree, *expression, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "ok");
            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "nested");
                assert_node!(parser.tree, *left, Expression::Maybe { left, position: PostfixPosition::Direct } => {
                    assert_value_expression_path!(parser, parser.tree.get(*left), "shared");
                });
            });
        });
        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
            assert_eq!(*value, TypeLiteral::Boolean);
        });
    });
}

/// Parse super member access.
#[test]
fn test_parse_super_member_expression() {
    let mut test = TestParser::new_with_language("super.value", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // super.value
    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_node!(parser.tree, *left, Expression::Super);
        assert_string!(parser, *name, "value");
    });
}

#[test]
fn test_parse_member_expression_with_newline_after_dot() {
    let mut test = TestParser::new_with_language("receiver.\nnext", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // receiver.\nnext
    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "next");
        assert_expression_path!(parser, parser.tree.get(*left), "receiver");
    });
}

#[test]
fn test_parse_call_chain_with_newline_after_dot() {
    let mut test =
        TestParser::new_with_language("receiver().\nthen(value)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // receiver().\nthen(value)
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });

        // receiver().then
        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "then");
            assert_node!(parser.tree, *left, Expression::Call { left, arguments, .. } => {
                assert!(arguments.is_empty());
                assert_expression_path!(parser, parser.tree.get(*left), "receiver");
            });
        });
    });
}

/// Keep member-hop boundary comments on the hop owner expressions.
#[test]
fn test_parse_member_hop_comments_attach_to_boundary_owners() {
    let mut test = TestParser::new_with_language(
        "source /* hop-a */ .first() /* hop-b */ .second()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert!(arguments.is_empty());

        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "second");

            let first_call_id = *left;
            assert_node!(parser.tree, first_call_id, Expression::Call { left, arguments, .. } => {
                assert!(arguments.is_empty());

                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "first");
                    assert_expression_path!(parser, parser.tree.get(*left), "source");
                });
            });
            let first_call_annotations = parser.tree.get_decorators(first_call_id.id);
            assert!(first_call_annotations.is_empty());
        });
    });

    let (second_member_id, first_member_id) = match parser.tree.get(expression_id) {
        Expression::Call { left, .. } => match parser.tree.get(*left) {
            Expression::Member {
                left: first_call_id,
                ..
            } => match parser.tree.get(*first_call_id) {
                Expression::Call {
                    left: first_member_id,
                    ..
                } => (*left, *first_member_id),
                other => panic!("unexpected inner call owner: {other:?}"),
            },
            other => panic!("unexpected second hop owner: {other:?}"),
        },
        other => panic!("unexpected expression: {other:?}"),
    };

    assert_eq!(parser.tree.comments().len(), 2);

    let _ = first_member_id;
    let _ = second_member_id;
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " hop-a");
    assert_comment!(parser, 1, CommentKind::SingleLineBlock, " hop-b");
}

/// Attach call-boundary comments to the call separator owner.
#[test]
fn test_parse_call_boundary_comment_attaches_to_call_separator() {
    let mut test = TestParser::new_with_language(
        "run /* callee-note */ (first, second)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();

    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " callee-note");
    let _ = expression_id;
}

#[test]
fn test_parse_private_member_expression_with_newline_before_dot() {
    let mut test = TestParser::new_with_language("this\n.#value", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::PrivateMember { left, name } => {
        assert_node!(parser.tree, *left, Expression::This);
        assert_string!(parser, *name, "value");
    });
}

#[test]
fn test_parse_destack_rejects_private_member_expression() {
    let mut test = TestParser::new("this.#value");
    let mut parser = test.prepare();

    let error = parser.eat_expression(parser.flags).unwrap_err();
    assert_eq!(parser.get_span_str(error.leaf_span()), "#");
}

#[test]
fn test_parse_member_expression_with_line_comment_before_dot() {
    let mut test = TestParser::new_with_language(
        "container // marker\n.left as PropertyAccessExpression",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();

    test.assert_no_errors(&parser);
    assert_node!(
        parser.tree,
        expression_id,
        Expression::As {
            expression,
            target_type
        } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "container.left");
            assert_expression_path!(parser, parser.tree.get(*target_type), "PropertyAccessExpression");
        }
    );

    assert_eq!(parser.tree.comments().len(), 1);
    let comment = parser.tree.comments()[0];
    assert_comment!(parser, 0, CommentKind::Line, "marker");

    let token_before = parser
        .tokens()
        .iter()
        .rev()
        .find(|token| token.span.end <= comment.span.start)
        .copied()
        .expect("line comment should have one preceding token");
    let token_after = parser
        .tokens()
        .iter()
        .find(|token| {
            token.span.start >= comment.span.end && !matches!(token.token.ty, TokenType::End)
        })
        .copied()
        .expect("line comment should have one following token");
    assert_eq!(token_before.token.ty, TokenType::Identifier);
    assert_eq!(token_after.token.ty, TokenType::Dot);
}

/// Keep full-function line comments before member dots attached to the dot boundary.
#[test]
fn test_parse_function_member_comment_boundary_before_dot() {
    let mut test = TestParser::new_with_language(
        "function f(container) { return ((container // marker\n.left as PropertyAccessExpression).expression as PropertyAccessExpression).expression; }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let _ = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();

    test.assert_no_errors(&parser);
    assert_eq!(parser.tree.comments().len(), 1);
    let comment = parser.tree.comments()[0];
    assert_comment!(parser, 0, CommentKind::Line, "marker");

    let token_before = parser
        .tokens()
        .iter()
        .rev()
        .find(|token| token.span.end <= comment.span.start)
        .copied()
        .expect("line comment should have one preceding token");
    let token_after = parser
        .tokens()
        .iter()
        .find(|token| {
            token.span.start >= comment.span.end && !matches!(token.token.ty, TokenType::End)
        })
        .copied()
        .expect("line comment should have one following token");
    assert_eq!(token_before.token.ty, TokenType::Identifier);
    assert_eq!(token_after.token.ty, TokenType::Dot);
}

/// Attach block comments before member continuations to the dot boundary.
#[test]
fn test_parse_parenthesized_member_comment_attaches_to_dot_boundary() {
    let mut test = TestParser::new_with_language(
        "(activeService as unknown as QuickInputController) /* boundary note */ .pick()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let _ = parser.eat_expression(parser.flags).unwrap();
    parser.attach_comments();

    test.assert_no_errors(&parser);
    assert_eq!(parser.tree.comments().len(), 1);

    let comment = parser.tree.comments()[0];
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " boundary note");

    let token_after = parser
        .tokens()
        .iter()
        .find(|token| {
            token.span.start >= comment.span.end && !matches!(token.token.ty, TokenType::End)
        })
        .copied()
        .expect("member hop comment should attach to one boundary token");
    assert_eq!(token_after.token.ty, TokenType::Dot);
}

#[test]
fn test_parse_object_property_private_member_cast_with_newline_before_dot() {
    let mut test = TestParser::new_with_language(
        "({ value: this\n.#javascriptTransformer as unknown as JavaScriptTransformer })",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                assert_string!(parser, *name, "value");
                assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                    assert_node!(parser.tree, *expression, Expression::As { expression, target_type } => {
                        assert_node!(parser.tree, *expression, Expression::PrivateMember { left, name } => {
                            assert_node!(parser.tree, *left, Expression::This);
                            assert_string!(parser, *name, "javascriptTransformer");
                        });
                        assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Unknown);
                        });
                    });
                    assert_expression_path!(parser, parser.tree.get(*target_type), "JavaScriptTransformer");
                });
            });
        });
    });
}

#[test]
fn test_reject_decimal_integer_member_access_without_separator() {
    for language in [LanguageType::Destack, LanguageType::TypeScript] {
        let mut test = TestParser::new_with_language("1.foo", language);
        let mut parser = test.prepare();

        let result = parser.eat_expression(parser.flags);
        assert!(result.is_err());
    }
}

#[test]
fn test_parse_parenthesized_integer_member_access() {
    let mut test = TestParser::new("(1).foo");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "foo");
        assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}

#[test]
fn test_parse_double_dot_member_access_after_numeric_literal() {
    let mut test = TestParser::new_with_language("0..value", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "value");
        assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::Float(value)) => {
            assert_eq!(*value, 0.0);
        });
    });
}

#[test]
fn test_parse_double_dot_member_call_after_numeric_literal() {
    for language in [LanguageType::JavaScript, LanguageType::TypeScript] {
        let mut test = TestParser::new_with_language("123..a(1)", language);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.flags).unwrap();

        assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::Float(value)) => {
                    assert_eq!(*value, 123.0);
                });
            });
        });
    }
}

#[test]
fn test_parse_destack_double_dot_as_range() {
    let mut test = TestParser::new_with_language("0..a", LanguageType::Destack);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_node!(parser.tree, start.expect("expected start bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        assert_node!(parser.tree, end.expect("expected end bound"), Expression::Identifier { name } => {
            assert_string!(parser, *name, "a");
        });
    });
}

#[test]
fn test_reject_hex_integer_member_separator() {
    let mut test = TestParser::new_with_language("0x1..a", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let _ = parser.parse();

    // parser should not reinterpret `..` as a decimal separator for non-decimal integers
    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.get_span_str(parser.errors[0].leaf_span()), "0x1.");
}

/// Parse this member access in variant context.
#[test]
fn test_parse_this_member_expression_in_variant_context() {
    let mut test = TestParser::new_with_language("this.port1.onmessage", LanguageType::TypeScript);
    let mut parser = test.prepare();
    parser.flags = parser.flags.in_variant();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // this.port1.onmessage
    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "onmessage");
        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "port1");
            assert_node!(parser.tree, *left, Expression::This);
        });
    });
}

/// Parse this member access in call arguments in variant context.
#[test]
fn test_parse_call_argument_this_member_expression_in_variant_context() {
    let mut test = TestParser::new_with_language(
        "setTimeout(this.port1.onmessage, 0)",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    parser.flags = parser.flags.in_variant();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // setTimeout(this.port1.onmessage, 0)
    assert_node!(parser.tree, expression_id, Expression::Call { arguments, .. } => {
        assert_eq!(arguments.len(), 2);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            // this.port1.onmessage
            assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "onmessage");
                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "port1");
                    assert_node!(parser.tree, *left, Expression::This);
                });
            });
        });
    });
}

/// Parse multi-line member and calls.
#[test]
fn test_parse_member_multiline() {
    let mut test = TestParser::new(
        r"
self
    .foo()
    .baz()
",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Call { left: baz_recv, .. } => {
        assert_node!(parser.tree, *baz_recv, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "baz");
            assert_node!(parser.tree, *left, Expression::Call { left: foo_recv, .. } => {
                assert_expression_path!(parser, parser.tree.get(*foo_recv), "self.foo");
            })
        });
    });
}

/// Parse boolean IdentifierName member access.
#[test]
fn test_parse_member_boolean_identifier_name() {
    let mut test = TestParser::new_with_language("a.true", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Member { name, .. } => {
        assert_string!(parser, *name, "true");
    });
}

/// Parse null IdentifierName path access.
#[test]
fn test_parse_path_null_identifier_name() {
    let mut test = TestParser::new_with_language("a.null", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "null");
        assert_node!(parser.tree, *left, Expression::Identifier { name } => {
            assert_string!(parser, *name, "a");
        });
    });
}

/// Parse default IdentifierName member access.
#[test]
fn test_parse_member_default_identifier_name_after_parenthesized_await_call() {
    let mut test = TestParser::new_with_language(
        r#"(await load(join("file://", process.argv[2]))).default"#,
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Member { name, .. } => {
        assert_string!(parser, *name, "default");
    });
}
