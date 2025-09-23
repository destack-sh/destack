//! Parse all kinds of types.

use std::str::FromStr;

use dyst_language_token::TokenType;

use crate::parse::ParserOptions;
use crate::parse::expression::ExpressionParserOptions;
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
    /// boolean | &int32
    /// []float32
    /// [3]float64
    /// (int32, int32)
    /// &T // reference to T
    /// &?T // reference to Maybe<T>
    /// ?&T // Maybe reference to T
    /// ?&?T // Maybe reference to Maybe<T>
    /// $T // virtual type T
    /// T<int32>
    /// T<Validate: false>
    /// MyEnum
    /// simulation.geometry.Vector2
    ///
    /// struct MyResponse { x: int32, y: int32 }
    /// enum { Good, Bad }
    /// union { A(int), B(float) } // explicit anonymous union
    /// boolean | int32 // implicit anonymous union
    /// function (int32) => int32
    /// function () => Result<int32, struct Error { message: string }>
    /// ```
    pub fn eat_type(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        let token = self.peek()?;
        let keyword = self.peek_any_keyword().ok();

        // tuple
        if token.token.r#type == TokenType::OpenParenthesis {
            self.eat_tuple_type()
        }
        // array or slice
        else if token.token.r#type == TokenType::OpenBracket {
            self.eat_array_or_slice_type()
        }
        // struct
        else if keyword == Some(Keyword::Struct) {
            let struct_id = self.eat_struct(None)?;
            let ty_id = self
                .tree
                .allocate(Type::Struct(struct_id), self.get_span_from(start));
            Ok(ty_id)
        // enum
        } else if keyword == Some(Keyword::Enum) {
            let enum_id = self.eat_enum(None)?;
            let ty_id = self
                .tree
                .allocate(Type::Enum(enum_id), self.get_span_from(start));
            Ok(ty_id)
        }
        // union
        else if keyword == Some(Keyword::Union) {
            let union_id = self.eat_union(None)?;
            let ty_id = self
                .tree
                .allocate(Type::Union(union_id), self.get_span_from(start));
            Ok(ty_id)
        // function
        } else if keyword == Some(Keyword::Function) {
            let function_id = self.eat_function(None)?;
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
    /// null
    /// boolean
    /// character
    /// int32
    /// uint7
    /// float32
    /// ```
    pub fn peek_primitive_type(&self) -> ParseResult<PrimitiveType> {
        let next = self.peek()?;
        // NOTE: void and null are parsed as literals, but it's fine since we match on the raw span string
        let next_str = self.get_span_str(next.span);
        match next_str {
            // void
            "void" => Ok(PrimitiveType::Void),
            // null
            "null" => Ok(PrimitiveType::Null),
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
            _ => Err(ParseError::expected(next.span, TokenType::Identifier)),
        }
    }

    /// Eat a primitive type (e.g., `void`, `boolean`, `int32`, `uint7`, `float32`).
    ///
    /// Examples:
    /// ```
    /// void
    /// null
    /// boolean
    /// character
    /// int32
    /// uint7
    /// float32
    /// ```
    pub fn eat_primitive_type(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        let primitive_type = self.peek_primitive_type()?;
        self.bump();
        let ty_id = self
            .tree
            .allocate(Type::Primitive(primitive_type), self.get_span_from(start));
        Ok(ty_id)
    }

    /// Eat a scalar type (Infer, Never, Path with optional static arguments).
    ///
    /// Examples:
    /// ```
    /// void
    /// null
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

        // maybe with `?`
        if next.token.r#type == TokenType::Maybe {
            self.bump();
            let inner_type = self.eat_type()?;
            let ty_id = self
                .tree
                .allocate(Type::Maybe(inner_type), self.get_span_from(start));
            Ok(ty_id)
        }
        // not or never with `!`
        else if next.token.r#type == TokenType::Not {
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
        }
        // pointer with `*` (also accept `&`)
        else if next.token.r#type == TokenType::Multiply
            || next.token.r#type == TokenType::BitwiseAnd
        {
            self.bump();
            let mutability = if let Ok(identifier) = self.peek_token(TokenType::Identifier)
                && self.get_token_str(*identifier) == "var"
            {
                self.bump(); // eat var
                Mutability::Mutable
            } else {
                Mutability::Immutable
            };
            let inner_type = self.eat_type()?;
            let ty_id = self.tree.allocate(
                Type::Reference {
                    mutability,
                    target: inner_type,
                },
                self.get_span_from(start),
            );
            Ok(ty_id)
        }
        // virtual with `$`
        else if next.token.r#type == TokenType::Virtual {
            self.bump();
            let inner_type = self.eat_type()?;
            let ty_id = self
                .tree
                .allocate(Type::Virtual(inner_type), self.get_span_from(start));
            Ok(ty_id)
        }
        // variadic with `..`
        else if next.token.r#type == TokenType::Range {
            self.bump(); // eat range
            let inner_type = self.eat_type()?;
            let ty_id = self
                .tree
                .allocate(Type::Variadic(inner_type), self.get_span_from(start));
            Ok(ty_id)
        }
        // infer with `_`
        else if next.token.r#type == TokenType::Wildcard {
            self.bump();
            let ty_id = self.tree.allocate(Type::Infer, self.get_span_from(start));
            Ok(ty_id)
        }
        // primitive
        else if let Ok(primitive_type) = self.peek_primitive_type() {
            self.bump();
            let ty_id = self
                .tree
                .allocate(Type::Primitive(primitive_type), self.get_span_from(start));
            Ok(ty_id)
        }
        // identifier
        else if next.token.r#type == TokenType::Identifier {
            let path = self.eat_path()?;
            // eat static arguments if present
            let static_arguments = if self.peek_token(TokenType::LessThan).is_ok() {
                self.bump(); // eat less than
                let static_arguments = self.with_options(
                    ParserOptions {
                        in_static_type: true,
                        ..self.options
                    },
                    |parser| parser.eat_arguments_body(),
                )?;
                self.eat_token(TokenType::GreaterThan)?;
                Some(static_arguments)
            } else {
                None
            };
            let ty_id = self.tree.allocate(
                Type::Path {
                    path,
                    static_arguments,
                },
                self.get_span_from(start),
            );
            Ok(ty_id)
        }
        // error
        else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Eat a tuple type (including the `(` and `)`).
    ///
    /// Examples:
    /// ```
    /// (int32)
    /// (int32, int32)
    /// ```
    fn eat_tuple_type(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        let tuple_id = self.eat_tuple()?;
        let ty_id = self
            .tree
            .allocate(Type::Tuple(tuple_id), self.get_span_from(start));
        Ok(ty_id)
    }

    /// Eat an array or slice type (including the `[` and `]` prefix).
    ///
    /// Examples:
    /// ```
    /// []int32 // slice
    /// [5]int32 // array (fixed size)
    /// ```
    fn eat_array_or_slice_type(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        self.eat_token(TokenType::OpenBracket)?;

        // slice: []T
        if self.peek_token(TokenType::CloseBracket).is_ok() {
            self.eat_token(TokenType::CloseBracket)?;
            let element_type = self.eat_type()?;
            let ty_id = self.tree.allocate(
                Type::Slice {
                    element: element_type,
                },
                self.get_span_from(start),
            );
            return Ok(ty_id);
        }

        // array: [N]T
        let count = self.eat_expression(ExpressionParserOptions::default())?;
        self.eat_token(TokenType::CloseBracket)?;
        let element_type = self.eat_type()?;
        let ty_id = self.tree.allocate(
            Type::Array {
                element: element_type,
                count,
            },
            self.get_span_from(start),
        );
        Ok(ty_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Argument, Expression, FloatType, IntType, Mutability, PrimitiveType, Type, assert_node,
        assert_path, assert_string,
    };

    #[test]
    fn test_parse_type_void() {
        let mut test = TestParser::new("void");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(parser.tree, ty_id, Type::Primitive(PrimitiveType::Void));
    }

    #[test]
    fn test_parse_type_null() {
        let mut test = TestParser::new("null");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(parser.tree, ty_id, Type::Primitive(PrimitiveType::Null));
    }

    #[test]
    fn test_parse_type_boolean() {
        let mut test = TestParser::new("boolean");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(parser.tree, ty_id, Type::Primitive(PrimitiveType::Boolean));
    }

    #[test]
    fn test_parse_type_character() {
        let mut test = TestParser::new("character");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Primitive(PrimitiveType::Character)
        );
    }

    #[test]
    fn test_parse_type_int32() {
        let mut test = TestParser::new("int32");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Primitive(PrimitiveType::Int(IntType {
                width: 32,
                is_signed: true,
            }))
        );
    }

    #[test]
    fn test_parse_type_uint7() {
        let mut test = TestParser::new("uint7");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Primitive(PrimitiveType::Int(IntType {
                width: 7,
                is_signed: false,
            }))
        );
    }

    #[test]
    fn test_parse_type_uint0() {
        let mut test = TestParser::new("uint0");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Primitive(PrimitiveType::Int(IntType {
                width: 0,
                is_signed: false,
            }))
        );
    }

    #[test]
    fn test_parse_type_uint999() {
        let mut test = TestParser::new("uint999");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Primitive(PrimitiveType::Int(IntType {
                width: 999,
                is_signed: false,
            }))
        );
    }

    #[test]
    fn test_parse_type_int128() {
        let mut test = TestParser::new("int128");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Primitive(PrimitiveType::Int(IntType {
                width: 128,
                is_signed: true,
            }))
        );
    }

    #[test]
    fn test_parse_type_float32() {
        let mut test = TestParser::new("float32");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Primitive(PrimitiveType::Float(FloatType::Float32))
        );
    }

    #[test]
    fn test_parse_type_float64() {
        let mut test = TestParser::new("float64");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Primitive(PrimitiveType::Float(FloatType::Float64))
        );
    }

    #[test]
    fn test_parse_type_infer() {
        let mut test = TestParser::new("_");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(parser.tree, ty_id, Type::Infer);
    }

    #[test]
    fn test_parse_type_path_simple() {
        let mut test = TestParser::new("geom.Vector2");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Path {
                path,
                static_arguments: None
            } => {
                assert_path!(parser.session, *path, "geom.Vector2");
            }
        );
    }

    #[test]
    fn test_parse_type_path_with_static_arguments() {
        let mut test = TestParser::new("Mesh<false, Dims: 3>");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Path {
                path,
                static_arguments,
            } => {
                assert_path!(parser.session, *path, "Mesh");
                let args = static_arguments.as_ref().expect("expected static args");
                assert_eq!(args.len(), 2);

                // false
                let arg0 = parser.tree.get(args[0]);
                assert_node!(
                    arg0,
                    Argument::Positional { value } => {
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::ScalarLiteral(lit_id) => {
                                let lit = parser.tree.get(*lit_id);
                                assert_eq!(*lit, crate::ScalarLiteral::Boolean(false));
                            }
                        );
                    }
                );

                // Dims: 3
                let arg1 = parser.tree.get(args[1]);
                assert_node!(
                    arg1,
                    Argument::Named { name, value } => {
                        assert_string!(parser.session, *name, "Dims");
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::ScalarLiteral(lit_id) => {
                                let lit = parser.tree.get(*lit_id);
                                assert_node!(
                                    lit,
                                    crate::ScalarLiteral::Integer(n, int_ty) => {
                                        assert_eq!(*n, 3);
                                        assert_eq!(*int_ty, IntType::INT32);
                                    }
                                );
                            }
                        );
                    }
                );
            }
        );
    }

    #[test]
    #[ignore = ":Broken: unglue << and >> for static type arguments?"]
    fn test_parse_type_path_with_nested_static_arguments() {
        let mut test = TestParser::new("HashMap<Key<int32>, Value: List<number>>");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Path {
                path,
                static_arguments,
            } => {
                assert_path!(parser.session, *path, "HashMap");
                let args = static_arguments.as_ref().expect("expected static args");
                assert_eq!(args.len(), 2);

                // Key<int32>
                assert_node!(
                    parser.tree.get(args[0]),
                    Argument::Positional { value } => {
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::Path(..) => {

                            }
                        );
                    }
                );


                // Value: List<number>
                assert_node!(
                    parser.tree.get(args[1]),
                    Argument::Named { name, value } => {
                        assert_string!(parser.session, *name, "Value");
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::Path(..) => {
                            }
                        );
                    }
                );
            }
        );
    }

    #[test]
    fn test_parse_type_maybe() {
        let mut test = TestParser::new("?float32");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Maybe(inner_id) => {
                assert_node!(
                    parser.tree,
                    *inner_id,
                    Type::Primitive(PrimitiveType::Float(FloatType::Float32))
                );
            }
        );
    }

    #[test]
    fn test_parse_type_never() {
        let mut test = TestParser::new("!");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(parser.tree, ty_id, Type::Never);
    }

    #[test]
    fn test_parse_type_not() {
        let mut test = TestParser::new("!Time");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Not(inner_id) => {
                assert_node!(
                    parser.tree,
                    *inner_id,
                    Type::Path {
                        path,
                        static_arguments: None
                    } => {
                        assert_path!(parser.session, *path, "Time");
                    }
                );
            }
        );
    }

    #[test]
    fn test_parse_type_pointer_immutable() {
        let mut test = TestParser::new("*Vector2");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Reference {
                mutability,
                target: inner_id,
            } => {
                assert_eq!(*mutability, Mutability::Immutable);
                assert_node!(
                    parser.tree,
                    *inner_id,
                    Type::Path {
                        path,
                        static_arguments: None
                    } => {
                        assert_path!(parser.session, *path, "Vector2");
                    }
                );
            }
        );
    }

    #[test]
    fn test_parse_type_pointer_mutable() {
        let mut test = TestParser::new("*var T");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Reference {
                mutability,
                target: inner_id,
            } => {
                assert_eq!(*mutability, Mutability::Mutable);
                assert_node!(
                    parser.tree,
                    *inner_id,
                    Type::Path {
                        path,
                        static_arguments: None
                    } => {
                        assert_path!(parser.session, *path, "T");
                    }
                );
            }
        );
    }

    #[test]
    fn test_parse_type_virtual() {
        let mut test = TestParser::new("$T");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(parser.tree, ty_id, Type::Virtual(inner_id) => {
            assert_node!(parser.tree, *inner_id, Type::Path {
                path,
                static_arguments: None
            } => {
                assert_path!(parser.session, *path, "T");
            });
        });
    }

    #[test]
    fn test_parse_type_variadic() {
        let mut test = TestParser::new("..T");
        let mut parser = test.parser();
        let ty_id = parser.eat_type().unwrap();

        assert_node!(parser.tree, ty_id, Type::Variadic(inner_id) => {
            assert_node!(parser.tree, *inner_id, Type::Path {
                path,
                static_arguments: None
            } => {
                assert_path!(parser.session, *path, "T");
            });
        });
    }
}
