use std::borrow::Cow;

use crate::parse::prelude::*;
use crate::{
    Argument, Expression, FloatType, IntType, NodeId, NodeType, NumberBase, Parser, ParserError,
    ParserResult, RawLiteralType, ScalarLiteral, TokenSpan, TokenType, TypeLiteral, UnaryOperator,
};

impl<'a> Parser<'a> {
    /// Peek a scalar literal token.
    #[inline]
    pub fn peek_scalar_literal(&self) -> ParserResult<&TokenSpan> {
        if self.peek_token(TokenType::Literal).is_ok() {
            Ok(self.peek()?)
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a scalar literal and return its value.
    ///
    /// Examples:
    /// ```
    /// true
    /// false
    /// 1
    /// 1.0
    /// 0x1234
    /// "hello"
    /// ```
    pub fn eat_scalar_literal(&mut self) -> ParserResult<ScalarLiteral> {
        let literal_span = *self.eat()?;
        let Some(body) = literal_span.token.body else {
            return Err(ParserError::unexpected(literal_span.span));
        };
        let literal_str = self.get_span_str(literal_span.span);

        match body {
            // boolean literal
            RawLiteralType::Boolean { value } => Ok(ScalarLiteral::Boolean(value)),

            // int literal
            RawLiteralType::Int { base, is_empty } => {
                if is_empty {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // strip underscores for parsing
                let cleaned: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                // handle base-specific prefixes
                let parsed = match base {
                    NumberBase::Decimal => cleaned.parse::<i64>(),
                    NumberBase::Binary => i64::from_str_radix(cleaned.trim_start_matches("0b"), 2),
                    NumberBase::Octal => i64::from_str_radix(cleaned.trim_start_matches("0o"), 8),
                    NumberBase::Hexadecimal => {
                        i64::from_str_radix(cleaned.trim_start_matches("0x"), 16)
                    }
                };

                parsed.map(ScalarLiteral::Integer).map_err(|_| {
                    ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    )
                })
            }

            // float literal
            RawLiteralType::Float {
                base: _,
                is_empty_exponent,
            } => {
                if is_empty_exponent {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                let cleaned: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                cleaned
                    .parse::<f64>()
                    .map(ScalarLiteral::Float)
                    .map_err(|_| {
                        ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
            }

            // character literal (ignore quotes)
            RawLiteralType::Character { is_terminated } => {
                if !is_terminated {
                    return Err(ParserError::expected_for(
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
                        ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
            }

            // byte character literal (ignore quotes)
            RawLiteralType::Byte { is_terminated } => {
                if !is_terminated {
                    return Err(ParserError::expected_for(
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
                        ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
            }

            // string literal (ignore quotes)
            RawLiteralType::String { is_terminated } => {
                if !is_terminated {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = literal_str.trim_start_matches('"').trim_end_matches('"');
                let string_id = self.intern_string(content);
                Ok(ScalarLiteral::String(string_id))
            }

            // byte string literal (ignore quotes)
            RawLiteralType::ByteString { is_terminated } => {
                if !is_terminated {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = literal_str.trim_start_matches("b\"").trim_end_matches('"');
                Ok(ScalarLiteral::ByteString(content.as_bytes().to_vec()))
            }

            // raw string literal (ignore quotes and hashes)
            RawLiteralType::RawString { hashes } => {
                let Some(hashes) = hashes else {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                };
                let num_hashes = hashes as usize;
                let prefix_len = 1 /* r */ + num_hashes + 1 /* opening " */;
                let suffix_len = 1 /* closing " */ + num_hashes;
                if literal_str.len() < prefix_len + suffix_len {
                    return Err(ParserError::expected(literal_span.span, TokenType::Literal));
                }
                let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                let string_id = self.intern_string(content);
                Ok(ScalarLiteral::String(string_id))
            }

            // raw byte string literal (ignore quotes and hashes)
            RawLiteralType::RawByteString { hashes } => {
                let Some(hashes) = hashes else {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                };
                let num_hashes = hashes as usize;
                let prefix_len = 2 /* br */ + num_hashes + 1 /* opening " */;
                let suffix_len = 1 /* closing " */ + num_hashes;
                if literal_str.len() < prefix_len + suffix_len {
                    return Err(ParserError::expected_for(
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

    /// Whether the token type can start an expression.
    fn is_start_of_expression(&self, token_type: TokenType) -> bool {
        token_type == TokenType::OpenParenthesis
            || token_type == TokenType::Identifier
            || token_type == TokenType::Literal
            // (if we're before a block then { is a terminator, not the start of a block)
            || (token_type == TokenType::OpenBrace && !self.options.in_before_block)
            || UnaryOperator::from_token_type(token_type).is_some()
    }

    /// Whether the token string encodes a type literal with an explicit width.
    fn is_type_with_width(&self, prefix: &'static str, target: &str) -> Option<u16> {
        if let Some(target) = target.strip_prefix(prefix) {
            target.parse::<u16>().ok()
        } else {
            None
        }
    }

    /// Peek a type literal.
    pub fn peek_type_literal(&self) -> ParserResult<TypeLiteral> {
        let next = self.peek()?;
        let next_type = next.token.ty;
        let next_next = self.peek_next();
        let next_next_type = next_next.as_ref().map(|next| next.token.ty).ok();

        // !, $, _
        if (next_type == TokenType::Not
            || next_type == TokenType::Virtual
            || next_type == TokenType::Wildcard)
            // if next token doesn't start a related expression
            && (next_next_type.is_none() || !self.is_start_of_expression(next_next_type.unwrap()))
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
            "boolean" | "bool" => Ok(TypeLiteral::Boolean),
            // character
            "character" | "char" => Ok(TypeLiteral::Character),
            // string
            "string" | "str" => Ok(TypeLiteral::String),
            // number
            "number" => Ok(TypeLiteral::Number),
            // Self
            "Self"
                // if next token doesn't start a related expression
                if next_next_type.is_none()
                    || next_next_type.unwrap() != TokenType::OpenBrace
                    || self.options.in_before_block =>
            {
                Ok(TypeLiteral::Self_)
            }
            // int (followed by number or nothing)
            "int" => Ok(TypeLiteral::Int(IntType {
                width: None,
                is_signed: true,
            })),
            int_str if let Some(width) = self.is_type_with_width("int", int_str) => {
                Ok(TypeLiteral::Int(IntType {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            int_str if let Some(width) = self.is_type_with_width("i", int_str) => {
                Ok(TypeLiteral::Int(IntType {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            // uint (followed by number or nothing)
            "uint" => Ok(TypeLiteral::Int(IntType {
                width: None,
                is_signed: false,
            })),
            uint_str if let Some(width) = self.is_type_with_width("uint", uint_str) => {
                Ok(TypeLiteral::Int(IntType {
                    width: Some(width),
                    is_signed: false,
                }))
            }
            uint_str if let Some(width) = self.is_type_with_width("u", uint_str) => {
                Ok(TypeLiteral::Int(IntType {
                    width: Some(width),
                    is_signed: false,
                }))
            }
            // float (followed by number or nothing)
            "float" => Ok(TypeLiteral::Float(FloatType { width: None })),
            float_str if let Some(width) = self.is_type_with_width("float", float_str) => {
                Ok(TypeLiteral::Float(FloatType { width: Some(width) }))
            }
            float_str if let Some(width) = self.is_type_with_width("f", float_str) => {
                Ok(TypeLiteral::Float(FloatType { width: Some(width) }))
            }
            // composite type
            _ => Err(ParserError::unexpected(next.span)),
        }
    }

    /// Eat a type literal and return its value.
    pub fn eat_type_literal(&mut self) -> ParserResult<TypeLiteral> {
        let literal = self.peek_type_literal()?;
        self.bump();
        Ok(literal)
    }

    /// Eat the body of a tuple literal (excluding the surrounding parenthesis).
    pub fn eat_tuple_literal_body(
        &mut self,
        first_element: Option<NodeId<Argument>>,
    ) -> ParserResult<Vec<NodeId<Argument>>> {
        let mut elements = Vec::new();
        if let Some(first) = first_element {
            elements.push(first);
        }
        loop {
            // stop at closing parenthesis
            if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
                continue;
            }

            // keep eating elements
            let element = self
                .eat_tuple_literal_element()
                .for_node_type(NodeType::Argument)?;
            elements.push(element);
        }
        Ok(elements)
    }

    /// Eat a single tuple literal element.
    pub fn eat_tuple_literal_element(&mut self) -> ParserResult<NodeId<Argument>> {
        let start = self.mark();
        // named field
        if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let name = self.eat_identifier()?;
            self.eat_token(TokenType::Colon)?;
            let value = self.eat_expression()?;
            let argument_id = self
                .tree
                .allocate(Argument::Named { name, value }, self.get_span_from(start));
            Ok(argument_id)
        }
        // positional field
        else {
            let value = self.eat_expression()?;
            let argument_id = self
                .tree
                .allocate(Argument::Positional { value }, self.get_span_from(start));
            Ok(argument_id)
        }
    }

    /// Eat an array literal body and return its element expressions.
    pub fn eat_array_literal(&mut self) -> ParserResult<Vec<NodeId<Expression>>> {
        self.eat_token(TokenType::OpenBracket)
            .for_node_type(NodeType::Expression)?;
        self.eat_newlines_maybe()?;

        let mut elements = Vec::new();
        loop {
            // stop at closing bracket
            if self.peek_token(TokenType::CloseBracket).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
                continue;
            }
            // keep eating elements
            let element = self.eat_expression().for_node_type(NodeType::Expression)?;
            elements.push(element);
        }

        self.eat_token(TokenType::CloseBracket)
            .for_node_type(NodeType::Expression)?;
        Ok(elements)
    }

    /// Eat the body of a struct literal (excluding the receiver expression).
    pub(crate) fn eat_struct_literal_body(&mut self) -> ParserResult<Vec<NodeId<Argument>>> {
        self.eat_token(TokenType::OpenBrace)
            .for_node_type(NodeType::Expression)?;
        self.eat_newlines_maybe()?;

        // empty struct
        if self.peek_token(TokenType::CloseBrace).is_ok() {
            self.eat_token(TokenType::CloseBrace)
                .for_node_type(NodeType::Expression)?;
            return Ok(vec![]);
        }

        let mut fields = Vec::new();
        loop {
            // stop at closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }

            let start = self.mark();
            let name = self.eat_identifier()?;
            // named field
            let field_id = if self.peek_token(TokenType::Colon).is_ok() {
                self.eat_token(TokenType::Colon)?;
                let value = self.eat_expression()?;
                self.tree
                    .allocate(Argument::Named { name, value }, self.get_span_from(start))
            }
            // shorthand field
            else {
                self.tree
                    .allocate(Argument::NamedShorthand { name }, self.get_span_from(start))
            };
            fields.push(field_id);

            // item stop
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
            }
        }

        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Expression)?;
        Ok(fields)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{Argument, Expression, ScalarLiteral, TokenType, TypeLiteral};

    /// Parse integer literals in various formats.
    #[test]
    fn test_parse_integer_literal() {
        let mut test = TestParser::new("1 731 0x1234");
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
    }

    /// Parse scientific notation and decimal floats.
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
        let mut test = TestParser::new(r#""hello" b"abc""#);
        let mut parser = test.prepare();

        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(
            parser.strings.get(match literal {
                ScalarLiteral::String(id) => id,
                other => panic!("expected string literal, got {other:?}"),
            }),
            "hello"
        );

        let literal = parser.eat_scalar_literal().unwrap();
        match literal {
            ScalarLiteral::ByteString(bytes) => assert_eq!(bytes, b"abc"),
            other => panic!("expected byte string literal, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_type_literal() {
        let mut test = TestParser::new("int32 uint8 float bool");
        let mut parser = test.prepare();

        assert!(
            matches!(parser.eat_type_literal().unwrap(), TypeLiteral::Int(int_ty) if int_ty.width == Some(32))
        );
        assert!(
            matches!(parser.eat_type_literal().unwrap(), TypeLiteral::Int(int_ty) if !int_ty.is_signed && int_ty.width == Some(8))
        );
        assert!(
            matches!(parser.eat_type_literal().unwrap(), TypeLiteral::Float(float_ty) if float_ty.width.is_none())
        );
        assert!(matches!(
            parser.eat_type_literal().unwrap(),
            TypeLiteral::Boolean
        ));
    }

    #[test]
    fn test_struct_literal_body() {
        let mut test = TestParser::new("{ x: 1, y } ");
        let mut parser = test.prepare();

        let fields = parser.eat_struct_literal_body().unwrap();
        assert_eq!(fields.len(), 2);
        assert!(matches!(parser.tree.get(fields[0]), Argument::Named { .. }));
        assert!(matches!(
            parser.tree.get(fields[1]),
            Argument::NamedShorthand { .. }
        ));
    }

    #[test]
    fn test_tuple_literal_body() {
        let mut test = TestParser::new("(a: 1, 2)");
        let mut parser = test.prepare();

        parser.eat_token(TokenType::OpenParenthesis).unwrap();
        let first = parser.eat_tuple_literal_element().unwrap();
        let elements = parser.eat_tuple_literal_body(Some(first)).unwrap();
        assert_eq!(elements.len(), 2);
        assert!(matches!(
            parser.tree.get(elements[0]),
            Argument::Named { .. }
        ));
        assert!(matches!(
            parser.tree.get(elements[1]),
            Argument::Positional { .. }
        ));
    }

    #[test]
    fn test_array_literal() {
        let mut test = TestParser::new("[1, 2]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        assert_eq!(elements.len(), 2);
        assert!(matches!(
            parser.tree.get(elements[0]),
            Expression::ScalarLiteral(ScalarLiteral::Integer(1))
        ));
        assert!(matches!(
            parser.tree.get(elements[1]),
            Expression::ScalarLiteral(ScalarLiteral::Integer(2))
        ));
    }
}
