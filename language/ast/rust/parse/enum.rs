//! Parse enums.

use destack_language_token::TokenType;

use crate::{Enum, Keyword, NodeId, ParseResult, Parser, Type, UnionField};

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

        // optional name (avoid consuming `using` as a name)
        let name = if let Ok(next) = self.peek_token(TokenType::Identifier) {
            let ident_str = self.get_token_str(*next);
            if ident_str != Keyword::Using.as_str()
                && self.peek_token(TokenType::OpenBrace).is_err()
            {
                Some(self.eat_identifier()?)
            } else {
                None
            }
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

        // bail on empty body
        let mut fields: Vec<NodeId<UnionField>> = Vec::new();
        if self.peek_token(TokenType::CloseBrace).is_ok() {
            let enum_id = self.tree.allocate(
                Enum {
                    name: None,
                    r#type: None,
                    fields,
                },
                self.get_span_from(start),
            );
            return Ok(enum_id);
        }

        // parse first field
        let first_field = self.eat_enum_field_as_union_field()?;
        fields.push(first_field);

        // parse more fields while comma/newline separated
        while self.peek_item_stop().is_ok() {
            self.eat_item_stop()?;
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            let field = self.eat_enum_field_as_union_field()?;
            fields.push(field);
        }

        let enum_id = self.tree.allocate(
            Enum {
                name: None,
                r#type: None,
                fields,
            },
            self.get_span_from(start),
        );
        Ok(enum_id)
    }

    /// Eat a single enum field and return it as a UnionField node id.
    fn eat_enum_field_as_union_field(&mut self) -> ParseResult<NodeId<UnionField>> {
        let start = self.mark();
        let name = self.eat_identifier()?;

        // optional `= <expr>` value
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression(None)?)
        } else {
            None
        };

        let field_id = self.tree.allocate(
            UnionField {
                name,
                r#type: None,
                value,
            },
            self.get_span_from(start),
        );
        Ok(field_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Expression, Parser, PrimitiveType, ScalarLiteral, Type};

    #[test]
    fn test_parse_enum_anonymous_simple() {
        let input = r###"
enum {
    Success
    Failure
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let enum_id = parser.eat_enum().unwrap();
        let r#enum = parser.tree.get(enum_id);
        assert_eq!(r#enum.name, None);
        assert!(r#enum.r#type.is_none());
        assert_eq!(r#enum.fields.len(), 2);
        let f0 = parser.tree.get(r#enum.fields[0]);
        assert_eq!(f0.name, parser.strings.intern("Success"));
        assert!(f0.r#type.is_none());
        assert!(f0.value.is_none());
        let f1 = parser.tree.get(r#enum.fields[1]);
        assert_eq!(f1.name, parser.strings.intern("Failure"));
        assert!(f1.r#type.is_none());
        assert!(f1.value.is_none());
    }

    #[test]
    fn test_parse_enum_with_type_name_and_values() {
        let input = r###"
enum(uint8) Foo {

    Baz = 1

    Qux = 2
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // enum(uint8) Foo
        let enum_id = parser.eat_enum().unwrap();
        let r#enum = parser.tree.get(enum_id);
        assert_eq!(r#enum.name, Some(parser.strings.intern("Foo")));
        let ty = r#enum.r#type.expect("expected explicit type");
        match parser.tree.get(ty) {
            Type::Primitive(PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 8);
                assert!(!int_ty.is_signed);
            }
            _ => panic!("expected path type"),
        }
        assert_eq!(r#enum.fields.len(), 2);

        // Baz = 1
        let baz = parser.tree.get(r#enum.fields[0]);
        assert_eq!(baz.name, parser.strings.intern("Baz"));
        assert!(baz.r#type.is_none());
        let baz_val = baz.value.expect("expected value");
        match parser.tree.get(baz_val) {
            Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                _ => panic!("expected integer"),
            },
            _ => panic!("expected scalar literal"),
        }

        // Qux = 2
        let qux = parser.tree.get(r#enum.fields[1]);
        assert_eq!(qux.name, parser.strings.intern("Qux"));
        let qux_val = qux.value.expect("expected value");
        match parser.tree.get(qux_val) {
            Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                ScalarLiteral::Integer(n, _) => assert_eq!(*n, 2),
                _ => panic!("expected integer"),
            },
            _ => panic!("expected scalar literal"),
        }
    }
}
