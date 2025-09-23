//! Parse all kinds of types.

use std::str::FromStr;

use dyst_language_source::Span;
use dyst_language_token::TokenType;

use crate::parse::ParserOptions;
use crate::parse::expression::ExpressionParserOptions;
use crate::{
    Expression, FloatType, IntType, Keyword, Mutability, NodeId, ParseError, ParseResult, Parser,
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

#[derive(Debug, Copy, Clone, Default)]
pub struct TypeParserOptions {
    /// Whether to parse an implicit union.
    pub in_implicit_union: bool = false,
}

impl<'a> Parser<'a> {
    /// Eat any Type (including nominal and anonymous declarations and implicit unions).
    /// Also consumes any prefix and postfix modifiers.
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
    /// function (int32) => int32
    /// function () => Result<int32, struct Error { message: string }>
    /// ```
    pub fn eat_type(&mut self, _options: TypeParserOptions) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        let next = self.peek()?;
        let keyword = self.peek_any_keyword().ok();

        let type_id = {
            // ?
            if next.token.r#type == TokenType::Maybe {
                self.bump();
                let inner_type = self.eat_type(_options)?;
                self.tree
                    .allocate(Type::Maybe(inner_type), self.get_span_from(start))
            }
            // !
            else if next.token.r#type == TokenType::Not {
                self.bump();
                if self.peek_token(TokenType::Identifier).is_ok() {
                    let inner_type = self.eat_type(_options)?;
                    self.tree
                        .allocate(Type::Not(inner_type), self.get_span_from(start))
                } else {
                    self.tree.allocate(Type::Never, self.get_span_from(start))
                }
            }
            // * or &
            else if next.token.r#type == TokenType::Multiply
                || next.token.r#type == TokenType::BitwiseAnd
            {
                self.bump(); // eat `*` or `&`
                let mutability = if let Ok(identifier) = self.peek_token(TokenType::Identifier)
                    && self.get_token_str(*identifier) == "var"
                {
                    self.bump(); // eat `var`
                    Mutability::Mutable
                } else {
                    Mutability::Immutable
                };
                let inner_type = self.eat_type(_options)?;
                self.tree.allocate(
                    Type::Reference {
                        mutability,
                        target: inner_type,
                    },
                    self.get_span_from(start),
                )
            }
            // $
            else if next.token.r#type == TokenType::Virtual {
                self.bump();
                let inner_type = self.eat_type(_options)?;
                self.tree
                    .allocate(Type::Virtual(inner_type), self.get_span_from(start))
            }
            // ..
            else if next.token.r#type == TokenType::Range {
                self.bump();
                let inner_type = self.eat_type(_options)?;
                self.tree
                    .allocate(Type::Variadic(inner_type), self.get_span_from(start))
            }
            // `(`
            else if next.token.r#type == TokenType::OpenParenthesis {
                self.eat_tuple_type()?
            }
            // `[`
            else if next.token.r#type == TokenType::OpenBracket {
                self.eat_array_or_slice_type()?
            }
            // `struct`
            else if keyword == Some(Keyword::Struct) {
                let start = self.mark();
                let struct_id = self.eat_struct(None)?;
                self.tree
                    .allocate(Type::Struct(struct_id), self.get_span_from(start))
            }
            // `enum`
            else if keyword == Some(Keyword::Enum) {
                let start = self.mark();
                let enum_id = self.eat_enum(None)?;
                self.tree
                    .allocate(Type::Enum(enum_id), self.get_span_from(start))
            }
            // `union`
            else if keyword == Some(Keyword::Union) {
                let start = self.mark();
                let union_id = self.eat_union(None)?;
                self.tree
                    .allocate(Type::Union(union_id), self.get_span_from(start))
            }
            // `function`
            else if keyword == Some(Keyword::Function) {
                let start = self.mark();
                let function_id = self.eat_function(None)?;
                self.tree
                    .allocate(Type::Function(function_id), self.get_span_from(start))
            }
            // _
            else {
                self.eat_scalar_type()?
            }
        };

        self.eat_any_type_postfix_modifiers(type_id)
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
    fn peek_primitive_type(&self) -> ParseResult<PrimitiveType> {
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
            // int_
            int_str if int_str.starts_with("int") => {
                let width = int_str
                    .trim_start_matches("int")
                    .parse::<u16>()
                    .unwrap_or(0);
                Ok(PrimitiveType::Int(IntType {
                    width,
                    is_signed: true,
                }))
            }
            // uint_
            uint_str if uint_str.starts_with("uint") => {
                let width = uint_str
                    .trim_start_matches("uint")
                    .parse::<u16>()
                    .unwrap_or(0);
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

    /// Apply postfix modifiers (`&`, `*`, `[]`, `[N]`) to an already parsed type.
    fn eat_any_type_postfix_modifiers(
        &mut self,
        mut type_id: NodeId<Type>,
    ) -> ParseResult<NodeId<Type>> {
        loop {
            let Ok(next) = self.peek() else {
                break;
            };
            match next.token.r#type {
                // NOTE :Broken: "unglue" `??` for maybe-maybe Types (prefix and postfix, like for `>>`)
                // `?`
                TokenType::Maybe => {
                    self.bump(); // eat `?`
                    let span = self.tree.get_span(type_id).extend(self.pos());
                    type_id = self.tree.allocate(Type::Maybe(type_id), span);
                }
                // `&` or `*`
                TokenType::BitwiseAnd | TokenType::Multiply => {
                    self.bump(); // eat `&` or `*`
                    let mutability = {
                        if let Ok(identifier) = self.peek_token(TokenType::Identifier)
                            && self.get_token_str(*identifier) == "var"
                        {
                            self.bump();
                            Mutability::Mutable
                        } else {
                            Mutability::Immutable
                        }
                    };
                    let span = self.tree.get_span(type_id).extend(self.pos());
                    type_id = self.tree.allocate(
                        Type::Reference {
                            mutability,
                            target: type_id,
                        },
                        span,
                    );
                }
                // `$`
                TokenType::Virtual => {
                    self.bump(); // eat `$`
                    let span = self.tree.get_span(type_id).extend(self.pos());
                    type_id = self.tree.allocate(Type::Virtual(type_id), span);
                }
                // `[]` or `[N]`
                TokenType::OpenBracket => {
                    let modifier_start = self.mark();
                    self.bump(); // eat `[`
                    let count = self.eat_array_or_slice_suffix()?;
                    let modifier_span = self.get_span_from(modifier_start);
                    let element_span = self.tree.get_span(type_id);
                    let span = element_span.merge(modifier_span);
                    type_id = self.create_array_or_slice_type(type_id, count, span);
                }
                // nothing
                _ => break,
            }
        }

        Ok(type_id)
    }

    /// Eat the core scalar type without prefix modifiers.
    ///
    /// Examples:
    /// ```
    /// void
    /// null
    /// uint8
    /// long.Path<TypeArg>
    /// _
    /// ```
    fn eat_scalar_type(&mut self) -> ParseResult<NodeId<Type>> {
        let start = self.mark();
        let next = *self.peek()?;

        // _
        if next.token.r#type == TokenType::Wildcard {
            self.bump();
            let ty_id = self.tree.allocate(Type::Infer, self.get_span_from(start));
            Ok(ty_id)
        }
        // primitive type
        else if let Ok(primitive_type) = self.peek_primitive_type() {
            self.bump();
            let ty_id = self
                .tree
                .allocate(Type::Primitive(primitive_type), self.get_span_from(start));
            Ok(ty_id)
        }
        // path
        else if next.token.r#type == TokenType::Identifier {
            let path = self.eat_path()?;
            let static_arguments = if self.peek_token(TokenType::LessThan).is_ok() {
                self.bump();
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

    /// Parse the trailing `[]` section and return an optional array count.
    fn eat_array_or_slice_suffix(&mut self) -> ParseResult<Option<NodeId<Expression>>> {
        if self.peek_token(TokenType::CloseBracket).is_ok() {
            self.eat_token(TokenType::CloseBracket)?;
            Ok(None)
        } else {
            let count = self.eat_expression(ExpressionParserOptions::default())?;
            self.eat_token(TokenType::CloseBracket)?;
            Ok(Some(count))
        }
    }

    /// Allocate an Array or Slice type node for the given element.
    fn create_array_or_slice_type(
        &mut self,
        element: NodeId<Type>,
        count: Option<NodeId<Expression>>,
        span: Span,
    ) -> NodeId<Type> {
        match count {
            Some(count) => self.tree.allocate(Type::Array { element, count }, span),
            None => self.tree.allocate(Type::Slice { element }, span),
        }
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
        let count = self.eat_array_or_slice_suffix()?;
        let element_type = self.eat_type(TypeParserOptions::default())?;
        let span = self.get_span_from(start);
        Ok(self.create_array_or_slice_type(element_type, count, span))
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::parse::r#type::TypeParserOptions;
    use crate::{
        Argument, Expression, FloatType, IntType, Mutability, PrimitiveType, Type, assert_int,
        assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_type_void() {
        let mut test = TestParser::new("void");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

        assert_node!(parser.tree, ty_id, Type::Primitive(PrimitiveType::Void));
    }

    #[test]
    fn test_parse_type_null() {
        let mut test = TestParser::new("null");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

        assert_node!(parser.tree, ty_id, Type::Primitive(PrimitiveType::Null));
    }

    #[test]
    fn test_parse_type_boolean() {
        let mut test = TestParser::new("boolean");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

        assert_node!(parser.tree, ty_id, Type::Primitive(PrimitiveType::Boolean));
    }

    #[test]
    fn test_parse_type_character() {
        let mut test = TestParser::new("character");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

        assert_node!(parser.tree, ty_id, Type::Infer);
    }

    #[test]
    fn test_parse_type_path_simple() {
        let mut test = TestParser::new("geom.Vector2");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

        assert_node!(parser.tree, ty_id, Type::Never);
    }

    #[test]
    fn test_parse_type_not() {
        let mut test = TestParser::new("!Time");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
    fn test_parse_type_reference_ampersand_prefix() {
        let mut test = TestParser::new("&Vector2");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
    fn test_parse_type_reference_ampersand_postfix() {
        let mut test = TestParser::new("Vector2&");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
    fn test_parse_type_reference_ampersand_postfix_mutable() {
        let mut test = TestParser::new("Vector2&var");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
                        assert_path!(parser.session, *path, "Vector2");
                    }
                );
            }
        );
    }

    #[test]
    fn test_parse_type_slice_prefix() {
        let mut test = TestParser::new("[]int32");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Slice { element } => {
                assert_node!(
                    parser.tree,
                    *element,
                    Type::Primitive(PrimitiveType::Int(IntType {
                        width: 32,
                        is_signed: true,
                    }))
                );
            }
        );
    }

    #[test]
    fn test_parse_type_slice_postfix() {
        let mut test = TestParser::new("int32[]");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Slice { element } => {
                assert_node!(
                    parser.tree,
                    *element,
                    Type::Primitive(PrimitiveType::Int(IntType {
                        width: 32,
                        is_signed: true,
                    }))
                );
            }
        );
    }

    #[test]
    fn test_parse_type_array_prefix() {
        let mut test = TestParser::new("[5]int32");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Array { element, count } => {
                assert_node!(
                    parser.tree,
                    *element,
                    Type::Primitive(PrimitiveType::Int(IntType {
                        width: 32,
                        is_signed: true,
                    }))
                );
                assert_node!(
                    parser.tree,
                    *count,
                    Expression::ScalarLiteral(literal_id) => {
                        assert_int!(parser.tree, *literal_id, 5);
                    }
                );
            }
        );
    }

    #[test]
    fn test_parse_type_array_postfix() {
        let mut test = TestParser::new("int32[5]");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

        assert_node!(
            parser.tree,
            ty_id,
            Type::Array { element, count } => {
                assert_node!(
                    parser.tree,
                    *element,
                    Type::Primitive(PrimitiveType::Int(IntType {
                        width: 32,
                        is_signed: true,
                    }))
                );
                assert_node!(
                    parser.tree,
                    *count,
                    Expression::ScalarLiteral(literal_id) => {
                        assert_int!(parser.tree, *literal_id, 5);
                    }
                );
            }
        );
    }

    #[test]
    fn test_parse_type_virtual() {
        let mut test = TestParser::new("$T");
        let mut parser = test.parser();
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
        let ty_id = parser.eat_type(TypeParserOptions::default()).unwrap();

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
