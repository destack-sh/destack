//! Parse structs.
#![allow(clippy::type_complexity)]

use dyst_language_token::TokenType;

use crate::parse::ParserOptions;
use crate::parse::expression::ExpressionParserOptions;
use crate::{
    Keyword, NodeId, ParseError, ParseResult, Parser, Statement, Struct, StructField, StructStyle,
    Type, Visibility,
};

impl<'a> Parser<'a> {
    /// Eat a struct declaration.
    ///
    /// Examples:
    /// ```
    /// struct {} // empty anonymous struct
    ///
    /// struct _ {} // explicit anonymous struct (for disambiguation)
    ///
    /// struct A() // unit struct (no fields)
    ///
    /// struct Number(int32) // tuple struct (1 field)
    ///
    /// struct Number(int32, isAwesome: boolean) { // tuple struct (2 fields)
    ///     ...
    /// }
    ///
    /// struct { a: int32, b: boolean }
    ///
    /// struct { // anonymous struct (for use as a value)
    ///     myField: int32 // colon optional
    ///     myOtherField: boolean
    /// }
    ///
    /// struct(uint64) Bar { // 64-bit representation
    ///     myField: int32
    ///     myOtherField: boolean
    /// }
    ///
    /// struct Foo<T>: Baz { // Foo has a Baz
    ///     myField: int32
    ///     myOtherField: T
    ///
    ///     let x: int32 = 7 // constant
    ///
    ///     use Bar // Foo has a Bar
    ///
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    /// ```
    pub fn eat_struct(&mut self, visibility: Option<Visibility>) -> ParseResult<NodeId<Struct>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Struct)?;

        // optional representation type: ( ... )
        let representation_type = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump(); // eat open parenthesis
            let representation_type = self.eat_type()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            Some(representation_type)
        } else {
            None
        };

        // optional name
        let name = self.eat_identifier_or_wildcard_maybe()?;

        // style / tuple struct
        let (style, tuple_fields) = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.eat_token(TokenType::OpenParenthesis)?;
            let tuple_fields = self.eat_struct_tuple_body()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            (StructStyle::Tuple, Some(tuple_fields))
        } else {
            (StructStyle::Struct, None)
        };

        // optional static parameters: < ... >
        let static_parameters = if self.peek_token(TokenType::LessThan).is_ok() {
            self.bump(); // eat less than
            let static_parameters = self.with_options(
                ParserOptions {
                    in_static_type: true,
                    ..self.options
                },
                |parser| parser.eat_parameters_body(),
            )?;
            self.eat_token(TokenType::GreaterThan)?;
            Some(static_parameters)
        } else {
            None
        };

        // optional super types: : ...
        let super_types = if self.peek_token(TokenType::Colon).is_ok() {
            self.bump(); // eat colon
            let mut super_types: Vec<NodeId<Type>> = Vec::new();
            loop {
                // eat until open parenthesis
                if self.peek_token(TokenType::OpenBrace).is_ok() {
                    break;
                }
                // consume any stop
                else if self.peek_any_stop().is_ok() {
                    self.eat_any_stop_with_newlines()?;
                }
                // keep eating super types
                else {
                    let super_type = self.eat_type()?;
                    super_types.push(super_type);
                }
            }
            Some(super_types)
        } else {
            None
        };

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)?;
        self.eat_newlines_maybe()?;
        let (mut fields, statements) = self.eat_struct_body(style)?;
        if let Some(tuple_fields) = tuple_fields {
            // merge in tuple fields
            fields.extend(tuple_fields);
        }
        self.eat_token(TokenType::CloseBrace)?;

        // struct
        let struct_id = self.tree.allocate(
            Struct {
                name,
                visibility,
                style,
                super_types,
                static_parameters,
                representation_type,
                fields,
                statements,
            },
            self.get_span_from(start),
        );

        Ok(struct_id)
    }

    /// Eat a struct tuple body (without the parenthesis).
    /// Because it's just a tuple body, it doesn't have any statements.
    pub fn eat_struct_tuple_body(&mut self) -> ParseResult<Vec<NodeId<StructField>>> {
        let mut tuple_fields: Vec<NodeId<StructField>> = Vec::new();
        loop {
            // stop at closing parenthesis
            if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
            // keep eating tuple fields
            else {
                let field = self.eat_struct_field()?;
                tuple_fields.push(field);
            }
        }
        Ok(tuple_fields)
    }

    /// Eat a struct body (without the header or `{` and `}`).
    /// Struct fields are only parsed if it's a struct-style struct.
    pub fn eat_struct_body(
        &mut self,
        style: StructStyle,
    ) -> ParseResult<(Vec<NodeId<StructField>>, Vec<NodeId<Statement>>)> {
        // eat everything
        let mut fields: Vec<NodeId<StructField>> = Vec::new();
        let mut statements: Vec<NodeId<Statement>> = Vec::new();
        loop {
            // stop on closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
            // struct field
            else if style == StructStyle::Struct && self.peek_struct_field().is_ok() {
                let field = self.eat_struct_field()?;
                fields.push(field);
            }
            // eat statements
            else if let Some(statement_id) = self.try_eat_statement()? {
                statements.push(statement_id);
            }
        }

        Ok((fields, statements))
    }

    /// Peek a struct field: `name: Type` with optional default `= <expr>`.
    fn peek_struct_field(&self) -> ParseResult<()> {
        if self.peek_identifier().is_ok()
            && (self.peek_next_token(TokenType::Colon).is_ok()
                || self.peek_next_token(TokenType::Assign).is_ok()
                || self.peek_next_token(TokenType::Newline).is_ok()
                || self.peek_next_token(TokenType::Comma).is_ok()
                || self.peek_next_token(TokenType::Semicolon).is_ok()
                || self.peek_next_token(TokenType::CloseBrace).is_ok())
        {
            Ok(())
        } else {
            Err(ParseError::expected_token(
                self.peek()?.span,
                TokenType::Identifier,
            ))
        }
    }

    /// Eat a single struct field: `T`,`name: T`, or `name: T = <expr>`.
    fn eat_struct_field(&mut self) -> ParseResult<NodeId<StructField>> {
        let start = self.mark();

        // name:
        let name =
            if self.peek_identifier().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
                let name = self.eat_identifier()?;
                self.eat_colon()?;
                Some(name)
            } else {
                None
            };

        // type
        let r#type = self.eat_type()?;

        // optional default value: `= <expr>`
        let default = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression(ExpressionParserOptions::default())?)
        } else {
            None
        };

        let field_id = self.tree.allocate(
            StructField {
                name,
                r#type,
                default,
            },
            self.get_span_from(start),
        );
        Ok(field_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        IntType, Parameter, PrimitiveType, Struct, StructField, StructStyle, Type, assert_node,
    };

    #[test]
    fn test_parse_struct_anonymous() {
        let test = TestParser::new(
            r###"
struct { x: int32, y: boolean
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        // struct { x: int32, y: boolean }
        let struct_id = parser.eat_struct(None).unwrap();
        assert_node!(parser.tree, struct_id, Struct { name, static_parameters, fields, statements, .. } => {
            assert_eq!(*name, None);
            assert_eq!(*static_parameters, None);
            assert!(statements.is_empty());
            assert_eq!(fields.len(), 2);

            // x: int32
            assert_node!(parser.tree, fields[0], StructField { name, r#type, default } => {
                assert_eq!(name.unwrap(), parser.intern_string("x"));
                assert!(default.is_none());
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                    assert_eq!(*width, 32);
                    assert!(*is_signed);
                });
            });

            // y: boolean
            assert_node!(parser.tree, fields[1], StructField { name, r#type, default } => {
                assert_eq!(name.unwrap(), parser.intern_string("y"));
                assert!(default.is_none());
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Boolean));
            });
        });
    }

    #[test]
    fn test_parse_struct_with_super_types() {
        let test = TestParser::new(
            r###"
struct Foo: Bar {}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(None).unwrap();
        assert_node!(parser.tree, struct_id, Struct { name, super_types, fields, statements, .. } => {
            assert_eq!(*name, Some(parser.intern_string("Foo")));
            assert!(statements.is_empty());
            assert!(fields.is_empty());

            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Type::Path { path, .. } => {
                assert_eq!(*path, parser.intern_path(vec![parser.intern_string("Bar")]));
            });
        });
    }

    #[test]
    fn test_parse_struct_with_tuple_style() {
        let test = TestParser::new(
            r###"
struct Foo(int32, boolean) {}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(None).unwrap();
        assert_node!(parser.tree, struct_id, Struct { name, style, fields, statements, .. } => {
            assert_eq!(*name, Some(parser.intern_string("Foo")));
            assert_eq!(*style, StructStyle::Tuple);
            assert_eq!(fields.len(), 2);
            assert!(statements.is_empty());

            // int32
            assert_node!(parser.tree, fields[0], StructField { name, r#type, default } => {
                assert!(name.is_none());
                assert!(default.is_none());
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                    assert_eq!(*width, 32);
                    assert!(*is_signed);
                });
            });

            // boolean
            assert_node!(parser.tree, fields[1], StructField { name, r#type, default } => {
                assert!(name.is_none());
                assert!(default.is_none());
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Boolean));
            });
        });
    }

    #[test]
    fn test_parse_struct_with_struct_style() {
        let test = TestParser::new(
            r###"
struct Foo<T: Numeric>: Boz {
    use Bar, Baz
    
    let x: int32 = 4

    a: T
    b: int32 = 4

    function myFunc() { // nested declaration
    }
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(None).unwrap();
        assert_node!(parser.tree, struct_id, Struct { name, static_parameters, fields, statements, super_types, .. } => {
            assert_eq!(*name, Some(parser.intern_string("Foo")));
            assert_eq!(statements.len(), 3);
            assert_eq!(fields.len(), 2);

            // T: Numeric
            assert!(static_parameters.is_some());
            let static_parameters = static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter { name, r#type, .. } => {
                // T
                assert_eq!(*name, parser.intern_string("T"));
                // Numeric
                assert!(r#type.is_some());
                assert_node!(parser.tree, r#type.unwrap(), Type::Path { path, .. } => {
                    assert_eq!(*path, parser.intern_path(vec![parser.intern_string("Numeric")]));
                });
            });

            // Boz
            assert!(super_types.is_some());
            let super_types = super_types.as_ref().unwrap();
            assert_eq!(super_types.len(), 1);
            assert_node!(parser.tree, super_types[0], Type::Path { path, .. } => {
                assert_eq!(*path, parser.intern_path(vec![parser.intern_string("Boz")]));
            });

            // a: boolean
            assert_node!(parser.tree, fields[0], StructField { name, r#type, default } => {
                assert_eq!(name.unwrap(), parser.intern_string("a"));
                assert!(default.is_none());
                assert_node!(parser.tree, *r#type, Type::Path { path, .. } => {
                    assert_eq!(*path, parser.intern_path(vec![parser.intern_string("T")]));
                });
            });

            // b: int32 = 4
            assert_node!(parser.tree, fields[1], StructField { name, r#type, default } => {
                assert_eq!(name.unwrap(), parser.intern_string("b"));
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                    assert_eq!(*width, 32);
                    assert!(*is_signed);
                });
                assert!(default.is_some());
            });
        });
    }
}
