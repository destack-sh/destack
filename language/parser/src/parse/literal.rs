use std::borrow::Cow;

use crate::lex::decode_html_entity;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

use dyst_ast::{
    Argument, Expression, LiteralType, NodeId, NodeType, NumberBase, Path, Property, ScalarLiteral,
    StringId, TemplateLiteral, TokenSpan, TokenType,
};

impl<'a> Parser<'a> {
    /// Peek a scalar literal token.
    #[inline]
    pub fn peek_scalar_literal(&self) -> ParseResult<&TokenSpan> {
        if self.peek_token(TokenType::Literal).is_ok() {
            Ok(self.peek()?)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a scalar literal and return its value.
    ///
    /// Examples:
    /// ```
    /// true
    /// false
    /// 1
    /// 1n
    /// 1.0
    /// 0x1234
    /// "Hello, world!"
    /// 'a'
    /// b'a'
    /// b"abc
    /// /abc/
    /// /abc/g
    /// ```
    pub fn eat_scalar_literal(&mut self) -> ParseResult<ScalarLiteral> {
        let literal_span = *self.eat()?;
        let Some(body) = literal_span.token.literal else {
            return Err(ParseError::unexpected(literal_span.span));
        };
        let literal_str = self.get_span_str(literal_span.span);

        match body {
            // boolean literal
            LiteralType::Boolean { value } => Ok(ScalarLiteral::Boolean(value)),

            // int literal
            LiteralType::Int {
                base,
                is_empty,
                is_bigint,
            } => {
                if is_empty {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // strip underscores for parsing
                let content: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                // strip bigint suffix
                let content = if is_bigint {
                    Cow::Borrowed(content.trim_end_matches("n"))
                } else {
                    content
                };

                // handle base-specific prefixes
                let parsed = match base {
                    NumberBase::Decimal => content.parse::<i64>(),
                    NumberBase::Binary => i64::from_str_radix(content.trim_start_matches("0b"), 2),
                    NumberBase::Octal => i64::from_str_radix(content.trim_start_matches("0o"), 8),
                    NumberBase::Hexadecimal => {
                        i64::from_str_radix(content.trim_start_matches("0x"), 16)
                    }
                };

                // int
                if is_bigint {
                    parsed.map(ScalarLiteral::Bigint).map_err(|_| {
                        ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
                } else {
                    parsed.map(ScalarLiteral::Integer).map_err(|_| {
                        ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
                }
            }

            // float literal
            LiteralType::Float {
                base: _,
                is_empty_exponent,
            } => {
                if is_empty_exponent {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                let content: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                content
                    .parse::<f64>()
                    .map(ScalarLiteral::Float)
                    .map_err(|_| {
                        ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
            }

            // character literal (ignore quotes)
            LiteralType::Character {
                is_terminated,
                is_html_entity,
            } => {
                if is_html_entity {
                    decode_html_entity(literal_str)
                        .map(ScalarLiteral::Character)
                        .ok_or_else(|| {
                            ParseError::expected_for(
                                literal_span.span,
                                TokenType::Literal,
                                NodeType::Expression,
                            )
                        })
                } else {
                    if !is_terminated {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }
                    let content = literal_str.trim_start_matches('\'').trim_end_matches('\'');
                    content
                        .chars()
                        .next()
                        .map(ScalarLiteral::Character)
                        .ok_or_else(|| {
                            ParseError::expected_for(
                                literal_span.span,
                                TokenType::Literal,
                                NodeType::Expression,
                            )
                        })
                }
            }

            // byte character literal (ignore quotes)
            LiteralType::Byte { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = literal_str.trim_start_matches("b'").trim_end_matches('\'');
                content
                    .chars()
                    .next()
                    .map(|ch| ScalarLiteral::Byte(ch as u8))
                    .ok_or_else(|| {
                        ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
            }

            // string literal (ignore quotes)
            // supports both '...' and "..." delimited string literals
            LiteralType::String { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = {
                    // "..." or '...'
                    if literal_str.len() >= 2
                        && ((literal_str.starts_with('"') && literal_str.ends_with('"'))
                            || (literal_str.starts_with('\'') && literal_str.ends_with('\'')))
                    {
                        &literal_str[1..literal_str.len() - 1]
                    }
                    // unexpected (error elsewhere)
                    else {
                        literal_str
                    }
                };
                let string_id = self.strings.intern(content);
                Ok(ScalarLiteral::String(string_id))
            }

            // regex string literal (ignore quotes)
            LiteralType::RegexString { has_flags } => {
                // regex without flags
                if !has_flags {
                    let content = literal_str.trim_start_matches("/").trim_end_matches("/");
                    let string_id = self.strings.intern(content);
                    Ok(ScalarLiteral::RegexString {
                        content: string_id,
                        flags: None,
                    })
                }
                // regex with flags
                else {
                    let last_slash_index = literal_str.rfind('/').unwrap();
                    let content = &literal_str[1..last_slash_index];
                    let flags = &literal_str[last_slash_index + 1..];
                    let string_id = self.strings.intern(content);
                    let flags_id = self.strings.intern(flags);
                    Ok(ScalarLiteral::RegexString {
                        content: string_id,
                        flags: Some(flags_id),
                    })
                }
            }

            // raw string literal (ignore quotes and hashes)
            LiteralType::RawString { hashes } => {
                let Some(hashes) = hashes else {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                };
                let num_hashes = hashes as usize;
                let prefix_len = 1 /* r */ + num_hashes + 1 /* opening " */;
                let suffix_len = 1 /* closing " */ + num_hashes;
                if literal_str.len() < prefix_len + suffix_len {
                    return Err(ParseError::expected(literal_span.span, TokenType::Literal));
                }
                let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                let string_id = self.strings.intern(content);
                Ok(ScalarLiteral::String(string_id))
            }

            // byte string literal (ignore quotes)
            LiteralType::ByteString { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = literal_str.trim_start_matches("b\"").trim_end_matches('"');
                Ok(ScalarLiteral::ByteString(content.as_bytes().to_vec()))
            }

            // raw byte string literal (ignore quotes and hashes)
            LiteralType::RawByteString { hashes } => {
                let Some(hashes) = hashes else {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                };
                let num_hashes = hashes as usize;
                let prefix_len = 2 /* br */ + num_hashes + 1 /* opening " */;
                let suffix_len = 1 /* closing " */ + num_hashes;
                if literal_str.len() < prefix_len + suffix_len {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                Ok(ScalarLiteral::ByteString(content.as_bytes().to_vec()))
            }
        }
    }

    /// Peek a template literal.
    #[inline]
    pub fn peek_template_literal(&self) -> ParseResult<&TokenSpan> {
        if self.peek_token(TokenType::TemplateString).is_ok()
            || self.peek_token(TokenType::TemplateStringStart).is_ok()
        {
            Ok(self.peek()?)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a template literal. The path (i.e. tag) must be passed in explicitly.
    ///
    /// Examples:
    /// ```
    /// `hello`
    /// `hello ${name}`
    /// `SELECT * FROM users`
    /// `${stmt}`
    /// `SELECT * FROM users WHERE name = ${name}` AND age > ${group.age()} LIMIT 10`
    /// ```
    pub fn eat_template_literal(&mut self) -> ParseResult<TemplateLiteral> {
        let next = *self.eat()?;
        let next_str = self.get_span_str(next.span);

        // template string without interpolation
        if next.token.ty == TokenType::TemplateString {
            let string = next_str.trim_start_matches('`').trim_end_matches('`');
            let string_id = self.strings.intern(string);
            Ok(TemplateLiteral::String { string: string_id })
        }
        // template string with interpolation
        else if next.token.ty == TokenType::TemplateStringStart {
            let mut strings: Vec<StringId> = Vec::new();
            let mut arguments: Vec<NodeId<Argument>> = Vec::new();

            // start
            let string = &next_str[1..next_str.len() - 2]; // remove ` and ${
            let string_id = self.strings.intern(string);
            strings.push(string_id);

            // eat until the end
            while self.peek_token(TokenType::TemplateStringEnd).is_err() {
                // string
                if self.peek_token(TokenType::TemplateStringMiddle).is_ok() {
                    let token = *self.eat()?;
                    let string = self.get_span_str(token.span);
                    let string = &string[1..string.len() - 2]; // remove } and ${
                    let string_id = self.strings.intern(string);
                    strings.push(string_id);
                }
                // argument
                else {
                    self.eat_newlines_maybe()?;
                    let argument = self.eat_argument()?;
                    self.eat_newlines_maybe()?;
                    arguments.push(argument);
                }
            }

            // end
            let token = *self.eat_token(TokenType::TemplateStringEnd)?;
            let string = self.get_span_str(token.span);
            let string = &string[1..string.len() - 1]; // remove } and `
            let string_id = self.strings.intern(string);
            strings.push(string_id);

            Ok(TemplateLiteral::InterpolatedString { strings, arguments })
        }
        // error
        else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Eat an array literal (including the surrounding brackets).
    pub fn eat_array_literal(&mut self) -> ParseResult<Vec<NodeId<Argument>>> {
        self.eat_token(TokenType::OpenBracket)?;
        self.eat_newlines_maybe()?;
        let elements = if self.peek_token(TokenType::CloseBracket).is_ok() {
            vec![]
        } else {
            self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_sequence_literal_body(None, TokenType::CloseBracket)
            })?
        };
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseBracket)?;
        Ok(elements)
    }

    /// Eat the body of a sequence literal (excluding the surrounding parenthesis).
    pub fn eat_sequence_literal_body(
        &mut self,
        first_element: Option<NodeId<Argument>>,
        close_token: TokenType,
    ) -> ParseResult<Vec<NodeId<Argument>>> {
        let mut elements = Vec::new();
        if let Some(first) = first_element {
            elements.push(first);
        }
        while self.peek().is_ok() {
            // stop at closing parenthesis
            if self.peek_token(close_token).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_comma().is_ok() {
                self.eat_item_stop_with_newlines()?;
                continue;
            }
            // keep eating elements
            let element = self.eat_argument().for_node_type(NodeType::Argument)?;
            elements.push(element);
        }
        Ok(elements)
    }

    /// Eat a struct literal (including the surrounding braces, excluding any prefix type).
    ///
    /// Examples:
    /// ```
    /// { }
    /// { a: 1, b }
    /// { a(x): void }
    pub fn eat_struct_literal(&mut self) -> ParseResult<Vec<NodeId<Property>>> {
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;
        let properties = self.eat_properties()?;
        self.eat_token(TokenType::CloseBrace)?;
        Ok(properties)
    }

    /// Peek a tree literal (including the `<` and `>` tokens).
    #[inline]
    pub fn peek_tree_literal(&self) -> ParseResult<()> {
        if self.peek_token(TokenType::LessThan).is_ok()
            && (self.peek_next_token(TokenType::GreaterThan).is_ok()
                || self.peek_next_token(TokenType::Divide).is_ok()
                || self.peek_next_token(TokenType::Identifier).is_ok())
        {
            return Ok(());
        }
        Err(ParseError::unexpected(self.peek()?.span))
    }

    /// Eat a tree literal (including the `<` and `>` tokens).
    ///
    /// Examples:
    /// ```
    /// <Entity />
    /// <Entity a=1 test />
    /// <Level level=1>
    ///     player: <Entity name="Alfred" />
    ///     <Entity>2</Entity>
    ///     "some text"
    ///     ..someChildren.map(child => <Entity name={child.name} />)
    /// </Level>
    /// ```
    pub fn eat_tree_literal(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

        // path
        let path: Option<Path> = {
            if self.peek_token(TokenType::Identifier).is_ok() {
                Some(self.eat_path()?)
            } else {
                None
            }
        };
        self.eat_newlines_maybe()?;

        // header (arguments separated by `=`)
        let arguments: Option<Vec<NodeId<Argument>>> = {
            // fragment without arguments
            if self.peek_token(TokenType::Divide).is_ok()
                || self.peek_token(TokenType::GreaterThan).is_ok()
            {
                None
            }
            // fragment with arguments
            else {
                let mut arguments: Vec<NodeId<Argument>> = vec![];
                while self.peek_token(TokenType::Divide).is_err()
                    && self.peek_token(TokenType::GreaterThan).is_err()
                {
                    let argument = self.with_options(
                        self.options.not_in_position().in_tree_literal(),
                        |parser| parser.eat_tree_literal_argument(),
                    )?;
                    arguments.push(argument);
                    self.eat_newlines_maybe()?;
                }
                Some(arguments)
            }
        };
        self.eat_newlines_maybe()?;

        // body (either />, or > with child elements)
        let elements: Option<Vec<NodeId<Argument>>> = {
            // fragment without children (/>)
            if self.peek_token(TokenType::Divide).is_ok() {
                self.bump(); // eat /
                self.eat_token(TokenType::GreaterThan)?; // eat >
                None
            }
            // fragment with children (>)
            else {
                self.eat_token(TokenType::GreaterThan)?; // eat >
                self.eat_newlines_maybe()?;

                // eat children until closing fragment
                let mut elements: Vec<NodeId<Argument>> = vec![];
                while self.peek().is_ok() {
                    // stop at closing fragment (</)
                    if self.peek_token(TokenType::LessThan).is_ok()
                        && self.peek_next_token(TokenType::Divide).is_ok()
                    {
                        // special case for empty fragment (/>)
                        if path.is_none()
                            && self.peek_next_next_token(TokenType::GreaterThan).is_ok()
                        {
                            self.bump(); // eat <
                            self.bump(); // eat /
                            self.bump(); // eat >
                            break;
                        }
                        // check if closing fragment has same path
                        else if let Some(path) = &path {
                            let speculative_start = (self.mark(), self.tree.next_id());
                            // speculatively eat </path
                            self.bump(); // eat <
                            self.bump(); // eat /
                            match self.eat_path() {
                                Ok(closing_path) => {
                                    // found our closing tag
                                    if closing_path == *path {
                                        self.eat_token(TokenType::GreaterThan)?;
                                        break;
                                    }
                                    // not our closing tag
                                    else {
                                        self.restore(speculative_start.0, speculative_start.1);
                                    }
                                }
                                Err(_) => {
                                    // something else
                                    self.restore(speculative_start.0, speculative_start.1);
                                }
                            };
                        }
                    }

                    // keep eating child elements
                    let element = self.with_options(
                        self.options
                            .not_in_position()
                            .in_tree_literal()
                            .in_statement_position(),
                        |parser| parser.eat_argument(),
                    )?;
                    elements.push(element);
                    self.eat_newlines_maybe()?;
                }

                Some(elements)
            }
        };

        // tree literal
        let expression = Expression::TreeLiteral {
            path,
            arguments,
            elements,
        };
        Ok(self.tree.insert(expression, self.get_span_from(start)))
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        Argument, Block, Expression, FloatType, IntType, Name, ScalarLiteral, TemplateLiteral,
        TypeLiteral,
    };

    use crate::parse::tests::TestParser;
    use crate::{assert_expr_path, assert_node, assert_path, assert_string};

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
        let mut test = TestParser::new(r#""hello" 'hi there' b"abc""#);
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

        let literal = parser.eat_scalar_literal().unwrap();
        match literal {
            ScalarLiteral::ByteString(bytes) => assert_eq!(bytes, b"abc"),
            other => panic!("expected byte string literal, got {other:?}"),
        }
    }

    /// Parse a regex string literal.
    #[test]
    fn test_parse_regex_string_literal() {
        let mut test = TestParser::new("/abc/\n/abc/g");
        let mut parser = test.prepare();

        // /abc/
        let literal = parser.eat_scalar_literal().unwrap();
        match literal {
            ScalarLiteral::RegexString { content, flags } => {
                assert_string!(parser, content, "abc");
                assert!(flags.is_none());
            }
            other => panic!("expected regex string literal, got {other:?}"),
        }
        parser.eat_newline().unwrap();

        // /abc/g
        let literal = parser.eat_scalar_literal().unwrap();
        match literal {
            ScalarLiteral::RegexString { content, flags } => {
                assert_string!(parser, content, "abc");
                assert_string!(parser, flags.unwrap(), "g");
            }
            other => panic!("expected regex string literal, got {other:?}"),
        }
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
        parser.eat_newline().unwrap();

        // `hello`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::String { string: template } => {
                // hello
                assert_string!(parser, template, "hello");
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `hello ${name}`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(arguments.len(), 1);
                assert_eq!(strings.len(), 2);
                // hello
                assert_string!(parser, strings[0], "hello ");
                // name
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "name");
                });
                //
                assert_string!(parser, strings[1], "");
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

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
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "stmt");
                });
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

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
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "start");
                });
                // middle
                assert_node!(parser.tree, arguments[1], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "middle");
                });
                // end
                assert_node!(parser.tree, arguments[2], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "end");
                });
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(arguments.len(), 2);
                assert_eq!(strings.len(), 3);
                // SELECT * FROM users WHERE name =
                assert_string!(parser, strings[0], "SELECT * FROM users WHERE name = ");
                // name
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "name");
                });
                // AND age >
                assert_string!(parser, strings[1], " AND age > ");
                // group.age()
                assert_node!(parser.tree, arguments[1], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Call { left, .. } => {
                        assert_expr_path!(parser, parser.tree.get(*left), "group.age");
                    });
                });
                // LIMIT 10
                assert_string!(parser, strings[2], " LIMIT 10");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn test_parse_type_literal() {
        let mut test = TestParser::new("int32 uint8 float boolean symbol unique symbol");
        let mut parser = test.prepare();
        parser.options.in_type = true;

        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            })
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Int(IntType::Arbitrary {
                width: Some(8),
                is_signed: false
            })
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Float(FloatType { width: None })
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Boolean
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
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            }
        );
        // 2
        assert_node!(
            parser.tree,
            elements[1],
            Argument::Positional { modifiers: _, value } => {
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
        assert_node!(parser.tree, expression, Expression::TreeLiteral { path, arguments, elements } => {
            // A
            assert_path!(parser, path.as_ref().unwrap(), "A");
            assert!(arguments.is_none());
            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_arguments() {
        let mut test = TestParser::new("<A a=1 annoying-bee=2 c=3 flag />");
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        // <A a=1 annoying-b=2 c=3 />
        assert_node!(parser.tree, expression, Expression::TreeLiteral { path, arguments, elements } => {
            // A
            assert_path!(parser, path.as_ref().unwrap(), "A");
            assert_eq!(arguments.as_ref().unwrap().len(), 4);
            // a=1
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            // annoying-b=2
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "annoyingBee");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
            // c=3
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "c");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
            // flag
            assert_node!(parser.tree, arguments.as_ref().unwrap()[3], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "flag");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });

            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_arguments_and_child() {
        let mut test = TestParser::new(
            r"
<Tooltip
    title=true
    flag
    something-else={false}
>
    {true}
</Tooltip>
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_tree_literal().unwrap();
        // <Tooltip>
        assert_node!(parser.tree, expression, Expression::TreeLiteral { path, arguments, elements } => {
            assert_path!(parser, path.as_ref().unwrap(), "Tooltip");
            assert_eq!(arguments.as_ref().unwrap().len(), 3);
            // title=true
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "title");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
            // flag
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "flag");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
            // something-else=false
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "somethingElse");
                assert_node!(parser.tree, *value, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                        assert_eq!(expressions.len(), 1);
                        assert_node!(parser.tree, expressions[0], Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                    });
                })
            });

            assert!(elements.is_some());
            // {true}
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                        assert_eq!(expressions.len(), 1);
                        assert_node!(parser.tree, expressions[0], Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
                    });
                })
            });
        });
    }

    #[test]
    fn test_parse_tree_nested_deep() {
        let mut test = TestParser::new(
            r"
<A>
    <B>
        <C>
            <D/>
            2
        </C>
    </B>
</A>
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_tree_literal().unwrap();
        // <A>
        assert_node!(parser.tree, expression, Expression::TreeLiteral { path, arguments, elements } => {
            assert_path!(parser, path.as_ref().unwrap(), "A");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            // <B>
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                    assert_path!(parser, path.as_ref().unwrap(), "B");
                    assert!(arguments.is_none());
                    assert!(elements.is_some());
                    // <C>
                    assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                            assert_path!(parser, path.as_ref().unwrap(), "C");
                            assert!(arguments.is_none());
                            assert!(elements.is_some());
                            // <D/>
                            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                                assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                                    assert_path!(parser, path.as_ref().unwrap(), "D");
                                    assert!(arguments.is_none());
                                    assert!(elements.is_none());
                                });
                            });
                            // 2
                            assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Positional { modifiers: _, value } => {
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
        header: "Hello"
    </div>
)
        "#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression, Expression::Parenthesized { expression } => {
            // <div className="font-semibold">
            assert_node!(parser.tree, *expression, Expression::TreeLiteral { path, arguments, elements } => {
                assert_path!(parser, path.as_ref().unwrap(), "div");
                assert!(arguments.is_some());
                assert_eq!(arguments.as_ref().unwrap().len(), 1);
                // className="font-semibold"
                assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                    // className
                    assert_string!(parser, *name, "className");
                    // font-semibold
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "font-semibold");
                    });
                });

                assert!(elements.is_some());
                assert_eq!(elements.as_ref().unwrap().len(), 2);
                // <Link subtle to={urls.annotation(annotation.id)}>
                assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                        // Link
                        assert_path!(parser, path.as_ref().unwrap(), "Link");
                        assert!(arguments.is_some());
                        assert_eq!(arguments.as_ref().unwrap().len(), 2);
                        // subtle
                        assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                            assert_string!(parser, *name, "subtle");
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
                        });
                        // to={1}
                        assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                            assert_string!(parser, *name, "to");
                            assert_node!(parser.tree, *value, Expression::Block(block_id) => {
                                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                                    assert_eq!(expressions.len(), 1);
                                    assert_node!(parser.tree, expressions[0], Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                                });
                            });
                        });

                        assert!(elements.is_some());
                        assert_eq!(elements.as_ref().unwrap().len(), 1);
                        // {2}
                        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                            assert_node!(parser.tree, *value, Expression::Block(block_id) => {
                                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                                    assert_eq!(expressions.len(), 1);
                                    assert_node!(parser.tree, expressions[0], Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                                });
                            });
                        });
                    });
                });
                // header="Hello"
                assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                    assert_string!(parser, *name, "header");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "Hello");
                    });
                });
            });
        });
    }
}
