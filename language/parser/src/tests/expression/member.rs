use crate::tests::TestParser;
use crate::{
    ExpressionPosition, ExpressionStop, assert_comment, assert_expression_path, assert_node,
    assert_string, assert_value_expression_path,
};
use tspp_dir::{
    Argument, CommentKind, Expression, Literal, RangeEnd, TokenType, TypeExpression, TypeLiteral,
};

#[test]
fn test_parse_member_expression_as_member_chain() {
    let test = TestParser::new("foo.bar");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "bar");
        assert_node!(parser.tree, *left, Expression::Identifier { name } => {
            assert_string!(parser, *name, "foo");
        });
    });
    assert_value_expression_path!(parser, parser.tree.get(expression_id), "foo.bar");
}

/// Parse contextual type keyword heads in TS++ member chains.
#[test]
fn test_parse_destack_contextual_type_keyword_member_expression() {
    let input = r#"
keyof.nested.ok satisfies string;
readonly.nested.ok satisfies number;
shared?.nested.ok satisfies boolean;
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 3);
    assert_node!(parser.tree, expressions[0], Expression::Satisfies { expression, target_type } => {
        assert_value_expression_path!(parser, parser.tree.get(*expression), "keyof.nested.ok");
        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::String);
        });
    });
    assert_node!(parser.tree, expressions[1], Expression::Satisfies { expression, target_type } => {
        assert_value_expression_path!(parser, parser.tree.get(*expression), "readonly.nested.ok");
        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Number);
        });
    });
    assert_node!(parser.tree, expressions[2], Expression::Satisfies { expression, target_type } => {
        assert_node!(parser.tree, *expression, Expression::Chain { expression } => {
            assert_node!(parser.tree, *expression, Expression::Member { left, name, is_optional, .. } => {
                assert_string!(parser, *name, "ok");
                assert!(!*is_optional);
                assert_node!(parser.tree, *left, Expression::Member { left, name, is_optional, .. } => {
                    assert_string!(parser, *name, "nested");
                    assert!(*is_optional);
                    assert_value_expression_path!(parser, parser.tree.get(*left), "shared");
                });
            });
        });
        assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Boolean);
        });
    });
}

/// Parse super member access.
#[test]
fn test_parse_super_member_expression() {
    let test = TestParser::new("super.value");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // super.value
    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_node!(parser.tree, *left, Expression::Super);
        assert_string!(parser, *name, "value");
    });
}

#[test]
fn test_parse_member_expression_with_newline_after_dot() {
    let test = TestParser::new("receiver.\nnext");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // receiver.\nnext
    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "next");
        assert_expression_path!(parser, parser.tree.get(*left), "receiver");
    });
}

#[test]
fn test_parse_call_chain_with_newline_after_dot() {
    let test = TestParser::new("receiver().\nthen(value)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new("source /* hop-a */ .first() /* hop-b */ .second()");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

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

    assert_eq!(parser.comments().len(), 2);

    let _ = first_member_id;
    let _ = second_member_id;
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " hop-a");
    assert_comment!(parser, 1, CommentKind::SingleLineBlock, " hop-b");
}

/// Attach call-boundary comments to the call separator owner.
#[test]
fn test_parse_call_boundary_comment_attaches_to_call_separator() {
    let test = TestParser::new("run /* callee-note */ (first, second)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " callee-note");
    let _ = expression_id;
}

#[test]
fn test_parse_member_expression_with_line_comment_before_dot() {
    let test = TestParser::new("container // marker\n.left as PropertyAccessExpression");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

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

    assert_eq!(parser.comments().len(), 1);
    let comment = parser.comments()[0];
    assert_comment!(parser, 0, CommentKind::Line, "marker");

    let token_before = parser
        .consumed_tokens()
        .iter()
        .rev()
        .find(|token| token.span.end <= comment.span.start)
        .copied()
        .expect("line comment should have one preceding token");
    let token_after = parser
        .consumed_tokens()
        .iter()
        .find(|token| {
            token.span.start >= comment.span.end && !matches!(token.token.ty(), TokenType::End)
        })
        .copied()
        .expect("line comment should have one following token");
    assert_eq!(token_before.token.ty(), TokenType::Identifier);
    assert_eq!(token_after.token.ty(), TokenType::Dot);
}

/// Keep full-function line comments before member dots attached to the dot boundary.
#[test]
fn test_parse_function_member_comment_boundary_before_dot() {
    let test = TestParser::new(
        "function f(container) { return ((container // marker\n.left as PropertyAccessExpression).expression as PropertyAccessExpression).expression; }",
    );
    let mut parser = test.prepare();
    let _ = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    test.assert_no_errors(&parser);
    assert_eq!(parser.comments().len(), 1);
    let comment = parser.comments()[0];
    assert_comment!(parser, 0, CommentKind::Line, "marker");

    let token_before = parser
        .consumed_tokens()
        .iter()
        .rev()
        .find(|token| token.span.end <= comment.span.start)
        .copied()
        .expect("line comment should have one preceding token");
    let token_after = parser
        .consumed_tokens()
        .iter()
        .find(|token| {
            token.span.start >= comment.span.end && !matches!(token.token.ty(), TokenType::End)
        })
        .copied()
        .expect("line comment should have one following token");
    assert_eq!(token_before.token.ty(), TokenType::Identifier);
    assert_eq!(token_after.token.ty(), TokenType::Dot);
}

/// Attach block comments before member continuations to the dot boundary.
#[test]
fn test_parse_parenthesized_member_comment_attaches_to_dot_boundary() {
    let test = TestParser::new(
        "(activeService as unknown as QuickInputController) /* boundary note */ .pick()",
    );
    let mut parser = test.prepare();
    let _ = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    test.assert_no_errors(&parser);
    assert_eq!(parser.comments().len(), 1);

    let comment = parser.comments()[0];
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " boundary note");

    let token_after = parser
        .consumed_tokens()
        .iter()
        .find(|token| {
            token.span.start >= comment.span.end && !matches!(token.token.ty(), TokenType::End)
        })
        .copied()
        .expect("member hop comment should attach to one boundary token");
    assert_eq!(token_after.token.ty(), TokenType::Dot);
}

#[test]
fn test_parse_decimal_integer_member_access() {
    let test = TestParser::new("1.foo");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "foo");
        assert_node!(parser.tree, *left, Expression::Literal(Literal::Integer(1)));
    });
}

#[test]
fn test_parse_parenthesized_integer_member_access() {
    let test = TestParser::new("(1).foo");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
        assert_string!(parser, *name, "foo");
        crate::assert_parenthesized!(parser.tree, *left, expression => {
            assert_node!(parser.tree, *expression, Expression::Literal(Literal::Integer(1)));
        });
    });
}

#[test]
fn test_parse_destack_double_dot_as_range() {
    let test = TestParser::new("0..a");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression_id, Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_node!(parser.tree, start.expect("expected start bound"), Expression::Literal(Literal::Integer(0)));
        assert_node!(parser.tree, end.expect("expected end bound"), Expression::Identifier { name } => {
            assert_string!(parser, *name, "a");
        });
    });
}
#[test]
fn test_parse_this_member_expression_in_variant_context() {
    let test = TestParser::new("this.port1.onmessage");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new("setTimeout(this.port1.onmessage, 0)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new(
        r"
self
    .foo()
    .baz()
",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("a.true");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Member { name, .. } => {
        assert_string!(parser, *name, "true");
    });
}

/// Parse null IdentifierName path access.
#[test]
fn test_parse_path_null_identifier_name() {
    let test = TestParser::new("a.null");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new(r#"(await load(join("file://", process.argv[2]))).default"#);
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Member { name, .. } => {
        assert_string!(parser, *name, "default");
    });
}
