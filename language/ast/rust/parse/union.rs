//! Parse unions and enums (which are just sugar for unions).

use destack_language_token::TokenType;

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
            if self.peek_next_token(TokenType::OpenParenthesis).is_ok() {
                self.eat_token(TokenType::OpenParenthesis)?;
                let ty = self.eat_type()?;
                self.eat_token(TokenType::CloseParenthesis)?;
                Some(ty)
            } else {
                None
            };

        // optional name (avoid consuming `using` as a name)
        let name = if self.peek_keyword(Keyword::Using).is_err()
            && self.peek_next_token(TokenType::Identifier).is_ok()
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
        if self.peek_next_token(TokenType::CloseBrace).is_ok() {
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
            if self.peek_next_token(TokenType::CloseBrace).is_ok() {
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
            if self.peek_next_token(TokenType::OpenParenthesis).is_ok() {
                let tuple_id = self.eat_tuple()?;
                let tuple = self.tree.get(tuple_id);
                // unwrap single positional tuple
                if tuple.elements.len() == 1
                    && let TupleField::Positional {
                        r#type: inner_type_id,
                    } = self.tree.get(tuple.elements[0])
                {
                    Some(*inner_type_id)
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
        let value = if self.peek_next_token(TokenType::Assign).is_ok() {
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

    /// Eat an implicit union (like `A | B | C`).
    pub fn eat_implicit_union(&mut self) -> ParseResult<NodeId<Union>> {
        todo!("nocheckin: implicit union")
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Expression, Parser, PrimitiveType, ScalarLiteral, TupleField, Type};

    #[test]
    fn test_parse_union_anonymous_simple() {
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
    fn test_parse_union_with_type_name_payload_and_values() {
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
}
