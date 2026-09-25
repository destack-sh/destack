use crate::tests::TestParser;
use crate::{
    ExpressionPosition, ExpressionStop, assert_expression_path, assert_node, assert_string,
};
use tspp_dir::{
    Argument, AssignOperator, BinaryOperator, Declaration, Declarator, DependencyItem, Expression,
    FunctionDeclaration, Literal, PostfixPosition, TemplateLiteral, UnaryOperator,
};

/// Parse a regex body containing direct non-ASCII source text.
#[test]
fn test_parse_regex_non_ascii_body() {
    let test = TestParser::new("/café/u");
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression, Expression::Literal(Literal::RegexString { content, flags }) => {
        assert_string!(parser, *content, "café");
        assert_string!(parser, flags.unwrap(), "u");
    });
    test.assert_no_errors(&parser);
}

/// Parse regex literals inside template interpolation expressions.
#[test]
fn test_parse_tagged_template_with_regex_interpolation() {
    let test = TestParser::new("re`/^${/^$/}$/u`");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::TaggedTemplateExpression { tag, value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*tag), "re");
        match value {
            TemplateLiteral::InterpolatedString { chunks, arguments } => {
                assert_eq!(chunks.len(), 2);
                assert_eq!(arguments.len(), 1);
                assert_string!(parser, chunks[0].cooked.unwrap(), "/^");
                assert_string!(parser, chunks[1].cooked.unwrap(), "$/u");
                assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::Literal(Literal::RegexString { .. }));
                });
            }
            other => panic!("expected interpolated template, got {other:?}"),
        }
    });
}

/// Parse tagged templates with legacy octal escapes.
#[test]
fn test_parse_tagged_template_with_legacy_octal_escape() {
    let test = TestParser::new(r"String.raw`\1`");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // String.raw`\\1`
    assert_node!(parser.tree, expr_id, Expression::TaggedTemplateExpression { tag, value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*tag), "String.raw");

        // `\\1`
        match value {
            TemplateLiteral::String { chunk } => {
                assert!(chunk.cooked.is_none());
                assert_string!(parser, chunk.raw, r"\1");
            }
            other => panic!("expected string template literal, got {other:?}"),
        }
    });
}

/// Parse regex literal after binary addition.
#[test]
fn test_parse_regex_literal_after_binary_add() {
    // source: RegExp(prefix + /[A-Z]/.source)
    let test = TestParser::new("RegExp(prefix + /[A-Z]/.source)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // RegExp(prefix + /[A-Z]/.source)
    assert_node!(parser.tree, expr_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "RegExp");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Add);
                assert_expression_path!(parser, parser.tree.get(*left), "prefix");
                assert_node!(parser.tree, *right, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "source");
                    assert_node!(parser.tree, *left, Expression::Literal(Literal::RegexString { .. }));
                });
            });
        });
    });
}

/// Parse regex literal after binary subtraction.
#[test]
fn test_parse_regex_literal_after_binary_subtract() {
    // source: RegExp(prefix - /[A-Z]/.source)
    let test = TestParser::new("RegExp(prefix - /[A-Z]/.source)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // RegExp(prefix - /[A-Z]/.source)
    assert_node!(parser.tree, expr_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "RegExp");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Subtract);
                assert_expression_path!(parser, parser.tree.get(*left), "prefix");
                assert_node!(parser.tree, *right, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "source");
                    assert_node!(parser.tree, *left, Expression::Literal(Literal::RegexString { .. }));
                });
            });
        });
    });
}

/// Parse regex literal after binary divide.
#[test]
fn test_parse_regex_literal_after_binary_divide() {
    // source: value / /[0-9]/.exec(text).length
    let test = TestParser::new("value / /[0-9]/.exec(text).length");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // value / /[0-9]/.exec(text).length
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::Divide);
        assert_expression_path!(parser, parser.tree.get(*left), "value");
        assert_node!(parser.tree, *right, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "length");
            assert_node!(parser.tree, *left, Expression::Call { left, arguments, .. } => {
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "text");
                });
                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "exec");
                    assert_node!(parser.tree, *left, Expression::Literal(Literal::RegexString { .. }));
                });
            });
        });
    });
}

/// Parse regex literal after binary less than.
#[test]
fn test_parse_regex_literal_after_binary_less_than() {
    // source: value < /[A-Z]/.test(text)
    let test = TestParser::new("value < /[A-Z]/.test(text)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // value < /[A-Z]/.test(text)
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::LessThan);
        assert_expression_path!(parser, parser.tree.get(*left), "value");
        assert_node!(parser.tree, *right, Expression::Call { left, arguments, .. } => {
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "text");
            });
            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "test");
                assert_node!(parser.tree, *left, Expression::Literal(Literal::RegexString { .. }));
            });
        });
    });
}

/// Parse regex literal after binary in keyword.
#[test]
fn test_parse_regex_literal_after_binary_in_keyword() {
    // source: key in /[A-Z]/.source
    let test = TestParser::new("key in /[A-Z]/.source");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // key in /[A-Z]/.source
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::In);
        assert_expression_path!(parser, parser.tree.get(*left), "key");
        assert_node!(parser.tree, *right, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "source");
            assert_node!(parser.tree, *left, Expression::Literal(Literal::RegexString { .. }));
        });
    });
}

/// Parse regex literal after an instanceof keyword.
#[test]
fn test_parse_regex_literal_after_binary_instanceof_keyword() {
    // source: value instanceof /[A-Z]/
    let test = TestParser::new("value instanceof /[A-Z]/");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // value instanceof /[A-Z]/
    assert_node!(parser.tree, expr_id, Expression::InstanceOf { value, target } => {
        assert_expression_path!(parser, parser.tree.get(*value), "value");
        assert_node!(parser.tree, *target, Expression::Literal(Literal::RegexString { .. }));
    });
}

/// Parse regex literal after assign with a newline.
#[test]
fn test_parse_regex_literal_after_assign_newline() {
    // source: let match =\n/^foo$/i.exec(str)
    let test = TestParser::new("let match =\n/^foo$/i.exec(str)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // let match =\n/^foo$/i.exec(str)
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            assert_node!(parser.tree, value.expect("expected initializer"), Expression::Call { left, arguments, .. } => {
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "exec");
                    assert_node!(parser.tree, *left, Expression::Literal(Literal::RegexString { .. }));
                });
            });
        });
    });
}

/// Parse regex literal after an arrow.
#[test]
fn test_parse_regex_literal_after_arrow() {
    // source: () => /^foo$/.test(value)
    let test = TestParser::new("() => /^foo$/.test(value)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body, .. }) => {
            let body = body.expect("expected body");
            assert_node!(parser.tree, body, Expression::Call { left, arguments, .. } => {
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "test");
                    assert_node!(parser.tree, *left, Expression::Literal(Literal::RegexString { .. }));
                });
            });
        });
    });
}

/// Parse regex literal after unary not.
#[test]
fn test_parse_regex_literal_after_unary_not() {
    // source: !/[A-Z]/.test(k)
    let test = TestParser::new("!/[A-Z]/.test(k)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // !/[A-Z]/.test(k)
    assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
        assert_eq!(*operator, UnaryOperator::Not);
        assert_node!(parser.tree, *right, Expression::Call { left, arguments, .. } => {
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "test");
                assert_node!(parser.tree, *left, Expression::Literal(Literal::RegexString { .. }));
            });
        });
    });
}

/// Parse regex literal after coalesce assignment.
#[test]
fn test_parse_regex_literal_after_coalesce_assign() {
    // source: encoded ??= /[%+]/.test(url)
    let test = TestParser::new("encoded ??= /[%+]/.test(url)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // encoded ??= /[%+]/.test(url)
    assert_node!(parser.tree, expr_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::CoalesceAssign);
        assert_expression_path!(parser, parser.tree.get(*left), "encoded");
        assert_node!(parser.tree, *right, Expression::Call { left, arguments, .. } => {
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "test");
                assert_node!(parser.tree, *left, Expression::Literal(Literal::RegexString { .. }));
            });
        });
    });
}

/// Parse division after a non-null assertion.
#[test]
fn test_parse_divide_after_non_null_assertion() {
    // source: x! / 2
    let test = TestParser::new("x! / 2");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // x! / 2
    assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::Divide);
        assert_node!(parser.tree, *left, Expression::Must { left, position } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "x");
        });
        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(2)));
    });
}

/// Parse regex literal with slash inside a character class.
#[test]
fn test_parse_regex_literal_with_character_class_slash() {
    // source: let a = /[\]/]/
    let test = TestParser::new("let a = /[\\]/]/");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // let a = /[\]/]/
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            assert_node!(parser.tree, value.expect("expected initializer"), Expression::Literal(Literal::RegexString { .. }));
        });
    });
}

/// Parse string literal with long leading-zero code point escapes.
#[test]
fn test_parse_string_unicode_escape_with_long_leading_zeros() {
    // source: "\u{00000000034}"
    let test = TestParser::new("\"\\u{00000000034}\"");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // "4"
    assert_node!(parser.tree, expr_id, Expression::Literal(Literal::String(string_id)) => {
        assert_string!(parser, *string_id, "4");
    });
}

/// Parse a less-than comparison.
#[test]
fn test_parse_comparison_less_than() {
    let test = TestParser::new("x < y");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    // x < y
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { left, operator, right, .. } => {
            assert_eq!(*operator, BinaryOperator::LessThan);
            // x
            assert_expression_path!(parser, parser.tree.get(*left), "x");
            // y
            assert_expression_path!(parser, parser.tree.get(*right), "y");
        }
    );
}

/// Parse regex literal in export default.
#[test]
fn test_parse_export_default_regex_literal() {
    let test = TestParser::new("export default /foo/");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { value: Some(value), .. } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::RegexString { .. }));
        });
    });
}
