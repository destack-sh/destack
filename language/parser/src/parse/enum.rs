use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Declaration, DeclarationDescriptor, EnumField, Generics, Heritage, Keyword, LocalNodeId,
    NodeType, Property, TokenType,
};

impl Parser {
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
    /// enum Foo {
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
    pub fn eat_enum(
        &mut self,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Enum)?;

        // optional name
        descriptor = descriptor.with_name_maybe(self.eat_name_maybe()?);

        // optional static parameters: < ... >
        let static_parameters = self
            .eat_static_parameters_maybe()
            .for_node_type(NodeType::Declaration)?;

        // optional extends types
        let extends_types = self
            .eat_extends_types_maybe()
            .for_node_type(NodeType::Declaration)?;

        // optional implements types
        let implements_types = self
            .eat_implements_types_maybe()
            .for_node_type(NodeType::Declaration)?;

        // with
        let with_clauses = self
            .eat_with_header_maybe()
            .for_node_type(NodeType::Declaration)?;

        // where
        let where_clauses = self
            .eat_where_maybe()
            .for_node_type(NodeType::Declaration)?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        self.eat_newlines_maybe()?;
        let (fields, properties) = self.eat_enum_body().for_node_type(NodeType::Declaration)?;
        self.eat_token(TokenType::CloseBrace)?;

        let generics = Generics::new(static_parameters, with_clauses, where_clauses);
        let heritage = Heritage::new(extends_types, implements_types);
        let enum_id = self.tree.insert(
            Declaration::Enum {
                descriptor,
                generics,
                heritage,
                fields,
                properties,
            },
            self.get_span_from(start),
        );

        Ok(enum_id)
    }

    /// Eat an enum body (without the header or `{` and `}`)
    #[allow(clippy::type_complexity)]
    fn eat_enum_body(
        &mut self,
    ) -> ParseResult<(Vec<LocalNodeId<EnumField>>, Vec<LocalNodeId<Property>>)> {
        // eat everything
        let mut fields: Vec<LocalNodeId<EnumField>> = Vec::new();
        let mut properties: Vec<LocalNodeId<Property>> = Vec::new();
        while self.peek().is_ok() {
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
                let field = self.eat_enum_field().for_node_type(NodeType::EnumField)?;
                fields.push(field);
            }
            // eat properties
            else {
                let property_id = self
                    .with_options(self.options.nested().in_variant(), |parser| {
                        parser.try_eat_property(TokenType::Newline)
                    })
                    .for_node_type(NodeType::Property)?;
                properties.push(property_id);
            }
        }

        Ok((fields, properties))
    }

    /// Peek an enum field.
    fn peek_enum_field(&self) -> ParseResult<()> {
        if self.peek_name().is_ok()
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
    fn eat_enum_field(&mut self) -> ParseResult<LocalNodeId<EnumField>> {
        let start = self.mark();
        let name = self.eat_name().for_node_type(NodeType::EnumField)?;

        // optional `= <expr>` value
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            let value = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            Some(value)
        } else {
            None
        };

        let field_id = self
            .tree
            .insert(EnumField { name, value }, self.get_span_from(start));
        Ok(field_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Declaration, DeclarationDescriptor, DeclarationKind, EnumField, Expression, Parameter,
        ScalarLiteral, WhereClause, WithClause,
    };

    use crate::{TestParser, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_enum_with_extends_types() {
        let mut test = TestParser::new(
            r###"
enum Foo extends Day {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { descriptor, generics, heritage, fields, properties, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(properties.is_empty());
            assert!(fields.is_empty());
            assert!(generics.is_empty());

            assert!(heritage.implements_types.is_none());

            let supers = heritage.extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Day");
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

        let enum_id = parser.eat_enum(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { descriptor, fields, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(descriptor.name.is_none());
            assert_eq!(fields.len(), 2);
            // Success
            assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                assert_string!(parser, name.string(), "Success");
                assert!(value.is_none());
            });
            // Failure
            assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                assert_string!(parser, name.string(), "Failure");
                assert!(value.is_none());
            });
        });
    }

    #[test]
    fn test_parse_enum_with_type_name_and_values() {
        let mut test = TestParser::new(
            r###"
enum Foo extends Day {

    Baz = 1

    Qux = 2

    function myFunc() { // nested declaration
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { descriptor, fields, generics, heritage, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            // Foo
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");

            // extends: Day
            assert!(generics.is_empty());

            let supers = heritage.extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Day");
            });
            assert!(heritage.implements_types.is_none());

            assert_eq!(fields.len(), 2);

            // Baz = 1
            assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                assert_string!(parser, name.string(), "Baz");
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            // Qux = 2
            assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                assert_string!(parser, name.string(), "Qux");
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
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

        let enum_id = parser.eat_enum(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { descriptor, generics, fields, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            // Machine
            assert_string!(parser, descriptor.name.unwrap().string(), "Machine");

            // <T: int32 = 3, IsSomething: boolean = true>
            assert!(!generics.is_empty());
            assert!(generics.static_parameters.is_some());
            let static_parameters = generics.static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 2);
            // T: int32 = 3
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "T");
            });
            // IsSomething: boolean = true
            assert_node!(parser.tree, static_parameters[1], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "IsSomething");
            });

            assert_eq!(fields.len(), 3);
        });
    }

    #[test]
    fn test_parse_enum_with_with_and_where() {
        let mut test = TestParser::new(
            r###"
enum Foo with Context where Requirement: Interface {
    Value
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { descriptor, generics, fields, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            // with Context
            assert!(!generics.is_empty());
            let with_clauses = generics.with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_clauses.len(), 1);
            assert_node!(parser.tree, with_clauses[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });

            // where Requirement: Interface
            let where_clauses = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause::Assertion { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Interface");
                });
            });

            assert_eq!(fields.len(), 1);
        });
    }
}
