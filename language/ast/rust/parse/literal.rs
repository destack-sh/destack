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
        if self.peek_token(TokenType::OpenBracket).is_ok() {
            let array_literal = self.eat_array_literal()?;
            let expression_id = self.tree.allocate(
                Expression::ArrayLiteral(array_literal),
                self.get_span_from(start),
            );
            Ok(expression_id)
        // tuple
        } else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            let tuple_literal = self.eat_tuple_literal()?;
            let expression_id = self.tree.allocate(
                Expression::TupleLiteral(tuple_literal),
                self.get_span_from(start),
            );
            Ok(expression_id)
        // struct
        } else if self.peek_token(TokenType::OpenBrace).is_ok() {
            let struct_literal = self.eat_struct_literal()?;
            let expression_id = self.tree.allocate(
                Expression::StructLiteral(struct_literal),
                self.get_span_from(start),
            );
            Ok(expression_id)
        // scalar
        } else if self.peek_token(TokenType::RawLiteral).is_ok()
            || self.peek_identifier_str("true").is_ok()
            || self.peek_identifier_str("false").is_ok()
        {
            let scalar_literal = self.eat_scalar_literal()?;
            let expression_id = self.tree.allocate(
                Expression::ScalarLiteral(scalar_literal),
                self.get_span_from(start),
            );
            Ok(expression_id)
        } else {
            Err(ParseError::UnexpectedToken(self.peek()?.span))
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

        if self.peek_identifier_str("true").is_ok() {
            // boolean literal true
            self.bump();
            let scalar_literal = self
                .tree
                .allocate(ScalarLiteral::Boolean(true), self.get_span_from(start));
            Ok(scalar_literal)
        } else if self.peek_identifier_str("false").is_ok() {
            // boolean literal false
            self.bump();
            let scalar_literal = self
                .tree
                .allocate(ScalarLiteral::Boolean(false), self.get_span_from(start));
            Ok(scalar_literal)
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
    /// [1.0, 2.0, .0]
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

        // parse first element
        let first_element = self.eat_expression()?;

        if self.peek_token(TokenType::Semicolon).is_ok() {
            // repeated array: [value; count]
            self.eat_token(TokenType::Semicolon)?;
            let count = self.eat_expression()?;
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
                self.eat_item_stop()?;
                if self.peek_token(TokenType::CloseBracket).is_ok() {
                    break;
                }
                let element = self.eat_expression()?;
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
        let mut elements: Vec<NodeId<Expression>> = vec![];
        while self.peek_item_stop().is_ok() {
            self.eat_item_stop()?;
            if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                break;
            }
            let element = self.eat_expression()?;
            elements.push(element);
        }
        self.eat_token(TokenType::CloseParenthesis)?;
        let tuple_literal = self
            .tree
            .allocate(TupleLiteral { elements }, self.get_span_from(start));
        Ok(tuple_literal)
    }

    /// Eat a struct literal (including the type prefix).
    ///
    /// Examples:
    /// ```
    /// Vector2 { x: 1.0, y: 2.0, z }
    ///
    /// destack.geometry.Mesh2 {
    ///     vertices: [Vector3 { x: 1.0, y: 2.0, z: .0 }], // optional comma
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
        let mut fields: Vec<NodeId<FieldLiteral>> = vec![];
        while self.peek_item_stop().is_ok() {
            self.eat_item_stop()?;
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
        }
        self.eat_token(TokenType::CloseBrace)?;
        Ok(fields)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{ArrayLiteral, Expression, FloatType, IntType, Parser, ScalarLiteral};

    #[test]
    fn test_scalar_literal() {
        let input = r###"
true
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

        // true
        let literal_id = parser.eat_scalar_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        assert_eq!(*literal, ScalarLiteral::Boolean(true));
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
    []
    [1, ]
    [10, false, "Hi"]
    [1.0, 
     2.0
     3.0
    ]
    [0; 10]
        "###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // []
        let literal_id = parser.eat_array_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        match literal {
            ArrayLiteral::Fixed { elements } => assert_eq!(elements.len(), 0),
            _ => panic!("expected fixed array"),
        }
        parser.eat_newline().unwrap();

        // [1, ]
        let literal_id = parser.eat_array_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        match literal {
            ArrayLiteral::Fixed { elements } => {
                assert_eq!(elements.len(), 1);
                // [0] = 1
                let element_0 = parser.tree.get(elements[0]);
                match element_0 {
                    &Expression::ScalarLiteral(scalar_literal_id) => {
                        let scalar_literal = parser.tree.get(scalar_literal_id);
                        match scalar_literal {
                            ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                            _ => panic!("expected integer"),
                        }
                    }
                    _ => panic!("expected scalar literal"),
                };
            }
            _ => panic!("expected fixed array"),
        }
        parser.eat_newline().unwrap();

        // [10, false, "Hi"]
        let literal_id = parser.eat_array_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        match literal {
            ArrayLiteral::Fixed { elements } => {
                assert_eq!(elements.len(), 3);
                // [0] = 10
                let element_0 = parser.tree.get(elements[0]);
                match element_0 {
                    &Expression::ScalarLiteral(scalar_literal_id) => {
                        let scalar_literal = parser.tree.get(scalar_literal_id);
                        match scalar_literal {
                            ScalarLiteral::Integer(n, _) => assert_eq!(*n, 10),
                            _ => panic!("expected integer"),
                        }
                    }
                    _ => panic!("expected scalar literal"),
                };
                // [1] = false
                let element_1 = parser.tree.get(elements[1]);
                match element_1 {
                    &Expression::ScalarLiteral(scalar_literal_id) => {
                        let scalar_literal = parser.tree.get(scalar_literal_id);
                        match scalar_literal {
                            ScalarLiteral::Boolean(b) => assert!(!(*b)),
                            _ => panic!("expected boolean"),
                        }
                    }
                    _ => panic!("expected scalar literal"),
                };
                // [2] = "Hi"
                let element_2 = parser.tree.get(elements[2]);
                match element_2 {
                    &Expression::ScalarLiteral(scalar_literal_id) => {
                        let scalar_literal = parser.tree.get(scalar_literal_id);
                        match scalar_literal {
                            ScalarLiteral::String(string_id) => {
                                let expected_string_id = parser.strings.intern("Hi");
                                assert_eq!(*string_id, expected_string_id);
                            }
                            _ => panic!("expected string"),
                        }
                    }
                    _ => panic!("expected scalar literal"),
                };
            }
            _ => panic!("expected fixed array"),
        }
        parser.eat_newline().unwrap();

        // [1.0,
        //  2.0
        //  3.0
        // ]
        let literal_id = parser.eat_array_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        match literal {
            ArrayLiteral::Fixed { elements } => {
                assert_eq!(elements.len(), 3);
                // [0] = 1.0
                let element_0 = parser.tree.get(elements[0]);
                match element_0 {
                    &Expression::ScalarLiteral(scalar_literal_id) => {
                        let scalar_literal = parser.tree.get(scalar_literal_id);
                        match scalar_literal {
                            ScalarLiteral::Float(f, _) => assert_eq!(*f, 1.0),
                            _ => panic!("expected float"),
                        }
                    }
                    _ => panic!("expected scalar literal"),
                };
                // [1] = 2.0
                let element_1 = parser.tree.get(elements[1]);
                match element_1 {
                    &Expression::ScalarLiteral(scalar_literal_id) => {
                        let scalar_literal = parser.tree.get(scalar_literal_id);
                        match scalar_literal {
                            ScalarLiteral::Float(f, _) => assert_eq!(*f, 2.0),
                            _ => panic!("expected float"),
                        }
                    }
                    _ => panic!("expected scalar literal"),
                };
                // [2] = 3.0
                let element_2 = parser.tree.get(elements[2]);
                match element_2 {
                    &Expression::ScalarLiteral(scalar_literal_id) => {
                        let scalar_literal = parser.tree.get(scalar_literal_id);
                        match scalar_literal {
                            ScalarLiteral::Float(f, _) => assert_eq!(*f, 3.0),
                            _ => panic!("expected float"),
                        }
                    }
                    _ => panic!("expected scalar literal"),
                };
            }
            _ => panic!("expected fixed array"),
        }
        parser.eat_newline().unwrap();

        // [0; 10]
        let literal_id = parser.eat_array_literal().unwrap();
        let literal = parser.tree.get(literal_id);
        match literal {
            &ArrayLiteral::Repeated {
                element: element_id,
                count: count_id,
            } => {
                let element = parser.tree.get(element_id);
                match element {
                    &Expression::ScalarLiteral(scalar_literal_id) => {
                        let scalar_literal = parser.tree.get(scalar_literal_id);
                        match scalar_literal {
                            ScalarLiteral::Integer(n, _) => assert_eq!(*n, 0),
                            _ => panic!("expected integer"),
                        }
                    }
                    _ => panic!("expected scalar literal"),
                };
                let count = parser.tree.get(count_id);
                match count {
                    &Expression::ScalarLiteral(scalar_literal_id) => {
                        let scalar_literal = parser.tree.get(scalar_literal_id);
                        match scalar_literal {
                            ScalarLiteral::Integer(n, _) => assert_eq!(*n, 10),
                            _ => panic!("expected integer"),
                        }
                    }
                    _ => panic!("expected scalar literal"),
                };
            }
            _ => panic!("expected repeated array"),
        }
        parser.eat_newline().unwrap();
    }
}
