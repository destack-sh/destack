use crate::{ExpressionPosition, ExpressionStop};
use tspp_dir::{
    Argument, BinaryOperator, CommentKind, ConditionOperand, Declaration, Expression, FloatType,
    FunctionDeclaration, FunctionForm, GenericArgument, GenericParameter, IfForm, IntegerType,
    Literal, Name, NodeType, Parameter, Pattern, Property, ScalarAlias, TemplateLiteral, TokenType,
    TreeAttribute, TreeAttributeValue, TreeChild, TypeExpression, TypeLiteral,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

use crate::{
    TestParser, assert_comment, assert_expression_path, assert_node, assert_path, assert_string,
    block_expression_ids,
};

/// Parse integer literals in various formats.
#[test]
fn test_parse_integer_literal() {
    let test = TestParser::new("1 731 0x1234 2n");
    let mut parser = test.prepare();

    assert_eq!(parser.parse_scalar_literal().unwrap(), Literal::Integer(1));
    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Integer(731)
    );
    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Integer(0x1234)
    );
    assert_eq!(parser.parse_scalar_literal().unwrap(), Literal::Bigint(2));
}

/// Parse integer literals with uppercase radix prefixes.
#[test]
fn test_parse_integer_literal_uppercase_radix_prefixes() {
    let test = TestParser::new("0B101 0O77 0Xff");
    let mut parser = test.prepare();

    assert_eq!(parser.parse_scalar_literal().unwrap(), Literal::Integer(5));
    assert_eq!(parser.parse_scalar_literal().unwrap(), Literal::Integer(63));
    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Integer(255)
    );
}

/// Recover the elements of an array literal missing its closing bracket.
#[test]
fn test_parse_array_literal_with_missing_close_bracket() {
    let test = TestParser::new("[first, second");
    let mut parser = test.prepare();
    let elements = parser.parse_array_literal(Default::default()).unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(elements.len(), 2);

    assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
        assert_expression_path!(parser, parser.tree.get(*value), "first");
    });
    assert_node!(parser.tree, elements[1], Argument::Positional { value } => {
        assert_expression_path!(parser, parser.tree.get(*value), "second");
    });
}

/// Recover the properties of an object literal missing its closing brace.
#[test]
fn test_parse_object_literal_with_missing_close_brace() {
    let test = TestParser::new("{ foo: 1");
    let mut parser = test.prepare();
    let properties = parser.parse_object_literal().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(properties.len(), 1);

    assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
        assert_string!(parser, *name, "foo");
        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
    });
}

/// Parse large integer literals without overflow errors.
#[test]
fn test_parse_integer_literal_saturating() {
    let test = TestParser::new("9999999999999999999999999 9999999999999999999999999n");
    let mut parser = test.prepare();

    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Integer(i64::MAX)
    );
    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Bigint(i64::MAX)
    );
}

/// Parse scientific notation and decimal floats.2
#[test]
fn test_parse_float_literal() {
    let test = TestParser::new("10e37 1.0");
    let mut parser = test.prepare();

    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Float(1.0e38)
    );
    assert_eq!(parser.parse_scalar_literal().unwrap(), Literal::Float(1.0));
}

/// Parse true and false literals.
#[test]
fn test_parse_boolean_literal() {
    let test = TestParser::new("true false");
    let mut parser = test.prepare();

    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Boolean(true)
    );
    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Boolean(false)
    );
}

/// Parse a single quoted literal as a character.
#[test]
fn test_parse_single_quoted_character_literal() {
    let test = TestParser::new("'a'");
    let mut parser = test.prepare();

    let literal = parser.parse_scalar_literal().unwrap();

    assert_eq!(literal, Literal::Character('a'));
}

/// Parse escaped TS++ single quoted literals as characters.
#[test]
fn test_parse_escaped_single_quoted_character_literal() {
    let test = TestParser::new(r#"'\n' '\'' '\u{41}'"#);
    let mut parser = test.prepare();

    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Character('\n')
    );
    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Character('\'')
    );
    assert_eq!(
        parser.parse_scalar_literal().unwrap(),
        Literal::Character('A')
    );
}

/// Parse regular-expression pattern and flag source.
#[test]
fn test_parse_regex_string_literal() {
    let test = TestParser::new("/abc/\n/abc/g\n/(/future2");
    let mut parser = test.prepare();

    // /abc/
    let literal = parser.parse_regex_literal().unwrap();
    match literal {
        Literal::RegexString { content, flags } => {
            assert_string!(parser, content, "abc");
            assert!(flags.is_none());
        }
        other => panic!("expected regex string literal, got {other:?}"),
    }

    // /abc/g
    let literal = parser.parse_regex_literal().unwrap();
    match literal {
        Literal::RegexString { content, flags } => {
            assert_string!(parser, content, "abc");
            assert_string!(parser, flags.unwrap(), "g");
        }
        other => panic!("expected regex string literal, got {other:?}"),
    }

    // /(/future2
    let literal = parser.parse_regex_literal().unwrap();
    match literal {
        Literal::RegexString { content, flags } => {
            assert_string!(parser, content, "(");
            assert_string!(parser, flags.unwrap(), "future2");
        }
        other => panic!("expected regex string literal, got {other:?}"),
    }
}

/// Parse a template string literal.
#[test]
fn test_parse_template_literal() {
    let test = TestParser::new(
        r#"
`hello`
`hello ${name}`
`${stmt}`
`${start}${middle}${end}`
`SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`
"#,
    );
    let mut parser = test.prepare();

    // `hello`
    let literal = parser.parse_template_literal().unwrap();
    match literal {
        TemplateLiteral::String { chunk } => {
            // hello
            assert_string!(parser, chunk.cooked.unwrap(), "hello");
        }
        other => panic!("unexpected {other:?}"),
    }

    // `hello ${name}`
    let literal = parser.parse_template_literal().unwrap();
    match literal {
        TemplateLiteral::InterpolatedString { chunks, arguments } => {
            assert_eq!(arguments.len(), 1);
            assert_eq!(chunks.len(), 2);
            // hello
            assert_string!(parser, chunks[0].cooked.unwrap(), "hello ");
            // name
            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "name");
            });
            //
            assert_string!(parser, chunks[1].cooked.unwrap(), "");
        }
        other => panic!("unexpected {other:?}"),
    }

    // `${stmt}`
    let literal = parser.parse_template_literal().unwrap();
    match literal {
        TemplateLiteral::InterpolatedString { chunks, arguments } => {
            assert_eq!(chunks.len(), 2);
            assert_eq!(arguments.len(), 1);
            // empty start & empty end
            assert_string!(parser, chunks[0].cooked.unwrap(), "");
            assert_string!(parser, chunks[1].cooked.unwrap(), "");
            // stmt
            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "stmt");
            });
        }
        other => panic!("unexpected {other:?}"),
    }

    // `${start}${middle}${end}`
    let literal = parser.parse_template_literal().unwrap();
    match literal {
        TemplateLiteral::InterpolatedString { chunks, arguments } => {
            assert_eq!(chunks.len(), 4);
            assert_eq!(arguments.len(), 3);
            // empty string before & after each argument
            assert_string!(parser, chunks[0].cooked.unwrap(), "");
            assert_string!(parser, chunks[1].cooked.unwrap(), "");
            assert_string!(parser, chunks[2].cooked.unwrap(), "");
            assert_string!(parser, chunks[3].cooked.unwrap(), "");
            // start
            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "start");
            });
            // middle
            assert_node!(parser.tree, arguments[1], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "middle");
            });
            // end
            assert_node!(parser.tree, arguments[2], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "end");
            });
        }
        other => panic!("unexpected {other:?}"),
    }

    // `SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`
    let literal = parser.parse_template_literal().unwrap();
    match literal {
        TemplateLiteral::InterpolatedString { chunks, arguments } => {
            assert_eq!(arguments.len(), 2);
            assert_eq!(chunks.len(), 3);
            // SELECT * FROM users WHERE name =
            assert_string!(
                parser,
                chunks[0].cooked.unwrap(),
                "SELECT * FROM users WHERE name = "
            );
            // name
            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "name");
            });
            // AND age >
            assert_string!(parser, chunks[1].cooked.unwrap(), " AND age > ");
            // group.age()
            assert_node!(parser.tree, arguments[1], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "group.age");
                });
            });
            // LIMIT 10
            assert_string!(parser, chunks[2].cooked.unwrap(), " LIMIT 10");
        }
        other => panic!("unexpected {other:?}"),
    }
}

/// Parse template interpolation with an `as` cast.
#[test]
fn test_parse_template_literal_as_cast_expression() {
    let test = TestParser::new("`${type as string}`");
    let mut parser = test.prepare();
    let literal = parser.parse_template_literal().unwrap();

    match literal {
        TemplateLiteral::InterpolatedString { chunks, arguments } => {
            assert_eq!(chunks.len(), 2);
            assert_eq!(arguments.len(), 1);
            assert_string!(parser, chunks[0].cooked.unwrap(), "");
            assert_string!(parser, chunks[1].cooked.unwrap(), "");

            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "type");
                    assert_node!(parser.tree, *target_type, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
            });
        }
        other => panic!("unexpected {other:?}"),
    }
}

/// Parse template literals with escaped `${` text before interpolation.
#[test]
fn test_parse_template_literal_with_escaped_interpolation_prefix() {
    let test = TestParser::new(r"`\${${value}}`");
    let mut parser = test.prepare();
    let literal = parser.parse_template_literal().unwrap();

    match literal {
        TemplateLiteral::InterpolatedString { chunks, arguments } => {
            assert_eq!(chunks.len(), 2);
            assert_eq!(arguments.len(), 1);
            assert_string!(parser, chunks[0].cooked.unwrap(), "${");
            assert_string!(parser, chunks[0].raw, r"\${");
            assert_string!(parser, chunks[1].cooked.unwrap(), "}");

            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "value");
            });
        }
        other => panic!("unexpected {other:?}"),
    }
}

/// Report untagged template literals with legacy octal escapes.
#[test]
fn test_report_template_literal_legacy_octal_escape() {
    let test = TestParser::new(r"`\1`");
    let mut parser = test.prepare();
    let error = parser.parse_template_literal().unwrap_err();

    assert_eq!(parser.range_str(error.range()), r"`\1`");
}

/// Parse each builtin scalar type keyword as a type literal.
#[test]
fn test_parse_type_literal() {
    let test = TestParser::new("int32 uint8 float float32 float64 boolean char");
    let mut parser = test.prepare();

    assert!(matches!(
        parser.parse_type_literal().unwrap(),
        TypeLiteral::Integer(IntegerType::Fixed {
            width: 32,
            is_signed: true
        })
    ));
    assert!(matches!(
        parser.parse_type_literal().unwrap(),
        TypeLiteral::Integer(IntegerType::Fixed {
            width: 8,
            is_signed: false
        })
    ));
    assert!(matches!(
        parser.parse_type_literal().unwrap(),
        TypeLiteral::Alias(ScalarAlias::Float)
    ));
    assert!(matches!(
        parser.parse_type_literal().unwrap(),
        TypeLiteral::Float(FloatType::Float32)
    ));
    assert!(matches!(
        parser.parse_type_literal().unwrap(),
        TypeLiteral::Float(FloatType::Float64)
    ));
    assert!(matches!(
        parser.parse_type_literal().unwrap(),
        TypeLiteral::Boolean
    ));
    assert!(matches!(
        parser.parse_type_literal().unwrap(),
        TypeLiteral::Character
    ));
}

/// Parse an array literal into positional elements.
#[test]
fn test_parse_array_literal() {
    let test = TestParser::new("[1, 2]");
    let mut parser = test.prepare();

    let elements = parser.parse_array_literal(Default::default()).unwrap();
    assert_eq!(elements.len(), 2);
    // 1
    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        }
    );
    // 2
    assert_node!(
        parser.tree,
        elements[1],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
        }
    );
}

/// Parse comma separated array elements as separate positional elements.
#[test]
fn test_parse_array_literal_disallows_sequence_elements() {
    let test = TestParser::new("[1, 2, 3]");
    let mut parser = test.prepare();

    let elements = parser.parse_array_literal(Default::default()).unwrap();
    assert_eq!(elements.len(), 3);

    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        }
    );

    assert_node!(
        parser.tree,
        elements[1],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
        }
    );

    assert_node!(
        parser.tree,
        elements[2],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(3)));
        }
    );
}

/// Parse an elided element between two array elements.
#[test]
fn test_parse_sparse_array_middle_hole() {
    let test = TestParser::new("[1, , 3]");
    let mut parser = test.prepare();

    let elements = parser.parse_array_literal(Default::default()).unwrap();
    assert_eq!(elements.len(), 3);
    // 1
    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        }
    );
    // hole
    assert_node!(parser.tree, elements[1], Argument::Elision);
    // 3
    assert_node!(
        parser.tree,
        elements[2],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(3)));
        }
    );
}

/// Parse an elided element ahead of the first array element.
#[test]
fn test_parse_sparse_array_leading_hole() {
    let test = TestParser::new("[, 1]");
    let mut parser = test.prepare();

    let elements = parser.parse_array_literal(Default::default()).unwrap();
    assert_eq!(elements.len(), 2);
    // hole
    assert_node!(parser.tree, elements[0], Argument::Elision);
    // 1
    assert_node!(
        parser.tree,
        elements[1],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        }
    );
}

/// Parse a trailing comma as the array end, keeping one element.
#[test]
fn test_parse_sparse_array_trailing_hole() {
    let test = TestParser::new("[1, ]");
    let mut parser = test.prepare();

    let elements = parser.parse_array_literal(Default::default()).unwrap();
    // trailing comma without hole is allowed
    assert_eq!(elements.len(), 1);
    // 1
    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        }
    );
}

/// Parse array elements separated by a comma on the following line.
#[test]
fn test_parse_array_literal_with_newline_prefixed_comma_separator() {
    let test = TestParser::new("[1\n, 2]");
    let mut parser = test.prepare();

    let elements = parser.parse_array_literal(Default::default()).unwrap();
    assert_eq!(elements.len(), 2);

    // 1
    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        }
    );

    // 2
    assert_node!(
        parser.tree,
        elements[1],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
        }
    );
}

/// Parse a self closing tree element.
#[test]
fn test_parse_tree_fragment() {
    let test = TestParser::new("<A/>");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    // <A/>
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        // A
        assert_expression_path!(parser, parser.tree.get(*left), "A");
        assert!(attributes.is_none());
        assert!(children.is_none());
    });
}

/// Parse inline whitespace-only tree text as a meaningful child.
#[test]
fn test_parse_tree_inline_whitespace_text_child() {
    let test = TestParser::new("<Text> </Text>");
    let mut parser = test.prepare();

    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { children, .. } => {
        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
            assert_string!(parser, *string_id, " ");
        });
    });
}

/// Parse tree literal body source spans separately from opening tags.
#[test]
fn test_parse_tree_literal_records_body_span() {
    let test = TestParser::new("<Link>\n  Docs\n</Link>");
    let mut parser = test.prepare();

    let expression = parser.parse_tree_literal().unwrap();
    let body_span = parser
        .tree
        .get_side_span(expression, NodeSpanType::Region(NodeSpanRegion::Body))
        .expect("expected tree literal body span");

    assert_eq!(parser.span_str(body_span), "\n  Docs\n");
}

/// Parse a kebab case tree tag into its camel case path.
#[test]
fn test_parse_tree_fragment_with_kebab_tag() {
    let test = TestParser::new("<amp-something />");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    // <amp-something />
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "ampSomething");
        assert_eq!(parser.span_str(parser.tree.get_span(*left)), "amp-something");
        assert!(attributes.is_none());
        assert!(children.is_none());
    });
}

/// Parse named tree attributes with expression values and implicit flags.
#[test]
fn test_parse_tree_fragment_with_attributes() {
    // pure tree syntax: numeric values need {}, boolean flags are implicit true
    let test = TestParser::new("<A a={1} annoying-bee={2} c={3} flag />");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "A");
        assert_eq!(attributes.as_ref().unwrap().len(), 4);
        // a={1}
        assert_node!(parser.tree, attributes.as_ref().unwrap()[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });
        // annoying-bee={2}
        assert_node!(parser.tree, attributes.as_ref().unwrap()[1], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "annoyingBee");
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
        });
        // c={3}
        assert_node!(parser.tree, attributes.as_ref().unwrap()[2], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "c");
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(3)));
        });
        // flag (implicit true)
        assert_node!(parser.tree, attributes.as_ref().unwrap()[3], TreeAttribute::Named { name: Name::Identifier(name), value: None } => {
            assert_string!(parser, *name, "flag");
        });

        assert!(children.is_none());
    });
}

/// Recover one malformed tree attribute without losing following attributes.
#[test]
fn test_recover_tree_attribute_missing_value() {
    let test = TestParser::new(r#"<Panel broken= next="ok" />"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::TreeAttribute),
            Some(TokenType::Identifier),
            None,
            "next",
        )],
    );
    assert_node!(parser.tree, expression, Expression::TreeExpression { attributes: Some(attributes), .. } => {
        assert_eq!(attributes.len(), 2);
        assert_node!(parser.tree, attributes[0], TreeAttribute::Error);
        assert_eq!(parser.span_str(parser.tree.get_span(attributes[0])), "broken=");
        assert_node!(parser.tree, attributes[1], TreeAttribute::Named { name, value: Some(TreeAttributeValue::String(value)) } => {
            assert_string!(parser, name.string(), "next");
            assert_string!(parser, *value, "ok");
        });
    });
}

/// Recover one malformed spread attribute without losing following attributes.
#[test]
fn test_recover_tree_spread_attribute_missing_value() {
    let test = TestParser::new("<Panel {...} next />");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::TreeAttribute),
            Some(TokenType::CloseBrace),
            None,
            "}",
        )],
    );
    assert_node!(parser.tree, expression, Expression::TreeExpression { attributes: Some(attributes), .. } => {
        assert_eq!(attributes.len(), 2);
        assert_node!(parser.tree, attributes[0], TreeAttribute::Error);
        assert_eq!(parser.span_str(parser.tree.get_span(attributes[0])), "{...}");
        assert_node!(parser.tree, attributes[1], TreeAttribute::Named { name, value: None } => {
            assert_string!(parser, name.string(), "next");
        });
    });
}

/// Recover one malformed expression child without losing following children.
#[test]
fn test_recover_tree_expression_child() {
    let test = TestParser::new("<Panel>{,}<Child /></Panel>");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    test.assert_errors(
        &parser,
        &[(Some(NodeType::TreeChild), Some(TokenType::Comma), None, ",")],
    );
    assert_node!(parser.tree, expression, Expression::TreeExpression { children: Some(children), .. } => {
        assert_eq!(children.len(), 2);
        assert_node!(parser.tree, children[0], TreeChild::Error);
        assert_eq!(parser.span_str(parser.tree.get_span(children[0])), "{,}");
        assert_node!(parser.tree, children[1], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { .. });
        });
    });
}

/// Recover one unterminated expression child at its enclosing closing tag.
#[test]
fn test_recover_unterminated_tree_expression_child() {
    let test = TestParser::new("<Panel>{value + ;</Panel>");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    test.assert_errors(
        &parser,
        &[
            (Some(NodeType::Expression), None, None, ";"),
            (
                Some(NodeType::TreeChild),
                Some(TokenType::Semicolon),
                None,
                ";",
            ),
        ],
    );
    assert_node!(parser.tree, expression, Expression::TreeExpression { children: Some(children), .. } => {
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Error);
        assert_eq!(parser.span_str(parser.tree.get_span(children[0])), "{value + ;");
    });
}

/// Recover missing nested closing tags at the matching ancestor closing tag.
#[test]
fn test_recover_tree_ancestor_closing_tag() {
    let test = TestParser::new("<Panel><Item></Panel>");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    test.assert_errors(
        &parser,
        &[(Some(NodeType::Expression), None, None, "</Panel>")],
    );
    assert_node!(parser.tree, expression, Expression::TreeExpression { children: Some(children), .. } => {
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { children: Some(children), .. } => {
                assert!(children.is_empty());
            });
        });
    });
}

/// Recover a malformed nested opening before a matching ancestor closing tag.
#[test]
fn test_recover_tree_opening_at_ancestor_closing_tag() {
    let test = TestParser::new(
        "const broken = <Panel><Item><Child flag={ ;</Panel>;\nconst recovered = 1;",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_errors(
        &parser,
        &[
            (
                Some(NodeType::TreeChild),
                Some(TokenType::Semicolon),
                None,
                ";",
            ),
            (Some(NodeType::Expression), None, None, "</Panel>"),
        ],
    );
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let declarator = parser.tree.get(declarators[0]);
        assert_node!(parser.tree, declarator.pattern, Pattern::Binding { name, .. } => {
            assert_string!(parser, *name, "recovered");
        });
    });
}

/// Recover one mismatched closing tag without losing later children.
#[test]
fn test_recover_tree_mismatched_closing_tag() {
    let test = TestParser::new("<Panel></Other><Child /></Panel>");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    test.assert_errors(
        &parser,
        &[(Some(NodeType::TreeChild), None, None, "</Other>")],
    );
    assert_node!(parser.tree, expression, Expression::TreeExpression { children: Some(children), .. } => {
        assert_eq!(children.len(), 2);
        assert_node!(parser.tree, children[0], TreeChild::Error);
        assert_eq!(parser.span_str(parser.tree.get_span(children[0])), "</Other>");
        assert_node!(parser.tree, children[1], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { .. });
        });
    });
}

/// Recover missing closing tags at EOF without losing nested children.
#[test]
fn test_recover_tree_closing_tag_at_eof() {
    let test = TestParser::new("<Panel><Child />");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);
    assert_node!(parser.tree, expression, Expression::TreeExpression { children: Some(children), .. } => {
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { .. });
        });
    });
}

/// Parse a tree element carrying attributes and one expression child.
#[test]
fn test_parse_tree_fragment_with_attributes_and_child() {
    // pure tree syntax
    let test = TestParser::new(
        r"
<Tooltip
    title={true}
    flag
    something-else={false}
>
    {true}
</Tooltip>
",
    );
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Tooltip");
        assert_eq!(attributes.as_ref().unwrap().len(), 3);
        // title={true}
        assert_node!(parser.tree, attributes.as_ref().unwrap()[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "title");
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Boolean(true)));
        });
        // flag (implicit true)
        assert_node!(parser.tree, attributes.as_ref().unwrap()[1], TreeAttribute::Named { name: Name::Identifier(name), value: None } => {
            assert_string!(parser, *name, "flag");
        });
        // something-else={false}
        assert_node!(parser.tree, attributes.as_ref().unwrap()[2], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "somethingElse");
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Boolean(false)));
        });

        assert!(children.is_some());
        // {true} child expression
        assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Boolean(true)));
        });
    });
}

/// Parse tree elements nested several levels deep.
#[test]
fn test_parse_tree_nested_deep() {
    // pure tree syntax: children must be children or {expression}
    let test = TestParser::new(
        r"
<A>
    <B>
        <C>
            <D/>
            {2}
        </C>
    </B>
</A>
",
    );
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "A");
        assert!(attributes.is_none());
        assert!(children.is_some());
        // <B>
        assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "B");
                assert!(attributes.is_none());
                assert!(children.is_some());
                // <C>
                assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Tree { value } => {
                    assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "C");
                        assert!(attributes.is_none());
                        assert!(children.is_some());
                        // <D/>
                        assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Tree { value } => {
                            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "D");
                                assert!(attributes.is_none());
                                assert!(children.is_none());
                            });
                        });
                        // {2}
                        assert_node!(parser.tree, children.as_ref().unwrap()[1], TreeChild::Expression { value } => {
                            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
                        });
                    });
                });
            });
        });
    });
}

/// Parse a tree element written inside parentheses.
#[test]
fn test_parse_tree_in_parenthesis() {
    let test = TestParser::new(
        r#"
(
    <div className="font-semibold">
        <Link subtle to={1}>
            {2}
        </Link>
    </div>
)
        "#,
    );
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    crate::assert_parenthesized!(parser.tree, expression, expression => {
        // <div className="font-semibold">
        assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(attributes.is_some());
            assert_eq!(attributes.as_ref().unwrap().len(), 1);
            // className="font-semibold"
            assert_node!(parser.tree, attributes.as_ref().unwrap()[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::String(string_id)) } => {
                // className
                assert_string!(parser, *name, "className");
                // font-semibold
                assert_string!(parser, *string_id, "font-semibold");
            });

            assert!(children.is_some());
            assert_eq!(children.as_ref().unwrap().len(), 1);
            // <Link subtle to={1}>
            assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Tree { value } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
                    // Link
                    assert_expression_path!(parser, parser.tree.get(*left), "Link");
                    assert!(attributes.is_some());
                    assert_eq!(attributes.as_ref().unwrap().len(), 2);
                    // subtle
                    assert_node!(parser.tree, attributes.as_ref().unwrap()[0], TreeAttribute::Named { name: Name::Identifier(name), value: None } => {
                        assert_string!(parser, *name, "subtle");
                    });
                    // to={1}: {} is the expression container, value is just 1
                    assert_node!(parser.tree, attributes.as_ref().unwrap()[1], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
                        assert_string!(parser, *name, "to");
                        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
                    });

                    assert!(children.is_some());
                    assert_eq!(children.as_ref().unwrap().len(), 1);
                    // {2}: {} is the expression container, value is just 2
                    assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Expression { value } => {
                        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
                    });
                });
            });
        });
    });
}

/// Parse tree literal with generic attributes on the tag.
#[test]
fn test_parse_tree_with_generic_arguments() {
    let test = TestParser::new(r#"<Component<unknown>></Component>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), generic_arguments, attributes, children, .. } => {
        assert_node!(parser.tree, *left, Expression::Identifier { name } => {
            assert_string!(parser, *name, "Component");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Unknown);
                    });
            });
        });
        assert!(attributes.is_none());
        assert!(children.is_some());
        assert_eq!(children.as_ref().unwrap().len(), 0);
    });
}

/// Parse generic attributes containing a shift-left-like generic arrow.
#[test]
fn test_parse_generic_arguments_with_shift_left_generic_arrow() {
    let test = TestParser::new(r#"<<T>(v: T) => void>"#);
    let mut parser = test.prepare();

    // parse the generic attributes
    let generic_arguments = parser
        .parse_generic_argument_list(Default::default())
        .unwrap();
    assert_eq!(generic_arguments.len(), 1);

    // verify the generic arrow argument shape
    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
        assert_node!(parser.tree, *value, TypeExpression::Function(function) => {
            assert_eq!(function.generic_parameters.len(), 1);
            assert_node!(parser.tree, function.generic_parameters[0], GenericParameter::Type { name, constraint, default, .. } => {
                assert_string!(parser, *name, "T");
                assert!(constraint.is_none());
                assert!(default.is_none());
            });
            assert_eq!(function.parameters.len(), 1);
            assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
                assert_string!(parser, *name, "v");
                assert_node!(parser.tree, *ty, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Void);
            });
        });
    });
}

/// Parse tree literal with shift-left-like generic attributes on the tag.
#[test]
fn test_parse_tree_with_shift_left_generic_arguments() {
    let test = TestParser::new(r#"<Component<<T>(v: T) => void> />"#);
    let mut parser = test.prepare();

    // parse the tree literal
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), generic_arguments, attributes, children, .. } => {
        assert_node!(parser.tree, *left, Expression::Identifier { name } => {
            assert_string!(parser, *name, "Component");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Function(function) => {
                        assert_eq!(function.parameters.len(), 1);
                    });
            });
        });
        assert!(attributes.is_none());
        assert!(children.is_none());
    });
}

/// Parse a tree literal with generic attributes and multiline attributes.
#[test]
fn test_parse_tree_with_generic_arguments_and_multiline_attributes() {
    let test = TestParser::new(
        r#"<Tags<ValueTagData>
  defaultValue={value}
/>"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser.parse_tree_literal().unwrap();

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { left: Some(left), generic_arguments, attributes, children, .. } => {
        assert_node!(parser.tree, *left, Expression::Identifier { name } => {
            assert_string!(parser, *name, "Tags");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, .. } => {
                        assert_path!(parser, *path, "ValueTagData");
                    });
            });
        });

        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.len(), 1);
        assert_node!(parser.tree, attributes[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "defaultValue");
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });

        assert!(children.is_none());
    });
}

/// Parse tree attribute comments without expanding the tag name span.
#[test]
fn test_parse_tree_attribute_leading_comments_keep_tag_name_span() {
    let test = TestParser::new(
        r#"<Widget
  // props-leading
  {...props} // props-tail
  kind="primary"
  // extra-leading
  {...extra}
/>"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser.parse_tree_literal().unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_eq!(parser.span_str(parser.tree.get_span(*left)), "Widget");

        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.len(), 3);
        assert_eq!(parser.span_str(parser.tree.get_span(attributes[0])), "{...props}");
        assert_eq!(parser.span_str(parser.tree.get_span(attributes[1])), "kind=\"primary\"");
        assert_eq!(parser.span_str(parser.tree.get_span(attributes[2])), "{...extra}");
        assert!(children.is_none());

        let comments = parser.comments();
        assert_eq!(comments.len(), 3);
        assert_eq!(parser.span_str(comments[0].span), "// props-leading");
        assert_eq!(comments[0].following_token_start(), Some(parser.tree.get_span(attributes[0]).start));
        assert!(comments[0].is_leading());
        assert_eq!(parser.span_str(comments[1].span), "// props-tail");
        assert!(comments[1].is_trailing());
        assert!(!comments[1].is_leading());
        assert_eq!(parser.span_str(comments[2].span), "// extra-leading");
        assert_eq!(comments[2].following_token_start(), Some(parser.tree.get_span(attributes[2]).start));
        assert!(comments[2].is_leading());
    });
}

/// Parse a tree element mixing text children with expression children.
#[test]
fn test_parse_tree_with_text_content() {
    let test = TestParser::new(r#"<h4>Tool: {part.toolName}</h4>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "h4");
        assert!(attributes.is_none());
        assert!(children.is_some());
        assert_eq!(children.as_ref().unwrap().len(), 2);
        // Tool:
        assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Text { value } => {
            assert_string!(parser, *value, "Tool: ");
        });
        // {part.toolName}
        assert!(matches!(
            parser.tree.get(children.as_ref().unwrap()[1]),
            TreeChild::Expression { .. }
        ));
    });
}

/// Parse a nested tree element holding text and expression children.
#[test]
fn test_parse_nested_tree_with_text_content() {
    // <div><h4>Tool: {x}</h4></div>
    let test = TestParser::new(r#"<div><h4>Tool: {x}</h4></div>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(attributes.is_none());
        assert!(children.is_some());
        assert_eq!(children.as_ref().unwrap().len(), 1);
        // nested <h4>...</h4>
        assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { .. });
        });
    });
}

/// Parse a tree element with one attribute and one nested child.
#[test]
fn test_parse_tree_with_attribute_and_children() {
    // <div key={index}><h4>Tool: {x}</h4></div>
    let test = TestParser::new(r#"<div key={index}><h4>Tool: {x}</h4></div>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(attributes.is_some());
        assert_eq!(attributes.as_ref().unwrap().len(), 1);
        assert!(children.is_some());
        assert_eq!(children.as_ref().unwrap().len(), 1);
    });
}

/// Tree fragment containing a callback that returns nested tree literals.
#[test]
fn test_parse_tree_fragment_with_nested_callback() {
    let test = TestParser::new(r#"<>{x.map(() => (<div><h4>T: {y}</h4></div>))}</>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, attributes, children, .. } => {
        assert!(attributes.is_none());
        assert!(children.is_some());
        assert_eq!(children.as_ref().unwrap().len(), 1);
    });
}

/// Parse a comment container `{/* */}` as an empty expression container.
#[test]
fn test_parse_tree_with_comment_container() {
    let test = TestParser::new(r#"<div>{/* comment */}</div>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(_), attributes, children, .. } => {
        assert!(attributes.is_none());
        assert!(children.is_some());
    });
}

/// Parse a tree fragment with keyword text followed by an expression container.
#[test]
fn test_parse_tree_fragment_with_keyword_text_and_expression() {
    let test = TestParser::new("<>for {x}</>");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { left, children, .. } => {
        assert!(left.is_none());
        let children = children.as_ref().expect("expected fragment children");
        assert_eq!(children.len(), 2);

        assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
            assert_string!(parser, *string_id, "for ");
        });

        assert_node!(parser.tree, children[1], TreeChild::Expression { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "x");
        });
    });
}

/// Parse tree fragment with comments between the angle brackets.
#[test]
fn test_parse_tree_fragment_with_comments() {
    let test = TestParser::new("<\n// comment\n/* comment */\n>\n</>");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, attributes, children, .. } => {
        assert!(attributes.is_none());
        assert!(children.is_some());
        assert!(children.as_ref().unwrap().is_empty());
    });
}

/// Parse tree fragment with a closing tag that has trivia before the slash.
#[test]
fn test_parse_tree_fragment_closing_with_comment_retention() {
    let test = TestParser::new("<>\n< /* comment */ / >");
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, attributes, children, .. } => {
        assert!(attributes.is_none());
        assert!(children.is_some());
        assert!(children.as_ref().unwrap().is_empty());
    });
}

/// Parse tree literal with namespace tag and attribute.
#[test]
fn test_parse_tree_with_namespace_tag() {
    let test = TestParser::new(r#"<Foo:Bar n:foo="bar" />"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Foo:Bar");
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.len(), 1);
        assert_node!(parser.tree, attributes[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::String(value)) } => {
            assert_string!(parser, *name, "n:foo");
            assert_string!(parser, *value, "bar");
        });
        assert!(children.is_none());
    });
}

/// Parse a ternary whose branch holds a tree literal containing `&&`.
#[test]
fn test_parse_ternary_with_and_in_tree() {
    // parse the ternary alone, its condition is a bare name
    let test = TestParser::new(r#"a ? <>{y && <E />}</> : null"#);
    let mut parser = test.prepare();
    let expr = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    // a ? ... : null -> If with IfForm::Ternary
    assert_node!(parser.tree, expr, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        // condition: a
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Identifier { name } => {
            assert_string!(parser, *name, "a");
        });
        // consequence: <>{y && <E />}</>
        assert_node!(parser.tree, *then_expression, Expression::TreeExpression { left: None, attributes, children, .. } => {
            assert!(attributes.is_none());
            assert!(children.is_some());
            assert_eq!(children.as_ref().unwrap().len(), 1);
            // {y && <E />} - the && expression
            assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Binary { .. });
            });
        });
        // alternative: null
        assert!(else_expression.is_some());
        assert_node!(parser.tree, else_expression.unwrap(), Expression::Literal(Literal::Null));
    });
}

/// Parse a nested tree literal inside an attribute expression container.
#[test]
fn test_parse_nested_tree_in_attribute() {
    let test = TestParser::new(r#"<Button icon={<Icon />} />"#);
    let mut parser = test.prepare();
    let expr = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Button");
        assert!(attributes.is_some());
        assert_eq!(attributes.as_ref().unwrap().len(), 1);
        // icon={<Icon />}
        assert_node!(parser.tree, attributes.as_ref().unwrap()[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "icon");
            // value is <Icon />
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(inner_left), attributes: inner_args, children: inner_elems, .. } => {
                assert_expression_path!(parser, parser.tree.get(*inner_left), "Icon");
                assert!(inner_args.is_none());
                assert!(inner_elems.is_none());
            });
        });
        assert!(children.is_none());
    });
}

/// Parse multiline tree attribute expression containers before a tag close.
#[test]
fn test_parse_multiline_tree_attribute_expression_before_tag_close() {
    let test = TestParser::new(
        r#"<PopoverProvider
  popover={
    <TooltipContent>
      <Picker />
    </TooltipContent>
  }
>
  <PopoverTrigger />
</PopoverProvider>"#,
    );
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "PopoverProvider");

        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.len(), 1);
        assert_node!(parser.tree, attributes[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "popover");
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), children, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "TooltipContent");
                let children = children.as_ref().expect("expected tooltip children");
                assert_eq!(children.len(), 1);
                assert_node!(parser.tree, children[0], TreeChild::Tree { value } => {
                    assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "Picker");
                        assert!(attributes.is_none());
                        assert!(children.is_none());
                    });
                });
            });
        });

        let children = children.as_ref().expect("expected provider children");
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "PopoverTrigger");
                assert!(attributes.is_none());
                assert!(children.is_none());
            });
        });
    });
}

/// Parse a tree attribute holding a tree with a nested map callback.
#[test]
fn test_parse_tree_attribute_tree_with_nested_map_before_tag_close() {
    let test = TestParser::new(
        r#"<PopoverProvider
  popover={
    <TooltipContent>
      {presets.length > 0 && (
        <Swatches>
          {presets.map((preset, index: number) => (
            <SwatchColor
              key={`${preset?.value || index}-${index}`}
              onClick={() => preset && updateValue(preset.value || '')}
            />
          ))}
        </Swatches>
      )}
    </TooltipContent>
  }
>
  <PopoverTrigger style={{ margin: 4 }} />
</PopoverProvider>"#,
    );
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "PopoverProvider");

        let attributes = attributes.as_ref().expect("expected provider attributes");
        assert_eq!(attributes.len(), 1);
        assert_node!(parser.tree, attributes[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "popover");
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "TooltipContent");
            });
        });

        let children = children.as_ref().expect("expected provider children");
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), attributes, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "PopoverTrigger");
                let attributes = attributes.as_ref().expect("expected trigger attributes");
                let has_style_argument = attributes.iter().any(|argument| {
                    matches!(
                        parser.tree.get(*argument),
                        TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) }
                            if parser.strings.get(*name) == "style"
                                && matches!(parser.tree.get(*value), Expression::ObjectExpression { .. })
                    )
                });
                assert!(has_style_argument);
            });
        });
    });
}

/// Tree fragment text in attribute expression containers parses as tree string.
#[test]
fn test_parse_tree_fragment_text_in_attribute_expression() {
    let test = TestParser::new(
        r#"<Show when={shouldShow()} fallback={<>off</>}><>{props.children}</></Show>"#,
    );
    let mut parser = test.prepare();
    let expr = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), attributes, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Show");
        let attributes = attributes.as_ref().expect("expected attributes");
        assert!(attributes.len() >= 2);
        assert_node!(parser.tree, attributes[1], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "fallback");
            assert_node!(parser.tree, *value, Expression::TreeExpression { left, children, .. } => {
                assert!(left.is_none());
                let children = children.as_ref().expect("expected fragment children");
                assert_eq!(children.len(), 1);
                assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
                    assert_string!(parser, *string_id, "off");
                });
            });
        });
    });
}

/// Parse a named tree literal with text content inside an attribute expression container.
#[test]
fn test_parse_tree_named_text_in_attribute_expression() {
    let test = TestParser::new(
        r#"<ParentComponent prop={
  <Child>
    test
  </Child>
}/>;"#,
    );
    let mut parser = test.prepare();
    let expr = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), attributes, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "ParentComponent");
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_node!(parser.tree, attributes[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "prop");
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(inner_left), children, .. } => {
                assert_expression_path!(parser, parser.tree.get(*inner_left), "Child");
                let children = children.as_ref().expect("expected children");
                assert_eq!(children.len(), 1);
                assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
                    let text = parser.strings.get(*string_id);
                    assert_eq!(text.trim(), "test");
                });
            });
        });
    });
}

/// Parse tree attribute objects with callback values that return ternary fragments.
#[test]
fn test_parse_tree_attribute_object_callback_with_fragment_ternary() {
    let input = r#"<F
  values={{
    resend: (chunks) => (
      <button>
        {resendCooldown > 0 ? (
          <>
            {chunks} ({resendCooldown})
          </>
        ) : (
          chunks
        )}
      </button>
    )
  }}
/>"#;

    // parse one tree literal from the tree-tag input
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression_id = parser.parse_tree_literal().unwrap();

    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    assert_node!(parser.tree, expression_id, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "F");
        assert!(children.is_none());

        let attributes = attributes.as_ref().expect("expected tree attributes");
        assert_eq!(attributes.len(), 1);
        assert_node!(parser.tree, attributes[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "values");

            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(key_name), value: callback, .. } => {
                    assert_string!(parser, *key_name, "resend");
                    assert_node!(parser.tree, *callback, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                            assert_eq!(signature.form, FunctionForm::Lambda);
                            assert_eq!(signature.parameters.len(), 1);
                            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "chunks");
                            });

                            crate::assert_parenthesized!(parser.tree, body.expect("expected callback body"), expression => {
                                assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(button_left), children, .. } => {
                                    assert_expression_path!(parser, parser.tree.get(*button_left), "button");
                                    let children = children.as_ref().expect("expected button children");
                                    assert_eq!(children.len(), 1);
                                    assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
                                        assert_node!(parser.tree, *value, Expression::If { form, .. } => {
                                            assert_eq!(*form, IfForm::Ternary);
                                        });
                                    });
                                });
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Parse spread attributes with newline and comments after the container open.
#[test]
fn test_parse_tree_attribute_spread_with_multiline_comments() {
    let test = TestParser::new(
        r#"<Tag
  {
    // comment before spread
    ...(rootProps as unknown)
  }
/>"#,
    );
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Tag");
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.len(), 1);
        assert_node!(parser.tree, attributes[0], TreeAttribute::Spread { value } => {
            crate::assert_parenthesized!(parser.tree, *value, expression => {
                assert_node!(parser.tree, *expression, Expression::As { .. } => {
                });
            });
        });
        assert!(children.is_none());
    });
}

/// Parse spread tree attributes holding a cast expression.
#[test]
fn test_parse_tree_attribute_spread_with_cast() {
    let test = TestParser::new(
        r#"<WrappedComponent {...(this.props as P & DependentProps)} {...this.state} />"#,
    );
    let mut parser = test.prepare();
    let expr = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "WrappedComponent");
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.len(), 2);
        // {...(this.props as P & DependentProps)}
        assert_node!(parser.tree, attributes[0], TreeAttribute::Spread { value } => {
            crate::assert_parenthesized!(parser.tree, *value, expression => {
                assert_node!(parser.tree, *expression, Expression::As { .. } => {
                });
            });
        });
        // {...this.state}
        assert_node!(parser.tree, attributes[1], TreeAttribute::Spread { value } => {
            assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                assert_node!(parser.tree, *left, Expression::This);
                assert_string!(parser, *name, "state");
            });
        });
        assert!(children.is_none());
    });
}

/// Parse deeply nested tree literals inside attributes.
#[test]
fn test_parse_deeply_nested_tree_in_attribute() {
    let test = TestParser::new(r#"<Outer title={<div><Button icon={<Icon />} /></div>} />"#);
    let mut parser = test.prepare();
    let expr = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Outer");
        assert!(attributes.is_some());
        assert_eq!(attributes.as_ref().unwrap().len(), 1);
        // title={<div>...</div>}
        assert_node!(parser.tree, attributes.as_ref().unwrap()[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "title");
            // <div><Button icon={<Icon />} /></div>
            assert_node!(parser.tree, *value, Expression::TreeExpression { children: div_elems, .. } => {
                assert!(div_elems.is_some());
                assert_eq!(div_elems.as_ref().unwrap().len(), 1);
            });
        });
        assert!(children.is_none());
    });
}

/// Object literal inside attribute expression container.
#[test]
fn test_parse_tree_attr_object_literal() {
    let test = TestParser::new(r#"<Rive style={{width: 400, height: 400}} />"#);
    let mut parser = test.prepare();
    let expr = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Rive");
        assert!(attributes.is_some());
        assert_eq!(attributes.as_ref().unwrap().len(), 1);
        // style={{width: 400, height: 400}}
        assert_node!(parser.tree, attributes.as_ref().unwrap()[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "style");
            // {width: 400, height: 400}
            assert_node!(parser.tree, *value, Expression::ObjectExpression { .. });
        });
        assert!(children.is_none());
    });
}

/// Multiline tree literal with expression container and sibling children.
#[test]
fn test_parse_multiline_tree_with_siblings() {
    let code = "<div>\n\t{x}\n\t<form onClick={() => {}}></form>\n</div>";
    let test = TestParser::new(code);
    let mut parser = test.prepare();
    let expr = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(attributes.is_none());
        assert!(children.is_some());
        assert_eq!(children.as_ref().unwrap().len(), 2);
        // {x}
        assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Identifier { name } => {
                assert_string!(parser, *name, "x");
            });
        });
        // <form onClick={() => {}}></form>
        assert_node!(parser.tree, children.as_ref().unwrap()[1], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(form_left), attributes: form_args, .. } => {
                assert_expression_path!(parser, parser.tree.get(*form_left), "form");
                assert!(form_args.is_some());
                assert_eq!(form_args.as_ref().unwrap().len(), 1);
            });
        });
    });
}

/// Logical && pattern inside tree content.
#[test]
fn test_parse_tree_with_logical_and() {
    let test = TestParser::new(r#"<div>{x && <span/>}</div>"#);
    let mut parser = test.prepare();
    let expr = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(attributes.is_none());
        assert!(children.is_some());
        assert_eq!(children.as_ref().unwrap().len(), 1);
        // {x && <span/>}
        assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Binary { left: bin_left, right: bin_right, .. } => {
                // x
                assert_node!(parser.tree, *bin_left, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "x");
                });
                // <span/>
                assert_node!(parser.tree, *bin_right, Expression::TreeExpression { .. });
            });
        });
    });
}

/// Parse relational operators inside tree expression containers.
#[test]
fn test_parse_tree_expression_container_relational() {
    let test = TestParser::new(r#"<div>{a < b}</div>"#);
    let mut parser = test.prepare();
    let expr = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(attributes.is_none());
        assert!(children.is_some());
        assert_eq!(children.as_ref().unwrap().len(), 1);
        // {a < b}
        assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, left: bin_left, right: bin_right, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
                assert_node!(parser.tree, *bin_left, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "a");
                });
                assert_node!(parser.tree, *bin_right, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "b");
                });
            });
        });
    });
}

/// Parse generic calls inside tree expression containers.
#[test]
fn test_parse_tree_expression_container_generic_call() {
    let test = TestParser::new(r#"<div>{foo<T>(x)}</div>"#);
    let mut parser = test.prepare();
    let expr = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(attributes.is_none());
        assert!(children.is_some());
        assert_eq!(children.as_ref().unwrap().len(), 1);
        // {foo<T>(x)}
        assert_node!(parser.tree, children.as_ref().unwrap()[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Call { left, generic_arguments, arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "T");
                });
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "x");
                });
            });
        });
    });
}

/// Parse logical and with an inline tree containing attributes and text.
#[test]
fn test_parse_tree_expression_container_logical_and_inline_tree_with_text() {
    let test =
        TestParser::new(r#"<div>{errors.Checkbox && <p id="Checkbox">Checkbox Error</p>}</div>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        let children = children.as_ref().expect("expected div children");
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::And);
                assert_node!(parser.tree, *right, Expression::TreeExpression { left: Some(right_left), attributes: Some(attributes), children: Some(right_children), .. } => {
                    assert_expression_path!(parser, parser.tree.get(*right_left), "p");
                    assert_eq!(attributes.len(), 1);
                    assert_eq!(right_children.len(), 1);
                    assert_node!(parser.tree, right_children[0], TreeChild::Text { value } => {
                        assert_string!(parser, *value, "Checkbox Error");
                    });
                });
            });
        });
    });
}

/// Tree literal should parse after a closing class block on a new line.
#[test]
fn test_parse_tree_after_class_block_newline() {
    let test = TestParser::new("class C extends D<T> {}\n<C/>");
    let mut parser = test.prepare();

    // class declaration
    let class_expr = parser.parse_statement();
    assert_node!(parser.tree, class_expr, Expression::Declaration(_));

    // tree literal expression
    let tree_expr = parser.parse_statement();
    assert_node!(parser.tree, tree_expr, Expression::TreeExpression { .. });
}

/// Empty template literal.
#[test]
fn test_parse_template_literal_empty() {
    let test = TestParser::new("``");
    let mut parser = test.prepare();
    let literal = parser.parse_template_literal().unwrap();

    let TemplateLiteral::String { chunk } = literal else {
        panic!("expected plain template literal");
    };
    assert_string!(parser, chunk.cooked.unwrap(), "");
}

/// Template literal interpolation should allow optional chaining.
#[test]
fn test_parse_template_literal_optional_chain() {
    let test = TestParser::new(r#"`value ${theme?.activeColor}`"#);
    let mut parser = test.prepare();
    let literal = parser.parse_template_literal().unwrap();

    let TemplateLiteral::InterpolatedString { chunks, arguments } = literal else {
        panic!("expected interpolated template literal");
    };
    assert_eq!(chunks.len(), 2);
    assert_eq!(arguments.len(), 1);
    assert_string!(parser, chunks[0].cooked.unwrap(), "value ");
    assert_string!(parser, chunks[1].cooked.unwrap(), "");
    assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
        assert_node!(parser.tree, *value, Expression::Chain { expression } => {
            assert_node!(parser.tree, *expression, Expression::Member { left, name, is_optional } => {
                assert_string!(parser, *name, "activeColor");
                assert!(*is_optional);
                assert_expression_path!(parser, parser.tree.get(*left), "theme");
            });
        });
    });
}

/// Template literal interpolation should allow ternary expressions.
#[test]
fn test_parse_template_literal_ternary() {
    let test = TestParser::new(r#"`value ${mode === "dark" ? "dark" : "light"}`"#);
    let mut parser = test.prepare();
    let literal = parser.parse_template_literal().unwrap();

    let TemplateLiteral::InterpolatedString { chunks, arguments } = literal else {
        panic!("expected interpolated template literal");
    };
    assert_eq!(chunks.len(), 2);
    assert_eq!(arguments.len(), 1);
    assert_string!(parser, chunks[0].cooked.unwrap(), "value ");
    assert_string!(parser, chunks[1].cooked.unwrap(), "");
    assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
        assert_node!(parser.tree, *value, Expression::If { form, condition, then_expression, else_expression } => {
            assert_eq!(*form, IfForm::Ternary);
            assert_eq!(condition.operands.len(), 1);
            assert_node!(&condition.operands[0], ConditionOperand::Expression { condition } => {
                assert_node!(parser.tree, *condition, Expression::Binary { operator, .. } => {
                    assert_eq!(*operator, BinaryOperator::EqualStrict);
                });
            });
            assert_node!(parser.tree, *then_expression, Expression::Literal(Literal::String(value)) => {
                assert_string!(parser, *value, "dark");
            });
            let else_expression = else_expression.expect("expected false branch");
            assert_node!(parser.tree, else_expression, Expression::Literal(Literal::String(value)) => {
                assert_string!(parser, *value, "light");
            });
        });
    });
}

/// Parse tree text that includes `=` after opening tags.
#[test]
fn test_parse_tree_text_with_equals_after_tag_with_attribute_no_space() {
    let test = TestParser::new(r#"<div className={styles.foo}>=</div>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { children, .. } => {
        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
            assert_string!(parser, *string_id, "=");
        });
    });
}

/// Parse tree text that includes `=` after opening tags.
#[test]
fn test_parse_tree_text_with_equals_after_tag_with_attribute_with_space() {
    let test = TestParser::new(r#"<div className={styles.foo} >=</div>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { children, .. } => {
        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
            assert_string!(parser, *string_id, "=");
        });
    });
}

/// Parse tree text that includes `=` after opening tags.
#[test]
fn test_parse_tree_text_with_equals_after_simple_tag_no_space() {
    let test = TestParser::new(r#"<div>=</div>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { children, .. } => {
        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
            assert_string!(parser, *string_id, "=");
        });
    });
}

/// Parse tree fragments containing text after opening tags.
#[test]
fn test_parse_tree_fragment_text_with_equals_prefix() {
    let test = TestParser::new(r#"<>=x</>"#);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression, Expression::TreeExpression { children, .. } => {
        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
            assert_string!(parser, *string_id, "=x");
        });
    });
}

/// Parse nested tree fragment text that starts with `=`.
#[test]
fn test_parse_nested_tree_fragment_text_with_equals_prefix() {
    let input = r#"
<>
    <>=x</>
</>;
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::TreeExpression { left, children, .. } => {
        assert!(left.is_none());
        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 1);

        assert_node!(parser.tree, children[0], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left, children, .. } => {
                assert!(left.is_none());
                let children = children.as_ref().expect("expected nested children");
                assert_eq!(children.len(), 1);

                assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
                    assert_string!(parser, *string_id, "=x");
                });
            });
        });
    });
}

/// Parse tree fragments followed by `>=1` as binary expressions.
#[test]
fn test_parse_tree_fragment_followed_by_greater_than_or_equal() {
    let test = TestParser::new(r#"<>x</>>=1"#);
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
        assert_node!(parser.tree, *left, Expression::TreeExpression { .. });
        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(1)));
    });
}

/// Parse tree children followed by `>=1` as binary expressions.
#[test]
fn test_parse_tree_element_followed_by_greater_than_or_equal() {
    let test = TestParser::new(r#"<span>x</span>>=1"#);
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
        assert_node!(parser.tree, *left, Expression::TreeExpression { .. });
        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(1)));
    });
}

/// Parse a top-level sequence of tree text and comparison forms.
#[test]
fn test_parse_tree_text_and_greater_than_or_equal_sequence() {
    let input = r#"
<>=x</>;
<>x</>>=1;
<span>=x</span>;
<span>x</span>>=1;
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 4);
    assert_node!(
        parser.tree,
        expressions[0],
        Expression::TreeExpression { .. }
    );
    assert_node!(parser.tree, expressions[1], Expression::Binary { operator, .. } => {
        assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
    });
    assert_node!(
        parser.tree,
        expressions[2],
        Expression::TreeExpression { .. }
    );
    assert_node!(parser.tree, expressions[3], Expression::Binary { operator, .. } => {
        assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
    });
}

/// Parse tree fragments with text between child children in arrays.
#[test]
fn test_parse_tree_fragment_equals_in_array() {
    let input = r#"<Y
    children={[
        <>
            <span>x</span>=
            <br />
        </>,
        true
    ]}
/>
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    // ensure the array includes a fragment and a boolean
    assert_node!(parser.tree, expression, Expression::TreeExpression { attributes, .. } => {
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.len(), 1);
        assert_node!(parser.tree, attributes[0], TreeAttribute::Named { name: Name::Identifier(name), value: Some(TreeAttributeValue::Expression(value)) } => {
            assert_string!(parser, *name, "children");
            assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::TreeExpression { left: None, .. });
                });
            });
        });
    });
}

/// Parse ternary expressions that return tree children inside expression containers.
#[test]
fn test_parse_tree_ternary_expression_container_in_xml_mode() {
    let input = r#"<div>{isLoading ? <div>loading</div> : <div>done</div>}</div>"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    // verify the ternary expression container
    assert_node!(parser.tree, expression, Expression::TreeExpression { children, .. } => {
        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::If { form, .. } => {
                assert_eq!(*form, IfForm::Ternary);
            });
        });
    });
}

/// Parse ternary expressions that return tree children inside expression containers.
#[test]
fn test_parse_tree_ternary_expression_container() {
    let input = r#"<div>{isLoading ? <div>loading</div> : <div>done</div>}</div>"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    // verify the ternary expression container
    assert_node!(parser.tree, expression, Expression::TreeExpression { children, .. } => {
        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 1);
        assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::If { form, .. } => {
                assert_eq!(*form, IfForm::Ternary);
            });
        });
    });
}

/// Parse tree siblings after a map callback returning a parenthesized tree literal.
#[test]
fn test_parse_tree_after_parenthesized_tree_in_expression_container() {
    let input = r#"<div>
  {items.map((item) => (
    <option>{item}</option>
  ))}
  <button />
</div>"#;

    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");

        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 2);

        assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Call { arguments, .. } => {
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                            assert_eq!(signature.form, FunctionForm::Lambda);

                            let body = body.expect("expected lambda body");
                            crate::assert_parenthesized!(parser.tree, body, expression => {
                                assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
                                    assert_expression_path!(parser, parser.tree.get(*left), "option");
                                    assert!(attributes.is_none());

                                    let children = children.as_ref().expect("expected option children");
                                    assert_eq!(children.len(), 1);
                                    assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
                                        assert_expression_path!(parser, parser.tree.get(*value), "item");
                                    });
                                });
                            });
                        });
                    });
                });
            });
        });

        assert_node!(parser.tree, children[1], TreeChild::Tree { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), attributes, children, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "button");
                assert!(attributes.is_none());
                assert!(children.is_none());
            });
        });
    });
}

/// Parse ternary fragments with text fallback.
#[test]
fn test_parse_tree_ternary_fragment_with_text_fallback() {
    let input = "shouldShow ? <>{children}</> : <>off</>";
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        let condition = condition.as_expression().expect("expected expression condition");
        assert_expression_path!(parser, parser.tree.get(condition), "shouldShow");

        assert_node!(parser.tree, *then_expression, Expression::TreeExpression { left, children, .. } => {
            assert!(left.is_none());
            let children = children.as_ref().expect("expected then children");
            assert_eq!(children.len(), 1);
            assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "children");
            });
        });

        let else_expression = else_expression.expect("expected else expression");
        assert_node!(parser.tree, else_expression, Expression::TreeExpression { left, children, .. } => {
            assert!(left.is_none());
            let children = children.as_ref().expect("expected else children");
            assert_eq!(children.len(), 1);
            assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
                assert_string!(parser, *string_id, "off");
            });
        });
    });
}

/// Parse tree fragment text nodes with standalone colon content.
#[test]
fn test_parse_tree_fragment_with_colon_text_node() {
    let input = r#"<code>{value && <>:</>}</code>"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    // verify logical-and fragment text parsing
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "code");
        let children = children.as_ref().expect("expected code children");
        assert_eq!(children.len(), 1);

        // verify the right side of `value && ...`
        assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::And);
                assert_node!(parser.tree, *right, Expression::TreeExpression { left: None, children, .. } => {
                    let children = children.as_ref().expect("expected fragment children");
                    assert_eq!(children.len(), 1);
                    assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
                        assert_string!(parser, *string_id, ":");
                    });
                });
            });
        });
    });
}

/// Parse logical and with a fragment that contains a nested ternary tree literal.
#[test]
fn test_parse_tree_logical_and_fragment_with_nested_ternary_tree() {
    let input = r#"<div>{condition && <>{show ? <Box /> : null}</>}</div>"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        let children = children.as_ref().expect("expected div children");
        assert_eq!(children.len(), 1);

        assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::And);
                assert_node!(parser.tree, *right, Expression::TreeExpression { left: None, children, .. } => {
                    let children = children.as_ref().expect("expected fragment children");
                    assert_eq!(children.len(), 1);
                    assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
                        assert_node!(parser.tree, *value, Expression::If { form, .. } => {
                            assert_eq!(*form, IfForm::Ternary);
                        });
                    });
                });
            });
        });
    });
}

/// Parse a fragment that starts with keyword text before an expression container.
#[test]
fn test_parse_tree_fragment_with_keyword_text_before_expression() {
    let input = r#"<strong>{componentNameJsx && <>for {componentNameJsx}</>}</strong>"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expression = parser.parse_tree_literal().unwrap();

    // strong element with one expression child
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), children, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "strong");
        let children = children.as_ref().expect("expected strong children");
        assert_eq!(children.len(), 1);

        // componentNameJsx && <>{...}</>
        assert_node!(parser.tree, children[0], TreeChild::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::And);
                assert_expression_path!(parser, parser.tree.get(*left), "componentNameJsx");

                // fragment children: "for " and componentNameJsx
                assert_node!(parser.tree, *right, Expression::TreeExpression { left: None, children, .. } => {
                    let children = children.as_ref().expect("expected fragment children");
                    assert_eq!(children.len(), 2);
                    assert_node!(parser.tree, children[0], TreeChild::Text { value: string_id } => {
                        assert_string!(parser, *string_id, "for ");
                    });
                    assert_node!(parser.tree, children[1], TreeChild::Expression { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "componentNameJsx");
                    });
                });
            });
        });
    });
}

/// Report tree literal namespace and member combinations during parse.
#[test]
fn test_report_tree_literal_namespace_member_path_parse_error() {
    let test = TestParser::new("<a.b:c />");
    let mut parser = test.prepare();

    let error = parser
        .parse_tree_literal()
        .expect_err("expected parse failure for namespace member path");
    assert_eq!(parser.range_str(error.range()), "/");
}

/// Parse tree children after newline-terminated declarations.
#[test]
fn test_parse_tree_after_let_newline_inside_function() {
    let input = r#"
function x() {
    let x
    <div />
}
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body, .. }) => {
            let body = body.expect("expected body");
            assert_node!(parser.tree, body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                let expressions = block_expression_ids(block);
                assert_eq!(expressions.len(), 2);
                let tree_expression = expressions[1];
                assert_node!(parser.tree, tree_expression, Expression::TreeExpression { .. });
            });
        });
    });

    let input = r#"
class Foo {}
<>
<Comp></Comp>
<Comp></Comp>
</>
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    test.assert_no_errors(&parser);

    // ensure fragments after classes parse with multiple children
    assert_eq!(expressions.len(), 2);
    let tree_expression = expressions[1];
    assert_node!(parser.tree, tree_expression, Expression::TreeExpression { left, children, .. } => {
        assert!(left.is_none());
        let children = children.as_ref().expect("expected children");
        assert_eq!(children.len(), 2);
    });
}

/// Parse tree children after newline-terminated declarations and blocks.
#[test]
fn test_parse_tree_after_newline_terminated_roots() {
    let input = r#"
let x
<Comp></Comp>

let y

<Comp></Comp>

let z;
<Comp></Comp>

function x() {
    let value
    <div />
}

{ foo: "test" }
<Comp></Comp>

function test1() {}
<Comp></Comp>

class Foo {}
<>
<Comp></Comp>
<Comp></Comp>
</>
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 13);
    assert_node!(parser.tree, expressions[0], Expression::Let { .. });
    assert_node!(
        parser.tree,
        expressions[1],
        Expression::TreeExpression { .. }
    );
    assert_node!(parser.tree, expressions[2], Expression::Let { .. });
    assert_node!(
        parser.tree,
        expressions[3],
        Expression::TreeExpression { .. }
    );
    assert_node!(parser.tree, expressions[4], Expression::Let { .. });
    assert_node!(
        parser.tree,
        expressions[5],
        Expression::TreeExpression { .. }
    );
    assert_node!(parser.tree, expressions[6], Expression::Declaration(_));
    assert_node!(
        parser.tree,
        expressions[7],
        Expression::ObjectExpression { .. }
    );
    assert_node!(
        parser.tree,
        expressions[8],
        Expression::TreeExpression { .. }
    );
    assert_node!(parser.tree, expressions[9], Expression::Declaration(_));
    assert_node!(
        parser.tree,
        expressions[10],
        Expression::TreeExpression { .. }
    );
    assert_node!(parser.tree, expressions[11], Expression::Declaration(_));
    assert_node!(parser.tree, expressions[12], Expression::TreeExpression { children, .. } => {
        let children = children.as_ref().expect("expected fragment children");
        assert_eq!(children.len(), 2);
    });
}

/// Parse tree children after return with newline termination.
#[test]
fn test_parse_tree_after_return_newline() {
    let input = r#"
function test() {
    return
    <Comp />
}
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // ensure return is terminated and tree literal follows
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body, .. }) => {
            let body = body.expect("expected body");
            assert_node!(parser.tree, body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                let expressions = block_expression_ids(block);
                assert_eq!(expressions.len(), 2);
                let return_expression = expressions[0];
                assert_node!(parser.tree, return_expression, Expression::Return { value } => {
                    assert!(value.is_none());
                });
                let tree_expression = expressions[1];
                assert_node!(parser.tree, tree_expression, Expression::TreeExpression { .. });
            });
        });
    });
}

/// Parse return parenthesized tree literal containing logical-and fragment with long text.
#[test]
fn test_parse_return_parenthesized_tree_with_logical_fragment_long_text() {
    let input = r#"
function app() {
  return (
    <Box>
      {obj.alpha.size > 0 && <>
        <Text wrap={`wrap`}>
          Because of those files having been modified, the following workspaces may need to be released again (note that private workspaces are also shown here, because even though they won't be published, releasing them will allow us to flag their dependents for potential re-release):
        </Text>
      </>}
    </Box>
  );
}
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { body, .. }) => {
            let body = body.expect("expected function body");
            assert_node!(parser.tree, body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                let expressions = block_expression_ids(block);
                assert_eq!(expressions.len(), 1);
                let return_expression = expressions[0];
                assert_node!(parser.tree, return_expression, Expression::Return { value } => {
                    let value = value.expect("expected return value");
                    crate::assert_parenthesized!(parser.tree, value, expression => {
                        assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), children, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "Box");
                            let children = children.as_ref().expect("expected box children");
                            assert_eq!(children.len(), 1);
                        });
                    });
                });
            });
        });
    });
}

/// Attach trailing line and block comments to the property they follow.
#[test]
fn test_parse_object_property_trailing_comments_on_property_owners() {
    let test = TestParser::new(
        r#"const config = {
  first: 1, // first-tail
  second: 2 /* second-tail */
}"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let statement_id = expressions[0];
    assert_node!(parser.tree, statement_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let value = parser
            .tree
            .get(declarators[0])
            .value
            .expect("expected declarator value");
        assert_node!(parser.tree, value, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 2);

            let first_annotations = parser.tree.get_decorators(properties[0].id);
            assert!(first_annotations.is_empty());

            let second_annotations = parser.tree.get_decorators(properties[1].id);
            assert!(second_annotations.is_empty());

            let second_property_span = parser.tree.get_span(properties[1]);
            let second_comment = parser.comments()
                .iter()
                .copied()
                .find(|comment| parser.span_str(comment.span).contains("second-tail"))
                .expect("expected second-tail comment");

            assert!(
                second_property_span.end <= second_comment.span.start,
                "property span should stop before trailing comment: property={second_property_span:?} comment={:?}",
                second_comment.span,
            );
        });
    });
    assert_eq!(parser.comments().len(), 2);
    assert_comment!(parser, 0, CommentKind::Line, "first-tail");
    assert_comment!(parser, 1, CommentKind::SingleLineBlock, " second-tail");
}
