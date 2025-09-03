//! Parse unions and enums (which are just sugar for unions).

use destack_language_token::{TokenType, clean_identifier};

use crate::{
    Keyword, NodeId, ParseResult, Parser, TupleField, Type, Union, UnionField, UnionStyle, Using,
};

impl<'a> Parser<'a> {
    /// Eat a union declaration.
    ///
    /// Examples:
    /// ```
    /// union { // anonymous union (for use as a value)
    ///     myField: int32
    ///     myOtherField: boolean
    /// }
    ///
    /// union(uint4) Foo {
    ///     A // semicolon optional
    ///     B { x: int32, y: int32 } = 4
    ///     C(boolean)
    ///     D(boolean, int32) = 6
    /// }
    ///
    /// // unions can be tagged with enums and include other types with using (like structs)
    /// union(TetrisShapeType) TetrisShape using GameObject {
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

        // optional name (avoid consuming `using` as a name)
        let name = if self.peek_keyword(Keyword::Using).is_err()
            && self.peek_token(TokenType::Identifier).is_ok()
        {
            Some(self.eat_identifier()?)
        } else {
            None
        };

        // optional `using ...` header
        let using: Option<NodeId<Using>> = if self.peek_keyword(Keyword::Using).is_ok() {
            self.eat_keyword(Keyword::Using)?;
            let using = self.eat_using_header()?;
            Some(using)
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
        union.using = using;
        self.tree.set_span(union_id, self.span_from(start));

        Ok(union_id)
    }

    /// Eat a union body (without the header or `{` and `}`)
    pub fn eat_union_body(&mut self) -> ParseResult<NodeId<Union>> {
        let start = self.mark();

        // empty body
        let mut fields: Vec<NodeId<UnionField>> = Vec::new();
        if self.peek_token(TokenType::CloseBrace).is_ok() {
            let union_id = self.tree.allocate(
                Union {
                    name: None,
                    style: UnionStyle::Explicit,
                    r#type: None,
                    fields,
                    using: None,
                },
                self.span_from(start),
            );
            return Ok(union_id);
        }

        // parse first field
        let first_field = self.eat_union_field()?;
        fields.push(first_field);

        // parse more fields while comma/newline separated
        while self.peek_item_stop().is_ok() {
            self.eat_item_stop()?;
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            let field = self.eat_union_field()?;
            fields.push(field);
        }

        let union_id = self.tree.allocate(
            Union {
                name: None,
                style: UnionStyle::Explicit,
                r#type: None,
                fields,
                using: None,
            },
            self.span_from(start),
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
                        .allocate(Type::Tuple(tuple_id), self.span_from(start));
                    Some(type_id)
                }
            } else {
                None
            };

        // optional default value: `= <expr>`
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression()?)
        } else {
            None
        };

        let field_id = self.tree.allocate(
            UnionField {
                name,
                r#type: payload_type,
                value,
            },
            self.span_from(start),
        );
        Ok(field_id)
    }

    /// Eat an implicit union of scalars (like `A | B | C`).
    /// Field names are just the literal type names.
    pub fn eat_implicit_union(&mut self) -> ParseResult<NodeId<Union>> {
        let start = self.mark();
        let mut fields: Vec<NodeId<UnionField>> = Vec::new();

        // eat types separated with `|`
        loop {
            let field_start = self.mark();
            let field_id = self.eat_scalar_type()?;

            // field name is just type name without prefix/fluff
            let field_span = self.tree.get_span(field_id);
            let mut field_str = self.get_span_str(field_span);
            if field_str.contains(' ') {
                // split `*var T` and such cleanly
                field_str = field_str.split(' ').nth_back(0).unwrap()
            }
            let field_str_clean = clean_identifier(field_str);

            let union_field_id = self.tree.allocate(
                UnionField {
                    name: self.strings.intern(field_str_clean),
                    value: None,
                    r#type: Some(field_id),
                },
                self.span_from(field_start),
            );
            fields.push(union_field_id);

            if self.peek_token(TokenType::BitwiseOr).is_ok() {
                self.bump();
                continue;
            } else {
                break;
            }
        }

        let union_id = self.tree.allocate(
            Union {
                name: None,
                style: UnionStyle::Implicit,
                r#type: None,
                fields,
                using: None,
            },
            self.span_from(start),
        );
        Ok(union_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{
        Expression, Mutability, Parser, PrimitiveType, ScalarLiteral, TupleField, Type, UnionStyle,
    };

    #[test]
    fn test_parse_union_anonymous() {
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
        assert!(uni.using.is_none());
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
    fn test_parse_union_with_types() {
        let input = r###"
union(uint4) Foo using Bar {
    A

    C(boolean)
    
    D(boolean, count: int32) = 6
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // union(uint4) Foo using Bar
        let union_id = parser.eat_union().unwrap();
        let uni = parser.tree.get(union_id);
        assert_eq!(uni.name, Some(parser.strings.intern("Foo")));
        // (uint4)
        let ty = uni.r#type.expect("expected explicit type");
        match parser.tree.get(ty) {
            Type::Primitive(crate::PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 4);
                assert!(!int_ty.is_signed);
            }
            other => panic!("expected primitive int type, got {other:?}"),
        }
        // using Bar
        assert!(uni.using.is_some());
        assert_eq!(uni.fields.len(), 3);

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
                            Type::Primitive(PrimitiveType::Int(crate::IntType {
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
        let union_id = parser.eat_implicit_union().unwrap();
        let union = parser.tree.get(union_id);
        assert_eq!(union.name, None);
        assert_eq!(union.style, UnionStyle::Implicit);
        assert!(union.r#type.is_none());
        assert!(union.using.is_none());
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
