use crate::parse::prelude::*;
use crate::{Parser, ParserError, ParserResult};

use dyst_ast::{
    Definition, DefinitionMeta, EnumField, Expression, Keyword, NodeId, NodeType, Property,
    TokenType,
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
    pub fn eat_enum(&mut self, mut meta: DefinitionMeta) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Enum)?;

        // optional explicit tag type in `(Type)`
        let tag_type: Option<NodeId<Expression>> =
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open parenthesis
                let ty = self
                    .with_options(self.options.in_type(), |parser| parser.eat_expression())
                    .for_node_type(NodeType::Definition)?;
                self.eat_token(TokenType::CloseParenthesis)?;
                Some(ty)
            } else {
                None
            };

        // optional name
        meta = meta.with_name_maybe(self.eat_name_maybe()?);

        // optional static parameters: < ... >
        let static_parameters = self
            .eat_static_parameters_maybe()
            .for_node_type(NodeType::Definition)?;

        // optional extends types
        let extends_types = self
            .eat_extends_types_maybe()
            .for_node_type(NodeType::Definition)?;

        // optional implements types
        let implements_types = self
            .eat_implements_types_maybe()
            .for_node_type(NodeType::Definition)?;

        // with
        let with_clauses = self
            .eat_with_header_maybe()
            .for_node_type(NodeType::Definition)?;

        // where
        let where_clauses = self.eat_where_maybe().for_node_type(NodeType::Definition)?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;
        self.eat_newlines_maybe()?;
        let (fields, properties) = self.eat_enum_body().for_node_type(NodeType::Definition)?;
        self.eat_token(TokenType::CloseBrace)?;

        let enum_id = self.tree.insert(
            Definition::Enum {
                meta,
                tag_type,
                static_parameters,
                extends_types,
                implements_types,
                with_clauses,
                where_clauses,
                fields,
                properties,
            },
            self.get_span_from(start),
        );

        Ok(enum_id)
    }

    /// Eat an enum body (without the header or `{` and `}`)
    #[allow(clippy::type_complexity)]
    fn eat_enum_body(&mut self) -> ParserResult<(Vec<NodeId<EnumField>>, Vec<NodeId<Property>>)> {
        // eat everything
        let mut fields: Vec<NodeId<EnumField>> = Vec::new();
        let mut properties: Vec<NodeId<Property>> = Vec::new();
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
                let field = self.eat_enum_field().for_node_type(NodeType::EnumField)?;
                fields.push(field);
            }
            // eat properties
            else {
                let property_id = self
                    .with_options(self.options.nested_in_variant(), |parser| {
                        parser.try_eat_property(TokenType::Newline)
                    })
                    .for_node_type(NodeType::Property)?;
                properties.push(property_id);
            }
        }

        Ok((fields, properties))
    }

    /// Peek an enum field.
    fn peek_enum_field(&self) -> ParserResult<()> {
        if self.peek_name().is_ok()
            && (self.peek_next_token(TokenType::Assign).is_ok()
                || self.peek_next_token(TokenType::Newline).is_ok()
                || self.peek_next_token(TokenType::Comma).is_ok()
                || self.peek_next_token(TokenType::Semicolon).is_ok()
                || self.peek_next_token(TokenType::CloseBrace).is_ok())
        {
            Ok(())
        } else {
            Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Identifier,
            ))
        }
    }

    /// Eat a single enum field and return it as a UnionField node id.
    fn eat_enum_field(&mut self) -> ParserResult<NodeId<EnumField>> {
        let start = self.mark();
        let name = self.eat_name().for_node_type(NodeType::EnumField)?;

        // optional `= <expr>` value
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression().for_node_type(NodeType::EnumField)?)
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
    use dyst_ast::{
        DeclarationKind, Definition, DefinitionMeta, EnumField, Expression, IntType, Parameter,
        ScalarLiteral, TypeLiteral, WhereClause, WithClause,
    };

    use crate::parse::tests::TestParser;
    use crate::{assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_enum_with_extends_types() {
        let mut test = TestParser::new(
            r###"
enum Foo extends Day {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, enum_id, Definition::Enum { meta, extends_types, implements_types, fields, properties, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert!(properties.is_empty());
            assert!(fields.is_empty());
            assert!(where_clauses.is_none());

            assert!(implements_types.is_none());

            let supers = extends_types.as_ref().expect("expected extends types");
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

        let enum_id = parser.eat_enum(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, enum_id, Definition::Enum { meta, tag_type, fields, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(meta.name.is_none());
            assert!(tag_type.is_none());
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
enum(uint8) Foo extends Day {

    Baz = 1

    Qux = 2

    function myFunc() { // nested declaration
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, enum_id, Definition::Enum { meta, tag_type, fields, extends_types, implements_types, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            // enum name
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert!(where_clauses.is_none());

            // enum type
            assert_node!(parser.tree, tag_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(8), is_signed: false })));

            // extends: Day
            let supers = extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Day");
            });
            assert!(implements_types.is_none());

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

        let enum_id = parser.eat_enum(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, enum_id, Definition::Enum { meta, static_parameters, fields, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            // Machine
            assert_string!(parser, meta.name.unwrap().string(), "Machine");
            assert!(where_clauses.is_none());

            // <T: int32 = 3, IsSomething: boolean = true>
            assert!(static_parameters.is_some());
            let static_parameters = static_parameters.as_ref().unwrap();
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

        let enum_id = parser.eat_enum(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, enum_id, Definition::Enum { meta, with_clauses, where_clauses, fields, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            // with Context
            let with_clauses = with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_clauses.len(), 1);
            assert_node!(parser.tree, with_clauses[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });

            // where Requirement: Interface
            let where_clauses = where_clauses.as_ref().expect("expected where clauses");
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
