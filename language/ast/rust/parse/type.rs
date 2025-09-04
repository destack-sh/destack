//! Parse all kinds of types.

use std::str::FromStr;

use destack_language_token::TokenType;

use crate::{
    FloatType, IntType, Keyword, Mutability, NodeId, ParseError, ParseResult, Parser,
    PrimitiveType, Type,
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
    /// Eat any Type (including nominal and anonymous declarations and implicit unions).
    ///
    /// Examples:
    /// ```
    /// int32
    /// boolean
    /// [float32]
    /// [float64; 3]
    /// (int32, int32)
    /// *T // pointer to T
    /// *?T // pointer to Maybe<T>
    /// ?*T // Maybe pointer to T
    /// T<int32>
    /// T<Validate: false>
    /// MyEnum
    /// simulation.geometry.Vector2
    ///
    /// struct MyResponse { x: int32, y: int32 }
    /// enum { Good, Bad }
    /// union { A(int), B(float) } // explicit anonymous union
    /// boolean | *int32 // implicit anonymous union
    /// function (int32) => int32
    /// function () => int32, Vector2 // implicitly returns a tuple
    /// function () => Result<int32, struct Error { message: string }>
    /// ```
    pub fn eat_type(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();

        // tuple
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.eat_tuple_type()

        // array or slice
        } else if self.peek_token(TokenType::OpenBracket).is_ok() {
            self.eat_array_or_slice_type()

        // struct
        } else if self.peek_keyword(Keyword::Struct).is_ok() {
            let struct_id = self.eat_struct()?;
            let ty_id = self
                .tree
                .allocate(Type::Struct(struct_id), self.get_span_from(start));
            Ok(ty_id)

        // enum
        } else if self.peek_keyword(Keyword::Enum).is_ok() {
            let enum_id = self.eat_enum()?;
            let ty_id = self
                .tree
                .allocate(Type::Enum(enum_id), self.get_span_from(start));
            Ok(ty_id)
        }
        // union
        else if self.peek_keyword(Keyword::Union).is_ok() {
            let union_id = self.eat_union()?;
            let ty_id = self
                .tree
                .allocate(Type::Union(union_id), self.get_span_from(start));
            Ok(ty_id)

        // function
        } else if self.peek_keyword(Keyword::Function).is_ok() {
            let function_id = self.eat_function()?;
            let ty_id = self
                .tree
                .allocate(Type::Function(function_id), self.get_span_from(start));
            Ok(ty_id)

        // scalar
        } else {
            Ok(self.eat_scalar_type()?)
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
        let next = self.peek()?;
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
            .allocate(Type::Primitive(primitive_type?), self.get_span_from(start));
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
    /// *float32
    /// _
    /// !
    /// !Time
    /// geom.Vector<Dims: 2, float32>
    /// ```
    pub fn eat_scalar_type(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        let next = *self.peek()?;

        // maybe
        if next.token.r#type == TokenType::Question {
            self.bump();
            let inner_type = self.eat_type()?;
            let ty_id = self
                .tree
                .allocate(Type::Maybe(inner_type), self.get_span_from(start));
            Ok(ty_id)
        }
        // not or never
        else if next.token.r#type == TokenType::Bang {
            self.bump();
            if self.peek_token(TokenType::Identifier).is_ok() {
                let inner_type = self.eat_type()?;
                let ty_id = self
                    .tree
                    .allocate(Type::Not(inner_type), self.get_span_from(start));
                Ok(ty_id)
            } else {
                let ty_id = self.tree.allocate(Type::Never, self.get_span_from(start));
                Ok(ty_id)
            }

        // pointer
        } else if next.token.r#type == TokenType::Multiply {
            self.bump();
            let mutability = if let Ok(identifier) = self.peek_token(TokenType::Identifier)
                && self.get_token_str(*identifier) == "var"
            {
                self.eat_token(TokenType::Identifier)?;
                Mutability::Mutable
            } else {
                Mutability::Immutable
            };
            let inner_type = self.eat_type()?;
            let ty_id = self.tree.allocate(
                Type::Pointer {
                    mutability,
                    target: inner_type,
                },
                self.get_span_from(start),
            );
            Ok(ty_id)

        // primitive
        } else if let Ok(primitive_type) = self.peek_primitive_type() {
            self.bump();
            Ok(primitive_type)

        // identifier (infer or path)
        } else if next.token.r#type == TokenType::Identifier {
            let identifier = self.get_span_str(next.span);

            // infer
            if identifier == "_" {
                self.bump();
                let ty_id = self.tree.allocate(Type::Infer, self.get_span_from(start));
                Ok(ty_id)
            }
            // path
            else {
                let path = self.eat_path()?;
                // eat static arguments if present
                if self.peek_token(TokenType::LessThan).is_ok() {
                    self.eat_token(TokenType::LessThan)?;
                    let static_arguments = self.eat_arguments_body()?;
                    self.eat_token(TokenType::GreaterThan)?;
                    let ty_id = self.tree.allocate(
                        Type::Path {
                            path,
                            static_arguments: Some(static_arguments),
                        },
                        self.get_span_from(start),
                    );
                    Ok(ty_id)
                } else {
                    let ty_id = self.tree.allocate(
                        Type::Path {
                            path,
                            static_arguments: None,
                        },
                        self.get_span_from(start),
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
        let start = self.mark();
        let tuple_id = self.eat_tuple()?;
        let ty_id = self
            .tree
            .allocate(Type::Tuple(tuple_id), self.get_span_from(start));
        Ok(ty_id)
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
            let count = self.eat_expression(None)?;
            let ty_id = self.tree.allocate(
                Type::Array {
                    element_type,
                    count,
                },
                self.get_span_from(start),
            );
            Ok(ty_id)
        } else {
            let ty_id = self.tree.allocate(
                Type::Slice {
                    element: element_type,
                },
                self.get_span_from(start),
            );
            Ok(ty_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{
        Argument, Expression, FloatType, IntType, Mutability, Parser, PrimitiveType, Type,
    };

    #[test]
    fn test_primitive_type() {
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
    fn test_scalar_type() {
        let source = r##"
float32
geom.Vector2 // path
MyMesh<false, Dims: 3> // path with static arguments
?float32 // maybe type
! // never type
!Time // never type
*Vector2 // pointer type
*var T // mutable pointer type
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
        let expected_path = parser.paths.intern(vec![
            parser.strings.intern("geom"),
            parser.strings.intern("Vector2"),
        ]);
        assert_eq!(
            *ty,
            Type::Path {
                path: expected_path,
                static_arguments: None
            }
        );
        parser.eat_newline().unwrap();

        // MyMesh<false, Dims: 3>
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        let expected_path = parser.paths.intern(vec![parser.strings.intern("MyMesh")]);
        match ty {
            Type::Path {
                path,
                static_arguments,
            } => {
                assert_eq!(*path, expected_path);
                let args = static_arguments.as_ref().expect("expected static args");
                assert_eq!(args.len(), 2);
                // false
                let arg0 = parser.tree.get(args[0]);
                match arg0 {
                    Argument::Positional { value } => {
                        let expr = parser.tree.get(*value);
                        match expr {
                            Expression::ScalarLiteral(lit_id) => {
                                let lit = parser.tree.get(*lit_id);
                                // false
                                assert_eq!(*lit, crate::ScalarLiteral::Boolean(false));
                            }
                            _ => panic!("expected scalar literal"),
                        }
                    }
                    _ => panic!("expected positional argument"),
                }
                // Dims: 3
                let arg1 = parser.tree.get(args[1]);
                match arg1 {
                    Argument::Named { name, value } => {
                        assert_eq!(*name, parser.strings.intern("Dims"));
                        let expr = parser.tree.get(*value);
                        match expr {
                            Expression::ScalarLiteral(lit_id) => {
                                let lit = parser.tree.get(*lit_id);
                                match lit {
                                    crate::ScalarLiteral::Integer(n, int_ty) => {
                                        assert_eq!(*n, 3);
                                        assert_eq!(*int_ty, IntType::INT32);
                                    }
                                    _ => panic!("expected integer literal"),
                                }
                            }
                            _ => panic!("expected scalar literal"),
                        }
                    }
                    _ => panic!("expected named argument"),
                }
            }
            _ => panic!("expected path type with static arguments"),
        }
        parser.eat_newline().unwrap();

        // ?float32
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        match ty {
            Type::Maybe(inner_id) => {
                let inner = parser.tree.get(*inner_id);
                assert_eq!(
                    *inner,
                    Type::Primitive(PrimitiveType::Float(FloatType::Float32))
                );
            }
            _ => panic!("expected Maybe<float32>"),
        }
        parser.eat_newline().unwrap();

        // !
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        assert_eq!(*ty, Type::Never);
        parser.eat_newline().unwrap();

        // !Time
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        match ty {
            Type::Not(inner_id) => {
                let inner = parser.tree.get(*inner_id);
                let expected_time_path = parser.paths.intern(vec![parser.strings.intern("Time")]);
                assert_eq!(
                    *inner,
                    Type::Path {
                        path: expected_time_path,
                        static_arguments: None
                    }
                );
            }
            _ => panic!("expected !Time"),
        }
        parser.eat_newline().unwrap();

        // *Vector2
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        match ty {
            Type::Pointer {
                mutability,
                target: inner_id,
            } => {
                let inner = parser.tree.get(*inner_id);
                assert_eq!(
                    *inner,
                    Type::Path {
                        path: parser.paths.intern(vec![parser.strings.intern("Vector2")]),
                        static_arguments: None
                    }
                );
                assert_eq!(*mutability, Mutability::Immutable);
            }
            _ => panic!("expected *Vector2"),
        }
        parser.eat_newline().unwrap();

        // *var T
        let ty_id = parser.eat_type().unwrap();
        let ty = parser.tree.get(ty_id);
        match ty {
            Type::Pointer {
                mutability,
                target: inner_id,
            } => {
                let inner = parser.tree.get(*inner_id);
                assert_eq!(
                    *inner,
                    Type::Path {
                        path: parser.paths.intern(vec![parser.strings.intern("T")]),
                        static_arguments: None
                    }
                );
                assert_eq!(*mutability, Mutability::Mutable);
            }
            _ => panic!("expected *var T"),
        }
        parser.eat_newline().unwrap();
    }
}
