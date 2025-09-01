use destack_language_token::{NumberBase, RawLiteralType, TokenType};
use std::borrow::Cow;

use crate::{
    ArrayLiteral, Expression, FieldLiteral, FloatType, IntType, NodeId, ParseError, ParseResult,
    Parser, ScalarLiteral, StructLiteral, TupleLiteral,
};

impl<'a> Parser<'a> {
    /// Eat a literal.
    pub fn eat_literal(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();
        // array
        if self.peek_next_token(TokenType::OpenBracket).is_ok() {
            let array_literal = self.eat_array_literal()?;
            let expression_id = self
                .tree
                .allocate_from_mark(Expression::ArrayLiteral(array_literal), start);
            Ok(expression_id)
        // tuple
        } else if self.peek_next_token(TokenType::OpenParenthesis).is_ok() {
            let tuple_literal = self.eat_tuple_literal()?;
            let expression_id = self
                .tree
                .allocate_from_mark(Expression::TupleLiteral(tuple_literal), start);
            Ok(expression_id)
        // struct
        } else if self.peek_next_token(TokenType::OpenBrace).is_ok() {
            let struct_literal = self.eat_struct_literal()?;
            let expression_id = self
                .tree
                .allocate_from_mark(Expression::StructLiteral(struct_literal), start);
            Ok(expression_id)
        // scalar
        } else {
            let scalar_literal = self.eat_scalar_literal()?;
            let expression_id = self
                .tree
                .allocate_from_mark(Expression::ScalarLiteral(scalar_literal), start);
            Ok(expression_id)
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
    pub fn eat_scalar_literal(&mut self) -> ParseResult<NodeId<ScalarLiteral>> {
        let start = self.mark();

        if self.peek_next_token(TokenType::Identifier).is_ok() {
            // may be reference to a built-in constant value
            // boolean literal (just an identifier)
            let string_id = self.eat_identifier()?;
            let identifier = self.strings.get(string_id);
            if identifier == "true" {
                let scalar_literal = self
                    .tree
                    .allocate_from_mark(ScalarLiteral::Boolean(true), start);
                Ok(scalar_literal)
            } else if identifier == "false" {
                let scalar_literal = self
                    .tree
                    .allocate_from_mark(ScalarLiteral::Boolean(false), start);
                Ok(scalar_literal)
            } else {
                Err(ParseError::UnexpectedToken(self.prev().unwrap().span))
            }
        } else {
            // regular literal
            let literal_span = &self.eat_next()?.clone();
            let literal_str = self.get_span_str(literal_span.span);
            if literal_span.token.r#type != TokenType::RawLiteral {
                return Err(ParseError::UnexpectedToken(literal_span.span));
            }
            let Some(literal) = literal_span.token.body else {
                return Err(ParseError::UnexpectedToken(literal_span.span));
            };

            match literal {
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
                            let scalar_literal = self.tree.allocate_from_mark(
                                ScalarLiteral::Integer(value, IntType::INT32),
                                start,
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
                            let scalar_literal = self.tree.allocate_from_mark(
                                ScalarLiteral::Float(value, FloatType::Float64),
                                start,
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
                        let scalar_literal = self
                            .tree
                            .allocate_from_mark(ScalarLiteral::Character(literal_char), start);
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
                        let scalar_literal = self
                            .tree
                            .allocate_from_mark(ScalarLiteral::Byte(literal_char as u8), start);
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
                        .allocate_from_mark(ScalarLiteral::String(string_id), start);
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
                        .allocate_from_mark(ScalarLiteral::ByteString(bytes), start);
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
                            .allocate_from_mark(ScalarLiteral::String(string_id), start);
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
                            .allocate_from_mark(ScalarLiteral::ByteString(bytes), start);
                        Ok(scalar_literal)
                    } else {
                        Err(ParseError::UnexpectedToken(literal_span.span))
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
    pub fn eat_array_literal(&mut self) -> ParseResult<NodeId<ArrayLiteral>> {
        let start = self.mark();

        self.eat_token(TokenType::OpenBracket)?;
        let mut elements: Vec<NodeId<Expression>> = vec![];
        let first_element = self.eat_expression()?;
        elements.push(first_element);

        if self.peek_next_token(TokenType::Semicolon).is_ok() {
            // repeated array
            self.eat_token(TokenType::Semicolon)?;
            let count = self.eat_expression()?;
            elements.push(count);
            self.eat_token(TokenType::CloseBracket)?;
            todo!("repeated array")
        } else {
            // fixed array
            while self.peek_next_token(TokenType::Comma).is_ok() {
                self.eat_token(TokenType::Comma)?;
                let element = self.eat_expression()?;
                elements.push(element);
            }
            self.eat_token(TokenType::CloseBracket)?;
            let array_literal = self
                .tree
                .allocate_from_mark(ArrayLiteral::Fixed { elements }, start);
            Ok(array_literal)
        }
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
    pub fn eat_tuple_literal(&mut self) -> ParseResult<NodeId<TupleLiteral>> {
        let start = self.mark();
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut elements: Vec<NodeId<Expression>> = vec![];
        while self.peek_next_token(TokenType::Comma).is_ok() {
            self.eat_token(TokenType::Comma)?;
            let element = self.eat_expression()?;
            elements.push(element);
        }
        self.eat_token(TokenType::CloseParenthesis)?;
        let tuple_literal = self
            .tree
            .allocate_from_mark(TupleLiteral { elements }, start);
        Ok(tuple_literal)
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
    pub fn eat_struct_literal(&mut self) -> ParseResult<NodeId<StructLiteral>> {
        let start = self.mark();
        let r#type = self.eat_type()?;
        let fields = self.eat_struct_literal_body()?;
        let struct_literal = self
            .tree
            .allocate_from_mark(StructLiteral { r#type, fields }, start);
        Ok(struct_literal)
    }

    /// Eat the body of a struct literal (excluding the path prefix).
    ///
    /// Examples:
    /// ```
    /// { x: 1.0, y: 2.0, z }
    /// ```
    pub fn eat_struct_literal_body(&mut self) -> ParseResult<Vec<NodeId<FieldLiteral>>> {
        self.eat_token(TokenType::OpenBrace)?;
        let mut fields: Vec<NodeId<FieldLiteral>> = vec![];
        while self.peek_next_token(TokenType::Comma).is_ok() {
            self.eat_token(TokenType::Comma)?;
            let field_start = self.mark();
            let name = self.eat_identifier()?;
            if self.peek_next_token(TokenType::Colon).is_ok() {
                self.eat_token(TokenType::Colon)?;
                let value = self.eat_expression()?;
                let field_literal = self.tree.allocate_from_mark(
                    FieldLiteral {
                        name,
                        value: Some(value),
                    },
                    field_start,
                );
                fields.push(field_literal);
            } else {
                let field_literal = self
                    .tree
                    .allocate_from_mark(FieldLiteral { name, value: None }, field_start);
                fields.push(field_literal);
            }
        }
        self.eat_token(TokenType::CloseBrace)?;
        Ok(fields)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{FloatType, IntType, Parser, ScalarLiteral};

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
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::Integer(1, IntType::INT32));
        parser.eat_newline().unwrap();

        // 731
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::Integer(731, IntType::INT32));
        parser.eat_newline().unwrap();

        // 0x1234
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::Integer(0x1234, IntType::INT32));
        parser.eat_newline().unwrap();

        // 10e37
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::Float(1.0e38, FloatType::Float64));
        parser.eat_newline().unwrap();

        // 1.0
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::Float(1.0, FloatType::Float64));
        parser.eat_newline().unwrap();

        // true
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::Boolean(true));
        parser.eat_newline().unwrap();

        // false
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::Boolean(false));
        parser.eat_newline().unwrap();

        // 'a'
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::Character('a'));
        parser.eat_newline().unwrap();

        // b'a'
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::Byte(b'a'));
        parser.eat_newline().unwrap();

        // "Hello, world!"
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        let string_id = parser.strings.intern("Hello, world!");
        assert_eq!(*literal, ScalarLiteral::String(string_id));
        parser.eat_newline().unwrap();

        // b"abc"
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::ByteString(b"abc".to_vec()));
        parser.eat_newline().unwrap();

        // r"abc"
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        let string_id = parser.strings.intern("abc");
        assert_eq!(*literal, ScalarLiteral::String(string_id));
        parser.eat_newline().unwrap();

        // r##"a#b#c"##
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        let string_id = parser.strings.intern("a#b#c");
        assert_eq!(*literal, ScalarLiteral::String(string_id));
        parser.eat_newline().unwrap();

        // br"abc"
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::ByteString(b"abc".to_vec()));
        parser.eat_newline().unwrap();

        // br##"a#b#c"##
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::ByteString(b"a#b#c".to_vec()));
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

        // nocheckin
        // // [1, 2, 3]
        // let literal = parser.eat_array_literal().unwrap();
        // assert_eq!(
        //     literal,
        //     ArrayLiteral::Fixed {
        //         elements: vec![
        //             Expression::ScalarLiteral(ScalarLiteral::Integer(1, IntType::INT32)),
        //             Expression::ScalarLiteral(ScalarLiteral::Integer(2, IntType::INT32)),
        //             Expression::ScalarLiteral(ScalarLiteral::Integer(3, IntType::INT32))
        //         ]
        //     }
        // );
        // parser.eat_newline().unwrap();

        // // [1.0, 2.0, 3.0]
        // let literal = parser.eat_array_literal().unwrap();
        // assert_eq!(
        //     literal,
        //     ArrayLiteral::Fixed {
        //         elements: vec![
        //             Expression::ScalarLiteral(ScalarLiteral::Float(1.0, FloatType::Float64)),
        //             Expression::ScalarLiteral(ScalarLiteral::Float(2.0, FloatType::Float64)),
        //             Expression::ScalarLiteral(ScalarLiteral::Float(3.0, FloatType::Float64))
        //         ]
        //     }
        // );
        // parser.eat_newline().unwrap();

        // // [10, false, "Hi"]
        // let literal = parser.eat_array_literal().unwrap();
        // assert_eq!(
        //     literal,
        //     ArrayLiteral::Fixed {
        //         elements: vec![
        //             Expression::ScalarLiteral(ScalarLiteral::Integer(10, IntType::INT32)),
        //             Expression::ScalarLiteral(ScalarLiteral::Boolean(false)),
        //             Expression::ScalarLiteral(ScalarLiteral::String("Hi".to_string()))
        //         ]
        //     }
        // );
        // parser.eat_newline().unwrap();
    }
}
