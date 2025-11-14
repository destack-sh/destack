use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

use dyst_ast::{
    Definition, DefinitionMeta, Keyword, NodeId, NodeType, Property, TokenType, UnionField,
};

impl<'a> Parser<'a> {
    /// Eat an *explicit* union declaration (with `union` keyword).
    ///
    /// Examples:
    /// ```
    /// union { // anonymous union (for use as a value)
    ///     myField: int32
    ///     myOtherField: boolean
    /// }
    ///
    /// union(uint4, uint60) Foo<T> { // 4-bit tag with 60-bit content
    ///     A
    ///     B { x: int32, y: T } = 4
    ///     C(boolean)
    ///     D(boolean, count: int32) = 6
    /// }
    ///
    /// // unions can be tagged with enums and include other types with use (like structs)
    /// union(TetrisShapeType) TetrisShape { // TetrisShape has Entity as super
    ///     ..TetrisGameObject
    ///
    ///     myFunc() { // nested declaration
    ///     }
    /// }
    ///
    /// // implicit anonymous union
    /// boolean | &int32
    /// // desugars to
    /// union { boolean(boolean) = boolean, int32(&int32) = &int32 }
    /// ```
    pub fn eat_union(&mut self, mut meta: DefinitionMeta) -> ParseResult<NodeId<Definition>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Union)?;

        // optional explicit tag / representation type in `(Type)`
        let (tag_name, tag_type, representation_name, representation_type) =
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open parenthesis

                // tag type
                let tag_type =
                    self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;

                // representation type
                if self.peek_token(TokenType::Comma).is_ok() {
                    self.bump(); // eat comma
                    let representation_type = self
                        .with_options(self.options.in_type(), |parser| parser.eat_expression())?;
                    self.eat_token(TokenType::CloseParenthesis)?;
                    (None, Some(tag_type), None, Some(representation_type))
                }
                // no representation type
                else {
                    self.eat_token(TokenType::CloseParenthesis)?;
                    (None, Some(tag_type), None, None)
                }
            } else {
                (None, None, None, None)
            };

        // optional name
        meta = meta.with_name_maybe(self.eat_name_maybe()?);

        // optional static parameters: < ... >
        let static_parameters = self.eat_static_parameters_maybe()?;

        // optional extends types
        let extends_types = self.eat_extends_types_maybe()?;

        // optional implements types
        let implements_types = self.eat_implements_types_maybe()?;

        // with
        let with_clauses = self.eat_with_header_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;
        self.eat_newlines_maybe()?;
        let (fields, properties) = self.eat_union_body()?;
        self.eat_token(TokenType::CloseBrace)?;

        // union
        let union_id = self.tree.insert(
            Definition::Union {
                meta,
                static_parameters,
                extends_types,
                implements_types,
                tag_name,
                tag_type,
                representation_name,
                representation_type,
                with_clauses,
                where_clauses,
                fields,
                properties,
            },
            self.get_span_from(start),
        );
        self.tree.set_span(union_id, self.get_span_from(start));

        Ok(union_id)
    }

    /// Eat a union body (without the header or `{` and `}`)
    #[allow(clippy::type_complexity)]
    fn eat_union_body(&mut self) -> ParseResult<(Vec<NodeId<UnionField>>, Vec<NodeId<Property>>)> {
        // eat everything
        let mut fields: Vec<NodeId<UnionField>> = Vec::new();
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
            // union field
            else if self.peek_union_field().is_ok() {
                let field = self.eat_union_field()?;
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

    /// Peek a union field.
    fn peek_union_field(&self) -> ParseResult<()> {
        if self.peek_identifier().is_ok()
            && (self.peek_next_token(TokenType::OpenParenthesis).is_ok()
                || self.peek_next_token(TokenType::OpenBrace).is_ok()
                || self.peek_next_token(TokenType::Assign).is_ok()
                || self.peek_next_token(TokenType::Newline).is_ok()
                || self.peek_next_token(TokenType::Comma).is_ok()
                || self.peek_next_token(TokenType::Semicolon).is_ok()
                || self.peek_next_token(TokenType::CloseBrace).is_ok())
        {
            Ok(())
        } else {
            Err(ParseError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ))
        }
    }

    /// Eat a single union field.
    ///
    /// Examples:
    /// ```
    /// A
    /// B(int32) // tuple struct
    /// C(boolean, vec: Vector2) // struct struct
    /// D { x: int32, y: int32 } = 4 // struct struct with tag value
    /// ```
    fn eat_union_field(&mut self) -> ParseResult<NodeId<UnionField>> {
        let start = self.mark();
        let name = self.eat_identifier()?;

        let mut union_field: UnionField = {
            // tuple field
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open parenthesis
                let fields = self
                    .eat_arguments_body(TokenType::CloseParenthesis)
                    .for_node_type(NodeType::UnionField)?;
                self.eat_token(TokenType::CloseParenthesis)?;
                UnionField::Tuple {
                    name,
                    fields,
                    value: None,
                }
            }
            // struct field
            else if self.peek_token(TokenType::OpenBrace).is_ok() {
                self.bump(); // eat open brace
                let fields = self
                    .eat_arguments_body(TokenType::CloseBrace)
                    .for_node_type(NodeType::UnionField)?;
                self.eat_token(TokenType::CloseBrace)?;
                UnionField::Struct {
                    name,
                    fields,
                    value: None,
                }
            }
            // unit field
            else {
                UnionField::Unit { name, value: None }
            }
        };

        // optional default value: `= <expr>`
        if self.peek_token(TokenType::Assign).is_ok() {
            self.bump(); // eat assign 
            let discriminator_value = self.eat_expression().for_node_type(NodeType::UnionField)?;
            match &mut union_field {
                UnionField::Tuple { value, .. }
                | UnionField::Struct { value, .. }
                | UnionField::Unit { value, .. } => *value = Some(discriminator_value),
            }
        }

        let field_id = self.tree.insert(union_field, self.get_span_from(start));
        Ok(field_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        Argument, BinaryOperator, DeclarationKind, Definition, DefinitionMeta, Expression, IntType,
        Name, Parameter, ScalarLiteral, TypeLiteral, UnionField, WhereClause, WithClause,
    };

    use crate::parse::tests::TestParser;
    use crate::{assert_expr_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_explicit_anonymous_union() {
        let mut test = TestParser::new(
            r###"
union { A, B }
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let union_id = parser.eat_union(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, union_id, Definition::Union { meta, tag_type, fields, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(meta.name.is_none());
            assert!(tag_type.is_none());
            assert_eq!(fields.len(), 2);
            assert!(where_clauses.is_none());

            // A
            assert_node!(parser.tree, fields[0], UnionField::Unit { name, value } => {
                assert_string!(parser, *name, "A");
                assert!(value.is_none());
            });

            // B
            assert_node!(parser.tree, fields[1], UnionField::Unit { name, value } => {
                assert_string!(parser, *name, "B");
                assert!(value.is_none());
            });
        });
    }

    #[test]
    fn test_parse_explicit_union_with_extends_types() {
        let mut test = TestParser::new(
            r###"
union Foo extends Bar {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let union_id = parser
            .eat_union(DefinitionMeta {
                kind: DeclarationKind::Definition,
                name: None,
                ..DefinitionMeta::default()
            })
            .unwrap();
        assert_node!(parser.tree, union_id, Definition::Union { meta, extends_types, implements_types, fields, properties, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert!(properties.is_empty());
            assert!(fields.is_empty());
            assert!(where_clauses.is_none());

            assert!(implements_types.is_none());

            let supers = extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_explicit_union_with_type_and_name() {
        let mut test = TestParser::new(
            r###"
union(uint4, uint60) Foo<T> extends Boz implements Shape {
    A
    C(boolean)
    D(boolean, count: int32) = 6
    E { x: int32, y: T }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let union_id = parser
            .eat_union(DefinitionMeta {
                kind: DeclarationKind::Definition,
                name: None,
                ..DefinitionMeta::default()
            })
            .unwrap();
        assert_node!(parser.tree, union_id, Definition::Union { meta, tag_type, representation_type, static_parameters, fields, properties, extends_types, implements_types, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert!(where_clauses.is_none());

            // (uint4, uint60)
            assert_node!(parser.tree, tag_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width, is_signed })) => {
                assert_eq!(*width, Some(4));
                assert!(!*is_signed);
            });
            assert_node!(parser.tree, representation_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width, is_signed })) => {
                assert_eq!(*width, Some(60));
                assert!(!*is_signed);
            });

            // <T>
            assert!(static_parameters.is_some());
            let static_parameters = static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "T");
                assert!(ty.is_none());
            });

            // extends Boz
            let supers = extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Boz");
            });

            // implements Shape
            let implements = implements_types.as_ref().expect("expected implements types");
            assert_eq!(implements.len(), 1);
            assert_node!(parser.tree, implements[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Shape");
            });

            assert_eq!(fields.len(), 4);
            assert_eq!(properties.len(), 1);

            // A
            assert_node!(parser.tree, fields[0], UnionField::Unit { name, value } => {
                assert_string!(parser, *name, "A");
                assert!(value.is_none());
            });

            // C(boolean)
            assert_node!(parser.tree, fields[1], UnionField::Tuple { name, fields, value } => {
                assert_string!(parser, *name, "C");
                // boolean
                assert_node!(parser.tree, fields[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Boolean));
                });
                assert!(value.is_none());
            });

            // D(boolean, count: int32) = 6
            assert_node!(parser.tree, fields[2], UnionField::Tuple { name, fields, value } => {
                assert_string!(parser, *name, "D");

                // boolean
                assert_node!(parser.tree, fields[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Boolean));
                });

                // count: int32
                assert_node!(parser.tree, fields[1], Argument::Named { name: Name::Identifier(name), value, .. } => {
                    assert_string!(parser, *name, "count");
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width, is_signed })) => {
                        assert_eq!(*width, Some(32));
                        assert!(*is_signed);
                    });
                });

                // = 6
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(6)));
            });

            // E { x: int32, y: T }
            assert_node!(parser.tree, fields[3], UnionField::Struct { name, fields, value: _ } => {
                assert_string!(parser, *name, "E");
                // x: int32
                assert_node!(parser.tree, fields[0], Argument::Named { name: Name::Identifier(name), value, .. } => {
                    // x
                    assert_string!(parser, *name, "x");
                    // int32
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width, is_signed })) => {
                        assert_eq!(*width, Some(32));
                        assert!(*is_signed);
                    });
                });

                // y: T
                assert_node!(parser.tree, fields[1], Argument::Named { name: Name::Identifier(name), value, .. } => {
                    // y
                    assert_string!(parser, *name, "y");
                    // T
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_union_with_with_and_where() {
        let mut test = TestParser::new(
            r###"
union Foo with Context where Guard > Limit {
    Value
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let union_id = parser
            .eat_union(DefinitionMeta {
                kind: DeclarationKind::Definition,
                name: None,
                ..DefinitionMeta::default()
            })
            .unwrap();
        assert_node!(parser.tree, union_id, Definition::Union { meta, with_clauses, where_clauses, fields: _, properties, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(properties.is_empty());

            // with Context
            let with_clauses = with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_clauses.len(), 1);
            assert_node!(parser.tree, with_clauses[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });

            // where Guard > Limit
            let where_clauses = where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause::Guard { guard } => {
                assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expr_path!(parser, parser.tree.get(*left), "Guard");
                    assert_expr_path!(parser, parser.tree.get(*right), "Limit");
                });
            });
        });
    }
}
