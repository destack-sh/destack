//! Parse unions.

use destack_language_token::{TokenType, clean_identifier};

use crate::{
    Keyword, Let, NodeId, ParseResult, Parser, TupleField, Type, Union, UnionField, UnionStyle, Use,
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
        let mut usings: Vec<NodeId<Use>> = Vec::new();
        let mut lets: Vec<NodeId<Let>> = Vec::new();
        loop {
            // stop on closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop()?;
            }
            // let
            else if self.peek_keyword(Keyword::Let).is_ok()
                || self.peek_keyword(Keyword::Var).is_ok()
            {
                let let_declaration = self.eat_let_or_var()?;
                lets.push(let_declaration);
            }
            // use
            else if self.peek_keyword(Keyword::Use).is_ok() {
                let using = self.eat_use()?;
                usings.push(using);
            }
            // field
            else {
                let field = self.eat_union_field()?;
                fields.push(field);
            }
        }

        let union_id = self.tree.allocate(
            Union {
                name: None,
                style: UnionStyle::Explicit,
                r#type: None,
                fields,
                usings,
                lets,
            },
            self.get_span_from(start),
        );
        Ok(union_id)
    }

    /// Eat a single union field.
    fn eat_union_field(&mut self) -> ParseResult<NodeId<UnionField>> {
        let start = self.mark();
        let name = self.eat_identifier()?;

        // optional payload type: (Type ...) or none for unit variant
        // if there is only one field we unwrap the implicit tuple
        let payload_type: Option<NodeId<Type>> =
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let tuple_id = self.eat_tuple()?;
                let tuple = self.tree.get(tuple_id);
                // unwrap single positional tuple
                if tuple.elements.is_empty() {
                    self.tree.free(tuple_id);
                    None
                } else if tuple.elements.len() == 1
                    && let &TupleField::Positional {
                        r#type: inner_type_id,
                    } = self.tree.get(tuple.elements[0])
                {
                    self.tree.free(tuple_id);
                    Some(inner_type_id)
                } else {
                    let type_id = self
                        .tree
                        .allocate(Type::Tuple(tuple_id), self.get_span_from(start));
                    Some(type_id)
                }
            } else {
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
                usings: Vec::new(),
                lets: Vec::new(),
            },
            self.get_span_from(start),
        );
        Ok(union_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{
        Expression, IntType, Mutability, Parser, PrimitiveType, ScalarLiteral, TupleField, Type,
        UnionStyle,
    };

    #[test]
    fn test_parse_explicit_anonymous_union() {
        let input = r###"
union { A, B }
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // union
        let union_id = parser.eat_union().unwrap();
        let uni = parser.tree.get(union_id);
        assert_eq!(uni.name, None);
        assert!(uni.r#type.is_none());
        assert!(uni.usings.is_empty());
        assert!(uni.lets.is_empty());
        assert_eq!(uni.fields.len(), 2);

        // A
        let f0 = parser.tree.get(uni.fields[0]);
        assert_eq!(f0.name, parser.strings.intern("A"));
        assert!(f0.r#type.is_none());
        assert!(f0.value.is_none());

        // B
        let f1 = parser.tree.get(uni.fields[1]);
        assert_eq!(f1.name, parser.strings.intern("B"));
        assert!(f1.r#type.is_none());
        assert!(f1.value.is_none());
    }

    #[test]
    fn test_parse_explicit_heterogeneous_union() {
        let input = r###"
union(uint4) Foo {
    A

    use Bar

    C(boolean)
    
    D(boolean, count: int32) = 6
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // union(uint4) Foo use Bar
        let union_id = parser.eat_union().unwrap();
        let uni = parser.tree.get(union_id);
        assert_eq!(uni.name, Some(parser.strings.intern("Foo")));
        // (uint4)
        let ty = uni.r#type.expect("expected explicit type");
        match parser.tree.get(ty) {
            Type::Primitive(PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 4);
                assert!(!int_ty.is_signed);
            }
            other => panic!("expected primitive int type, got {other:?}"),
        }

        assert_eq!(uni.usings.len(), 1);
        assert!(uni.lets.is_empty());
        assert_eq!(uni.fields.len(), 3);

        // use Bar
        let using = parser.tree.get(uni.usings[0]);
        assert_eq!(using.clauses.len(), 1);

        // A
        let a = parser.tree.get(uni.fields[0]);
        assert_eq!(a.name, parser.strings.intern("A"));
        assert!(a.r#type.is_none());
        assert!(a.value.is_none());

        // C(boolean)
        let c = parser.tree.get(uni.fields[1]);
        // (boolean)
        match c.r#type {
            Some(ty_id) => match parser.tree.get(ty_id) {
                Type::Primitive(PrimitiveType::Boolean) => {}
                _ => panic!("expected boolean"),
            },
            None => panic!("expected boolean"),
        }

        // D(boolean, count: int32) = 6
        let d = parser.tree.get(uni.fields[2]);
        assert!(d.r#type.is_some());
        match parser.tree.get(d.r#type.unwrap()) {
            &Type::Tuple(tuple_id) => {
                let tuple = parser.tree.get(tuple_id);
                assert_eq!(tuple.elements.len(), 2);
                // boolean
                match parser.tree.get(tuple.elements[0]) {
                    TupleField::Positional { r#type } => {
                        assert_eq!(
                            *parser.tree.get(*r#type),
                            Type::Primitive(PrimitiveType::Boolean)
                        )
                    }
                    _ => panic!("expected boolean"),
                }
                // count: int32
                match parser.tree.get(tuple.elements[1]) {
                    TupleField::Named { name, r#type } => {
                        assert_eq!(*name, parser.strings.intern("count"));
                        assert_eq!(
                            *parser.tree.get(*r#type),
                            Type::Primitive(PrimitiveType::Int(IntType {
                                width: 32,
                                is_signed: true
                            }))
                        );
                    }
                    _ => panic!("expected named field 'count'"),
                }
            }
            _ => panic!("expected tuple"),
        }
        // = 6
        assert!(d.value.is_some());
        match parser.tree.get(d.value.unwrap()) {
            Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                ScalarLiteral::Integer(n, _) => assert_eq!(*n, 6),
                _ => panic!("expected integer"),
            },
            _ => panic!("expected integer"),
        }
    }

    #[test]
    fn test_parse_implicit_union() {
        let input = r"A | ?B | *C | ?*var D";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        // A | ?B | *C | ?*var D
        let union_id = parser.eat_implicit_union(None).unwrap();
        let union = parser.tree.get(union_id);
        assert_eq!(union.name, None);
        assert_eq!(union.style, UnionStyle::Implicit);
        assert!(union.r#type.is_none());
        assert!(union.usings.is_empty());
        assert!(union.lets.is_empty());
        assert_eq!(union.fields.len(), 4);

        // A
        let a = parser.tree.get(union.fields[0]);
        assert_eq!(a.name, parser.strings.intern("A"));
        match a.r#type {
            Some(ty_id) => match parser.tree.get(ty_id) {
                Type::Path {
                    path: path_id,
                    static_arguments: _,
                } => {
                    let path = parser.paths.get(*path_id);
                    assert_eq!(path.segments.len(), 1);
                    assert_eq!(path.segments[0], parser.strings.intern("A"));
                }
                _ => panic!("expected path type"),
            },
            None => panic!("expected path type"),
        }
        assert!(a.value.is_none());

        // ?B
        let b = parser.tree.get(union.fields[1]);
        assert_eq!(b.name, parser.strings.intern("B"));
        match b.r#type {
            Some(ty_id) => match parser.tree.get(ty_id) {
                Type::Maybe(inner_ty_id) => match parser.tree.get(*inner_ty_id) {
                    Type::Path {
                        path: path_id,
                        static_arguments: _,
                    } => {
                        let path = parser.paths.get(*path_id);
                        assert_eq!(path.segments.len(), 1);
                        assert_eq!(path.segments[0], parser.strings.intern("B"));
                    }
                    _ => panic!("expected path type"),
                },
                _ => panic!("expected optional type"),
            },
            None => panic!("expected optional type"),
        }
        assert!(b.value.is_none());

        // *C
        let c = parser.tree.get(union.fields[2]);
        assert_eq!(c.name, parser.strings.intern("C"));
        match c.r#type {
            Some(ty_id) => match parser.tree.get(ty_id) {
                &Type::Pointer {
                    target: ptr_ty_id,
                    mutability: _,
                } => match parser.tree.get(ptr_ty_id) {
                    &Type::Path {
                        path: path_id,
                        static_arguments: _,
                    } => {
                        let path = parser.paths.get(path_id);
                        assert_eq!(path.segments.len(), 1);
                        assert_eq!(path.segments[0], parser.strings.intern("C"));
                    }
                    _ => panic!("expected path type"),
                },
                _ => panic!("expected pointer type"),
            },
            None => panic!("expected pointer type"),
        }
        assert!(c.value.is_none());

        // ?*var D
        let d = parser.tree.get(union.fields[3]);
        assert_eq!(d.name, parser.strings.intern("D"));
        match d.r#type {
            Some(ty_id) => match parser.tree.get(ty_id) {
                Type::Maybe(inner_ty_id) => match parser.tree.get(*inner_ty_id) {
                    Type::Pointer {
                        target: ptr_ty_id,
                        mutability,
                    } => {
                        assert_eq!(*mutability, Mutability::Mutable);
                        match parser.tree.get(*ptr_ty_id) {
                            Type::Path {
                                path: path_id,
                                static_arguments: _,
                            } => {
                                let path = parser.paths.get(*path_id);
                                assert_eq!(path.segments.len(), 1);
                                assert_eq!(path.segments[0], parser.strings.intern("D"));
                            }
                            _ => panic!("expected path type"),
                        }
                    }
                    _ => panic!("expected pointer type"),
                },
                _ => panic!("expected optional type"),
            },
            None => panic!("expected optional pointer type"),
        }
        assert!(d.value.is_none());
    }
}
