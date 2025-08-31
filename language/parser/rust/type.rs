//! Parse all kinds of types.

use destack_language_lexer::TokenType;

use crate::{Keyword, ParseError, ParseResult, Parser, Type};

impl<'a> Parser<'a> {
    /// Eat a Type.
    ///
    /// Examples:
    /// ```
    /// int32
    /// (int32)
    /// (int32, int32)
    /// (int32, int32) => int32
    /// [float32]
    /// [float32; 5]
    /// ```
    pub fn eat_type(&mut self) -> ParseResult<Type> {
        // tuple
        if self.peek_next_token(TokenType::OpenParenthesis).is_ok() {
            self.eat_tuple_type()
        // array
        } else if self.peek_next_token(TokenType::OpenBracket).is_ok() {
            self.eat_array_or_slice_type()
        // struct
        } else if self.peek_keyword(Keyword::Struct).is_ok() {
            self.eat_struct()
        // union
        } else if self.peek_keyword(Keyword::Union).is_ok()
            || self.peek_keyword(Keyword::Enum).is_ok()
        {
            self.eat_union_or_enum()
        // function
        } else if self.peek_keyword(Keyword::Function).is_ok() {
            self.eat_function_signature()
        // scalar
        } else {
            self.eat_scalar_type()
        }
    }

    /// Eat a scalar type (Infer, Never, Path with optional static arguments).
    ///
    /// Examples:
    /// ```
    /// float32
    /// ?float32
    /// _
    /// !
    /// !Time
    /// geom.Vector<Dims: 2, float32>
    /// ```
    pub fn eat_scalar_type(&mut self) -> ParseResult<Type> {
        let next = self.peek_next()?;
        // maybe
        if next.token.r#type == TokenType::Question {
            self.bump();
            let inner_type = self.eat_type()?;
            Ok(Type::Maybe(Some(Box::new(inner_type))))
        }
        // never
        else if next.token.r#type == TokenType::Bang {
            self.bump();
            if self.peek_next_token(TokenType::Identifier).is_ok() {
                let inner_type = self.eat_type()?;
                Ok(Type::Never(Some(Box::new(inner_type))))
            } else {
                Ok(Type::Never(None))
            }
        // identifier (infer or path)
        } else if next.token.r#type == TokenType::Identifier {
            let identifier = self.get_span_str(next.span);
            if identifier == "_" {
                self.bump();
                Ok(Type::Infer)
            } else {
                let path = self.eat_path()?;
                // eat static arguments if present
                if self.peek_next_token(TokenType::LessThan).is_ok() {
                    self.eat_token(TokenType::LessThan)?;
                    let static_arguments = self.eat_arguments_body()?;
                    self.eat_token(TokenType::GreaterThan)?;
                    Ok(Type::Path {
                        path,
                        static_arguments: Some(static_arguments),
                    })
                } else {
                    Ok(Type::Path {
                        path,
                        static_arguments: None,
                    })
                }
            }
        }
        // error
        else {
            Err(ParseError::UnexpectedToken(next.span))
        }
    }

    /// Eat a tuple type (including the `(` and `)`).
    ///
    /// Examples:
    /// ```
    /// (int32)
    /// (int32, int32)
    /// ```
    pub fn eat_tuple_type(&mut self) -> ParseResult<Type> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let body = self.eat_tuple_type_body()?;
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(body)
    }

    /// Eat a tuple type body.
    ///
    /// Examples:
    /// ```
    /// int32
    /// int32, int32
    /// ```
    pub fn eat_tuple_type_body(&mut self) -> ParseResult<Type> {
        let mut elements: Vec<Type> = Vec::new();
        loop {
            let element = self.eat_type()?;
            elements.push(element);
            if self.peek_next_token(TokenType::Comma).is_ok() {
                self.eat_token(TokenType::Comma)?;
            } else {
                break;
            }
        }
        Ok(Type::Tuple { elements })
    }

    /// Eat an array or slice type (including the `[` and `]`).
    ///
    /// Examples:
    /// ```
    /// [int32] // slice
    /// [int32; 5] // array (fixed size)
    /// ```
    pub fn eat_array_or_slice_type(&mut self) -> ParseResult<Type> {
        self.eat_token(TokenType::OpenBracket)?;
        let body = self.eat_array_or_slice_type_body()?;
        self.eat_token(TokenType::CloseBracket)?;
        Ok(body)
    }

    /// Eat an array or slice type body (excluding the `[` and `]`).
    ///
    /// Examples:
    /// ```
    /// int32
    /// int32; 5
    /// ```
    pub fn eat_array_or_slice_type_body(&mut self) -> ParseResult<Type> {
        let element = self.eat_type()?;
        if self.peek_semicolon().is_ok() {
            self.eat_semicolon()?;
            let count = self.eat_expression()?;
            Ok(Type::Array {
                element: Box::new(element),
                count: Box::new(count),
            })
        } else {
            Ok(Type::Slice {
                element: Box::new(element),
            })
        }
    }

    /// Eat a function signature.
    ///
    /// Examples:
    /// ```
    /// () => int32
    /// (int32) => (int32, int32) // explicit tuple return type
    /// (int32) => int32, int32 // implicit tuple return type
    /// ```
    pub fn eat_function_signature(&mut self) -> ParseResult<Type> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use destack_language_lexer::{SourceFile, tokenize_semantic};

    use crate::{Argument, Expression, IntType, Parser, ScalarLiteral, Type};

    #[test]
    fn test_eat_scalar_type() {
        let source = r##"
float32
geom.Vector2 // path
MyMesh<false, Dims: 3> // path with static arguments
?float32 // maybe type
! // never type
!Time // never type with inner
"##;
        let tokens = tokenize_semantic(source);
        let mut parser = Parser::new(SourceFile::new(0, source, source.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // float32
        let r#type = parser.eat_type().unwrap();
        assert_eq!(
            r#type,
            Type::Path {
                path: "float32".parse().unwrap(),
                static_arguments: None
            }
        );
        parser.eat_newline().unwrap();

        // geom.Vector2
        let r#type = parser.eat_type().unwrap();
        assert_eq!(
            r#type,
            Type::Path {
                path: "geom.Vector2".parse().unwrap(),
                static_arguments: None
            }
        );
        parser.eat_newline().unwrap();

        // MyMesh<false, Dims: 3>
        let r#type = parser.eat_type().unwrap();
        assert_eq!(
            r#type,
            Type::Path {
                path: "MyMesh".parse().unwrap(),
                static_arguments: Some(vec![
                    Argument {
                        name: None,
                        value: Expression::ScalarLiteral(ScalarLiteral::Boolean(false))
                    },
                    Argument {
                        name: Some("Dims".to_string()),
                        value: Expression::ScalarLiteral(ScalarLiteral::Integer(3, IntType::Int32))
                    },
                ])
            }
        );
        parser.eat_newline().unwrap();

        // ?float32
        let r#type = parser.eat_type().unwrap();
        assert_eq!(
            r#type,
            Type::Maybe(Some(Box::new(Type::Path {
                path: "float32".parse().unwrap(),
                static_arguments: None
            })))
        );
        parser.eat_newline().unwrap();

        // !
        let r#type = parser.eat_type().unwrap();
        assert_eq!(r#type, Type::Never(None));
        parser.eat_newline().unwrap();

        // !Time
        let r#type = parser.eat_type().unwrap();
        assert_eq!(
            r#type,
            Type::Never(Some(Box::new(Type::Path {
                path: "Time".parse().unwrap(),
                static_arguments: None
            })))
        );
        parser.eat_newline().unwrap();
    }
}
