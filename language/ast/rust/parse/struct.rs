//! Parse structs.

use dyst_language_token::TokenType;

use crate::parse::ParserOptions;
use crate::parse::expression::ExpressionParserOptions;
use crate::{
    Keyword, NodeId, ParseError, ParseResult, Parser, Statement, Struct, StructField, Visibility,
};

impl<'a> Parser<'a> {
    /// Eat a struct declaration.
    ///
    /// Examples:
    /// ```
    /// struct { a: int32, b: boolean }
    ///
    /// struct { // anonymous struct (for use as a value)
    ///     myField: int32 // colon optional
    ///     myOtherField: boolean
    /// }
    ///
    /// struct Bar {
    ///     myField: int32
    ///     myOtherField: boolean
    /// }
    ///
    /// struct Foo<T> {
    ///     myField: int32
    ///     myOtherField: boolean
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

        // optional name
        let name = if self.peek_token(TokenType::Identifier).is_ok() {
            Some(self.eat_identifier()?)
        } else {
            None
        };

        // optional static parameters: < ... >
        let static_parameters = if self.peek_token(TokenType::LessThan).is_ok() {
            self.bump(); // eat less than
            let params = self.with_options(
                ParserOptions {
                    in_static_type: true,
                    ..self.options
                },
                |p| p.eat_parameters_body(),
            )?;
            self.eat_token(TokenType::GreaterThan)?;
            Some(params)
        } else {
            None
        };

        // body
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;
        let struct_id = self.eat_struct_body(visibility)?;
        self.eat_token(TokenType::CloseBrace)?;

        // fill in header data
        let struct_ = self.tree.get_mut(struct_id);
        struct_.name = name;
        struct_.static_parameters = static_parameters;
        self.tree.set_span(struct_id, self.get_span_from(start));

        Ok(struct_id)
    }

    // Eat a struct body (without the header or `{` and `}`)
    pub fn eat_struct_body(
        &mut self,
        visibility: Option<Visibility>,
    ) -> ParseResult<NodeId<Struct>> {
        let start = self.mark();

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
            else if self.peek_struct_field().is_ok() {
                let field = self.eat_struct_field()?;
                fields.push(field);
            }
            // eat statements
            else {
                let statement = self.eat_statement()?;
                statements.push(statement);
            }
        }

        let struct_id = self.tree.allocate(
            Struct {
                name: None,
                visibility,
                static_parameters: None,
                fields,
                statements,
            },
            self.get_span_from(start),
        );
        Ok(struct_id)
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
            Err(ParseError::UnexpectedToken(self.peek()?.span))
        }
    }

    /// Eat a single struct field: `name: Type` with optional default `= <expr>`.
    fn eat_struct_field(&mut self) -> ParseResult<NodeId<StructField>> {
        let start = self.mark();
        let name = self.eat_identifier()?;
        self.eat_colon()?;
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
    use crate::{IntType, Parameter, PrimitiveType, Struct, StructField, Type, assert_node};

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
                assert_eq!(*name, parser.strings.intern("x"));
                assert!(default.is_none());
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                    assert_eq!(*width, 32);
                    assert!(*is_signed);
                });
            });

            // y: boolean
            assert_node!(parser.tree, fields[1], StructField { name, r#type, default } => {
                assert_eq!(*name, parser.strings.intern("y"));
                assert!(default.is_none());
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Boolean));
            });
        });
    }

    #[test]
    fn test_parse_struct_with_name_and_using_and_default() {
        let test = TestParser::new(
            r###"
struct Foo<T: Numeric> {
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
        assert_node!(parser.tree, struct_id, Struct { name, static_parameters, fields, statements, .. } => {
            assert_eq!(*name, Some(parser.strings.intern("Foo")));
            assert_eq!(statements.len(), 3);
            assert_eq!(fields.len(), 2);

            // T: Numeric
            assert!(static_parameters.is_some());
            let static_parameters = static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter { name, r#type, .. } => {
                // T
                assert_eq!(*name, parser.strings.intern("T"));
                // Numeric
                assert!(r#type.is_some());
                assert_node!(parser.tree, r#type.unwrap(), Type::Path { path, .. } => {
                    assert_eq!(*path, parser.paths.intern(vec![parser.strings.intern("Numeric")]));
                });
            });

            // a: boolean
            assert_node!(parser.tree, fields[0], StructField { name, r#type, default } => {
                assert_eq!(*name, parser.strings.intern("a"));
                assert!(default.is_none());
                assert_node!(parser.tree, *r#type, Type::Path { path, .. } => {
                    assert_eq!(*path, parser.paths.intern(vec![parser.strings.intern("T")]));
                });
            });

            // b: int32 = 4
            assert_node!(parser.tree, fields[1], StructField { name, r#type, default } => {
                assert_eq!(*name, parser.strings.intern("b"));
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                    assert_eq!(*width, 32);
                    assert!(*is_signed);
                });
                assert!(default.is_some());
            });
        });
    }
}
