//! Parse enums.

use dyst_token::TokenType;

use crate::parse::prelude::*;
use crate::{
    Enum, EnumField, Expression, Keyword, NodeId, NodeType, ParseError, ParseResult, Parser,
    Visibility,
};

impl<'a> Parser<'a> {
    /// Eat an enum declaration.
    ///
    /// Examples:
    /// ```
    /// // anonymous enum (for use as a value)
    /// enum { Success, Failure }
    ///
    /// enum _ {} // explicit anonymous enum (for disambiguation)
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
    ///
    /// enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    ///     A = 1
    ///     B = T
    ///     @if(IsSomething)
    ///     C = 3
    /// }
    /// ```
    pub fn eat_enum(&mut self, visibility: Option<Visibility>) -> ParseResult<NodeId<Enum>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Enum)?;

        // optional explicit tag type in `(Type)`
        let explicit_type: Option<NodeId<Expression>> =
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open parenthesis
                let ty = self
                    .with_options(self.options.in_static_type(), |parser| {
                        parser.eat_expression()
                    })
                    .for_node_type(NodeType::Enum)?;
                self.eat_token(TokenType::CloseParenthesis)?;
                Some(ty)
            } else {
                None
            };

        // optional name
        let name = self.eat_identifier_or_wildcard_maybe()?;

        // optional static parameters: < ... >
        let static_parameters = self
            .eat_static_parameters_maybe()
            .for_node_type(NodeType::Enum)?;

        // optional super types: : ...
        let super_types = self.eat_super_types_maybe().for_node_type(NodeType::Enum)?;

        // body
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;
        let enum_id = self
            .eat_enum_body(visibility)
            .for_node_type(NodeType::Enum)?;
        self.eat_token(TokenType::CloseBrace)?;

        // fill header data
        let enum_ = self.tree.get_mut(enum_id);
        enum_.name = name;
        enum_.tag_type = explicit_type;
        enum_.static_parameters = static_parameters;
        enum_.super_types = super_types;
        self.tree.set_span(enum_id, self.get_span_from(start));

        Ok(enum_id)
    }

    /// Eat an enum body (without the header or `{` and `}`)
    fn eat_enum_body(&mut self, visibility: Option<Visibility>) -> ParseResult<NodeId<Enum>> {
        let start = self.mark();

        // eat everything
        let mut fields: Vec<NodeId<EnumField>> = Vec::new();
        let mut expressions: Vec<NodeId<Expression>> = Vec::new();
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
                let field = self.eat_enum_field().for_node_type(NodeType::Enum)?;
                fields.push(field);
            }
            // eat expressions
            else {
                let expression_id = self
                    .try_eat_expression_as_statement()
                    .for_node_type(NodeType::Expression)?;
                expressions.push(expression_id);
            }
        }

        let enum_id = self.tree.allocate(
            Enum {
                name: None,
                visibility,
                tag_type: None,
                static_parameters: None,
                super_types: None,
                fields,
                expressions,
            },
            self.get_span_from(start),
        );
        Ok(enum_id)
    }

    /// Peek an enum field.
    fn peek_enum_field(&self) -> ParseResult<()> {
        if self.peek_identifier().is_ok()
            && (self.peek_next_token(TokenType::Assign).is_ok()
                || self.peek_next_token(TokenType::Newline).is_ok()
                || self.peek_next_token(TokenType::Comma).is_ok()
                || self.peek_next_token(TokenType::Semicolon).is_ok()
                || self.peek_next_token(TokenType::CloseBrace).is_ok())
        {
            Ok(())
        } else {
            Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Identifier,
            ))
        }
    }

    /// Eat a single enum field and return it as a UnionField node id.
    fn eat_enum_field(&mut self) -> ParseResult<NodeId<EnumField>> {
        let start = self.mark();
        let name = self.eat_identifier().for_node_type(NodeType::EnumField)?;

        // optional `= <expr>` value
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression().for_node_type(NodeType::EnumField)?)
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
    use crate::{
        Enum, EnumField, Expression, IntType, Parameter, TypeLiteral, assert_int, assert_node,
        assert_path, assert_string,
    };

    #[test]
    fn test_parse_enum_with_super_types() {
        let mut test = TestParser::new(
            r###"
enum Foo: Day {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum(None).unwrap();
        assert_node!(parser.tree, enum_id, Enum { name, super_types, fields, expressions, .. } => {
            assert_string!(parser.session, name.unwrap(), "Foo");
            assert!(expressions.is_empty());
            assert!(fields.is_empty());

            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Day");
            });
        });
    }

    #[test]
    fn test_parse_enum_anonymous_simple() {
        let mut test = TestParser::new(
            r###"
enum {
    Success
    Failure
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum(None).unwrap();
        assert_node!(parser.tree, enum_id, Enum { name, tag_type, fields, expressions, .. } => {
            assert!(name.is_none());
            assert!(tag_type.is_none());
            assert!(expressions.is_empty());
            assert_eq!(fields.len(), 2);

            // Success
            assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                assert_string!(parser.session, *name, "Success");
                assert!(value.is_none());
            });

            // Failure
            assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                assert_string!(parser.session, *name, "Failure");
                assert!(value.is_none());
            });
        });
    }

    #[test]
    fn test_parse_enum_with_type_name_and_values() {
        let mut test = TestParser::new(
            r###"
enum(uint8) Foo: Day {

    Baz = 1

    Qux = 2

    function myFunc() { // nested declaration
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum(None).unwrap();
        assert_node!(parser.tree, enum_id, Enum { name, tag_type, fields, super_types, .. } => {
            // enum name
            assert_string!(parser.session, name.unwrap(), "Foo");

            // enum type
            assert_node!(parser.tree, tag_type.unwrap(), Expression::TypeLiteral(literal_id) => {
                assert_node!(parser.tree, *literal_id, TypeLiteral::Int(IntType { width: 8, is_signed: false }));
            });

            // super: Day
            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Day");
            });

            assert_eq!(fields.len(), 2);

            // Baz = 1
            assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                assert_string!(parser.session, *name, "Baz");
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(literal_id) => {
                    assert_int!(parser.tree, *literal_id, 1);
                });
            });

            // Qux = 2
            assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                assert_string!(parser.session, *name, "Qux");
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(literal_id) => {
                    assert_int!(parser.tree, *literal_id, 2);
                });
            });
        });
    }

    #[test]
    fn test_parse_enum_with_static_parameters() {
        let mut test = TestParser::new(
            r###"
enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    A = 1
    B = T
    @if(IsSomething)
    C = 3
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum(None).unwrap();
        assert_node!(parser.tree, enum_id, Enum { name, static_parameters, fields, .. } => {
            // Machine
            assert_string!(parser.session, name.unwrap(), "Machine");

            // <T: int32 = 3, IsSomething: boolean = true>
            assert!(static_parameters.is_some());
            let static_parameters = static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 2);
            // T: int32 = 3
            assert_node!(parser.tree, static_parameters[0], Parameter { name, .. } => {
                assert_string!(parser.session, *name, "T");
            });
            // IsSomething: boolean = true
            assert_node!(parser.tree, static_parameters[1], Parameter { name, .. } => {
                assert_string!(parser.session, *name, "IsSomething");
            });

            assert_eq!(fields.len(), 3);
        });
    }
}
