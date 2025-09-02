//! Parse all kinds of types.

use std::str::FromStr;

use destack_language_token::TokenType;

use crate::{
    FloatType, IntType, Keyword, NodeId, ParseError, ParseResult, Parser, PrimitiveType, Tuple,
    TupleElement, Type,
};

impl IntType {
    /// 8-bit signed integer
    pub const INT8: IntType = IntType {
        width: 8,
        is_signed: true,
    };
    /// 16-bit signed integer
    pub const INT16: IntType = IntType {
        width: 16,
        is_signed: true,
    };
    /// 32-bit signed integer
    pub const INT32: IntType = IntType {
        width: 32,
        is_signed: true,
    };
    /// 64-bit signed integer
    pub const INT64: IntType = IntType {
        width: 64,
        is_signed: true,
    };
    /// 128-bit signed integer
    pub const INT128: IntType = IntType {
        width: 128,
        is_signed: true,
    };

    /// 8-bit unsigned integer
    pub const UINT8: IntType = IntType {
        width: 8,
        is_signed: false,
    };
    /// 16-bit unsigned integer
    pub const UINT16: IntType = IntType {
        width: 16,
        is_signed: false,
    };
    /// 32-bit unsigned integer
    pub const UINT32: IntType = IntType {
        width: 32,
        is_signed: false,
    };
    /// 64-bit unsigned integer
    pub const UINT64: IntType = IntType {
        width: 64,
        is_signed: false,
    };
    /// 128-bit unsigned integer
    pub const UINT128: IntType = IntType {
        width: 128,
        is_signed: false,
    };
}

impl IntType {
    #[inline]
    pub fn as_str(self) -> String {
        let mut as_str = if self.is_signed {
            "int".to_string()
        } else {
            "uint".to_string()
        };
        as_str.push_str(&self.width.to_string());
        as_str
    }
}

impl FloatType {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            FloatType::Float32 => "float32",
            FloatType::Float64 => "float64",
        }
    }
}

impl FromStr for FloatType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "float32" => Ok(FloatType::Float32),
            "float64" => Ok(FloatType::Float64),
            _ => Err(()),
        }
    }
}

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
    pub fn eat_type(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        // tuple
        if self.peek_next_token(TokenType::OpenParenthesis).is_ok() {
            self.eat_tuple_type()
        // array
        } else if self.peek_next_token(TokenType::OpenBracket).is_ok() {
            self.eat_array_or_slice_type()
        // struct
        } else if self.peek_keyword(Keyword::Struct).is_ok() {
            let struct_id = self.eat_struct()?;
            let ty_id = self
                .tree
                .allocate(Type::Struct(struct_id), self.span_from(start));
            Ok(ty_id)
        // union
        } else if self.peek_keyword(Keyword::Union).is_ok()
            || self.peek_keyword(Keyword::Enum).is_ok()
        {
            let union_id = self.eat_union_or_enum()?;
            let ty_id = self
                .tree
                .allocate(Type::Union(union_id), self.span_from(start));
            Ok(ty_id)
        // function
        } else if self.peek_keyword(Keyword::Function).is_ok() {
            let function_signature_id = self.eat_function_signature()?;
            let ty_id = self
                .tree
                .allocate(Type::Function(function_signature_id), self.span_from(start));
            Ok(ty_id)
        // scalar
        } else {
            self.eat_scalar_type()
        }
    }

    /// Peek a primitive type (e.g., `void`, `boolean`, `int32`, `uint7`, `float32`).
    ///
    /// Examples:
    /// ```
    /// void
    /// boolean
    /// character
    /// int32
    /// uint7
    /// float32
    /// ```
    pub fn peek_primitive_type(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        let next = self.peek_next()?;
        let next_str = self.get_span_str(next.span);
        let primitive_type = match next_str {
            // void
            "void" => Ok(PrimitiveType::Void),
            // boolean
            "boolean" => Ok(PrimitiveType::Boolean),
            // character
            "character" => Ok(PrimitiveType::Character),
            // int*
            int_str if int_str.starts_with("int") => {
                let width = int_str.trim_start_matches("int").parse::<u16>().unwrap();
                Ok(PrimitiveType::Int(IntType {
                    width,
                    is_signed: true,
                }))
            }
            // uint*
            uint_str if uint_str.starts_with("uint") => {
                let width = uint_str.trim_start_matches("uint").parse::<u16>().unwrap();
                Ok(PrimitiveType::Int(IntType {
                    width,
                    is_signed: false,
                }))
            }
            // float32
            "float32" => Ok(PrimitiveType::Float(FloatType::Float32)),
            // float64
            "float64" => Ok(PrimitiveType::Float(FloatType::Float64)),
            _ => Err(ParseError::UnexpectedToken(next.span)),
        };
        let ty_id = self
            .tree
            .allocate(Type::Primitive(primitive_type?), self.span_from(start));
        Ok(ty_id)
    }

    /// Eat a primitive type (e.g., `void`, `boolean`, `int32`, `uint7`, `float32`).
    ///
    /// Examples:
    /// ```
    /// void
    /// boolean
    /// character
    /// int32
    /// uint7
    /// float32
    /// ```
    pub fn eat_primitive_type(&mut self) -> ParseResult<NodeId<Type>> {
        let ty_id = self.peek_primitive_type()?;
        self.eat_identifier()?;
        Ok(ty_id)
    }

    /// Eat a scalar type (Infer, Never, Path with optional static arguments).
    ///
    /// Examples:
    /// ```
    /// void
    /// uint8
    /// int17
    /// float32
    /// ?float32
    /// _
    /// !
    /// !Time
    /// geom.Vector<Dims: 2, float32>
    /// ```
    pub fn eat_scalar_type(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        let next = *self.peek_next()?;

        // maybe
        if next.token.r#type == TokenType::Question {
            self.bump();
            let inner_type = self.eat_type()?;
            let ty_id = self
                .tree
                .allocate(Type::Maybe(Some(inner_type)), self.span_from(start));
            Ok(ty_id)
        }
        // never
        else if next.token.r#type == TokenType::Bang {
            self.bump();
            if self.peek_next_token(TokenType::Identifier).is_ok() {
                let inner_type = self.eat_type()?;
                let ty_id = self
                    .tree
                    .allocate(Type::Never(Some(inner_type)), self.span_from(start));
                Ok(ty_id)
            } else {
                let ty_id = self.tree.allocate(Type::Never(None), self.span_from(start));
                Ok(ty_id)
            }

        // primitive
        } else if let Ok(primitive_type) = self.peek_primitive_type() {
            self.bump();
            Ok(primitive_type)

        // identifier (infer or path)
        } else if next.token.r#type == TokenType::Identifier {
            let identifier = self.get_span_str(next.span);
            if identifier == "_" {
                self.bump();
                let ty_id = self.tree.allocate(Type::Infer, self.span_from(start));
                Ok(ty_id)
            } else {
                let path = self.eat_path()?;
                // eat static arguments if present
                if self.peek_next_token(TokenType::LessThan).is_ok() {
                    self.eat_token(TokenType::LessThan)?;
                    let static_arguments = self.eat_arguments_body()?;
                    self.eat_token(TokenType::GreaterThan)?;
                    let ty_id = self.tree.allocate(
                        Type::Path {
                            path,
                            static_arguments: Some(static_arguments),
                        },
                        self.span_from(start),
                    );
                    Ok(ty_id)
                } else {
                    let ty_id = self.tree.allocate(
                        Type::Path {
                            path,
                            static_arguments: None,
                        },
                        self.span_from(start),
                    );
                    Ok(ty_id)
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
    pub fn eat_tuple_type(&mut self) -> ParseResult<NodeId<Type>> {
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
    /// a: int32, b: boolean
    /// ```
    pub fn eat_tuple_type_body(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        let mut elements: Vec<NodeId<TupleElement>> = Vec::new();
        loop {
            let element = self.eat_tuple_element()?;
            elements.push(element);
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop()?;
            } else {
                break;
            }
        }
        let tuple_id = self
            .tree
            .allocate(Tuple { elements }, self.span_from(start));
        let ty_id = self
            .tree
            .allocate(Type::Tuple(tuple_id), self.span_from(start));
        Ok(ty_id)
    }

    /// Eat a tuple element.
    ///
    /// Examples:
    /// ```
    /// int32
    /// a: int32
    /// ```
    pub fn eat_tuple_element(&mut self) -> ParseResult<NodeId<TupleElement>> {
        let start = self.mark();

        if self.peek_next_next_token(TokenType::Colon).is_ok() {
            // named tuple element
            let name = self.eat_identifier()?;
            self.eat_colon()?;
            let r#type = self.eat_type()?;
            let tuple_element_id = self
                .tree
                .allocate(TupleElement::Named { name, r#type }, self.span_from(start));
            Ok(tuple_element_id)
        } else {
            // positional tuple element
            let r#type = self.eat_type()?;
            let tuple_element_id = self
                .tree
                .allocate(TupleElement::Positional { r#type }, self.span_from(start));
            Ok(tuple_element_id)
        }
    }

    /// Eat an array or slice type (including the `[` and `]`).
    ///
    /// Examples:
    /// ```
    /// [int32] // slice
    /// [int32; 5] // array (fixed size)
    /// ```
    pub fn eat_array_or_slice_type(&mut self) -> ParseResult<NodeId<Type>> {
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
    pub fn eat_array_or_slice_type_body(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        let element_type = self.eat_type()?;
        if self.peek_semicolon().is_ok() {
            self.eat_semicolon()?;
            let count = self.eat_expression()?;
            let ty_id = self.tree.allocate(
                Type::Array {
                    element_type,
                    count,
                },
                self.span_from(start),
            );
            Ok(ty_id)
        } else {
            let ty_id = self.tree.allocate(
                Type::Slice {
                    element: element_type,
                },
                self.span_from(start),
            );
            Ok(ty_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{FloatType, IntType, Parser, PrimitiveType, Type};

    #[test]
    fn test_eat_primitive_type() {
        let source = r##"
void
boolean
character
int32
uint7
uint0
uint999
int128
float32
float64
"##;
        let tokens = tokenize_semantic(source);
        let mut parser = Parser::new(SourceFile::new(0, source, source.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // void
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(*ty, Type::Primitive(PrimitiveType::Void));
        parser.eat_newline().unwrap();

        // boolean
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(*ty, Type::Primitive(PrimitiveType::Boolean));
        parser.eat_newline().unwrap();

        // character
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(*ty, Type::Primitive(PrimitiveType::Character));
        parser.eat_newline().unwrap();

        // int32
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(
            *ty,
            Type::Primitive(PrimitiveType::Int(IntType {
                width: 32,
                is_signed: true,
            }))
        );
        parser.eat_newline().unwrap();

        // uint7
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(
            *ty,
            Type::Primitive(PrimitiveType::Int(IntType {
                width: 7,
                is_signed: false,
            }))
        );
        parser.eat_newline().unwrap();

        // uint0
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(
            *ty,
            Type::Primitive(PrimitiveType::Int(IntType {
                width: 0,
                is_signed: false,
            }))
        );
        parser.eat_newline().unwrap();

        // uint999
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(
            *ty,
            Type::Primitive(PrimitiveType::Int(IntType {
                width: 999,
                is_signed: false,
            }))
        );
        parser.eat_newline().unwrap();

        // int128
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(
            *ty,
            Type::Primitive(PrimitiveType::Int(IntType {
                width: 128,
                is_signed: true,
            }))
        );
        parser.eat_newline().unwrap();

        // float32
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(
            *ty,
            Type::Primitive(PrimitiveType::Float(FloatType::Float32))
        );
        parser.eat_newline().unwrap();

        // float64
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(
            *ty,
            Type::Primitive(PrimitiveType::Float(FloatType::Float64))
        );
        parser.eat_newline().unwrap();
    }

    #[test]
    fn test_eat_scalar_type() {
        let source = r##"
float32
geom.Vector2 // path
MyMesh<false, Dims: 3> // path with static arguments
?float32 // maybe type
! // never type
!Time // never type
"##;
        let tokens = tokenize_semantic(source);
        let mut parser = Parser::new(SourceFile::new(0, source, source.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // float32
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(
            *ty,
            Type::Primitive(PrimitiveType::Float(FloatType::Float32))
        );
        parser.eat_newline().unwrap();

        // geom.Vector2
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(
            *ty,
            Type::Path {
                path: parser.paths.intern(vec![
                    parser.strings.intern("geom"),
                    parser.strings.intern("Vector2")
                ]),
                static_arguments: None
            }
        );
        parser.eat_newline().unwrap();

        // nocheckin
        // MyMesh<false, Dims: 3>
        // let ty_id = parser.eat_type().unwrap();
        // let ty = parser.tree.get(ty_id);
        // assert_eq!(
        //     *ty,
        //     Type::Path {
        //         path: Path {
        //             segments: vec![PathSegment {
        //                 name: parser.strings.intern("MyMesh")
        //             }]
        //         },
        //         static_arguments: Some(vec![
        //             Argument {
        //                 name: None,
        //                 value: Expression::ScalarLiteral(ScalarLiteral::Boolean(false))
        //             },
        //             Argument {
        //                 name: Some(parser.strings.intern("Dims")),
        //                 value: Expression::ScalarLiteral(ScalarLiteral::Integer(3, IntType::INT32))
        //             },
        //         ])
        //     }
        // );
        // parser.eat_newline().unwrap();

        // // ?float32
        // let ty_id = parser.eat_type().unwrap();
        // let ty = parser.tree.get(ty_id);
        // assert_eq!(
        //     *ty,
        //     Type::Maybe(Some(Box::new(Type::Primitive(PrimitiveType::Float(
        //         FloatType::Float32
        //     )))))
        // );
        // parser.eat_newline().unwrap();

        // // !
        // let ty_id = parser.eat_type().unwrap();
        // assert_eq!(ty_id, Type::Never(None));
        // parser.eat_newline().unwrap();

        // // !Time
        // let ty_id = parser.eat_type().unwrap();
        // assert_eq!(
        //     ty_id,
        //     Type::Never(Some(Box::new(Type::Path {
        //         path: Path {
        //             segments: vec![PathSegment {
        //                 name: parser.strings.intern("Time")
        //             }]
        //         },
        //         static_arguments: None
        //     })))
        // );
        // parser.eat_newline().unwrap();
    }
}
