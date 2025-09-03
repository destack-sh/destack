//! Parse tuples.

use destack_language_token::TokenType;

use crate::{NodeId, ParseResult, Parser, Tuple, TupleField};

impl<'a> Parser<'a> {
    /// Eat a tuple type (including the `(` and `)`).
    ///
    /// Examples:
    /// ```
    /// (int32)
    /// (int32, int32)
    /// ```
    pub fn eat_tuple(&mut self) -> ParseResult<NodeId<Tuple>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let start = self.mark();
        let mut elements: Vec<NodeId<TupleField>> = Vec::new();
        self.eat_newlines_maybe()?;
        loop {
            if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                break;
            }
            let element = self.eat_tuple_field()?;
            elements.push(element);
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop()?;
            } else {
                break;
            }
        }
        let tuple_id = self
            .tree
            .allocate(Tuple { elements }, self.span_from(start));
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(tuple_id)
    }

    /// Eat a tuple field.
    ///
    /// Examples:
    /// ```
    /// int32
    /// a: int32
    /// ```
    pub fn eat_tuple_field(&mut self) -> ParseResult<NodeId<TupleField>> {
        let start = self.mark();

        if self.peek_next_token(TokenType::Colon).is_ok() {
            // named tuple element
            let name = self.eat_identifier()?;
            self.eat_colon()?;
            let r#type = self.eat_type()?;
            let tuple_element_id = self
                .tree
                .allocate(TupleField::Named { name, r#type }, self.span_from(start));
            Ok(tuple_element_id)
        } else {
            // positional tuple element
            let r#type = self.eat_type()?;
            let tuple_element_id = self
                .tree
                .allocate(TupleField::Positional { r#type }, self.span_from(start));
            Ok(tuple_element_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Parser, PrimitiveType, TupleField, Type};

    #[test]
    fn test_tuple_positional_simple() {
        let input = r#"
(int32)
"#;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();
        let tuple_id = parser.eat_tuple().unwrap();
        let tuple = parser.tree.get(tuple_id);
        assert_eq!(tuple.elements.len(), 1);
        match parser.tree.get(tuple.elements[0]) {
            TupleField::Positional { r#type } => match parser.tree.get(*r#type) {
                Type::Primitive(PrimitiveType::Int(int_ty)) => {
                    assert_eq!(int_ty.width, 32);
                    assert!(int_ty.is_signed);
                }
                _ => panic!("expected primitive int type"),
            },
            _ => panic!("expected positional tuple field"),
        }
    }

    #[test]
    fn test_tuple_positional_two_elements_comma() {
        let input = r###"
(int32, boolean)
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();
        let tuple_id = parser.eat_tuple().unwrap();
        let tuple = parser.tree.get(tuple_id);
        assert_eq!(tuple.elements.len(), 2);

        // int32
        match parser.tree.get(tuple.elements[0]) {
            TupleField::Positional { r#type } => match parser.tree.get(*r#type) {
                Type::Primitive(PrimitiveType::Int(int_ty)) => {
                    assert_eq!(int_ty.width, 32);
                    assert!(int_ty.is_signed);
                }
                _ => panic!("expected primitive int type"),
            },
            _ => panic!("expected positional tuple field"),
        }

        // boolean
        match parser.tree.get(tuple.elements[1]) {
            TupleField::Positional { r#type } => match parser.tree.get(*r#type) {
                Type::Primitive(PrimitiveType::Boolean) => {}
                _ => panic!("expected boolean"),
            },
            _ => panic!("expected positional tuple field"),
        }
    }

    #[test]
    fn test_tuple_positional_newline_separated() {
        let input = r###"
(
  int32
  boolean
)
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();
        let tuple_id = parser.eat_tuple().unwrap();
        let tuple = parser.tree.get(tuple_id);
        assert_eq!(tuple.elements.len(), 2);
    }

    #[test]
    fn test_tuple_named_elements() {
        let input = r###"
(x: int32, y: boolean)
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();
        let tuple_id = parser.eat_tuple().unwrap();
        let tuple = parser.tree.get(tuple_id);
        assert_eq!(tuple.elements.len(), 2);

        // x: int32
        match parser.tree.get(tuple.elements[0]) {
            TupleField::Named { name, r#type } => {
                assert_eq!(*name, parser.strings.intern("x"));
                match parser.tree.get(*r#type) {
                    Type::Primitive(PrimitiveType::Int(int_ty)) => {
                        assert_eq!(int_ty.width, 32);
                        assert!(int_ty.is_signed);
                    }
                    _ => panic!("expected primitive int type"),
                }
            }
            _ => panic!("expected named tuple field"),
        }

        // y: boolean
        match parser.tree.get(tuple.elements[1]) {
            TupleField::Named { name, r#type } => {
                assert_eq!(*name, parser.strings.intern("y"));
                match parser.tree.get(*r#type) {
                    Type::Primitive(PrimitiveType::Boolean) => {}
                    _ => panic!("expected boolean"),
                }
            }
            _ => panic!("expected named tuple field"),
        }
    }
}
