use dyst_token::{NumberBase, RawLiteralType, TokenSpan, TokenType};
use std::borrow::Cow;

use crate::parse::prelude::*;
use crate::{
    ArrayLiteral, FieldLiteral, FloatType, IntType, NodeId, NodeType, ParseError, ParseResult,
    Parser, ScalarLiteral, StructLiteral, TupleLiteral, TupleLiteralField, TypeLiteral,
    UnaryOperator,
};

impl<'a> Parser<'a> {
    /// Peek a scalar literal.
    pub fn peek_scalar_literal(&self) -> ParseResult<&TokenSpan> {
        if self.peek_token(TokenType::Literal).is_ok() {
            Ok(self.peek()?)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a scalar literal.
    ///
    /// Examples:
    /// ```
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

        let literal_span = *self.eat()?;
        let Some(literal) = literal_span.token.body else {
            return Err(ParseError::unexpected(literal_span.span));
        };
        let literal_str = self.get_span_str(literal_span.span);

        match literal {
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
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    ));
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
                        let scalar_literal = self
                            .tree
                            .allocate(ScalarLiteral::Integer(value), self.get_span_from(start));
                        Ok(scalar_literal)
                    }
                    Err(_) => Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    )),
                }
            }

            // float literal
            RawLiteralType::Float {
                base: _,
                is_empty_exponent,
            } => {
                if is_empty_exponent {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    ));
                }
                let cleaned_str: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };
                let parsed_float = cleaned_str.parse::<f64>();
                match parsed_float {
                    Ok(value) => {
                        let scalar_literal = self
                            .tree
                            .allocate(ScalarLiteral::Float(value), self.get_span_from(start));
                        Ok(scalar_literal)
                    }
                    Err(_) => Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    )),
                }
            }

            // character literal (ignore quotes)
            RawLiteralType::Character { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    ));
                }
                let content = literal_str.trim_start_matches('\'').trim_end_matches('\'');
                if let Some(literal_char) = content.chars().next() {
                    let scalar_literal = self.tree.allocate(
                        ScalarLiteral::Character(literal_char),
                        self.get_span_from(start),
                    );
                    Ok(scalar_literal)
                } else {
                    Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    ))
                }
            }

            // byte character literal (ignore quotes)
            RawLiteralType::Byte { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    ));
                }
                let content = literal_str.trim_start_matches("b'").trim_end_matches('\'');
                if let Some(literal_char) = content.chars().next() {
                    let scalar_literal = self.tree.allocate(
                        ScalarLiteral::Byte(literal_char as u8),
                        self.get_span_from(start),
                    );
                    Ok(scalar_literal)
                } else {
                    Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    ))
                }
            }

            // string literal (ignore quotes)
            RawLiteralType::String { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    ));
                }
                let content = literal_str.trim_start_matches('"').trim_end_matches('"');
                let string_id = self.intern_string(content);
                let scalar_literal = self
                    .tree
                    .allocate(ScalarLiteral::String(string_id), self.get_span_from(start));
                Ok(scalar_literal)
            }

            // byte string literal (ignore quotes)
            RawLiteralType::ByteString { is_terminated } => {
                if !is_terminated {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    ));
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
                        return Err(ParseError::expected(literal_span.span, TokenType::Literal));
                    }
                    let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                    let string_id = self.intern_string(content);
                    let scalar_literal = self
                        .tree
                        .allocate(ScalarLiteral::String(string_id), self.get_span_from(start));
                    Ok(scalar_literal)
                } else {
                    Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    ))
                }
            }

            // raw byte string literal (ignore quotes and hashes)
            RawLiteralType::RawByteString { hashes } => {
                if let Some(hashes) = hashes {
                    let num_hashes = hashes as usize;
                    let prefix_len = 2 /* br */ + num_hashes + 1 /* opening " */;
                    let suffix_len = 1 /* closing " */ + num_hashes;
                    if literal_str.len() < prefix_len + suffix_len {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::ScalarLiteral,
                        ));
                    }
                    let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                    let bytes = content.as_bytes().to_vec();
                    let scalar_literal = self
                        .tree
                        .allocate(ScalarLiteral::ByteString(bytes), self.get_span_from(start));
                    Ok(scalar_literal)
                } else {
                    Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::ScalarLiteral,
                    ))
                }
            }
        }
    }

    /// Whether the token type is maybe the start of a (type-related) expression.
    fn is_start_of_expression(&self, token_type: TokenType) -> bool {
        token_type == TokenType::OpenParenthesis
            || token_type == TokenType::Identifier
            || token_type == TokenType::Literal
            // (if we're before a block then { is a terminator, not the start of a block)
            || (token_type == TokenType::OpenBrace && !self.options.in_before_block)
            || UnaryOperator::from_token_type(token_type).is_some()
    }

    /// Peek a primitive type (e.g., `!`, `void`, `boolean`, `int32`, `uint7`, `float32`).
    ///
    /// Examples:
    /// ```
    /// !
    /// $
    /// _
    /// undefined
    /// void
    /// null
    /// boolean
    /// character
    /// int32
    /// uint7
    /// float32
    /// ```
    pub fn peek_type_literal(&self) -> ParseResult<TypeLiteral> {
        let next = self.peek()?;
        let next_type = next.token.r#type;
        let next_next = self.peek_next();
        let next_next_type = next_next.as_ref().map(|next| next.token.r#type).ok();

        // !, $, _
        if (next_type == TokenType::Not
            || next_type == TokenType::Virtual
            || next_type == TokenType::Wildcard)
            // if next token doesn't start a related expression
            && (next_next_type.is_none()
                || (next_next_type.is_some()
                    && !self.is_start_of_expression(next_next_type.unwrap())))
        {
            return match next_type {
                TokenType::Not => Ok(TypeLiteral::Never),
                TokenType::Virtual => Ok(TypeLiteral::Any),
                TokenType::Wildcard => Ok(TypeLiteral::Infer),
                _ => unreachable!(),
            };
        }

        // regular single-token type literals
        let next_str = self.get_span_str(next.span);
        match next_str {
            // undefined
            "undefined" => Ok(TypeLiteral::Undefined),
            // void
            "void" => Ok(TypeLiteral::Void),
            // null
            "null" => Ok(TypeLiteral::Null),
            // boolean
            "boolean" => Ok(TypeLiteral::Boolean),
            // character
            "character" => Ok(TypeLiteral::Character),
            // Self
            "Self"
                // if next token doesn't start a related expression
                if next_next_type.is_none()
                    || (next_next_type.unwrap() != TokenType::OpenBrace)
                    || self.options.in_before_block =>
            {
                Ok(TypeLiteral::Self_)
            }
            // int_
            int_str if int_str.starts_with("int") && int_str.len() > 3 => {
                let Ok(width) = int_str.trim_start_matches("int").parse::<u16>() else {
                    return Err(ParseError::expected(next.span, TokenType::Literal));
                };
                Ok(TypeLiteral::Int(IntType {
                    width,
                    is_signed: true,
                }))
            }
            // uint_
            uint_str if uint_str.starts_with("uint") && uint_str.len() > 4 => {
                let Ok(width) = uint_str.trim_start_matches("uint").parse::<u16>() else {
                    return Err(ParseError::expected(next.span, TokenType::Literal));
                };
                Ok(TypeLiteral::Int(IntType {
                    width,
                    is_signed: false,
                }))
            }
            // float32
            "float32" => Ok(TypeLiteral::Float(FloatType::Float32)),
            // float64
            "float64" => Ok(TypeLiteral::Float(FloatType::Float64)),
            _ => Err(ParseError::expected(next.span, TokenType::Identifier)),
        }
    }

    /// Eat a type literal.
    pub fn eat_type_literal(&mut self) -> ParseResult<NodeId<TypeLiteral>> {
        let start = self.mark();
        let type_literal = self.peek_type_literal()?;
        self.bump(); // eat type literal
        Ok(self.tree.allocate(type_literal, self.get_span_from(start)))
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
    /// (10, false, "Hi") // heterogenous tuple is okay because it's a tuple
    /// ```
    pub fn eat_tuple_literal(&mut self) -> ParseResult<NodeId<TupleLiteral>> {
        let start = self.mark();

        self.eat_token(TokenType::OpenParenthesis)
            .for_node_type(NodeType::TupleLiteral)?;
        self.eat_newlines_maybe()?;

        // empty tuple
        if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            self.bump();
            let tuple_literal = self
                .tree
                .allocate(TupleLiteral { elements: vec![] }, self.get_span_from(start));
            return Ok(tuple_literal);
        }

        // parse remaining elements separated by comma or newline, allow trailing comma
        let elements = self
            .eat_tuple_literal_body(None)
            .for_node_type(NodeType::TupleLiteral)?;
        self.eat_token(TokenType::CloseParenthesis)
            .for_node_type(NodeType::TupleLiteral)?;
        let tuple_literal = self
            .tree
            .allocate(TupleLiteral { elements }, self.get_span_from(start));
        Ok(tuple_literal)
    }

    /// Eat the body of a tuple literal (excluding the parenthesis).
    pub fn eat_tuple_literal_body(
        &mut self,
        first_element: Option<NodeId<TupleLiteralField>>,
    ) -> ParseResult<Vec<NodeId<TupleLiteralField>>> {
        let mut elements: Vec<NodeId<TupleLiteralField>> = vec![];
        if let Some(first_element) = first_element {
            elements.push(first_element);
        }
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
                let element = self
                    .eat_tuple_literal_field()
                    .for_node_type(NodeType::TupleLiteral)?;
                elements.push(element);
            }
        }
        Ok(elements)
    }

    /// Eat a tuple literal field.
    pub fn eat_tuple_literal_field(&mut self) -> ParseResult<NodeId<TupleLiteralField>> {
        let start = self.mark();
        // named field
        if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let name = self.eat_identifier()?;
            self.eat_token(TokenType::Colon)?;
            let value = self.eat_expression()?;
            let field_literal = self.tree.allocate(
                TupleLiteralField::Named { name, value },
                self.get_span_from(start),
            );
            Ok(field_literal)
        }
        // positional field
        else {
            let value = self.eat_expression()?;
            let field_literal = self.tree.allocate(
                TupleLiteralField::Positional { value },
                self.get_span_from(start),
            );
            Ok(field_literal)
        }
    }

    /// Eat an array literal (fixed).
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
    /// ```
    pub fn eat_array_literal(&mut self) -> ParseResult<NodeId<ArrayLiteral>> {
        let start = self.mark();

        self.eat_token(TokenType::OpenBracket)
            .for_node_type(NodeType::ArrayLiteral)?;
        self.eat_newlines_maybe()?;

        // eat everything
        let mut elements = vec![];
        loop {
            // stop on closing bracket
            if self.peek_token(TokenType::CloseBracket).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            }
            // keep eating elements
            else {
                let element = self
                    .eat_expression()
                    .for_node_type(NodeType::ArrayLiteral)?;
                elements.push(element);
            }
        }

        self.eat_token(TokenType::CloseBracket)
            .for_node_type(NodeType::ArrayLiteral)?;
        let array_literal = self
            .tree
            .allocate(ArrayLiteral::Fixed { elements }, self.get_span_from(start));
        Ok(array_literal)
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
        let r#type = self
            .with_options(self.options.in_before_block(), |parser| {
                parser.eat_expression()
            })
            .for_node_type(NodeType::StructLiteral)?;
        let fields = self
            .eat_struct_literal_body()
            .for_node_type(NodeType::StructLiteral)?;
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
    /// // multi-line struct with optional comma
    /// {
    ///    x: 1.0
    ///    y: 2.0
    ///    z
    /// }
    /// ```
    pub(crate) fn eat_struct_literal_body(&mut self) -> ParseResult<Vec<NodeId<FieldLiteral>>> {
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
                let value = self.eat_expression()?;
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
        ArrayLiteral, Expression, FieldLiteral, FloatType, IntType, ScalarLiteral, StructLiteral,
        TupleLiteral, TupleLiteralField, TypeLiteral, assert_bool, assert_char, assert_float,
        assert_int, assert_lit_string, assert_node,
    };

    /// Parse integer literals in various formats.
    #[test]
    fn test_parse_integer_literal() {
        let mut test = TestParser::new("1 731 0x1234");
        let mut parser = test.prepare();

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_int!(parser.tree, literal_id, 1);

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_int!(parser.tree, literal_id, 731);

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_int!(parser.tree, literal_id, 0x1234);
    }

    /// Parse scientific notation and decimal floats.
    #[test]
    fn test_parse_float_literal() {
        let mut test = TestParser::new("10e37 1.0");
        let mut parser = test.prepare();

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_float!(parser.tree, literal_id, 1.0e38);

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_float!(parser.tree, literal_id, 1.0);
    }

    /// Parse true and false literals.
    #[test]
    fn test_parse_boolean_literal() {
        let mut test = TestParser::new("true false");
        let mut parser = test.prepare();

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_bool!(parser.tree, literal_id, true);

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_bool!(parser.tree, literal_id, false);
    }

    /// Parse character and byte literals.
    #[test]
    fn test_parse_character_literal() {
        let mut test = TestParser::new("'a' b'a'");
        let mut parser = test.prepare();

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_char!(parser.tree, literal_id, 'a');

        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_node!(parser.tree, literal_id, ScalarLiteral::Byte(b'a'));
    }

    /// Parse various string literal formats including raw and byte strings.
    #[test]
    fn test_parse_string_literal() {
        let mut test = TestParser::new(
            r###""Hello, world!" b"abc" r"abc" r##"a#b#c"## br"abc" br##"a#b#c"##"###,
        );
        let mut parser = test.prepare();

        // "Hello, world!"
        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_lit_string!(parser.session, parser.tree.get(literal_id), "Hello, world!");

        // b"abc"
        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_node!(parser.tree, literal_id, ScalarLiteral::ByteString(bytes) => {
            let str = std::str::from_utf8(bytes).unwrap();
            assert_eq!(str, "abc");
        });

        // r"abc"
        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_lit_string!(parser.session, parser.tree.get(literal_id), "abc");

        // r##"a#b#c"##
        let literal_id = parser.eat_scalar_literal().unwrap();
        assert_lit_string!(parser.session, parser.tree.get(literal_id), "a#b#c");

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

    /// Parse empty array literal.
    #[test]
    fn test_parse_empty_array_literal() {
        // []
        let mut test = TestParser::new("[]");
        let mut parser = test.prepare();

        let literal_id = parser.eat_array_literal().unwrap();
        assert_node!(parser.tree, literal_id, ArrayLiteral::Fixed { elements } => {
            assert_eq!(elements.len(), 0);
        });
    }

    #[test]
    fn test_parse_single_element_array_literal() {
        // [1, ]
        let mut test = TestParser::new("[1, ]");
        let mut parser = test.prepare();

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
        let mut test = TestParser::new(r###"[10, false, "Hi"]"###);
        let mut parser = test.prepare();

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
                assert_lit_string!(
                    parser.session,
                    parser.tree.get(*scalar_literal_id),
                    "Hi"
                );
            });
        });
    }

    /// Multi-line array with implicit comma separation.
    #[test]
    fn test_parse_multiline_array_literal() {
        let mut test = TestParser::new(
            r###"[1.0, 
     2.0
     3.0
    ]"###,
        );
        let mut parser = test.prepare();

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
    fn test_parse_single_element_tuple_literal() {
        // (1, )
        let mut test = TestParser::new("(1, )");
        let mut parser = test.prepare();

        let literal_id = parser.eat_tuple_literal().unwrap();
        assert_node!(parser.tree, literal_id, TupleLiteral { elements } => {
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], TupleLiteralField::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_int!(parser.tree, *scalar_literal_id, 1);
                });
            });
        });
    }

    #[test]
    fn test_parse_multi_element_tuple_literal() {
        // (10, false, "Hi")
        let mut test = TestParser::new(r###"(10, false, "Hi")"###);
        let mut parser = test.prepare();

        let literal_id = parser.eat_tuple_literal().unwrap();
        assert_node!(parser.tree, literal_id, TupleLiteral { elements } => {
            assert_eq!(elements.len(), 3);

            // [0] = 10
            assert_node!(parser.tree, elements[0], TupleLiteralField::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_int!(parser.tree, *scalar_literal_id, 10);
                });
            });

            // [1] = false
            assert_node!(parser.tree, elements[1], TupleLiteralField::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_bool!(parser.tree, *scalar_literal_id, false);
                });
            });

            // [2] = "Hi"
            assert_node!(parser.tree, elements[2], TupleLiteralField::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_lit_string!(
                        parser.session,
                        parser.tree.get(*scalar_literal_id),
                        "Hi"
                    );
                });
            });
        });
    }

    #[test]
    fn test_parse_multiline_tuple_literal() {
        // Multi-line tuple with implicit comma separation
        let mut test = TestParser::new(
            r###"(
  1.0
  2.0
  3.0
)"###,
        );
        let mut parser = test.prepare();

        let literal_id = parser.eat_tuple_literal().unwrap();
        assert_node!(parser.tree, literal_id, TupleLiteral { elements } => {
            assert_eq!(elements.len(), 3);

            // [0] = 1.0
            assert_node!(parser.tree, elements[0], TupleLiteralField::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_float!(parser.tree, *scalar_literal_id, 1.0);
                });
            });

            // [1] = 2.0
            assert_node!(parser.tree, elements[1], TupleLiteralField::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_float!(parser.tree, *scalar_literal_id, 2.0);
                });
            });

            // [2] = 3.0
            assert_node!(parser.tree, elements[2], TupleLiteralField::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_float!(parser.tree, *scalar_literal_id, 3.0);
                });
            });
        });
    }

    #[test]
    fn test_parse_struct_literal() {
        let mut test = TestParser::new(
            r##"
destack.geometry.Mesh<2, int32> {
    vertices: [1.0, 2.0] // optional comma
    indices: 2, // optional comma
}
            "##,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct_literal().unwrap();
        assert_node!(parser.tree, struct_id, StructLiteral { r#type: _, fields } => {
            assert_eq!(fields.len(), 2);

            // vertices: []
            assert_node!(parser.tree, fields[0], FieldLiteral::Named { name, value } => {
                assert_eq!(parser.get_string(*name), "vertices");
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
                assert_eq!(parser.get_string(*name), "indices");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_int!(parser.tree, *scalar_literal_id, 2);
                });
            });
        });
    }

    /// Peek `!` as `Never` and reject glued tokens.
    #[test]
    fn test_peek_type_literal_never_boundaries() {
        let mut test = TestParser::new("!");
        let parser = test.prepare();
        let literal = parser.peek_type_literal().unwrap();
        assert_eq!(literal, TypeLiteral::Never);

        let mut test = TestParser::new("!1");
        let parser = test.prepare();
        assert!(parser.peek_type_literal().is_err());
    }

    /// Peek `$` as `Any` and reject glued tokens.
    #[test]
    fn test_peek_type_literal_any_boundaries() {
        let mut test = TestParser::new("$");
        let parser = test.prepare();
        let literal = parser.peek_type_literal().unwrap();
        assert_eq!(literal, TypeLiteral::Any);

        let mut test = TestParser::new("$1");
        let parser = test.prepare();
        assert!(parser.peek_type_literal().is_err());
    }

    /// Peek `_` as `Infer` and reject glued tokens.
    #[test]
    fn test_peek_type_literal_infer_boundaries() {
        let mut test = TestParser::new("_");
        let parser = test.prepare();
        let literal = parser.peek_type_literal().unwrap();
        assert_eq!(literal, TypeLiteral::Infer);

        let mut test = TestParser::new("_1");
        let parser = test.prepare();
        assert!(parser.peek_type_literal().is_err());
    }

    /// Peek keyword-based primitive type literals.
    #[test]
    fn test_peek_type_literal_keywords() {
        let mut test = TestParser::new("undefined void null boolean character Self");
        let mut parser = test.prepare();

        let undefined = parser.peek_type_literal().unwrap();
        assert_eq!(undefined, TypeLiteral::Undefined);
        parser.bump();

        let void = parser.peek_type_literal().unwrap();
        assert_eq!(void, TypeLiteral::Void);
        parser.bump();

        let null = parser.peek_type_literal().unwrap();
        assert_eq!(null, TypeLiteral::Null);
        parser.bump();

        let boolean = parser.peek_type_literal().unwrap();
        assert_eq!(boolean, TypeLiteral::Boolean);
        parser.bump();

        let character = parser.peek_type_literal().unwrap();
        assert_eq!(character, TypeLiteral::Character);
        parser.bump();

        let self_literal = parser.peek_type_literal().unwrap();
        assert_eq!(self_literal, TypeLiteral::Self_);
    }

    /// Peek signed integer type literals with different widths.
    #[test]
    fn test_peek_type_literal_ints() {
        let mut test = TestParser::new("int2 int32");
        let mut parser = test.prepare();

        let int2 = parser.peek_type_literal().unwrap();
        assert_eq!(
            int2,
            TypeLiteral::Int(IntType {
                width: 2,
                is_signed: true,
            })
        );
        parser.bump();

        let int32 = parser.peek_type_literal().unwrap();
        assert_eq!(
            int32,
            TypeLiteral::Int(IntType {
                width: 32,
                is_signed: true,
            })
        );
    }

    /// Peek floating point type literals.
    #[test]
    fn test_peek_type_literal_floats() {
        let mut test = TestParser::new("float32 float64");
        let mut parser = test.prepare();

        let float32 = parser.peek_type_literal().unwrap();
        assert_eq!(float32, TypeLiteral::Float(FloatType::Float32));
        parser.bump();

        let float64 = parser.peek_type_literal().unwrap();
        assert_eq!(float64, TypeLiteral::Float(FloatType::Float64));
    }
}
