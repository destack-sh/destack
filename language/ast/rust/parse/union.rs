//! Parse unions.

use dyst_language_token::{TokenType, clean_identifier};

use crate::{
    Keyword, NodeId, ParseError, ParseResult, Parser, Statement, TupleField, Type, Union,
    UnionField, UnionStyle,
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
    /// union(uint4) Foo {
    ///     A
    ///     B { x: int32, y: int32 } = 4
    ///     C(boolean)
    ///     D(boolean, int32) = 6
    /// }
    ///
    /// // unions can be tagged with enums and include other types with use (like structs)
    /// union(TetrisShapeType) TetrisShape {
    ///     use GameObject
    ///     ...
    ///    
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    /// ```
    pub fn eat_union(&mut self) -> ParseResult<NodeId<Union>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Union)?;

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
        let union_id = self.eat_union_body()?;
        self.eat_token(TokenType::CloseBrace)?;

        // fill in header data
        let union = self.tree.get_mut(union_id);
        union.name = name;
        union.r#type = explicit_type;
        self.tree.set_span(union_id, self.get_span_from(start));

        Ok(union_id)
    }

    /// Eat a union body (without the header or `{` and `}`)
    pub fn eat_union_body(&mut self) -> ParseResult<NodeId<Union>> {
        let start = self.mark();

        // eat everything
        let mut fields: Vec<NodeId<UnionField>> = Vec::new();
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
            // union field
            else if self.peek_union_field().is_ok() {
                let field = self.eat_union_field()?;
                fields.push(field);
            }
            // any other statement
            else {
                let statement = self.eat_statement()?;
                statements.push(statement);
            }
        }

        let union_id = self.tree.allocate(
            Union {
                name: None,
                style: UnionStyle::Explicit,
                r#type: None,
                fields,
                statements,
            },
            self.get_span_from(start),
        );
        Ok(union_id)
    }

    /// Peek a union field.
    fn peek_union_field(&mut self) -> ParseResult<()> {
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
            Err(ParseError::UnexpectedToken(self.peek()?.span))
        }
    }

    /// Eat a single union field.
    fn eat_union_field(&mut self) -> ParseResult<NodeId<UnionField>> {
        let start = self.mark();
        let name = self.eat_identifier()?;

        // optional payload type: (Type ...) or none for unit variant
        // if there is only one field we unwrap the implicit tuple (it's not really a tuple then)
        let payload_type: Option<NodeId<Type>> =
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let tuple_id = self.eat_tuple()?;
                let tuple = self.tree.get(tuple_id);

                // empty tuple
                if tuple.elements.is_empty() {
                    self.tree.free(tuple_id);
                    None
                }
                // single positional tuple (unwrap inner)
                else if tuple.elements.len() == 1
                    && let &TupleField::Positional {
                        r#type: inner_type_id,
                    } = self.tree.get(tuple.elements[0])
                {
                    self.tree.free(tuple_id);
                    Some(inner_type_id)
                }
                // normal tuple (keep outer)
                else {
                    let type_id = self
                        .tree
                        .allocate(Type::Tuple(tuple_id), self.get_span_from(start));
                    Some(type_id)
                }
            }
            // no payload type
            else {
                None
            };

        // optional default value: `= <expr>`
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression(None)?)
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

    /// Eat an implicit union of scalars (like `A | B | C`).
    /// Field names are just the literal type names.
    pub fn eat_implicit_union(
        &mut self,
        first_type: Option<NodeId<Type>>,
    ) -> ParseResult<NodeId<Union>> {
        let start = self.mark();

        // collect all types separated with `|`
        let mut types: Vec<NodeId<Type>> = first_type.into_iter().collect();
        loop {
            let type_id = self.eat_scalar_type()?;
            types.push(type_id);
            if self.peek_token(TokenType::BitwiseOr).is_err() {
                break;
            }
            self.eat_token(TokenType::BitwiseOr)?;
        }

        // map types to union fields
        let mut fields = Vec::new();
        for type_id in types {
            // field name is just type name without prefix/fluff
            let type_span = self.tree.get_span(type_id);
            let mut field_str = self.get_span_str(type_span);
            if field_str.contains(' ') {
                // split `*var T` and such cleanly
                field_str = field_str.split(' ').nth_back(0).unwrap()
            }
            if field_str.contains('.') {
                // split `simulation.geometry.Vector2` cleanly
                field_str = field_str.split('.').nth_back(0).unwrap()
            }
            let field_str_clean = clean_identifier(field_str);

            // map type to union field
            let union_field = self.tree.allocate(
                UnionField {
                    name: self.strings.intern(field_str_clean),
                    value: None,
                    r#type: Some(type_id),
                },
                type_span,
            );
            fields.push(union_field);
        }

        let union_id = self.tree.allocate(
            Union {
                name: None,
                style: UnionStyle::Implicit,
                r#type: None,
                fields,
                statements: Vec::new(),
            },
            self.get_span_from(start),
        );
        Ok(union_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Expression, Mutability, PrimitiveType, Statement, TupleField, Type, Union, UnionField,
        UnionStyle, Use, assert_int, assert_node,
    };

    #[test]
    fn test_parse_explicit_anonymous_union() {
        let test = TestParser::new(
            r###"
union { A, B }
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let union_id = parser.eat_union().unwrap();
        assert_node!(parser.tree, union_id, Union { name, r#type, style, fields, statements } => {
            assert!(name.is_none());
            assert!(r#type.is_none());
            assert_eq!(*style, UnionStyle::Explicit);
            assert!(statements.is_empty());
            assert_eq!(fields.len(), 2);

            // A
            assert_node!(parser.tree, fields[0], UnionField { name, r#type, value } => {
                assert_eq!(*name, parser.strings.intern("A"));
                assert!(r#type.is_none());
                assert!(value.is_none());
            });

            // B
            assert_node!(parser.tree, fields[1], UnionField { name, r#type, value } => {
                assert_eq!(*name, parser.strings.intern("B"));
                assert!(r#type.is_none());
                assert!(value.is_none());
            });
        });
    }

    #[test]
    fn test_parse_explicit_union_with_type_and_name() {
        let test = TestParser::new(
            r###"
union(uint4) Foo {
    A
    use Bar
    C(boolean)
    D(boolean, count: int32) = 6

    function myFunc() { // nested declaration
    }
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let union_id = parser.eat_union().unwrap();
        assert_node!(parser.tree, union_id, Union { name, r#type, style, fields, statements } => {
            assert_eq!(*name, Some(parser.strings.intern("Foo")));
            assert_eq!(*style, UnionStyle::Explicit);

            // (uint4)
            assert_node!(parser.tree, r#type.unwrap(), Type::Primitive(PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 4);
                assert!(!int_ty.is_signed);
            });

            assert_eq!(statements.len(), 2);
            assert_eq!(fields.len(), 3);

            // use Bar
            assert_node!(parser.tree, statements[0], Statement::Use(use_id) => {
                assert_node!(parser.tree, *use_id, Use { clauses, .. } => {
                    assert_eq!(clauses.len(), 1);
                });
            });

            // A
            assert_node!(parser.tree, fields[0], UnionField { name, r#type, value } => {
                assert_eq!(*name, parser.strings.intern("A"));
                assert!(r#type.is_none());
                assert!(value.is_none());
            });

            // C(boolean)
            assert_node!(parser.tree, fields[1], UnionField { name, r#type, value } => {
                assert_eq!(*name, parser.strings.intern("C"));
                assert_node!(parser.tree, r#type.unwrap(), Type::Primitive(PrimitiveType::Boolean));
                assert!(value.is_none());
            });

            // D(boolean, count: int32) = 6
            assert_node!(parser.tree, fields[2], UnionField { name, r#type, value } => {
                assert_eq!(*name, parser.strings.intern("D"));

                // tuple type
                assert_node!(parser.tree, r#type.unwrap(), Type::Tuple(tuple_id) => {
                    let tuple = parser.tree.get(*tuple_id);
                    assert_eq!(tuple.elements.len(), 2);

                    // boolean
                    assert_node!(parser.tree, tuple.elements[0], TupleField::Positional { r#type } => {
                        assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Boolean));
                    });

                    // count: int32
                    assert_node!(parser.tree, tuple.elements[1], TupleField::Named { name, r#type } => {
                        assert_eq!(*name, parser.strings.intern("count"));
                        assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(int_ty)) => {
                            assert_eq!(int_ty.width, 32);
                            assert!(int_ty.is_signed);
                        });
                    });
                });

                // = 6
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(literal_id) => {
                    assert_int!(parser.tree, *literal_id, 6);
                });
            });
        });
    }

    #[test]
    fn test_parse_implicit_union_simple() {
        let test = TestParser::new("A | B");
        let mut parser = test.parser();

        let union_id = parser.eat_implicit_union(None).unwrap();
        assert_node!(parser.tree, union_id, Union { name, r#type, style, fields, statements } => {
            assert!(name.is_none());
            assert!(r#type.is_none());
            assert_eq!(*style, UnionStyle::Implicit);
            assert!(statements.is_empty());
            assert_eq!(fields.len(), 2);

            // A
            assert_node!(parser.tree, fields[0], UnionField { name, r#type, value } => {
                assert_eq!(*name, parser.strings.intern("A"));
                assert_node!(parser.tree, r#type.unwrap(), Type::Path { path, static_arguments: _ } => {
                    let path_data = parser.paths.get(*path);
                    assert_eq!(path_data.segments.len(), 1);
                    assert_eq!(path_data.segments[0], parser.strings.intern("A"));
                });
                assert!(value.is_none());
            });

            // B
            assert_node!(parser.tree, fields[1], UnionField { name, r#type, value } => {
                assert_eq!(*name, parser.strings.intern("B"));
                assert_node!(parser.tree, r#type.unwrap(), Type::Path { path, static_arguments: _ } => {
                    let path_data = parser.paths.get(*path);
                    assert_eq!(path_data.segments.len(), 1);
                    assert_eq!(path_data.segments[0], parser.strings.intern("B"));
                });
                assert!(value.is_none());
            });
        });
    }

    #[test]
    fn test_parse_implicit_union_with_optional() {
        let test = TestParser::new("A | ?B");
        let mut parser = test.parser();

        let union_id = parser.eat_implicit_union(None).unwrap();
        assert_node!(parser.tree, union_id, Union { fields, .. } => {
            assert_eq!(fields.len(), 2);

            // A
            assert_node!(parser.tree, fields[0], UnionField { name, r#type, value: _ } => {
                assert_eq!(*name, parser.strings.intern("A"));
                assert_node!(parser.tree, r#type.unwrap(), Type::Path { path, static_arguments: _ } => {
                    let path_data = parser.paths.get(*path);
                    assert_eq!(path_data.segments[0], parser.strings.intern("A"));
                });
            });

            // ?B
            assert_node!(parser.tree, fields[1], UnionField { name, r#type, value: _ } => {
                assert_eq!(*name, parser.strings.intern("B"));
                assert_node!(parser.tree, r#type.unwrap(), Type::Maybe(inner_ty_id) => {
                    assert_node!(parser.tree, *inner_ty_id, Type::Path { path, static_arguments: _ } => {
                        let path_data = parser.paths.get(*path);
                        assert_eq!(path_data.segments[0], parser.strings.intern("B"));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_implicit_union_with_pointer() {
        let test = TestParser::new("A | *C");
        let mut parser = test.parser();

        let union_id = parser.eat_implicit_union(None).unwrap();
        assert_node!(parser.tree, union_id, Union { fields, .. } => {
            assert_eq!(fields.len(), 2);

            // *C
            assert_node!(parser.tree, fields[1], UnionField { name, r#type, value: _ } => {
                assert_eq!(*name, parser.strings.intern("C"));
                assert_node!(parser.tree, r#type.unwrap(), Type::Pointer { target, mutability } => {
                    assert_eq!(*mutability, Mutability::Immutable);
                    assert_node!(parser.tree, *target, Type::Path { path, static_arguments: _ } => {
                        let path_data = parser.paths.get(*path);
                        assert_eq!(path_data.segments[0], parser.strings.intern("C"));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_implicit_union_complex() {
        let test = TestParser::new("?*var D");
        let mut parser = test.parser();

        let union_id = parser.eat_implicit_union(None).unwrap();
        assert_node!(parser.tree, union_id, Union { fields, .. } => {
            assert_eq!(fields.len(), 1);

            // ?*var D
            assert_node!(parser.tree, fields[0], UnionField { name, r#type, value: _ } => {
                assert_eq!(*name, parser.strings.intern("D"));
                assert_node!(parser.tree, r#type.unwrap(), Type::Maybe(inner_ty_id) => {
                    assert_node!(parser.tree, *inner_ty_id, Type::Pointer { target, mutability } => {
                        assert_eq!(*mutability, Mutability::Mutable);
                        assert_node!(parser.tree, *target, Type::Path { path, static_arguments: _ } => {
                            let path_data = parser.paths.get(*path);
                            assert_eq!(path_data.segments[0], parser.strings.intern("D"));
                        });
                    });
                });
            });
        });
    }
}
