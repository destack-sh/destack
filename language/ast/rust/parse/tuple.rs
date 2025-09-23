//! Parse tuples.

use dyst_language_token::TokenType;

use crate::{NodeId, ParseResult, Parser, Tuple, TupleField, TypeParserOptions};

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
                self.eat_item_stop_with_newlines()?;
            } else {
                break;
            }
        }
        let tuple_id = self
            .tree
            .allocate(Tuple { elements }, self.get_span_from(start));
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
            let r#type = self.eat_type(TypeParserOptions::default())?;
            let tuple_element_id = self.tree.allocate(
                TupleField::Named { name, r#type },
                self.get_span_from(start),
            );
            Ok(tuple_element_id)
        } else {
            // positional tuple element
            let r#type = self.eat_type(TypeParserOptions::default())?;
            let tuple_element_id = self
                .tree
                .allocate(TupleField::Positional { r#type }, self.get_span_from(start));
            Ok(tuple_element_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{PrimitiveType, Tuple, TupleField, Type, assert_node};

    #[test]
    fn test_parse_tuple_positional_single() {
        let mut test = TestParser::new("(int32)");
        let mut parser = test.parser();
        let tuple_id = parser.eat_tuple().unwrap();

        assert_node!(parser.tree, tuple_id, Tuple { elements } => {
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], TupleField::Positional { r#type } => {
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(int_ty)) => {
                    assert_eq!(int_ty.width, 32);
                    assert!(int_ty.is_signed);
                });
            });
        });
    }

    #[test]
    fn test_parse_tuple_positional_two_elements_comma() {
        let mut test = TestParser::new("(int32, boolean)");
        let mut parser = test.parser();
        let tuple_id = parser.eat_tuple().unwrap();

        assert_node!(parser.tree, tuple_id, Tuple { elements } => {
            assert_eq!(elements.len(), 2);

            // int32
            assert_node!(parser.tree, elements[0], TupleField::Positional { r#type } => {
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(int_ty)) => {
                    assert_eq!(int_ty.width, 32);
                    assert!(int_ty.is_signed);
                });
            });

            // boolean
            assert_node!(parser.tree, elements[1], TupleField::Positional { r#type } => {
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Boolean));
            });
        });
    }

    #[test]
    fn test_parse_tuple_positional_newline_separated() {
        let mut test = TestParser::new(
            r###"
(
  int32
  boolean
)
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let tuple_id = parser.eat_tuple().unwrap();
        assert_node!(parser.tree, tuple_id, Tuple { elements } => {
            assert_eq!(elements.len(), 2);

            // int32
            assert_node!(parser.tree, elements[0], TupleField::Positional { r#type } => {
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(int_ty)) => {
                    assert_eq!(int_ty.width, 32);
                    assert!(int_ty.is_signed);
                });
            });

            // boolean
            assert_node!(parser.tree, elements[1], TupleField::Positional { r#type } => {
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Boolean));
            });
        });
    }

    #[test]
    fn test_parse_tuple_named_elements() {
        let mut test = TestParser::new("(x: int32, y: boolean)");
        let mut parser = test.parser();
        let tuple_id = parser.eat_tuple().unwrap();

        assert_node!(parser.tree, tuple_id, Tuple { elements } => {
            assert_eq!(elements.len(), 2);

            // x: int32
            assert_node!(parser.tree, elements[0], TupleField::Named { name, r#type } => {
                assert_eq!(parser.get_string(*name), "x");
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(int_ty)) => {
                    assert_eq!(int_ty.width, 32);
                    assert!(int_ty.is_signed);
                });
            });

            // y: boolean
            assert_node!(parser.tree, elements[1], TupleField::Named { name, r#type } => {
                assert_eq!(parser.get_string(*name), "y");
                assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Boolean));
            });
        });
    }

    #[test]
    fn test_parse_tuple_empty() {
        let mut test = TestParser::new("()");
        let mut parser = test.parser();
        let tuple_id = parser.eat_tuple().unwrap();

        assert_node!(parser.tree, tuple_id, Tuple { elements } => {
            assert_eq!(elements.len(), 0);
        });
    }
}
