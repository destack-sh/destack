//! Parse unions.

use dyst_token::TokenType;

use crate::parse::prelude::*;
use crate::{
    Expression, Keyword, NodeId, NodeType, ParseError, ParseResult, Parser, Struct, StructStyle,
    Union, UnionField, Visibility,
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
    /// union _ {} // explicit anonymous union (for disambiguation)
    ///
    /// union(uint4, uint60) Foo<T> { // 4-bit tag with 60-bit content
    ///     A
    ///     B { x: int32, y: T } = 4
    ///     C(boolean)
    ///     D(boolean, count: int32) = 6
    /// }
    ///
    /// // unions can be tagged with enums and include other types with use (like structs)
    /// union(TetrisShapeType) TetrisShape: Entity { // TetrisShape has Entity as super
    ///     ..TetrisGameObject
    ///
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    ///
    /// // implicit anonymous union
    /// boolean | &int32
    /// // desugars to
    /// union { boolean(boolean) = boolean, int32(&int32) = &int32 }
    /// ```
    pub fn eat_union(&mut self, visibility: Option<Visibility>) -> ParseResult<NodeId<Union>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Union)?;

        // optional explicit tag / representation type in `(Type)`
        let (explicit_type, representation_type) =
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open parenthesis
                // tag type
                let ty = self
                    .with_options(self.options.in_type(), |parser| parser.eat_expression())
                    .for_node_type(NodeType::Union)?;

                // representation type
                if self.peek_token(TokenType::Comma).is_ok() {
                    self.bump(); // eat comma
                    let representation_type = self
                        .with_options(self.options.in_type(), |parser| parser.eat_expression())
                        .for_node_type(NodeType::Union)?;
                    self.eat_token(TokenType::CloseParenthesis)
                        .for_node_type(NodeType::Union)?;
                    (Some(ty), Some(representation_type))
                }
                // no representation type
                else {
                    self.eat_token(TokenType::CloseParenthesis)
                        .for_node_type(NodeType::Union)?;
                    (Some(ty), None)
                }
            } else {
                (None, None)
            };

        // optional name
        let name = self.eat_identifier_or_wildcard_maybe()?;

        // optional static parameters: < ... >
        let static_parameters = self
            .eat_static_parameters_maybe()
            .for_node_type(NodeType::Union)?;

        // optional super types: : ...
        let super_types = self
            .eat_super_types_maybe()
            .for_node_type(NodeType::Union)?;

        // body
        self.eat_token(TokenType::OpenBrace)
            .for_node_type(NodeType::Union)?;
        self.eat_newlines_maybe()?;
        let union_id = self
            .eat_union_body(visibility)
            .for_node_type(NodeType::Union)?;
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Union)?;

        // fill in header data
        let union = self.tree.get_mut(union_id);
        union.name = name;
        union.static_parameters = static_parameters;
        union.tag_type = explicit_type;
        union.representation_type = representation_type;
        union.super_types = super_types;
        self.tree.set_span(union_id, self.get_span_from(start));

        Ok(union_id)
    }

    /// Eat a union body (without the header or `{` and `}`)
    fn eat_union_body(&mut self, visibility: Option<Visibility>) -> ParseResult<NodeId<Union>> {
        let start = self.mark();

        // eat everything
        let mut fields: Vec<NodeId<UnionField>> = Vec::new();
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
            // union field
            else if self.peek_union_field().is_ok() {
                let field = self.eat_union_field().for_node_type(NodeType::Union)?;
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

        let union_id = self.tree.allocate(
            Union {
                name: None,
                visibility,
                static_parameters: None,
                super_types: None,
                tag_type: None,
                representation_type: None,
                fields,
                expressions,
            },
            self.get_span_from(start),
        );
        Ok(union_id)
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

        // optional payload type
        let payload_type: Option<NodeId<Expression>> = {
            let fields = {
                // struct tuple type
                if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                    self.bump(); // eat open parenthesis
                    let tuple_fields = self
                        .eat_struct_tuple_body()
                        .for_node_type(NodeType::Union)?;
                    self.eat_token(TokenType::CloseParenthesis)
                        .for_node_type(NodeType::Union)?;
                    Some((StructStyle::Tuple, tuple_fields))
                }
                // struct struct type
                else if self.peek_token(TokenType::OpenBrace).is_ok() {
                    self.bump(); // eat open brace
                    let (fields, _) = self
                        .eat_struct_body(StructStyle::Struct)
                        .for_node_type(NodeType::Union)?;
                    self.eat_token(TokenType::CloseBrace)
                        .for_node_type(NodeType::Union)?;
                    Some((StructStyle::Struct, fields))
                }
                // no payload type
                else {
                    None
                }
            };

            // map to struct type
            if let Some((style, fields)) = fields {
                let struct_id = self.tree.allocate(
                    Struct {
                        name: None,
                        visibility: None,
                        style,
                        super_types: None,
                        representation_type: None,
                        static_parameters: None,
                        fields,
                        expressions: Vec::new(),
                    },
                    self.get_span_from(start),
                );
                Some(
                    self.tree
                        .allocate(Expression::Struct(struct_id), self.get_span_from(start)),
                )
            } else {
                None
            }
        };

        // optional default value: `= <expr>`
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.bump(); // eat assign
            Some(self.eat_expression().for_node_type(NodeType::Union)?)
        } else {
            None
        };

        let field_id = self.tree.allocate(
            UnionField {
                name,
                r#type: payload_type,
                value,
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
        Expression, Parameter, StructField, StructStyle, TypeLiteral, UnaryOperator, Union,
        UnionField, assert_int, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_explicit_anonymous_union() {
        let mut test = TestParser::new(
            r###"
union { A, B }
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let union_id = parser.eat_union(None).unwrap();
        assert_node!(parser.tree, union_id, Union { name, tag_type, fields, expressions, .. } => {
            assert!(name.is_none());
            assert!(tag_type.is_none());
            assert!(expressions.is_empty());
            assert_eq!(fields.len(), 2);

            // A
            assert_node!(parser.tree, fields[0], UnionField { name, r#type, value } => {
                assert_string!(parser.session, *name, "A");
                assert!(r#type.is_none());
                assert!(value.is_none());
            });

            // B
            assert_node!(parser.tree, fields[1], UnionField { name, r#type, value } => {
                assert_string!(parser.session, *name, "B");
                assert!(r#type.is_none());
                assert!(value.is_none());
            });
        });
    }

    #[test]
    fn test_parse_explicit_union_with_super_types() {
        let mut test = TestParser::new(
            r###"
union Foo: Bar {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let union_id = parser.eat_union(None).unwrap();
        assert_node!(parser.tree, union_id, Union { name, super_types, fields, expressions, .. } => {
            assert_string!(parser.session, name.unwrap(), "Foo");
            assert!(expressions.is_empty());
            assert!(fields.is_empty());

            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_explicit_union_with_type_and_name() {
        let mut test = TestParser::new(
            r###"
union(uint4, uint60) Foo<T>: Boz {
    A
    C(boolean)
    D(boolean, count: int32) = 6
    E { x: int32, y: T }
    
    ..Bar
    function myFunc() { // nested declaration
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let union_id = parser.eat_union(None).unwrap();
        assert_node!(parser.tree, union_id, Union { name, tag_type, representation_type, static_parameters, fields, expressions, super_types, .. } => {
            assert_string!(parser.session, name.unwrap(), "Foo");

            // (uint4, uint60)
            assert_node!(parser.tree, tag_type.unwrap(), Expression::TypeLiteral(literal_id) => {
                assert_node!(parser.tree, *literal_id, TypeLiteral::Int(int_ty) => {
                    assert_eq!(int_ty.width, Some(4));
                    assert!(!int_ty.is_signed);
                });
            });
            assert_node!(parser.tree, representation_type.unwrap(), Expression::TypeLiteral(literal_id) => {
                assert_node!(parser.tree, *literal_id, TypeLiteral::Int(int_ty) => {
                    assert_eq!(int_ty.width, Some(60));
                    assert!(!int_ty.is_signed);
                });
            });

            // <T>
            assert!(static_parameters.is_some());
            let static_parameters = static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter { name, r#type, .. } => {
                assert_string!(parser.session, *name, "T");
                assert!(r#type.is_none());
            });

            // : Boz
            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Boz");
            });

            assert_eq!(expressions.len(), 2);
            assert_eq!(fields.len(), 4);

            // A
            assert_node!(parser.tree, fields[0], UnionField { name, r#type, value } => {
                assert_string!(parser.session, *name, "A");
                assert!(r#type.is_none());
                assert!(value.is_none());
            });

            // C(boolean)
            assert_node!(parser.tree, fields[1], UnionField { name, r#type, value } => {
                assert_string!(parser.session, *name, "C");
                assert_node!(parser.tree, r#type.unwrap(), Expression::Struct(struct_id) => {
                    let struct_ = parser.tree.get(*struct_id);
                    assert_eq!(struct_.style, StructStyle::Tuple);
                    assert_eq!(struct_.fields.len(), 1);

                    // boolean
                    assert_node!(parser.tree, struct_.fields[0], StructField { name, r#type, .. } => {
                        assert!(name.is_none());
                        assert_node!(parser.tree, *r#type, Expression::TypeLiteral(literal_id) => {
                            assert_node!(parser.tree, *literal_id, TypeLiteral::Boolean);
                        });
                    });
                });
                assert!(value.is_none());
            });

            // D(boolean, count: int32) = 6
            assert_node!(parser.tree, fields[2], UnionField { name, r#type, value } => {
                assert_string!(parser.session, *name, "D");

                // (boolean, count: int32)
                assert_node!(parser.tree, r#type.unwrap(), Expression::Struct(struct_id) => {
                    let struct_ = parser.tree.get(*struct_id);
                    assert_eq!(struct_.style, StructStyle::Tuple);
                    assert_eq!(struct_.fields.len(), 2);

                    // boolean
                    assert_node!(parser.tree, struct_.fields[0], StructField { name, r#type, .. } => {
                        assert!(name.is_none());
                        assert_node!(parser.tree, *r#type, Expression::TypeLiteral(literal_id) => {
                            assert_node!(parser.tree, *literal_id, TypeLiteral::Boolean);
                        });
                    });

                    // count: int32
                    assert_node!(parser.tree, struct_.fields[1], StructField { name, r#type, .. } => {
                        assert_string!(parser.session, name.unwrap(), "count");
                        assert_node!(parser.tree, *r#type, Expression::TypeLiteral(literal_id) => {
                            assert_node!(parser.tree, *literal_id, TypeLiteral::Int(int_ty) => {
                                assert_eq!(int_ty.width, Some(32));
                                assert!(int_ty.is_signed);
                            });
                        });
                    });
                });

                // = 6
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(literal_id) => {
                    assert_int!(parser.tree, *literal_id, 6);
                });
            });

            // E { x: int32, y: T }
            assert_node!(parser.tree, fields[3], UnionField { name, r#type, .. } => {
                assert_string!(parser.session, *name, "E");
                assert_node!(parser.tree, r#type.unwrap(), Expression::Struct(struct_id) => {
                    let struct_ = parser.tree.get(*struct_id);
                    assert_eq!(struct_.style, StructStyle::Struct);
                    assert_eq!(struct_.fields.len(), 2);

                    // x: int32
                    assert_node!(parser.tree, struct_.fields[0], StructField { name, r#type, .. } => {
                        // x
                        assert_string!(parser.session, name.unwrap(), "x");
                        // int32
                        assert_node!(parser.tree, *r#type, Expression::TypeLiteral(literal_id) => {
                            assert_node!(parser.tree, *literal_id, TypeLiteral::Int(int_ty) => {
                                assert_eq!(int_ty.width, Some(32));
                                assert!(int_ty.is_signed);
                            });
                        });
                    });

                    // y: T
                    assert_node!(parser.tree, struct_.fields[1], StructField { name, r#type, .. } => {
                        // y
                        assert_string!(parser.session, name.unwrap(), "y");
                        // T
                        assert_node!(parser.tree, *r#type, Expression::Path { path, static_arguments: _ } => {
                            assert_path!(parser.session, *path, "T");
                        });
                    });
                });
            });

            // ..Bar
            assert_node!(parser.tree, expressions[0], Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::Spread);
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Bar");
                });
            });

        });
    }
}
