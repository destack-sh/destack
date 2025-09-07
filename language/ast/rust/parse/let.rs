use dyst_language_token::TokenType;

use crate::{Keyword, Let, LetInitialization, Mutability, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a let or var binding (incl. `let` or `var` keyword).
    ///
    /// Examples:
    /// ```
    /// let x = 1
    /// let x: int32 = 1
    /// var x = 1
    /// var x: int32 = 1
    /// var x: int32 // implicitly uninitialized, must be set before use
    /// var x: [float64; 3] = -- // explicitly uninitialized, can do whatever
    ///
    /// let Some(x) = someFunction()
    /// var Point { x, .. } = someFunction()
    /// ```
    pub fn eat_let_or_var(&mut self) -> ParseResult<NodeId<Let>> {
        let start = self.mark();
        // mutability
        let mutability = if self.peek_keyword(Keyword::Var).is_ok() {
            self.eat_keyword(Keyword::Var)?;
            Mutability::Mutable
        } else {
            self.eat_keyword(Keyword::Let)?;
            Mutability::Immutable
        };
        // pattern
        let pattern = self.eat_pattern(None)?;
        // type
        let r#type = if self.peek_colon().is_ok() {
            self.eat_colon()?;
            Some(self.eat_type()?)
        } else {
            None
        };
        // value
        let (value, initialization) = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            // explicitly uninitialized
            if self.peek_token(TokenType::Empty).is_ok() {
                (None, LetInitialization::Explicit)
            }
            // explicitly initialized
            else {
                (
                    Some(self.eat_expression(None)?),
                    LetInitialization::Explicit,
                )
            }
        } else {
            // implicitly uninitialized
            (None, LetInitialization::Implicit)
        };
        // let
        let let_id = self.tree.allocate(
            Let {
                pattern,
                mutability,
                r#type,
                value,
                initialization,
            },
            self.get_span_from(start),
        );
        Ok(let_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParse;
    use crate::{
        Expression, FloatType, Let, LetInitialization, Mutability, Pattern, PatternField,
        PrimitiveType, Type, assert_int, assert_node,
    };

    #[test]
    fn test_parse_let_scalar() {
        let test = TestParse::new(
            r###"
let x: int32 = 1
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();

        assert_node!(parser.tree, let_id, Let { pattern, mutability, r#type, value, initialization } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Identifier(name) => {
                assert_eq!(*name, parser.strings.intern("x"));
            });
            assert_eq!(*mutability, Mutability::Immutable);

            // int32
            let ty_id = r#type.expect("expected explicit type");
            assert_node!(parser.tree, ty_id, Type::Primitive(PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 32);
                assert!(int_ty.is_signed);
            });
            assert_eq!(*initialization, LetInitialization::Explicit);

            // 1
            let value_id = value.expect("expected value");
            assert_node!(parser.tree, value_id, Expression::ScalarLiteral(lit_id) => {
                assert_int!(parser.tree, *lit_id, 1);
            });
        });
    }

    #[test]
    fn test_parse_var_array_uninitialized() {
        let test = TestParse::new(
            r###"
var x: [float64; 3] = --
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();

        assert_node!(parser.tree, let_id, Let { pattern, mutability, r#type, value, initialization } => {
            // var (mutable)
            assert_eq!(*mutability, Mutability::Mutable);

            // pattern: x
            assert_node!(parser.tree, *pattern, Pattern::Identifier(name) => {
                assert_eq!(*name, parser.strings.intern("x"));
            });

            // [float64; 3]
            let ty_id = r#type.expect("expected explicit type");
            assert_node!(parser.tree, ty_id, Type::Array { element_type, count } => {
                assert_node!(parser.tree, *element_type, Type::Primitive(PrimitiveType::Float(FloatType::Float64)));
                assert_node!(parser.tree, *count, Expression::ScalarLiteral(lit_id) => {
                    assert_int!(parser.tree, *lit_id, 3);
                });
            });

            // initialization: explicit uninitialized and no value
            assert_eq!(*initialization, LetInitialization::Explicit);
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_parse_let_tuple_pattern() {
        let test = TestParse::new(
            r###"
let (x, y) = foo()
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();

        assert_node!(parser.tree, let_id, Let { pattern, mutability, r#type, value, initialization } => {
            // (x, y)
            assert_node!(parser.tree, *pattern, Pattern::Tuple { fields } => {
                assert_eq!(fields.len(), 2);
                // x
                assert_node!(parser.tree, fields[0], PatternField::Named { name, .. } => {
                    assert_eq!(*name, parser.strings.intern("x"));
                });
                // y
                assert_node!(parser.tree, fields[1], PatternField::Named { name, .. } => {
                    assert_eq!(*name, parser.strings.intern("y"));
                });
            });

            // let (immutable), no explicit type
            assert_eq!(*mutability, Mutability::Immutable);
            assert!(r#type.is_none());

            // foo()
            assert_eq!(*initialization, LetInitialization::Explicit);
            assert!(value.is_some());
        });
    }

    #[test]
    fn test_parse_let_implicit_uninitialized() {
        let test = TestParse::new("let x: int32");
        let mut parser = test.parser();

        let let_id = parser.eat_let_or_var().unwrap();

        // let x: int32
        assert_node!(parser.tree, let_id, Let { pattern, mutability, r#type, value, initialization } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Identifier(name) => {
                assert_eq!(*name, parser.strings.intern("x"));
            });
            assert_eq!(*mutability, Mutability::Immutable);
            // int32
            assert!(r#type.is_some());
            assert!(value.is_none());
            assert_eq!(*initialization, LetInitialization::Implicit);
        });
    }
}
