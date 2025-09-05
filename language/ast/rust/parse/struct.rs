//! Parse structs.

use destack_language_token::TokenType;

use crate::{Keyword, Let, NodeId, ParseResult, Parser, Struct, StructField, Use};

impl<'a> Parser<'a> {
    /// Eat a struct declaration.
    ///
    /// Examples:
    /// ```
    /// struct { a: int32, b: boolean }
    ///
    /// struct { // anonymous struct (for use as a value)
    ///     myField: int32 // colon optional
    ///     myOtherField: boolean
    /// }
    ///
    /// struct Bar {
    ///     myField: int32
    ///     myOtherField: boolean
    /// }
    ///
    /// struct Foo {
    ///     let x: int32 = 7 // constant
    ///
    ///     use Bar // Foo has a Bar
    ///
    ///     myField: int32
    ///     myOtherField: boolean
    /// }
    /// ```
    pub fn eat_struct(&mut self) -> ParseResult<NodeId<Struct>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Struct)?;

        // optional name
        let name = if self.peek_token(TokenType::Identifier).is_ok() {
            Some(self.eat_identifier()?)
        } else {
            None
        };

        // body
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;
        let struct_id = self.eat_struct_body()?;
        self.eat_token(TokenType::CloseBrace)?;

        // fill in header data
        let struct_ = self.tree.get_mut(struct_id);
        struct_.name = name;
        self.tree.set_span(struct_id, self.get_span_from(start));

        Ok(struct_id)
    }

    // Eat a struct body (without the header or `{` and `}`)
    pub fn eat_struct_body(&mut self) -> ParseResult<NodeId<Struct>> {
        let start = self.mark();

        // eat everything
        let mut fields: Vec<NodeId<StructField>> = Vec::new();
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
                let field = self.eat_struct_field()?;
                fields.push(field);
            }
        }

        let struct_id = self.tree.allocate(
            Struct {
                name: None,
                fields,
                usings,
                lets,
            },
            self.get_span_from(start),
        );
        Ok(struct_id)
    }

    /// Eat a single struct field: `name: Type` with optional default `= <expr>`.
    fn eat_struct_field(&mut self) -> ParseResult<NodeId<StructField>> {
        let start = self.mark();
        let name = self.eat_identifier()?;
        self.eat_colon()?;
        let r#type = self.eat_type()?;

        // optional default value: `= <expr>`
        let default = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression(None)?)
        } else {
            None
        };

        let field_id = self.tree.allocate(
            StructField {
                name,
                r#type,
                default,
            },
            self.get_span_from(start),
        );
        Ok(field_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{IntType, Parser, PrimitiveType, StructField, Type};

    #[test]
    fn test_parse_struct_anonymous() {
        let input = r###"
struct { x: int32, y: boolean
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // struct { x: int32, y: boolean }
        let struct_id = parser.eat_struct().unwrap();
        let r#struct = parser.tree.get(struct_id);
        assert_eq!(r#struct.name, None);
        assert!(r#struct.usings.is_empty());
        assert!(r#struct.lets.is_empty());
        assert_eq!(r#struct.fields.len(), 2);

        // x: int32
        let StructField {
            name,
            r#type,
            default,
        } = parser.tree.get(r#struct.fields[0]);
        {
            assert_eq!(*name, parser.strings.intern("x"));
            assert!(default.is_none());
            match parser.tree.get(*r#type) {
                Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                    assert_eq!(*width, 32);
                    assert!(*is_signed);
                }
                _ => panic!("expected primitive int type"),
            }
        }

        // y: boolean
        let StructField {
            name,
            r#type,
            default,
        } = parser.tree.get(r#struct.fields[1]);
        {
            assert_eq!(*name, parser.strings.intern("y"));
            assert!(default.is_none());
            match parser.tree.get(*r#type) {
                Type::Primitive(PrimitiveType::Boolean) => {}
                _ => panic!("expected boolean"),
            }
        }
    }

    #[test]
    fn test_parse_struct_with_name_and_using_and_default() {
        let input = r###"
struct Foo {
    use Bar, Baz
    
    let x: int32 = 4

    a: boolean
    b: int32 = 4
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct().unwrap();
        let r#struct = parser.tree.get(struct_id);
        assert_eq!(r#struct.name, Some(parser.strings.intern("Foo")));
        assert_eq!(r#struct.usings.len(), 1);
        assert_eq!(r#struct.lets.len(), 1);
        assert_eq!(r#struct.fields.len(), 2);

        // a: boolean
        let StructField {
            name,
            r#type,
            default,
        } = parser.tree.get(r#struct.fields[0]);
        {
            assert_eq!(*name, parser.strings.intern("a"));
            assert!(default.is_none());
            match parser.tree.get(*r#type) {
                Type::Primitive(PrimitiveType::Boolean) => {}
                _ => panic!("expected boolean"),
            }
        }

        // b: int32 = 4
        let StructField {
            name,
            r#type,
            default,
        } = parser.tree.get(r#struct.fields[1]);
        {
            assert_eq!(*name, parser.strings.intern("b"));
            match parser.tree.get(*r#type) {
                Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                    assert_eq!(*width, 32);
                    assert!(*is_signed);
                }
                _ => panic!("expected primitive int type"),
            }
            assert!(default.is_some());
        }
    }
}
