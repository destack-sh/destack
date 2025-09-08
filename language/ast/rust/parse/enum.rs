//! Parse enums.

use dyst_language_token::TokenType;

use crate::parse::expression::ExpressionParserOptions;
use crate::{Enum, EnumField, Keyword, NodeId, ParseError, ParseResult, Parser, Statement, Type};

impl<'a> Parser<'a> {
    /// Eat an enum declaration.
    ///
    /// Examples:
    /// ```
    /// // anonymous enum (for use as a value)
    /// enum { Success, Failure }
    ///
    /// enum Foo {
    ///     A // semicolon optional
    ///     B
    ///     C
    ///
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    ///
    /// enum(u8) Foo {
    ///     Baz = 1
    ///     Qux = 2
    /// }
    /// ```
    pub fn eat_enum(&mut self) -> ParseResult<NodeId<Enum>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Enum)?;

        // optional explicit tag type in `(Type)`
        let explicit_type: Option<NodeId<Type>> =
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.eat_token(TokenType::OpenParenthesis)?;
                let ty = self.eat_type()?;
                self.eat_token(TokenType::CloseParenthesis)?;
                Some(ty)
            } else {
                None
            };

        // optional name
        let name = if self.peek_token(TokenType::Identifier).is_ok() {
            Some(self.eat_identifier()?)
        } else {
            None
        };

        // body
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;
        let enum_id = self.eat_enum_body()?;
        self.eat_token(TokenType::CloseBrace)?;

        // fill header data
        let r#enum = self.tree.get_mut(enum_id);
        r#enum.name = name;
        r#enum.r#type = explicit_type;

        // extend span to include header and braces (best-effort)
        let _ = start; // NOTE @Cleanup: spans not updated post-allocation

        Ok(enum_id)
    }

    /// Eat an enum body (without the header or `{` and `}`)
    pub fn eat_enum_body(&mut self) -> ParseResult<NodeId<Enum>> {
        let start = self.mark();

        // eat everything
        let mut fields: Vec<NodeId<EnumField>> = Vec::new();
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
            // enum field
            else if self.peek_enum_field().is_ok() {
                let field = self.eat_enum_field()?;
                fields.push(field);
            }
            // eat statements
            else {
                let statement = self.eat_statement()?;
                statements.push(statement);
            }
        }

        let enum_id = self.tree.allocate(
            Enum {
                name: None,
                r#type: None,
                fields,
                statements,
            },
            self.get_span_from(start),
        );
        Ok(enum_id)
    }

    /// Peek an enum field.
    fn peek_enum_field(&mut self) -> ParseResult<()> {
        if self.peek_identifier().is_ok()
            && (self.peek_next_token(TokenType::Assign).is_ok()
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

    /// Eat a single enum field and return it as a UnionField node id.
    fn eat_enum_field(&mut self) -> ParseResult<NodeId<EnumField>> {
        let start = self.mark();
        let name = self.eat_identifier()?;

        // optional `= <expr>` value
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression(ExpressionParserOptions::default())?)
        } else {
            None
        };

        let field_id = self
            .tree
            .allocate(EnumField { name, value }, self.get_span_from(start));
        Ok(field_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{Enum, EnumField, Expression, PrimitiveType, Type, assert_int, assert_node};

    #[test]
    fn test_parse_enum_anonymous_simple() {
        let test = TestParser::new(
            r###"
enum {
    Success
    Failure
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum().unwrap();
        assert_node!(parser.tree, enum_id, Enum { name, r#type, fields, statements } => {
            assert!(name.is_none());
            assert!(r#type.is_none());
            assert!(statements.is_empty());
            assert_eq!(fields.len(), 2);

            // Success
            assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                assert_eq!(*name, parser.strings.intern("Success"));
                assert!(value.is_none());
            });

            // Failure
            assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                assert_eq!(*name, parser.strings.intern("Failure"));
                assert!(value.is_none());
            });
        });
    }

    #[test]
    fn test_parse_enum_with_type_name_and_values() {
        let test = TestParser::new(
            r###"
enum(uint8) Foo {

    Baz = 1

    Qux = 2

    function myFunc() { // nested declaration
    }
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum().unwrap();
        assert_node!(parser.tree, enum_id, Enum { name, r#type, fields, statements: _ } => {
            // enum name
            assert_eq!(*name, Some(parser.strings.intern("Foo")));

            // enum type
            assert_node!(parser.tree, r#type.unwrap(), Type::Primitive(PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 8);
                assert!(!int_ty.is_signed);
            });

            assert_eq!(fields.len(), 2);

            // Baz = 1
            assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                assert_eq!(*name, parser.strings.intern("Baz"));
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(literal_id) => {
                    assert_int!(parser.tree, *literal_id, 1);
                });
            });

            // Qux = 2
            assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                assert_eq!(*name, parser.strings.intern("Qux"));
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(literal_id) => {
                    assert_int!(parser.tree, *literal_id, 2);
                });
            });
        });
    }
}
