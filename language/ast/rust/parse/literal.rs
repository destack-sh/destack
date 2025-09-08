use dyst_language_token::{NumberBase, RawLiteralType, TokenSpan, TokenType};
use std::borrow::Cow;

use crate::parse::expression::ExpressionParserOptions;
use crate::{
    ArrayLiteral, Expression, FieldLiteral, FloatType, IntType, NodeId, ParseError, ParseResult,
    Parser, ScalarLiteral, StructLiteral, TupleLiteral,
};

impl<'a> Parser<'a> {
    /// Peek a scalar literal.
    pub fn peek_scalar_literal(&self) -> ParseResult<&TokenSpan> {
        if self.peek_token(TokenType::Literal).is_ok() {
            Ok(self.peek()?)
        } else {
            Err(ParseError::UnexpectedToken(self.peek()?.span))
        }
    }

    /// Eat a raw literal token.
    pub fn eat_raw_literal(&mut self) -> ParseResult<(TokenSpan, RawLiteralType)> {
        let literal_span = *self.eat_next()?;
        if let Some(literal) = literal_span.token.body {
            Ok((literal_span, literal))
        } else {
            Err(ParseError::UnexpectedToken(literal_span.span))
        }
    }

    /// Eat a scalar literal.
    ///
    /// Examples:
    /// ```
    /// void
    /// null
    /// true
    /// false
    /// 1
    /// 1.0
    /// 7
    /// "Hello, world!"
    /// 'a'
    /// b'a'
    /// b"abc"
    /// 0x1234
    /// ```
    pub fn eat_scalar_literal(&mut self) -> ParseResult<NodeId<ScalarLiteral>> {
        let start = self.mark();
        let (literal_span, literal) = self.eat_raw_literal()?;
        let literal_str = self.get_span_str(literal_span.span);

        match literal {
            // void literal
            RawLiteralType::Void => {
                let scalar_literal = self
                    .tree
                    .allocate(ScalarLiteral::Void, self.get_span_from(start));
                Ok(scalar_literal)
            }

            // null literal
            RawLiteralType::Null => {
                let scalar_literal = self
                    .tree
                    .allocate(ScalarLiteral::Null, self.get_span_from(start));
                Ok(scalar_literal)
            }

            // boolean literal
            RawLiteralType::Boolean { value } => {
                let scalar_literal = self
                    .tree
                    .allocate(ScalarLiteral::Boolean(value), self.get_span_from(start));
                Ok(scalar_literal)
            }

            // int literal
            RawLiteralType::Int { base, is_empty } => {
                if is_empty {
                    return Err(ParseError::UnexpectedToken(literal_span.span));
                }
                // strip underscores for parsing
                let cleaned_str: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };
                // handle base-specific prefixes
                let parsed_int = match base {
                    NumberBase::Decimal => cleaned_str.parse::<i64>(),
                    NumberBase::Binary => {
                        // strip leading 0b
                        let body = cleaned_str.trim_start_matches("0b");
                        i64::from_str_radix(body, 2)
                    }
                    NumberBase::Octal => {
                        let body = cleaned_str.trim_start_matches("0o");
                        i64::from_str_radix(body, 8)
                    }
                    NumberBase::Hexadecimal => {
                        let body = cleaned_str.trim_start_matches("0x");
                        i64::from_str_radix(body, 16)
                    }
                };
                match parsed_int {
                    Ok(value) => {
                        let scalar_literal = self.tree.allocate(
                            ScalarLiteral::Integer(value, IntType::INT32),
                            self.get_span_from(start),
                        );
                        Ok(scalar_literal)
                    }
                    Err(_) => Err(ParseError::UnexpectedToken(literal_span.span)),
                }
            }

            // float literal
            RawLiteralType::Float {
                base: _,
                is_empty_exponent,
            } => {
                if is_empty_exponent {
                    return Err(ParseError::UnexpectedToken(literal_span.span));
                }
                let cleaned_str: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };
                let parsed_float = cleaned_str.parse::<f64>();
                match parsed_float {
                    Ok(value) => {
                        let scalar_literal = self.tree.allocate(
                            ScalarLiteral::Float(value, FloatType::Float64),
                            self.get_span_from(start),
                        );
                        Ok(scalar_literal)
                    }
                    Err(_) => Err(ParseError::UnexpectedToken(literal_span.span)),
                }
            }

            // character literal (ignore quotes)
            RawLiteralType::Character { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::UnexpectedToken(literal_span.span));
                }
                let content = literal_str.trim_start_matches('\'').trim_end_matches('\'');
                if let Some(literal_char) = content.chars().next() {
                    let scalar_literal = self.tree.allocate(
                        ScalarLiteral::Character(literal_char),
                        self.get_span_from(start),
                    );
                    Ok(scalar_literal)
                } else {
                    Err(ParseError::UnexpectedToken(literal_span.span))
                }
            }

            // byte character literal (ignore quotes)
            RawLiteralType::Byte { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::UnexpectedToken(literal_span.span));
                }
                let content = literal_str.trim_start_matches("b'").trim_end_matches('\'');
                if let Some(literal_char) = content.chars().next() {
                    let scalar_literal = self.tree.allocate(
                        ScalarLiteral::Byte(literal_char as u8),
                        self.get_span_from(start),
                    );
                    Ok(scalar_literal)
                } else {
                    Err(ParseError::UnexpectedToken(literal_span.span))
                }
            }

            // string literal (ignore quotes)
            RawLiteralType::String { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::UnexpectedToken(literal_span.span));
                }
                let content = literal_str.trim_start_matches('"').trim_end_matches('"');
                let string_id = self.strings.intern(content);
                let scalar_literal = self
                    .tree
                    .allocate(ScalarLiteral::String(string_id), self.get_span_from(start));
                Ok(scalar_literal)
            }

            // byte string literal (ignore quotes)
            RawLiteralType::ByteString { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::UnexpectedToken(literal_span.span));
                }
                let content = literal_str.trim_start_matches("b\"").trim_end_matches('"');
                let bytes = content.as_bytes().to_vec();
                let scalar_literal = self
                    .tree
                    .allocate(ScalarLiteral::ByteString(bytes), self.get_span_from(start));
                Ok(scalar_literal)
            }

            // raw string literal (ignore quotes and hashes)
            RawLiteralType::RawString { hashes } => {
                if let Some(hashes) = hashes {
                    let num_hashes = hashes as usize;
                    let prefix_len = 1 /* r */ + num_hashes + 1 /* opening " */;
                    let suffix_len = 1 /* closing " */ + num_hashes;
                    if literal_str.len() < prefix_len + suffix_len {
                        return Err(ParseError::UnexpectedToken(literal_span.span));
                    }
                    let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                    let string_id = self.strings.intern(content);
                    let scalar_literal = self
                        .tree
                        .allocate(ScalarLiteral::String(string_id), self.get_span_from(start));
                    Ok(scalar_literal)
                } else {
                    Err(ParseError::UnexpectedToken(literal_span.span))
                }
            }

            // raw byte string literal (ignore quotes and hashes)
            RawLiteralType::RawByteString { hashes } => {
                if let Some(hashes) = hashes {
                    let num_hashes = hashes as usize;
                    let prefix_len = 2 /* br */ + num_hashes + 1 /* opening " */;
                    let suffix_len = 1 /* closing " */ + num_hashes;
                    if literal_str.len() < prefix_len + suffix_len {
                        return Err(ParseError::UnexpectedToken(literal_span.span));
                    }
                    let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                    let bytes = content.as_bytes().to_vec();
                    let scalar_literal = self
                        .tree
                        .allocate(ScalarLiteral::ByteString(bytes), self.get_span_from(start));
                    Ok(scalar_literal)
                } else {
                    Err(ParseError::UnexpectedToken(literal_span.span))
                }
            }
        }
    }

    /// Eat an array literal (fixed or repeated).
    /// The individual elements are full expressions, not just literals.
    ///
    /// Examples:
    /// ```
    /// [] // empty array
    /// [1, 2, ] // trailing comma is allowed
    /// // multi-line array with implicit comma
    /// [
    ///   1 // comma is optional here
    ///   2 // comma is optional here too
    /// ]
    /// [10, false, "Hi"] // hetereogenous array is invalid but okay in AST
    /// [0; 10] // repeated array
    pub fn eat_array_literal(&mut self) -> ParseResult<NodeId<ArrayLiteral>> {
        let start = self.mark();

        self.eat_token(TokenType::OpenBracket)?;
        self.eat_newlines_maybe()?;

        // empty array
        if self.peek_token(TokenType::CloseBracket).is_ok() {
            self.eat_token(TokenType::CloseBracket)?;
            let array_literal = self.tree.allocate(
                ArrayLiteral::Fixed { elements: vec![] },
                self.get_span_from(start),
            );
            return Ok(array_literal);
        }

        let first_element = self.eat_expression(ExpressionParserOptions::default())?;
        if self.peek_token(TokenType::Semicolon).is_ok() {
            // repeated array: [value; count]
            self.eat_token(TokenType::Semicolon)?;
            let count = self.eat_expression(ExpressionParserOptions::default())?;
            self.eat_token(TokenType::CloseBracket)?;
            let array_literal = self.tree.allocate(
                ArrayLiteral::Repeated {
                    element: first_element,
                    count,
                },
                self.get_span_from(start),
            );
            Ok(array_literal)
        } else {
            // fixed array: [elem1, elem2, elem3, ...]
            let mut elements = vec![first_element];
            while self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
                if self.peek_token(TokenType::CloseBracket).is_ok() {
                    break;
                }
                let element = self.eat_expression(ExpressionParserOptions::default())?;
                elements.push(element);
            }
            self.eat_token(TokenType::CloseBracket)?;
            let array_literal = self
                .tree
                .allocate(ArrayLiteral::Fixed { elements }, self.get_span_from(start));
            Ok(array_literal)
        }
    }

    /// Eat a tuple literal.
    /// The individual elements are full expressions, not just literals.
    ///
    /// Examples:
    /// ```
    /// (1, 2, ) // trailing comma is allowed
    /// // multi-line tuple with implicit comma
    /// (
    ///   1 // comma is optional here
    ///   2 // comma is optional here too
    /// )
    /// (1.0, 2.0, 0.0)
    /// (10, false, "Hi") // okay because it's a tuple
    /// ```
    pub fn eat_tuple_literal(&mut self) -> ParseResult<NodeId<TupleLiteral>> {
        let start = self.mark();

        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;

        // empty tuple
        if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            self.eat_token(TokenType::CloseParenthesis)?;
            let tuple_literal = self
                .tree
                .allocate(TupleLiteral { elements: vec![] }, self.get_span_from(start));
            return Ok(tuple_literal);
        }

        // parse first element
        let first_element = self.eat_expression(ExpressionParserOptions::default())?;

        // parse remaining elements separated by comma or newline, allow trailing comma
        let elements = self.eat_tuple_literal_body(first_element)?;
        self.eat_token(TokenType::CloseParenthesis)?;
        let tuple_literal = self
            .tree
            .allocate(TupleLiteral { elements }, self.get_span_from(start));
        Ok(tuple_literal)
    }

    /// Eat the body of a tuple literal (excluding the parenthesis).
    pub fn eat_tuple_literal_body(
        &mut self,
        first_element: NodeId<Expression>,
    ) -> ParseResult<Vec<NodeId<Expression>>> {
        let mut elements: Vec<NodeId<Expression>> = vec![first_element];
        loop {
            // stop at closing parenthesis
            if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            }
            // keep eating elements
            else {
                let element = self.eat_expression(ExpressionParserOptions::default())?;
                elements.push(element);
            }
        }
        Ok(elements)
    }

    /// Peek a struct literal.
    /// NOTE :Performance: peek_struct_literal uses large lookahead.
    #[inline]
    pub fn peek_struct_literal(&self) -> ParseResult<()> {
        // first name can't be a keyword
        if self.peek_any_keyword().is_ok() {
            return Err(ParseError::UnexpectedToken(self.peek()?.span));
        }

        let start = self.pos();
        let end = start + 20; // max lookahead
        let mut pos = self.pos();

        // identifier .identifier*
        // like `geom.Mesh`
        while pos < end {
            if let Some(token) = self.tokens.get(pos)
                && token.token.r#type == TokenType::Identifier
            {
                pos += 1;
                // keep going if there's a dot
                if let Some(token) = self.tokens.get(pos)
                    && token.token.r#type == TokenType::Dot
                {
                    pos += 1;
                    continue;
                }
            }
            break;
        }

        // {
        // like in `geom.Mesh { ... }`
        if let Some(token) = self.tokens.get(pos)
            && token.token.r#type == TokenType::OpenBrace
        {
            return Ok(());
        }
        // generics
        else if let Some(token) = self.tokens.get(pos)
            && token.token.r#type == TokenType::LessThan
        {
            // scan until '>'
            while pos < end {
                if let Some(token) = self.tokens.get(pos)
                    && token.token.r#type == TokenType::GreaterThan
                {
                    // if next token is '{', we have a struct literal
                    if let Some(token) = self.tokens.get(pos + 1)
                        && token.token.r#type == TokenType::OpenBrace
                    {
                        return Ok(());
                    }
                    break;
                }
                pos += 1;
            }
        }

        Err(ParseError::UnexpectedToken(self.peek()?.span))
    }

    /// Eat a struct literal (including the type prefix).
    ///
    /// Examples:
    /// ```
    /// Vector2 { x: 1.0, y: 2.0, z }
    ///
    /// destack.geometry.Mesh2 {
    ///     vertices: [Vector3 { x: 1.0, y: 2.0, z: 0.0 }] // optional comma
    ///     indices: [0, 1, 2] // optional comma
    /// }
    ///
    /// Mesh2<float64> { something: [] }
    /// ```
    pub fn eat_struct_literal(&mut self) -> ParseResult<NodeId<StructLiteral>> {
        let start = self.mark();
        let r#type = self.eat_type()?;
        let fields = self.eat_struct_literal_body()?;
        let struct_literal = self
            .tree
            .allocate(StructLiteral { r#type, fields }, self.get_span_from(start));
        Ok(struct_literal)
    }

    /// Eat the body of a struct literal (excluding the path prefix).
    ///
    /// Examples:
    /// ```
    /// { x: 1.0, y: 2.0, z }
    ///
    /// // multi-line struct with implicit comma
    /// {
    ///    x: 1.0
    ///    y: 2.0
    ///    z
    /// }
    /// ```
    pub fn eat_struct_literal_body(&mut self) -> ParseResult<Vec<NodeId<FieldLiteral>>> {
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;

        // empty struct
        if self.peek_token(TokenType::CloseBrace).is_ok() {
            self.eat_token(TokenType::CloseBrace)?;
            return Ok(vec![]);
        }

        // parse fields separated by item stops, allow trailing comma
        let mut fields: Vec<NodeId<FieldLiteral>> = vec![];
        loop {
            // stop at closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            let field_start = self.mark();
            let name = self.eat_identifier()?;
            // named field
            if self.peek_token(TokenType::Colon).is_ok() {
                self.eat_token(TokenType::Colon)?;
                let value = self.eat_expression(ExpressionParserOptions::default())?;
                let field_literal = self.tree.allocate(
                    FieldLiteral::Named { name, value },
                    self.get_span_from(field_start),
                );
                fields.push(field_literal);
            }
            // shorthand field
            else {
                let field_literal = self.tree.allocate(
                    FieldLiteral::NamedShorthand { name },
                    self.get_span_from(field_start),
                );
                fields.push(field_literal);
            }
            // item stop
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            }
        }
        self.eat_token(TokenType::CloseBrace)?;
        Ok(fields)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        ArrayLiteral, Expression, FieldLiteral, ScalarLiteral, StructLiteral, TupleLiteral,
        assert_bool, assert_char, assert_float, assert_int, assert_node, assert_string,
    };

    #[test]
    fn test_parse_integer_literals() {
        // Parse multiple integer literals including hex
        let test = TestParser::new("1 731 0x1234");
        let mut parser = test.parser();

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_int!(parser.tree, literal_id, 1);

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_int!(parser.tree, literal_id, 731);

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_int!(parser.tree, literal_id, 0x1234);
    }

    #[test]
    fn test_parse_float_literals() {
        // Parse scientific notation and decimal floats
        let test = TestParser::new("10e37 1.0");
        let mut parser = test.parser();

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_float!(parser.tree, literal_id, 1.0e38);

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_float!(parser.tree, literal_id, 1.0);
    }

    #[test]
    fn test_parse_boolean_literals() {
        // Parse true and false literals
        let test = TestParser::new("true false");
        let mut parser = test.parser();

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_bool!(parser.tree, literal_id, true);

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_bool!(parser.tree, literal_id, false);
    }

    #[test]
    fn test_parse_character_literals() {
        // Parse character and byte literals
        let test = TestParser::new("'a' b'a'");
        let mut parser = test.parser();

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_char!(parser.tree, literal_id, 'a');

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_node!(parser.tree, literal_id, ScalarLiteral::Byte(b'a'));
    }

    #[test]
    fn test_parse_string_literals() {
        // Parse various string literal formats including raw and byte strings
        let test = TestParser::new(
            r###""Hello, world!" b"abc" r"abc" r##"a#b#c"## br"abc" br##"a#b#c"##"###,
        );
        let mut parser = test.parser();

        // "Hello, world!"
        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_string!(
            parser.tree,
            literal_id,
            "Hello, world!",
            using | id | parser.strings.get(id)
        );

        // b"abc"
        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_node!(parser.tree, literal_id, ScalarLiteral::ByteString(bytes) => {
            let str = std::str::from_utf8(bytes).unwrap();
            assert_eq!(str, "abc");
        });

        // r"abc"
        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_string!(
            parser.tree,
            literal_id,
            "abc",
            using | id | parser.strings.get(id)
        );

        // r##"a#b#c"##
        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_string!(
            parser.tree,
            literal_id,
            "a#b#c",
            using | id | parser.strings.get(id)
        );

        // br"abc"
        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_node!(parser.tree, literal_id, ScalarLiteral::ByteString(bytes) => {
            let str = std::str::from_utf8(bytes).unwrap();
            assert_eq!(str, "abc");
        });

        // br##"a#b#c"##
        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_node!(parser.tree, literal_id, ScalarLiteral::ByteString(bytes) => {
            let str = std::str::from_utf8(bytes).unwrap();
            assert_eq!(str, "a#b#c");
        });
    }

    #[test]
    fn test_parse_empty_array_literal() {
        // []
        let test = TestParser::new("[]");
        let mut parser = test.parser();

        let literal_id = parser.eat_array_literal().unwrap();
        assert_node!(parser.tree, literal_id, ArrayLiteral::Fixed { elements } => {
            assert_eq!(elements.len(), 0);
        });
    }

    #[test]
    fn test_parse_single_element_array_literal() {
        // [1, ]
        let test = TestParser::new("[1, ]");
        let mut parser = test.parser();

        let literal_id = parser.eat_array_literal().unwrap();
        assert_node!(parser.tree, literal_id, ArrayLiteral::Fixed { elements } => {
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_int!(parser.tree, *scalar_literal_id, 1);
            });
        });
    }

    #[test]
    fn test_parse_multi_element_array_literal() {
        // [10, false, "Hi"]
        let test = TestParser::new(r###"[10, false, "Hi"]"###);
        let mut parser = test.parser();

        let literal_id = parser.eat_array_literal().unwrap();
        assert_node!(parser.tree, literal_id, ArrayLiteral::Fixed { elements } => {
            assert_eq!(elements.len(), 3);

            // [0] = 10
            assert_node!(parser.tree, elements[0], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_int!(parser.tree, *scalar_literal_id, 10);
            });

            // [1] = false
            assert_node!(parser.tree, elements[1], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_bool!(parser.tree, *scalar_literal_id, false);
            });

            // [2] = "Hi"
            assert_node!(parser.tree, elements[2], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_string!(
                    parser.tree,
                    *scalar_literal_id,
                    "Hi",
                    using | id | parser.strings.get(id)
                );
            });
        });
    }

    #[test]
    fn test_parse_multiline_array_literal() {
        // Multi-line array with implicit comma separation
        let test = TestParser::new(
            r###"[1.0, 
     2.0
     3.0
    ]"###,
        );
        let mut parser = test.parser();

        let literal_id = parser.eat_array_literal().unwrap();
        assert_node!(parser.tree, literal_id, ArrayLiteral::Fixed { elements } => {
            assert_eq!(elements.len(), 3);

            // [0] = 1.0
            assert_node!(parser.tree, elements[0], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_float!(parser.tree, *scalar_literal_id, 1.0);
            });

            // [1] = 2.0
            assert_node!(parser.tree, elements[1], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_float!(parser.tree, *scalar_literal_id, 2.0);
            });

            // [2] = 3.0
            assert_node!(parser.tree, elements[2], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_float!(parser.tree, *scalar_literal_id, 3.0);
            });
        });
    }

    #[test]
    fn test_parse_repeated_array_literal() {
        // [0; 10]
        let test = TestParser::new("[0; 10]");
        let mut parser = test.parser();

        let literal_id = parser.eat_array_literal().unwrap();
        assert_node!(parser.tree, literal_id, ArrayLiteral::Repeated { element, count } => {
            assert_node!(parser.tree, *element, Expression::ScalarLiteral(scalar_literal_id) => {
                assert_int!(parser.tree, *scalar_literal_id, 0);
            });
            assert_node!(parser.tree, *count, Expression::ScalarLiteral(scalar_literal_id) => {
                assert_int!(parser.tree, *scalar_literal_id, 10);
            });
        });
    }

    #[test]
    fn test_parse_single_element_tuple_literal() {
        // (1, )
        let test = TestParser::new("(1, )");
        let mut parser = test.parser();

        let literal_id = parser.eat_tuple_literal().unwrap();
        assert_node!(parser.tree, literal_id, TupleLiteral { elements } => {
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_int!(parser.tree, *scalar_literal_id, 1);
            });
        });
    }

    #[test]
    fn test_parse_multi_element_tuple_literal() {
        // (10, false, "Hi")
        let test = TestParser::new(r###"(10, false, "Hi")"###);
        let mut parser = test.parser();

        let literal_id = parser.eat_tuple_literal().unwrap();
        assert_node!(parser.tree, literal_id, TupleLiteral { elements } => {
            assert_eq!(elements.len(), 3);

            // [0] = 10
            assert_node!(parser.tree, elements[0], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_int!(parser.tree, *scalar_literal_id, 10);
            });

            // [1] = false
            assert_node!(parser.tree, elements[1], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_bool!(parser.tree, *scalar_literal_id, false);
            });

            // [2] = "Hi"
            assert_node!(parser.tree, elements[2], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_string!(
                    parser.tree,
                    *scalar_literal_id,
                    "Hi",
                    using | id | parser.strings.get(id)
                );
            });
        });
    }

    #[test]
    fn test_parse_multiline_tuple_literal() {
        // Multi-line tuple with implicit comma separation
        let test = TestParser::new(
            r###"(
  1.0
  2.0
  3.0
)"###,
        );
        let mut parser = test.parser();

        let literal_id = parser.eat_tuple_literal().unwrap();
        assert_node!(parser.tree, literal_id, TupleLiteral { elements } => {
            assert_eq!(elements.len(), 3);

            // [0] = 1.0
            assert_node!(parser.tree, elements[0], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_float!(parser.tree, *scalar_literal_id, 1.0);
            });

            // [1] = 2.0
            assert_node!(parser.tree, elements[1], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_float!(parser.tree, *scalar_literal_id, 2.0);
            });

            // [2] = 3.0
            assert_node!(parser.tree, elements[2], Expression::ScalarLiteral(scalar_literal_id) => {
                assert_float!(parser.tree, *scalar_literal_id, 3.0);
            });
        });
    }

    #[test]
    fn test_parse_struct_literal() {
        let test = TestParser::new(
            r##"
destack.geometry.Mesh<2, int32> {
    vertices: [1.0, 2.0] // optional comma
    indices: 2, // optional comma
}
            "##,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct_literal().unwrap();
        assert_node!(parser.tree, struct_id, StructLiteral { r#type: _, fields } => {
            assert_eq!(fields.len(), 2);

            // vertices: []
            assert_node!(parser.tree, fields[0], FieldLiteral::Named { name, value } => {
                assert_eq!(parser.strings.get(*name), "vertices");
                assert_node!(parser.tree, *value, Expression::ArrayLiteral(array_literal_id) => {
                    assert_node!(parser.tree, *array_literal_id, ArrayLiteral::Fixed { elements } => {
                        assert_eq!(elements.len(), 2);
                        assert_node!(parser.tree, elements[0], Expression::ScalarLiteral(scalar_literal_id) => {
                            assert_float!(parser.tree, *scalar_literal_id, 1.0);
                        });
                        assert_node!(parser.tree, elements[1], Expression::ScalarLiteral(scalar_literal_id) => {
                            assert_float!(parser.tree, *scalar_literal_id, 2.0);
                        });
                    });
                });
            });

            // indices: 2
            assert_node!(parser.tree, fields[1], FieldLiteral::Named { name, value } => {
                assert_eq!(parser.strings.get(*name), "indices");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_int!(parser.tree, *scalar_literal_id, 2);
                });
            });
        });
    }
}
