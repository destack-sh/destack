use destack_dir::{
    Argument, BinaryOperator, CommentKind, Declaration, Expression, FloatType, FunctionDeclaration,
    FunctionForm, GenericArgument, GenericParameter, IfCondition, IfForm, IntegerType, Key, Name,
    Parameter, Property, ScalarLiteral, TemplateLiteral, TokenType, TypeExpression, TypeLiteral,
};
use destack_source::LanguageType;

use crate::{
    TestParser, assert_comment, assert_expression_path, assert_node, assert_path, assert_string,
    block_expression_ids,
};

/// Parse integer literals in various formats.
#[test]
fn test_parse_integer_literal() {
    let mut test = TestParser::new("1 731 0x1234 2n");
    let mut parser = test.prepare();

    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Integer(1)
    );
    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Integer(731)
    );
    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Integer(0x1234)
    );
    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Bigint(2)
    );
}

/// Parse integer literals with uppercase radix prefixes.
#[test]
fn test_parse_integer_literal_uppercase_radix_prefixes() {
    let mut test = TestParser::new_with_language("0B101 0O77 0Xff", LanguageType::TypeScript);
    let mut parser = test.prepare();

    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Integer(5)
    );
    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Integer(63)
    );
    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Integer(255)
    );
}

#[test]
fn test_parse_array_literal_with_missing_close_bracket() {
    let mut test = TestParser::new("[first, second");
    let mut parser = test.prepare();
    let elements = parser.eat_array_literal().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(elements.len(), 2);

    assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*value), "first");
    });
    assert_node!(parser.tree, elements[1], Argument::Positional { value, .. } => {
        assert_expression_path!(parser, parser.tree.get(*value), "second");
    });
}

#[test]
fn test_parse_object_literal_with_missing_close_brace() {
    let mut test = TestParser::new("{ foo: 1");
    let mut parser = test.prepare();
    let properties = parser.eat_object_literal().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(properties.len(), 1);

    assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
        assert_string!(parser, *name, "foo");
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

/// Parse large integer literals without overflow errors.
#[test]
fn test_parse_integer_literal_saturating() {
    let mut test = TestParser::new("9999999999999999999999999 9999999999999999999999999n");
    let mut parser = test.prepare();

    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Integer(i64::MAX)
    );
    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Bigint(i64::MAX)
    );
}

/// Parse scientific notation and decimal floats.2
#[test]
fn test_parse_float_literal() {
    let mut test = TestParser::new("10e37 1.0");
    let mut parser = test.prepare();

    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Float(1.0e38)
    );
    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Float(1.0)
    );
}

/// Parse true and false literals.
#[test]
fn test_parse_boolean_literal() {
    let mut test = TestParser::new("true false");
    let mut parser = test.prepare();

    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Boolean(true)
    );
    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Boolean(false)
    );
}

/// Parse string literal.
#[test]
fn test_parse_string_literal() {
    let mut test = TestParser::new_with_language(r#""hello" 'hi there'"#, LanguageType::TypeScript);
    let mut parser = test.prepare();

    let literal = parser.eat_scalar_literal().unwrap();
    assert_string!(
        parser,
        match literal {
            ScalarLiteral::String(id) => id,
            other => panic!("expected string literal, got {other:?}"),
        },
        "hello"
    );

    let literal = parser.eat_scalar_literal().unwrap();
    assert_string!(
        parser,
        match literal {
            ScalarLiteral::String(id) => id,
            other => panic!("expected string literal, got {other:?}"),
        },
        "hi there"
    );
}

/// Parse Destack single quoted literals as characters.
#[test]
fn test_parse_single_quoted_character_literal() {
    let mut test = TestParser::new("'a'");
    let mut parser = test.prepare();

    let literal = parser.eat_scalar_literal().unwrap();

    assert_eq!(literal, ScalarLiteral::Character('a'));
}

/// Parse escaped Destack single quoted literals as characters.
#[test]
fn test_parse_escaped_single_quoted_character_literal() {
    let mut test = TestParser::new(r#"'\n' '\'' '\u{41}'"#);
    let mut parser = test.prepare();

    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Character('\n')
    );
    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Character('\'')
    );
    assert_eq!(
        parser.eat_scalar_literal().unwrap(),
        ScalarLiteral::Character('A')
    );
}

/// Parse a single quoted line separator string literal.
#[test]
fn test_parse_single_quoted_line_separator_as_string() {
    // source: ('\u{2028}')
    let mut test = TestParser::new_with_language("('\u{2028}')", LanguageType::TypeScript);
    let mut parser = test.prepare();

    parser.eat_token(TokenType::OpenParenthesis).unwrap();
    let literal = parser.eat_scalar_literal().unwrap();
    assert_string!(
        parser,
        match literal {
            ScalarLiteral::String(id) => id,
            other => panic!("expected string literal, got {other:?}"),
        },
        "\u{2028}"
    );
}

/// Parse a single quoted paragraph separator string literal.
#[test]
fn test_parse_single_quoted_paragraph_separator_as_string() {
    // source: ('\u{2029}')
    let mut test = TestParser::new_with_language("('\u{2029}')", LanguageType::TypeScript);
    let mut parser = test.prepare();

    parser.eat_token(TokenType::OpenParenthesis).unwrap();
    let literal = parser.eat_scalar_literal().unwrap();
    assert_string!(
        parser,
        match literal {
            ScalarLiteral::String(id) => id,
            other => panic!("expected string literal, got {other:?}"),
        },
        "\u{2029}"
    );
}

/// Parse a regex string literal.
#[test]
fn test_parse_regex_string_literal() {
    let mut test = TestParser::new("/abc/\n/abc/g");
    let mut parser = test.prepare();

    // /abc/
    let literal = parser.eat_regex_literal().unwrap();
    match literal {
        ScalarLiteral::RegexString { content, flags } => {
            assert_string!(parser, content, "abc");
            assert!(flags.is_none());
        }
        other => panic!("expected regex string literal, got {other:?}"),
    }

    // /abc/g
    let literal = parser.eat_regex_literal().unwrap();
    match literal {
        ScalarLiteral::RegexString { content, flags } => {
            assert_string!(parser, content, "abc");
            assert_string!(parser, flags.unwrap(), "g");
        }
        other => panic!("expected regex string literal, got {other:?}"),
    }
}

/// Reject unterminated regex literals.
#[test]
fn test_reject_unterminated_regex_literal() {
    // source: /42
    let mut test = TestParser::new("/42");
    let mut parser = test.prepare();

    let result = parser.eat_regex_literal();
    assert!(result.is_err());
}

/// Reject regex literals with raw line terminators.
#[test]
fn test_reject_regex_literal_with_line_terminator() {
    // source: /test
    // /
    let mut test = TestParser::new("/test\n/");
    let mut parser = test.prepare();

    let result = parser.eat_regex_literal();
    assert!(result.is_err());
}

/// Parse a template string literal.
#[test]
fn test_parse_template_literal() {
    let mut test = TestParser::new(
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
    let literal = parser.eat_template_literal().unwrap();
    match literal {
        TemplateLiteral::String { string: template } => {
            // hello
            assert_string!(parser, template, "hello");
        }
        other => panic!("unexpected {other:?}"),
    }

    // `hello ${name}`
    let literal = parser.eat_template_literal().unwrap();
    match literal {
        TemplateLiteral::InterpolatedString { strings, arguments } => {
            assert_eq!(arguments.len(), 1);
            assert_eq!(strings.len(), 2);
            // hello
            assert_string!(parser, strings[0], "hello ");
            // name
            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "name");
            });
            //
            assert_string!(parser, strings[1], "");
        }
        other => panic!("unexpected {other:?}"),
    }

    // `${stmt}`
    let literal = parser.eat_template_literal().unwrap();
    match literal {
        TemplateLiteral::InterpolatedString { strings, arguments } => {
            assert_eq!(strings.len(), 2);
            assert_eq!(arguments.len(), 1);
            // empty start & empty end
            assert_string!(parser, strings[0], "");
            assert_string!(parser, strings[1], "");
            // stmt
            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "stmt");
            });
        }
        other => panic!("unexpected {other:?}"),
    }

    // `${start}${middle}${end}`
    let literal = parser.eat_template_literal().unwrap();
    match literal {
        TemplateLiteral::InterpolatedString { strings, arguments } => {
            assert_eq!(strings.len(), 4);
            assert_eq!(arguments.len(), 3);
            // empty string before & after each argument
            assert_string!(parser, strings[0], "");
            assert_string!(parser, strings[1], "");
            assert_string!(parser, strings[2], "");
            assert_string!(parser, strings[3], "");
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
    let literal = parser.eat_template_literal().unwrap();
    match literal {
        TemplateLiteral::InterpolatedString { strings, arguments } => {
            assert_eq!(arguments.len(), 2);
            assert_eq!(strings.len(), 3);
            // SELECT * FROM users WHERE name =
            assert_string!(parser, strings[0], "SELECT * FROM users WHERE name = ");
            // name
            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "name");
            });
            // AND age >
            assert_string!(parser, strings[1], " AND age > ");
            // group.age()
            assert_node!(parser.tree, arguments[1], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "group.age");
                });
            });
            // LIMIT 10
            assert_string!(parser, strings[2], " LIMIT 10");
        }
        other => panic!("unexpected {other:?}"),
    }
}

/// Parse template interpolation with a TypeScript `as` cast.
#[test]
fn test_parse_template_literal_as_cast_expression() {
    let mut test = TestParser::new_with_language("`${type as string}`", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let literal = parser.eat_template_literal().unwrap();

    match literal {
        TemplateLiteral::InterpolatedString { strings, arguments } => {
            assert_eq!(strings.len(), 2);
            assert_eq!(arguments.len(), 1);
            assert_string!(parser, strings[0], "");
            assert_string!(parser, strings[1], "");

            assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::As { expression, target_type } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "type");
                    assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
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
    let mut test = TestParser::new(r"`\${${value}}`");
    let mut parser = test.prepare();
    let literal = parser.eat_template_literal().unwrap();

    match literal {
        TemplateLiteral::InterpolatedString { strings, arguments } => {
            assert_eq!(strings.len(), 2);
            assert_eq!(arguments.len(), 1);
            assert_string!(parser, strings[0], r"\${");
            assert_string!(parser, strings[1], "}");

            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "value");
            });
        }
        other => panic!("unexpected {other:?}"),
    }
}

/// Reject untagged template literals with legacy octal escapes.
#[test]
fn test_parse_template_literal_rejects_legacy_octal_escape() {
    let mut test = TestParser::new(r"`\1`");
    let mut parser = test.prepare();

    let result = parser.eat_template_literal();

    assert!(result.is_err());
}

#[test]
fn test_parse_type_literal() {
    let mut test = TestParser::new("int32 uint8 float boolean char symbol unique symbol");
    let mut parser = test.prepare();
    parser.flags.set_in_type(true);

    assert!(matches!(
        parser.eat_type_literal(None).unwrap(),
        TypeLiteral::Integer(IntegerType::Fixed {
            width: 32,
            is_signed: true
        })
    ));
    assert!(matches!(
        parser.eat_type_literal(None).unwrap(),
        TypeLiteral::Integer(IntegerType::Fixed {
            width: 8,
            is_signed: false
        })
    ));
    assert!(matches!(
        parser.eat_type_literal(None).unwrap(),
        TypeLiteral::Float(FloatType::Float)
    ));
    assert!(matches!(
        parser.eat_type_literal(None).unwrap(),
        TypeLiteral::Boolean
    ));
    assert!(matches!(
        parser.eat_type_literal(None).unwrap(),
        TypeLiteral::Character
    ));
    assert!(matches!(
        parser.eat_type_literal(None).unwrap(),
        TypeLiteral::Symbol
    ));
    assert!(matches!(
        parser.eat_type_literal(None).unwrap(),
        TypeLiteral::UniqueSymbol
    ));
}

#[test]
fn test_parse_array_literal() {
    let mut test = TestParser::new("[1, 2]");
    let mut parser = test.prepare();

    let elements = parser.eat_array_literal().unwrap();
    assert_eq!(elements.len(), 2);
    // 1
    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        }
    );
    // 2
    assert_node!(
        parser.tree,
        elements[1],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        }
    );
}

#[test]
fn test_parse_array_literal_disallows_sequence_elements() {
    let mut test = TestParser::new("[1, 2, 3]");
    let mut parser = test.prepare();

    let elements = parser.eat_array_literal().unwrap();
    assert_eq!(elements.len(), 3);

    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        }
    );

    assert_node!(
        parser.tree,
        elements[1],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        }
    );

    assert_node!(
        parser.tree,
        elements[2],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
        }
    );
}

#[test]
fn test_parse_sparse_array_middle_hole() {
    let mut test = TestParser::new("[1, , 3]");
    let mut parser = test.prepare();

    let elements = parser.eat_array_literal().unwrap();
    assert_eq!(elements.len(), 3);
    // 1
    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        }
    );
    // hole (stub)
    assert_node!(
        parser.tree,
        elements[1],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Stub);
        }
    );
    // 3
    assert_node!(
        parser.tree,
        elements[2],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
        }
    );
}

#[test]
fn test_parse_sparse_array_leading_hole() {
    let mut test = TestParser::new("[, 1]");
    let mut parser = test.prepare();

    let elements = parser.eat_array_literal().unwrap();
    assert_eq!(elements.len(), 2);
    // hole (stub)
    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Stub);
        }
    );
    // 1
    assert_node!(
        parser.tree,
        elements[1],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        }
    );
}

#[test]
fn test_parse_sparse_array_trailing_hole() {
    let mut test = TestParser::new("[1, ]");
    let mut parser = test.prepare();

    let elements = parser.eat_array_literal().unwrap();
    // trailing comma without hole is allowed
    assert_eq!(elements.len(), 1);
    // 1
    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        }
    );
}

#[test]
fn test_parse_array_literal_with_newline_prefixed_comma_separator() {
    let mut test = TestParser::new("[1\n, 2]");
    let mut parser = test.prepare();

    let elements = parser.eat_array_literal().unwrap();
    assert_eq!(elements.len(), 2);

    // 1
    assert_node!(
        parser.tree,
        elements[0],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        }
    );

    // 2
    assert_node!(
        parser.tree,
        elements[1],
        Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        }
    );
}

#[test]
fn test_parse_tree_fragment() {
    let mut test = TestParser::new("<A/>");
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    // <A/>
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        // A
        assert_expression_path!(parser, parser.tree.get(*left), "A");
        assert!(arguments.is_none());
        assert!(elements.is_none());
    });
}

/// Parse inline whitespace-only tree text as a meaningful child.
#[test]
fn test_parse_tree_inline_whitespace_text_child() {
    let mut test = TestParser::new_with_language("<Text> </Text>", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
        let elements = elements.as_ref().expect("expected child elements");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                assert_string!(parser, *string_id, " ");
            });
        });
    });
}

#[test]
fn test_parse_tree_fragment_with_kebab_tag() {
    let mut test = TestParser::new_with_language("<amp-something />", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    // <amp-something />
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "ampSomething");
        assert_eq!(parser.get_span_str(parser.tree.get_span(*left)), "amp-something");
        assert!(arguments.is_none());
        assert!(elements.is_none());
    });
}

#[test]
fn test_parse_tree_fragment_with_arguments() {
    // pure tree syntax: numeric values need {}, boolean flags are implicit true
    let mut test = TestParser::new("<A a={1} annoying-bee={2} c={3} flag />");
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "A");
        assert_eq!(arguments.as_ref().unwrap().len(), 4);
        // a={1}
        assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
        // annoying-bee={2}
        assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "annoyingBee");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });
        // c={3}
        assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "c");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
        });
        // flag (implicit true)
        assert_node!(parser.tree, arguments.as_ref().unwrap()[3], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "flag");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
        });

        assert!(elements.is_none());
    });
}

#[test]
fn test_parse_tree_fragment_with_arguments_and_child() {
    // pure tree syntax
    let mut test = TestParser::new(
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
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Tooltip");
        assert_eq!(arguments.as_ref().unwrap().len(), 3);
        // title={true}
        assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "title");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
        });
        // flag (implicit true)
        assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "flag");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
        });
        // something-else={false}
        assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "somethingElse");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
        });

        assert!(elements.is_some());
        // {true} child expression
        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
        });
    });
}

#[test]
fn test_parse_tree_nested_deep() {
    // pure tree syntax: children must be elements or {expression}
    let mut test = TestParser::new(
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
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "A");
        assert!(arguments.is_none());
        assert!(elements.is_some());
        // <B>
        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "B");
                assert!(arguments.is_none());
                assert!(elements.is_some());
                // <C>
                assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "C");
                        assert!(arguments.is_none());
                        assert!(elements.is_some());
                        // <D/>
                        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
                            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "D");
                                assert!(arguments.is_none());
                                assert!(elements.is_none());
                            });
                        });
                        // {2}
                        assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Positional { value } => {
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_tree_in_parenthesis() {
    let mut test = TestParser::new(
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
    let expression = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expression, Expression::Parenthesized { expression } => {
        // <div className="font-semibold">
        assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_some());
            assert_eq!(arguments.as_ref().unwrap().len(), 1);
            // className="font-semibold"
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
                // className
                assert_string!(parser, *name, "className");
                // font-semibold
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                    assert_string!(parser, *string_id, "font-semibold");
                });
            });

            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
            // <Link subtle to={1}>
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
                    // Link
                    assert_expression_path!(parser, parser.tree.get(*left), "Link");
                    assert!(arguments.is_some());
                    assert_eq!(arguments.as_ref().unwrap().len(), 2);
                    // subtle
                    assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
                        assert_string!(parser, *name, "subtle");
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
                    });
                    // to={1}: {} is the expression container, value is just 1
                    assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { name: Name::Identifier(name), value } => {
                        assert_string!(parser, *name, "to");
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                    });

                    assert!(elements.is_some());
                    assert_eq!(elements.as_ref().unwrap().len(), 1);
                    // {2}: {} is the expression container, value is just 2
                    assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                    });
                });
            });
        });
    });
}

/// Parse tree literal with generic arguments on the tag.
#[test]
fn test_parse_tree_with_generic_arguments() {
    let mut test = TestParser::new_with_language(
        r#"<Component<any>></Component>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), generic_arguments, arguments, elements, .. } => {
        assert_node!(parser.tree, *left, Expression::QualifiedReference { path, generic_arguments: left_generic_arguments } => {
            assert_path!(parser, *path, "Component");
            assert!(left_generic_arguments.is_empty());
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Any);
                    });
            });
        });
        assert!(arguments.is_none());
        assert!(elements.is_some());
        assert_eq!(elements.as_ref().unwrap().len(), 0);
    });
}

/// Parse generic arguments containing a shift-left-like generic arrow.
#[test]
fn test_parse_generic_arguments_with_shift_left_generic_arrow() {
    let mut test =
        TestParser::new_with_language(r#"<<T>(v: T) => void>"#, LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    // parse the generic arguments
    let generic_arguments = parser.eat_generic_arguments().unwrap();
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
            assert_node!(parser.tree, function.parameters[0], Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "v");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Void);
            });
        });
    });
}

/// Parse tree literal with shift-left-like generic arguments on the tag.
#[test]
fn test_parse_tree_with_shift_left_generic_arguments() {
    let mut test = TestParser::new_with_language(
        r#"<Component<<T>(v: T) => void> />"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();

    // parse the tree literal
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), generic_arguments, arguments, elements, .. } => {
        assert_node!(parser.tree, *left, Expression::QualifiedReference { path, generic_arguments: left_generic_arguments } => {
            assert_path!(parser, *path, "Component");
            assert!(left_generic_arguments.is_empty());
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Function(function) => {
                        assert_eq!(function.parameters.len(), 1);
                    });
            });
        });
        assert!(arguments.is_none());
        assert!(elements.is_none());
    });
}

/// Parse a tree literal with generic arguments and multiline attributes.
#[test]
fn test_parse_tree_with_generic_arguments_and_multiline_attributes() {
    let mut test = TestParser::new_with_language(
        r#"<Tags<ValueTagData>
  defaultValue={value}
/>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_tree_literal().unwrap();

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { left: Some(left), generic_arguments, arguments, elements, .. } => {
        assert_node!(parser.tree, *left, Expression::QualifiedReference { path, generic_arguments: left_generic_arguments } => {
            assert_path!(parser, *path, "Tags");
            assert!(left_generic_arguments.is_empty());
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, .. } => {
                        assert_path!(parser, *path, "ValueTagData");
                    });
            });
        });

        let arguments = arguments.as_ref().expect("expected attributes");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value, .. } => {
            assert_string!(parser, *name, "defaultValue");
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });

        assert!(elements.is_none());
    });
}

/// Parse tree attribute comments without expanding the tag name span.
#[test]
fn test_parse_tree_attribute_leading_comments_keep_tag_name_span() {
    let mut test = TestParser::new_with_language(
        r#"<Widget
  // props-leading
  {...props} // props-tail
  kind="primary"
  // extra-leading
  {...extra}
/>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_tree_literal().unwrap();
    parser.attach_comments();

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_eq!(parser.get_span_str(parser.tree.get_span(*left)), "Widget");

        let arguments = arguments.as_ref().expect("expected attributes");
        assert_eq!(arguments.len(), 3);
        assert_eq!(parser.get_span_str(parser.tree.get_span(arguments[0])), "{...props}");
        assert_eq!(parser.get_span_str(parser.tree.get_span(arguments[1])), "kind=\"primary\"");
        assert_eq!(parser.get_span_str(parser.tree.get_span(arguments[2])), "{...extra}");
        assert!(elements.is_none());

        let comments = parser.tree.comments();
        assert_eq!(comments.len(), 3);
        assert_eq!(parser.get_span_str(comments[0].span), "// props-leading");
        assert_eq!(comments[0].attached_to, parser.tree.get_span(arguments[0]).start);
        assert!(comments[0].is_leading());
        assert_eq!(parser.get_span_str(comments[1].span), "// props-tail");
        assert_eq!(comments[1].attached_to, 0);
        assert!(!comments[1].is_leading());
        assert_eq!(parser.get_span_str(comments[2].span), "// extra-leading");
        assert_eq!(comments[2].attached_to, parser.tree.get_span(arguments[2]).start);
        assert!(comments[2].is_leading());
    });
}

/// Reject ambiguous tree generic arrows without disambiguators.
#[test]
fn test_peek_tree_literal_ambiguous_tree_generic_arrow() {
    let mut test = TestParser::new_with_language("<T>(x: T) => x", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    // ambiguous tree generics are rejected without disambiguators
    assert!(!parser.can_start_generic_arrow_expression());
}

/// Reject tree literal parsing for disambiguated tree generic arrows.
#[test]
fn test_peek_tree_literal_disambiguated_tree_generic_arrow() {
    let mut test = TestParser::new_with_language("<T,>(x: T) => x", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    // disambiguators should allow generic arrow parsing
    assert!(parser.can_start_generic_arrow_expression());
    assert!(parser.peek_tree_literal().is_err());
}

/// Recognize tree generic arrows with extends disambiguators.
#[test]
fn test_peek_tree_generic_arrow_with_extends() {
    let mut test =
        TestParser::new_with_language("<T extends Foo>(x: T) => x", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    // extends should disambiguate
    assert!(parser.can_start_generic_arrow_expression());
}

/// Reject malformed tree generic arrows with an unterminated parameter list.
#[test]
fn test_peek_tree_generic_arrow_with_missing_parameter_close_parenthesis() {
    let mut test = TestParser::new_with_language("<T,>(x: T => x", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    // malformed generic arrows should not disambiguate as tree literals
    assert!(!parser.can_start_generic_arrow_expression());
}

/// Check generic arrow disambiguation in plain tree mode.
#[test]
fn test_peek_tree_generic_arrow_in_plain_tree_mode() {
    let mut test = TestParser::new_with_language("<div>() => {}", LanguageType::JavaScriptXml);
    let mut parser = test.prepare();

    // ambiguous tree heads can still look like generic arrows here
    assert!(parser.can_start_generic_arrow_expression());
    assert!(parser.peek_tree_literal().is_ok());
}

#[test]
fn test_parse_tree_with_text_content() {
    let mut test = TestParser::new(r#"<h4>Tool: {part.toolName}</h4>"#);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "h4");
        assert!(arguments.is_none());
        assert!(elements.is_some());
        assert_eq!(elements.as_ref().unwrap().len(), 2);
        // Tool:
        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(_)));
        });
        // {part.toolName}
        assert!(matches!(
            parser.tree.get(elements.as_ref().unwrap()[1]),
            Argument::Positional { .. }
        ));
    });
}

#[test]
fn test_parse_nested_tree_with_text_content() {
    // <div><h4>Tool: {x}</h4></div>
    let mut test = TestParser::new(r#"<div><h4>Tool: {x}</h4></div>"#);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(arguments.is_none());
        assert!(elements.is_some());
        assert_eq!(elements.as_ref().unwrap().len(), 1);
        // nested <h4>...</h4>
        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { .. });
        });
    });
}

#[test]
fn test_parse_tree_with_attribute_and_children() {
    // <div key={index}><h4>Tool: {x}</h4></div>
    let mut test = TestParser::new(r#"<div key={index}><h4>Tool: {x}</h4></div>"#);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(arguments.is_some());
        assert_eq!(arguments.as_ref().unwrap().len(), 1);
        assert!(elements.is_some());
        assert_eq!(elements.as_ref().unwrap().len(), 1);
    });
}

/// Tree fragment containing a callback that returns nested tree literals.
#[test]
fn test_parse_tree_fragment_with_nested_callback() {
    let mut test = TestParser::new(r#"<>{x.map(() => (<div><h4>T: {y}</h4></div>))}</>"#);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, arguments, elements, .. } => {
        assert!(arguments.is_none());
        assert!(elements.is_some());
        assert_eq!(elements.as_ref().unwrap().len(), 1);
    });
}

/// Tree literal with comment container syntax {/* */}.
/// The comment is filtered out, leaving an empty expression container.
#[test]
fn test_parse_tree_with_comment_container() {
    let mut test = TestParser::new(r#"<div>{/* comment */}</div>"#);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(_), arguments, elements, .. } => {
        assert!(arguments.is_none());
        assert!(elements.is_some());
    });
}

/// Parse a tree fragment with keyword text followed by an expression container.
#[test]
fn test_parse_tree_fragment_with_keyword_text_and_expression() {
    let mut test = TestParser::new_with_language("<>for {x}</>", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { left, elements, .. } => {
        assert!(left.is_none());
        let elements = elements.as_ref().expect("expected fragment children");
        assert_eq!(elements.len(), 2);

        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                assert_string!(parser, *string_id, "for ");
            });
        });

        assert_node!(parser.tree, elements[1], Argument::Positional { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "x");
        });
    });
}

/// Parse tree fragment with comments between the angle brackets.
#[test]
fn test_parse_tree_fragment_with_comments() {
    let mut test = TestParser::new_with_language(
        "<\n// comment\n/* comment */\n>\n</>",
        LanguageType::JavaScriptXml,
    );
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, arguments, elements, .. } => {
        assert!(arguments.is_none());
        assert!(elements.is_some());
        assert!(elements.as_ref().unwrap().is_empty());
    });
}

/// Parse tree fragment with a closing tag that has trivia before the slash.
#[test]
fn test_parse_tree_fragment_closing_with_trivia() {
    let mut test =
        TestParser::new_with_language("<>\n< /* comment */ / >", LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, arguments, elements, .. } => {
        assert!(arguments.is_none());
        assert!(elements.is_some());
        assert!(elements.as_ref().unwrap().is_empty());
    });
}

/// Parse tree literal with namespace tag and attribute.
#[test]
fn test_parse_tree_with_namespace_tag() {
    let mut test =
        TestParser::new_with_language(r#"<Foo:Bar n:foo="bar" />"#, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Foo:Bar");
        let arguments = arguments.as_ref().expect("expected arguments");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "n:foo");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(_)));
        });
        assert!(elements.is_none());
    });
}

/// Ternary with tree literal containing && inside expression container.
/// Regression test for nested tree expression precedence.
#[test]
fn test_parse_ternary_with_and_in_tree() {
    // Just the ternary part, without leading condition
    let mut test = TestParser::new(r#"a ? <>{y && <E />}</> : null"#);
    let mut parser = test.prepare();
    let expr = parser.eat_expression(parser.flags).unwrap();
    // a ? ... : null -> If with IfForm::Ternary
    assert_node!(parser.tree, expr, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        // condition: a
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        assert_node!(parser.tree, condition_id, Expression::Identifier { name } => {
            assert_string!(parser, *name, "a");
        });
        // consequence: <>{y && <E />}</>
        assert_node!(parser.tree, *then_expression, Expression::TreeExpression { left: None, arguments, elements, .. } => {
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
            // {y && <E />} - the && expression
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::Binary { .. });
            });
        });
        // alternative: null
        assert!(else_expression.is_some());
        assert_node!(parser.tree, else_expression.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Null));
    });
}

/// Nested tree literal in attribute expression container.
/// Regression test for from_content fix in TreeExpressionEntry.
#[test]
fn test_parse_nested_tree_in_attribute() {
    let mut test = TestParser::new(r#"<Button icon={<Icon />} />"#);
    let mut parser = test.prepare();
    let expr = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Button");
        assert!(arguments.is_some());
        assert_eq!(arguments.as_ref().unwrap().len(), 1);
        // icon={<Icon />}
        assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "icon");
            // value is <Icon />
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(inner_left), arguments: inner_args, elements: inner_elems, .. } => {
                assert_expression_path!(parser, parser.tree.get(*inner_left), "Icon");
                assert!(inner_args.is_none());
                assert!(inner_elems.is_none());
            });
        });
        assert!(elements.is_none());
    });
}

/// Parse multiline tree attribute expression containers before a tag close.
#[test]
fn test_parse_multiline_tree_attribute_expression_before_tag_close() {
    let mut test = TestParser::new_with_language(
        r#"<PopoverProvider
  popover={
    <TooltipContent>
      <Picker />
    </TooltipContent>
  }
>
  <PopoverTrigger />
</PopoverProvider>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "PopoverProvider");

        let arguments = arguments.as_ref().expect("expected arguments");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "popover");
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), elements, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "TooltipContent");
                let elements = elements.as_ref().expect("expected tooltip children");
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "Picker");
                        assert!(arguments.is_none());
                        assert!(elements.is_none());
                    });
                });
            });
        });

        let elements = elements.as_ref().expect("expected provider children");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "PopoverTrigger");
                assert!(arguments.is_none());
                assert!(elements.is_none());
            });
        });
    });
}

#[test]
fn test_parse_tree_attribute_tree_with_nested_map_before_tag_close() {
    let mut test = TestParser::new_with_language(
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
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "PopoverProvider");

        let arguments = arguments.as_ref().expect("expected provider arguments");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value, .. } => {
            assert_string!(parser, *name, "popover");
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "TooltipContent");
            });
        });

        let elements = elements.as_ref().expect("expected provider children");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "PopoverTrigger");
                let arguments = arguments.as_ref().expect("expected trigger arguments");
                let has_style_argument = arguments.iter().any(|argument| {
                    matches!(
                        parser.tree.get(*argument),
                        Argument::Named { name: Name::Identifier(name), value, .. }
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
    let mut test = TestParser::new_with_language(
        r#"<Show when={shouldShow()} fallback={<>off</>}><>{props.children}</></Show>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Show");
        let arguments = arguments.as_ref().expect("expected arguments");
        assert!(arguments.len() >= 2);
        assert_node!(parser.tree, arguments[1], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "fallback");
            assert_node!(parser.tree, *value, Expression::TreeExpression { left, elements, .. } => {
                assert!(left.is_none());
                let elements = elements.as_ref().expect("expected fragment elements");
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "off");
                    });
                });
            });
        });
    });
}

/// Parse a named tree literal with text content inside an attribute expression container.
#[test]
fn test_parse_tree_named_text_in_attribute_expression() {
    let mut test = TestParser::new_with_language(
        r#"<ParentComponent prop={
  <Child>
    test
  </Child>
}/>;"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "ParentComponent");
        let arguments = arguments.as_ref().expect("expected arguments");
        assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value, .. } => {
            assert_string!(parser, *name, "prop");
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(inner_left), elements, .. } => {
                assert_expression_path!(parser, parser.tree.get(*inner_left), "Child");
                let elements = elements.as_ref().expect("expected child elements");
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        let text = parser.strings.get(*string_id);
                        assert_eq!(text.trim(), "test");
                    });
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
    let mut test = TestParser::new_with_language(input, LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expression_id = parser.eat_tree_literal().unwrap();

    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    assert_node!(parser.tree, expression_id, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "F");
        assert!(elements.is_none());

        let arguments = arguments.as_ref().expect("expected tree arguments");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value, .. } => {
            assert_string!(parser, *name, "values");

            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(key_name)), value: callback, .. } => {
                    assert_string!(parser, *key_name, "resend");
                    assert_node!(parser.tree, *callback, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                            assert_eq!(signature.form, FunctionForm::Lambda);
                            assert_eq!(signature.parameters.len(), 1);
                            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "chunks");
                            });

                            assert_node!(parser.tree, body.expect("expected callback body"), Expression::Parenthesized { expression } => {
                                assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(button_left), elements, .. } => {
                                    assert_expression_path!(parser, parser.tree.get(*button_left), "button");
                                    let elements = elements.as_ref().expect("expected button children");
                                    assert_eq!(elements.len(), 1);
                                    assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
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
    let mut test = TestParser::new_with_language(
        r#"<Tag
  {
    // comment before spread
    ...(rootProps as any)
  }
/>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Tag");
        let arguments = arguments.as_ref().expect("expected arguments");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Spread { label: None, value } => {
            assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::As { .. } => {
                });
            });
        });
        assert!(elements.is_none());
    });
}

#[test]
fn test_parse_tree_attribute_spread_with_cast() {
    let mut test = TestParser::new_with_language(
        r#"<WrappedComponent {...(this.props as P & DependentProps)} {...this.state} />"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expr = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "WrappedComponent");
        let arguments = arguments.as_ref().expect("expected arguments");
        assert_eq!(arguments.len(), 2);
        // {...(this.props as P & DependentProps)}
        assert_node!(parser.tree, arguments[0], Argument::Spread { label: None, value } => {
            assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::As { .. } => {
                });
            });
        });
        // {...this.state}
        assert_node!(parser.tree, arguments[1], Argument::Spread { label: None, value } => {
            assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                assert_node!(parser.tree, *left, Expression::This);
                assert_string!(parser, *name, "state");
            });
        });
        assert!(elements.is_none());
    });
}

/// Deeply nested tree literals in attributes.
/// Regression test for from_content fix with multiple nesting levels.
#[test]
fn test_parse_deeply_nested_tree_in_attr() {
    let mut test = TestParser::new(r#"<Outer title={<div><Button icon={<Icon />} /></div>} />"#);
    let mut parser = test.prepare();
    let expr = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Outer");
        assert!(arguments.is_some());
        assert_eq!(arguments.as_ref().unwrap().len(), 1);
        // title={<div>...</div>}
        assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "title");
            // <div><Button icon={<Icon />} /></div>
            assert_node!(parser.tree, *value, Expression::TreeExpression { elements: div_elems, .. } => {
                assert!(div_elems.is_some());
                assert_eq!(div_elems.as_ref().unwrap().len(), 1);
            });
        });
        assert!(elements.is_none());
    });
}

/// Object literal inside attribute expression container.
#[test]
fn test_parse_tree_attr_object_literal() {
    let mut test = TestParser::new(r#"<Rive style={{width: 400, height: 400}} />"#);
    let mut parser = test.prepare();
    let expr = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Rive");
        assert!(arguments.is_some());
        assert_eq!(arguments.as_ref().unwrap().len(), 1);
        // style={{width: 400, height: 400}}
        assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "style");
            // {width: 400, height: 400}
            assert_node!(parser.tree, *value, Expression::ObjectExpression { .. });
        });
        assert!(elements.is_none());
    });
}

/// Multiline tree literal with expression container and sibling elements.
#[test]
fn test_parse_multiline_tree_with_siblings() {
    let code = "<div>\n\t{x}\n\t<form onClick={() => {}}></form>\n</div>";
    let mut test = TestParser::new(code);
    let mut parser = test.prepare();
    let expr = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(arguments.is_none());
        assert!(elements.is_some());
        assert_eq!(elements.as_ref().unwrap().len(), 2);
        // {x}
        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Identifier { name } => {
                assert_string!(parser, *name, "x");
            });
        });
        // <form onClick={() => {}}></form>
        assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(form_left), arguments: form_args, .. } => {
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
    let mut test = TestParser::new(r#"<div>{x && <span/>}</div>"#);
    let mut parser = test.prepare();
    let expr = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(arguments.is_none());
        assert!(elements.is_some());
        assert_eq!(elements.as_ref().unwrap().len(), 1);
        // {x && <span/>}
        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
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
    let mut test = TestParser::new(r#"<div>{a < b}</div>"#);
    let mut parser = test.prepare();
    let expr = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(arguments.is_none());
        assert!(elements.is_some());
        assert_eq!(elements.as_ref().unwrap().len(), 1);
        // {a < b}
        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
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
    let mut test =
        TestParser::new_with_language(r#"<div>{foo<T>(x)}</div>"#, LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expr = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        assert!(arguments.is_none());
        assert!(elements.is_some());
        assert_eq!(elements.as_ref().unwrap().len(), 1);
        // {foo<T>(x)}
        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
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
    let mut test = TestParser::new_with_language(
        r#"<div>{errors.Checkbox && <p id="Checkbox">Checkbox Error</p>}</div>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        let elements = elements.as_ref().expect("expected div children");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::And);
                assert_node!(parser.tree, *right, Expression::TreeExpression { left: Some(right_left), arguments: Some(arguments), elements: Some(right_elements), .. } => {
                    assert_expression_path!(parser, parser.tree.get(*right_left), "p");
                    assert_eq!(arguments.len(), 1);
                    assert_eq!(right_elements.len(), 1);
                    assert_node!(parser.tree, right_elements[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(value)) => {
                            assert_string!(parser, *value, "Checkbox Error");
                        });
                    });
                });
            });
        });
    });
}

/// Tree literal should parse after a closing class block on a new line.
#[test]
fn test_parse_tree_after_class_block_newline() {
    let mut test =
        TestParser::new_with_language("class C extends D<T> {}\n<C/>", LanguageType::TypeScriptXml);
    let mut parser = test.prepare();

    // class declaration
    let class_expr = parser.try_eat_statement_expression().unwrap();
    assert_node!(parser.tree, class_expr, Expression::Declaration(_));

    // tree literal expression
    let tree_expr = parser.try_eat_statement_expression().unwrap();
    assert_node!(parser.tree, tree_expr, Expression::TreeExpression { .. });
}

/// Valid template literal with interpolation should parse correctly.
#[test]
fn test_parse_template_literal_valid() {
    let mut test = TestParser::new("`hello ${name}!`");
    let mut parser = test.prepare();
    let result = parser.eat_template_literal();
    assert!(result.is_ok());
}

/// Valid template literal without interpolation.
#[test]
fn test_parse_template_literal_plain() {
    let mut test = TestParser::new("`hello world`");
    let mut parser = test.prepare();
    let result = parser.eat_template_literal();
    assert!(result.is_ok());
}

/// Empty template literal.
#[test]
fn test_parse_template_literal_empty() {
    let mut test = TestParser::new("``");
    let mut parser = test.prepare();
    let result = parser.eat_template_literal();
    assert!(result.is_ok());
}

/// Template literal with only interpolation `${foo}`.
#[test]
fn test_parse_template_literal_only_interpolation() {
    let mut test = TestParser::new("`${foo}`");
    let mut parser = test.prepare();
    let result = parser.eat_template_literal();
    assert!(result.is_ok());
}

/// Template literal with adjacent interpolations.
#[test]
fn test_parse_template_literal_adjacent_interpolations() {
    let mut test = TestParser::new("`${a}${b}${c}`");
    let mut parser = test.prepare();
    let result = parser.eat_template_literal();
    assert!(result.is_ok());
}

/// Template literal interpolation should allow optional chaining.
#[test]
fn test_parse_template_literal_optional_chain() {
    let mut test = TestParser::new(r#"`value ${theme?.activeColor}`"#);
    let mut parser = test.prepare();
    let result = parser.eat_template_literal();
    assert!(result.is_ok());
}

/// Template literal interpolation should allow ternary expressions.
#[test]
fn test_parse_template_literal_ternary() {
    let mut test = TestParser::new(r#"`value ${mode === "dark" ? "dark" : "light"}`"#);
    let mut parser = test.prepare();
    let result = parser.eat_template_literal();
    assert!(result.is_ok());
}

/// Parse tree text that includes `=` after opening tags.
#[test]
fn test_parse_tree_text_with_equals_after_tag_with_attribute_no_space() {
    let mut test = TestParser::new_with_language(
        r#"<div className={styles.foo}>=</div>"#,
        LanguageType::JavaScriptXml,
    );
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
        let elements = elements.as_ref().expect("expected elements");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                assert_string!(parser, *string_id, "=");
            });
        });
    });
}

/// Parse tree text that includes `=` after opening tags.
#[test]
fn test_parse_tree_text_with_equals_after_tag_with_attribute_with_space() {
    let mut test = TestParser::new_with_language(
        r#"<div className={styles.foo} >=</div>"#,
        LanguageType::JavaScriptXml,
    );
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
        let elements = elements.as_ref().expect("expected elements");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                assert_string!(parser, *string_id, "=");
            });
        });
    });
}

/// Parse tree text that includes `=` after opening tags.
#[test]
fn test_parse_tree_text_with_equals_after_simple_tag_no_space() {
    let mut test = TestParser::new_with_language(r#"<div>=</div>"#, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
        let elements = elements.as_ref().expect("expected elements");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                assert_string!(parser, *string_id, "=");
            });
        });
    });
}

/// Parse tree fragments containing text after opening tags.
#[test]
fn test_parse_tree_fragment_text_with_equals_prefix() {
    let mut test = TestParser::new_with_language(r#"<>=x</>"#, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
        let elements = elements.as_ref().expect("expected elements");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                assert_string!(parser, *string_id, "=x");
            });
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
    let mut test = TestParser::new_with_language(input, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    test.assert_no_errors(&parser);

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::TreeExpression { left, elements, .. } => {
        assert!(left.is_none());
        let elements = elements.as_ref().expect("expected elements");
        assert_eq!(elements.len(), 1);

        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left, elements, .. } => {
                assert!(left.is_none());
                let elements = elements.as_ref().expect("expected nested elements");
                assert_eq!(elements.len(), 1);

                assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "=x");
                    });
                });
            });
        });
    });
}

/// Parse tree fragments followed by `>=1` as binary expressions.
#[test]
fn test_parse_tree_fragment_followed_by_greater_than_or_equal() {
    let mut test = TestParser::new_with_language(r#"<>x</>>=1"#, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_expression(parser.flags).unwrap();
    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
        assert_node!(parser.tree, *left, Expression::TreeExpression { .. });
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
    });
}

/// Parse tree elements followed by `>=1` as binary expressions.
#[test]
fn test_parse_tree_element_followed_by_greater_than_or_equal() {
    let mut test =
        TestParser::new_with_language(r#"<span>x</span>>=1"#, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_expression(parser.flags).unwrap();
    test.assert_no_errors(&parser);

    assert_node!(parser.tree, expression, Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
        assert_node!(parser.tree, *left, Expression::TreeExpression { .. });
        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
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
    let mut test = TestParser::new_with_language(input, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expressions = parser.parse();
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

/// Parse tree fragments with text between child elements in arrays.
#[test]
fn test_parse_tree_fragment_equals_in_array() {
    let input = r#"<Y
    elements={[
        <>
            <span>x</span>=
            <br />
        </>,
        true
    ]}
/>
"#;
    let mut test = TestParser::new_with_language(input, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    // ensure the array includes a fragment and a boolean
    assert_node!(parser.tree, expression, Expression::TreeExpression { arguments, .. } => {
        let arguments = arguments.as_ref().expect("expected arguments");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "elements");
            assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::TreeExpression { left: None, .. });
                });
            });
        });
    });
}

/// Parse ternary expressions that return tree elements inside expression containers.
#[test]
fn test_parse_tree_ternary_expression_container_in_xml_mode() {
    let input = r#"<div>{isLoading ? <div>loading</div> : <div>done</div>}</div>"#;
    let mut test = TestParser::new_with_language(input, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    // verify the ternary expression container
    assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
        let elements = elements.as_ref().expect("expected elements");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::If { form, .. } => {
                assert_eq!(*form, IfForm::Ternary);
            });
        });
    });
}

/// Parse ternary expressions that return tree elements inside expression containers.
#[test]
fn test_parse_tree_ternary_expression_container() {
    let input = r#"<div>{isLoading ? <div>loading</div> : <div>done</div>}</div>"#;
    let mut test = TestParser::new_with_language(input, LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    // verify the ternary expression container
    assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
        let elements = elements.as_ref().expect("expected elements");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
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

    let mut test = TestParser::new_with_language(input, LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");

        let elements = elements.as_ref().expect("expected elements");
        assert_eq!(elements.len(), 2);

        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Call { arguments, .. } => {
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                            assert_eq!(signature.form, FunctionForm::Lambda);

                            let body = body.expect("expected lambda body");
                            assert_node!(parser.tree, body, Expression::Parenthesized { expression } => {
                                assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
                                    assert_expression_path!(parser, parser.tree.get(*left), "option");
                                    assert!(arguments.is_none());

                                    let elements = elements.as_ref().expect("expected option children");
                                    assert_eq!(elements.len(), 1);
                                    assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                                        assert_expression_path!(parser, parser.tree.get(*value), "item");
                                    });
                                });
                            });
                        });
                    });
                });
            });
        });

        assert_node!(parser.tree, elements[1], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "button");
                assert!(arguments.is_none());
                assert!(elements.is_none());
            });
        });
    });
}

/// Parse ternary fragments with text fallback.
#[test]
fn test_parse_tree_ternary_fragment_with_text_fallback() {
    let input = "shouldShow ? <>{children}</> : <>off</>";
    let mut test = TestParser::new_with_language(input, LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expression, Expression::If { form, condition, then_expression, else_expression } => {
        assert_eq!(*form, IfForm::Ternary);
        assert_node!(condition, IfCondition::Expression { condition } => {
            assert_expression_path!(parser, parser.tree.get(*condition), "shouldShow");
        });

        assert_node!(parser.tree, *then_expression, Expression::TreeExpression { left, elements, .. } => {
            assert!(left.is_none());
            let elements = elements.as_ref().expect("expected then elements");
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "children");
            });
        });

        let else_expression = else_expression.expect("expected else expression");
        assert_node!(parser.tree, else_expression, Expression::TreeExpression { left, elements, .. } => {
            assert!(left.is_none());
            let elements = elements.as_ref().expect("expected else elements");
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                    assert_string!(parser, *string_id, "off");
                });
            });
        });
    });
}

/// Parse tree fragment text nodes with standalone colon content.
#[test]
fn test_parse_tree_fragment_with_colon_text_node() {
    let input = r#"<code>{value && <>:</>}</code>"#;
    let mut test = TestParser::new_with_language(input, LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    // verify logical-and fragment text parsing
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "code");
        let elements = elements.as_ref().expect("expected code children");
        assert_eq!(elements.len(), 1);

        // verify the right side of `value && ...`
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::And);
                assert_node!(parser.tree, *right, Expression::TreeExpression { left: None, elements, .. } => {
                    let elements = elements.as_ref().expect("expected fragment children");
                    assert_eq!(elements.len(), 1);
                    assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                            assert_string!(parser, *string_id, ":");
                        });
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
    let mut test = TestParser::new_with_language(input, LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "div");
        let elements = elements.as_ref().expect("expected div children");
        assert_eq!(elements.len(), 1);

        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::And);
                assert_node!(parser.tree, *right, Expression::TreeExpression { left: None, elements, .. } => {
                    let elements = elements.as_ref().expect("expected fragment children");
                    assert_eq!(elements.len(), 1);
                    assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
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
    let mut test = TestParser::new_with_language(input, LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expression = parser.eat_tree_literal().unwrap();

    // strong element with one expression child
    assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "strong");
        let elements = elements.as_ref().expect("expected strong children");
        assert_eq!(elements.len(), 1);

        // componentNameJsx && <>{...}</>
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::And);
                assert_expression_path!(parser, parser.tree.get(*left), "componentNameJsx");

                // fragment children: "for " and componentNameJsx
                assert_node!(parser.tree, *right, Expression::TreeExpression { left: None, elements, .. } => {
                    let elements = elements.as_ref().expect("expected fragment children");
                    assert_eq!(elements.len(), 2);
                    assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                            assert_string!(parser, *string_id, "for ");
                        });
                    });
                    assert_node!(parser.tree, elements[1], Argument::Positional { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "componentNameJsx");
                    });
                });
            });
        });
    });
}

/// Reject tree literal namespace and member combinations during parse.
#[test]
fn test_reject_tree_literal_namespace_member_path_parse_error() {
    let mut test = TestParser::new_with_language("<a.b:c />", LanguageType::JavaScriptXml);
    let mut parser = test.prepare();

    let error = parser
        .eat_tree_literal()
        .expect_err("expected parse failure for namespace member path");
    assert_eq!(parser.get_span_str(error.leaf_span()), "/");
}

/// Parse tree elements after newline-terminated declarations.
#[test]
fn test_parse_tree_after_let_newline_inside_function() {
    let input = r#"
function x() {
    let x
    <div />
}
"#;
    let mut test = TestParser::new_with_language(input, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expressions = parser.parse();
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
    let mut test = TestParser::new_with_language(input, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    test.assert_no_errors(&parser);

    // ensure fragments after classes parse with multiple children
    assert_eq!(expressions.len(), 2);
    let tree_expression = expressions[1];
    assert_node!(parser.tree, tree_expression, Expression::TreeExpression { left, elements, .. } => {
        assert!(left.is_none());
        let elements = elements.as_ref().expect("expected elements");
        assert_eq!(elements.len(), 2);
    });
}

/// Parse tree elements after newline-terminated declarations and blocks.
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

{ foo: 'test' }
<Comp></Comp>

function test1() {}
<Comp></Comp>

class Foo {}
<>
<Comp></Comp>
<Comp></Comp>
</>
"#;
    let mut test = TestParser::new_with_language(input, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expressions = parser.parse();
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
    assert_node!(parser.tree, expressions[7], Expression::Block(_));
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
    assert_node!(parser.tree, expressions[12], Expression::TreeExpression { elements, .. } => {
        let elements = elements.as_ref().expect("expected fragment elements");
        assert_eq!(elements.len(), 2);
    });
}

/// Parse tree elements after return with newline termination.
#[test]
fn test_parse_tree_after_return_newline() {
    let input = r#"
function test() {
    return
    <Comp />
}
"#;
    let mut test = TestParser::new_with_language(input, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();
    let expressions = parser.parse();

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
    let mut test = TestParser::new_with_language(input, LanguageType::TypeScriptXml);
    let mut parser = test.prepare();
    let expressions = parser.parse();

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
                    assert_node!(parser.tree, value, Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "Box");
                            let elements = elements.as_ref().expect("expected box children");
                            assert_eq!(elements.len(), 1);
                        });
                    });
                });
            });
        });
    });
}

/// Reject tree-looking input when it continues an expression across a newline.
#[test]
fn test_parse_tree_after_expression_newline_is_error() {
    let input = "x\n<Comp />";
    let mut test = TestParser::new_with_language(input, LanguageType::JavaScriptXml);
    let mut parser = test.prepare();

    // reject a tree literal after expression newline
    let result = parser.eat_expression(parser.flags);
    assert!(result.is_err());
}

#[test]
fn test_parse_object_property_trailing_comments_on_property_owners() {
    let mut test = TestParser::new(
        r#"const config = {
  first: 1, // first-tail
  second: 2 /* second-tail */
}"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let statement_id = parser.unwrap_label_expression(expressions[0]);
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
            let second_comment = parser
                .tree
                .comments()
                .iter()
                .copied()
                .find(|comment| parser.get_span_str(comment.span).contains("second-tail"))
                .expect("expected second-tail comment");

            assert!(
                second_property_span.end <= second_comment.span.start,
                "property span should stop before trailing comment: property={second_property_span:?} comment={:?}",
                second_comment.span,
            );
        });
    });
    assert_eq!(parser.tree.comments().len(), 2);
    assert_comment!(parser, 0, CommentKind::Line, "first-tail");
    assert_comment!(parser, 1, CommentKind::SingleLineBlock, " second-tail");
}
