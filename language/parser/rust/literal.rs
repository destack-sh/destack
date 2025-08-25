use destack_language_lexer::{LiteralToken, NumberBase, TokenType};
use std::borrow::Cow;

use crate::{
    ArrayLiteral, Expression, FieldLiteral, FloatType, IntType, ParseError, ParseResult, Parser,
    ScalarLiteral, StructLiteral, TupleLiteral,
};

impl<'a> Parser<'a> {
    /// Eat a literal.
    pub fn eat_literal(&mut self) -> ParseResult<Expression> {
        // array
        if self.peek_next_token(TokenType::OpenBracket).is_ok() {
            let array_literal = self.eat_array_literal()?;
            Ok(Expression::ArrayLiteral(array_literal))
        // tuple
        } else if self.peek_next_token(TokenType::OpenParenthesis).is_ok() {
            let tuple_literal = self.eat_tuple_literal()?;
            Ok(Expression::TupleLiteral(tuple_literal))
        // struct
        } else if self.peek_next_token(TokenType::OpenBrace).is_ok() {
            let struct_literal = self.eat_struct_literal()?;
            Ok(Expression::StructLiteral(struct_literal))
        // scalar
        } else {
            let scalar_literal = self.eat_scalar_literal()?;
            Ok(Expression::ScalarLiteral(scalar_literal))
        }
    }

    /// Eat a scalar literal.
    ///
    /// Examples:
    /// ```
    /// 1
    /// 1.0
    /// 7
    /// "Hello, world!"
    /// 'a'
    /// b'a'
    /// b"abc"
    /// 0x1234
    /// true
    /// false
    /// ```
    pub fn eat_scalar_literal(&mut self) -> ParseResult<ScalarLiteral> {
        if self.peek_next_token(TokenType::Identifier).is_ok() {
            // may be reference to a built-in constant value
            // boolean literal (just an identifier)
            let identifier = self.eat_identifier()?;
            if identifier == "true" {
                Ok(ScalarLiteral::Boolean(true))
            } else if identifier == "false" {
                Ok(ScalarLiteral::Boolean(false))
            } else {
                Err(ParseError::UnexpectedToken(self.prev().unwrap().span))
            }
        } else {
            // regular literal
            let literal = &self.eat_next()?.clone();
            let literal_str = self.get_span_str(literal.span);
            let TokenType::Literal(r#type) = literal.token.r#type else {
                return Err(ParseError::UnexpectedToken(literal.span));
            };

            match r#type {
                // int literal
                LiteralToken::Int { base, is_empty } => {
                    if is_empty {
                        return Err(ParseError::UnexpectedToken(literal.span));
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
                        Ok(value) => Ok(ScalarLiteral::Integer(value, IntType::Int32)),
                        Err(_) => Err(ParseError::UnexpectedToken(literal.span)),
                    }
                }
                // float literal
                LiteralToken::Float {
                    base: _,
                    is_empty_exponent,
                } => {
                    if is_empty_exponent {
                        return Err(ParseError::UnexpectedToken(literal.span));
                    }
                    let cleaned_str: Cow<'_, str> = if literal_str.contains('_') {
                        Cow::Owned(literal_str.replace('_', ""))
                    } else {
                        Cow::Borrowed(literal_str)
                    };
                    let parsed_float = cleaned_str.parse::<f64>();
                    match parsed_float {
                        Ok(value) => Ok(ScalarLiteral::Float(value, FloatType::Float32)),
                        Err(_) => Err(ParseError::UnexpectedToken(literal.span)),
                    }
                }
                // character literal (ignore quotes)
                LiteralToken::Character { is_terminated } => {
                    if !is_terminated {
                        return Err(ParseError::UnexpectedToken(literal.span));
                    }
                    let content = literal_str.trim_start_matches('\'').trim_end_matches('\'');
                    if let Some(literal_char) = content.chars().next() {
                        Ok(ScalarLiteral::Character(literal_char))
                    } else {
                        Err(ParseError::UnexpectedToken(literal.span))
                    }
                }
                // byte character literal (ignore quotes)
                LiteralToken::Byte { is_terminated } => {
                    if !is_terminated {
                        return Err(ParseError::UnexpectedToken(literal.span));
                    }
                    let content = literal_str.trim_start_matches("b'").trim_end_matches('\'');
                    if let Some(literal_char) = content.chars().next() {
                        Ok(ScalarLiteral::Byte(literal_char as u8))
                    } else {
                        Err(ParseError::UnexpectedToken(literal.span))
                    }
                }
                // string literal (ignore quotes)
                LiteralToken::String { is_terminated } => {
                    if !is_terminated {
                        return Err(ParseError::UnexpectedToken(literal.span));
                    }
                    let content = literal_str.trim_start_matches('"').trim_end_matches('"');
                    Ok(ScalarLiteral::String(content.to_string()))
                }
                // byte string literal (ignore quotes)
                LiteralToken::ByteString { is_terminated } => {
                    if !is_terminated {
                        return Err(ParseError::UnexpectedToken(literal.span));
                    }
                    let content = literal_str.trim_start_matches("b\"").trim_end_matches('"');
                    let bytes = content.as_bytes().to_vec();
                    Ok(ScalarLiteral::ByteString(bytes))
                }
                // raw string literal (ignore quotes and hashes)
                LiteralToken::RawString { hashes } => {
                    if let Some(hashes) = hashes {
                        let num_hashes = hashes as usize;
                        let prefix_len = 1 /* r */ + num_hashes + 1 /* opening " */;
                        let suffix_len = 1 /* closing " */ + num_hashes;
                        if literal_str.len() < prefix_len + suffix_len {
                            return Err(ParseError::UnexpectedToken(literal.span));
                        }
                        let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                        Ok(ScalarLiteral::String(content.to_string()))
                    } else {
                        Err(ParseError::UnexpectedToken(literal.span))
                    }
                }
                // raw byte string literal (ignore quotes and hashes)
                LiteralToken::RawByteString { hashes } => {
                    if let Some(hashes) = hashes {
                        let num_hashes = hashes as usize;
                        let prefix_len = 2 /* br */ + num_hashes + 1 /* opening " */;
                        let suffix_len = 1 /* closing " */ + num_hashes;
                        if literal_str.len() < prefix_len + suffix_len {
                            return Err(ParseError::UnexpectedToken(literal.span));
                        }
                        let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                        let bytes = content.as_bytes().to_vec();
                        Ok(ScalarLiteral::ByteString(bytes))
                    } else {
                        Err(ParseError::UnexpectedToken(literal.span))
                    }
                }
            }
        }
    }

    /// Eat an array literal (fixed or repeated).
    /// The individual elements are full expressions, not just literals.
    ///
    /// Examples:
    /// ```
    /// [1, 2, ] // trailing comma is allowed
    /// [1.0, 2.0, .0]
    /// [10, false, "Hi"] // hetereogenous array is invalid but okay in AST
    /// [0; 10] // repeated array
    /// [false; 40] // repeated array
    pub fn eat_array_literal(&mut self) -> ParseResult<ArrayLiteral> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut elements: Vec<Expression> = vec![];
        while self.peek_next_token(TokenType::Comma).is_ok() {
            self.eat_token(TokenType::Comma)?;
            let element = self.eat_expression()?;
            elements.push(element);
        }
        self.eat_token(TokenType::CloseBracket)?;
        Ok(ArrayLiteral::Fixed { elements })
    }

    /// Eat a tuple literal.
    /// The individual elements are full expressions, not just literals.
    ///
    /// Examples:
    /// ```
    /// (1, 2, ) // trailing comma is allowed
    /// (1.0, 2.0, .0)
    /// (10, false, "Hi") // okay because it's a tuple
    /// ```
    pub fn eat_tuple_literal(&mut self) -> ParseResult<TupleLiteral> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut elements: Vec<Expression> = vec![];
        while self.peek_next_token(TokenType::Comma).is_ok() {
            self.eat_token(TokenType::Comma)?;
            let element = self.eat_expression()?;
            elements.push(element);
        }
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(TupleLiteral { elements })
    }

    /// Eat a struct literal (including the type prefix).
    ///
    /// Examples:
    /// ```
    /// Vector2 { x: 1.0, y: 2.0, z }
    ///
    /// destack.geometry.Mesh2 {
    ///     vertices: [Vector3 { x: 1.0, y: 2.0, z: .0 }],
    ///     indices: [0, 1, 2]
    /// }
    ///
    /// Mesh2<float64> { something: [] }
    /// ```
    pub fn eat_struct_literal(&mut self) -> ParseResult<StructLiteral> {
        let r#type = self.eat_type()?;
        let fields = self.eat_struct_literal_body()?;
        Ok(StructLiteral { r#type, fields })
    }

    /// Eat the body of a struct literal (excluding the path prefix).
    ///
    /// Examples:
    /// ```
    /// { x: 1.0, y: 2.0, z }
    /// ```
    pub fn eat_struct_literal_body(&mut self) -> ParseResult<Vec<FieldLiteral>> {
        self.eat_token(TokenType::OpenBrace)?;
        let mut fields: Vec<FieldLiteral> = vec![];
        while self.peek_next_token(TokenType::Comma).is_ok() {
            self.eat_token(TokenType::Comma)?;
            let name = self.eat_identifier()?;
            if self.peek_next_token(TokenType::Colon).is_ok() {
                self.eat_token(TokenType::Colon)?;
                let value = self.eat_expression()?;
                fields.push(FieldLiteral {
                    name,
                    value: Some(value),
                });
            } else {
                fields.push(FieldLiteral { name, value: None });
            }
        }
        self.eat_token(TokenType::CloseBrace)?;
        Ok(fields)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_lexer::{SourceFile, tokenize_semantic};

    use crate::{ArrayLiteral, Expression, FloatType, IntType, Parser, ScalarLiteral};

    #[test]
    fn test_scalar_literal() {
        let input = r###"
1
731
0x1234
10e37
1.0
true
false
'a'
b'a'
"Hello, world!"
b"abc"
r"abc"
r##"a#b#c"##
br"abc"
br##"a#b#c"##
        "###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // 1
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::Integer(1, IntType::Int32));
        parser.eat_newline().unwrap();

        // 731
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::Integer(731, IntType::Int32));
        parser.eat_newline().unwrap();

        // 0x1234
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::Integer(0x1234, IntType::Int32));
        parser.eat_newline().unwrap();

        // 10e37
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::Float(1.0e38, FloatType::Float32));
        parser.eat_newline().unwrap();

        // 1.0
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::Float(1.0, FloatType::Float32));
        parser.eat_newline().unwrap();

        // true
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::Boolean(true));
        parser.eat_newline().unwrap();

        // false
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::Boolean(false));
        parser.eat_newline().unwrap();

        // 'a'
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::Character('a'));
        parser.eat_newline().unwrap();

        // b'a'
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::Byte(b'a'));
        parser.eat_newline().unwrap();

        // "Hello, world!"
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::String("Hello, world!".to_string()));
        parser.eat_newline().unwrap();

        // b"abc"
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::ByteString(b"abc".to_vec()));
        parser.eat_newline().unwrap();

        // r"abc"
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::String("abc".to_string()));
        parser.eat_newline().unwrap();

        // r##"a#b#c"##
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::String("a#b#c".to_string()));
        parser.eat_newline().unwrap();

        // br"abc"
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::ByteString(b"abc".to_vec()));
        parser.eat_newline().unwrap();

        // br##"a#b#c"##
        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(literal, ScalarLiteral::ByteString(b"a#b#c".to_vec()));
        parser.eat_newline().unwrap();
    }

    #[test]
    fn test_array_literal() {
        let input = r###"
    [1, 2, 3]
    [1.0, 2.0, 3.0]
    [10, false, "Hi"]
    [0; 10]
    [false; 40]
        "###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // [1, 2, 3]
        let literal = parser.eat_array_literal().unwrap();
        assert_eq!(
            literal,
            ArrayLiteral::Fixed {
                elements: vec![
                    Expression::ScalarLiteral(ScalarLiteral::Integer(1, IntType::Int32)),
                    Expression::ScalarLiteral(ScalarLiteral::Integer(2, IntType::Int32)),
                    Expression::ScalarLiteral(ScalarLiteral::Integer(3, IntType::Int32))
                ]
            }
        );
        parser.eat_newline().unwrap();
    }
}
