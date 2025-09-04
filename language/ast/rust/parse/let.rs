use destack_language_token::TokenType;

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
    /// var x: [float64; 3] = --- // explicitly uninitialized, can do whatever
    ///
    /// // todo!: let pattern destructuring
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
        // name
        let name = self.eat_identifier()?;
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
            if self.peek_token(TokenType::Ellipsis).is_ok() {
                (None, LetInitialization::ExplicitUninitialized)
            } else {
                (
                    Some(self.eat_expression(None)?),
                    LetInitialization::ExplicitInitialized,
                )
            }
        } else {
            // implicitly uninitialized
            (None, LetInitialization::ImplicitUninitialized)
        };
        // let
        let let_id = self.tree.allocate(
            Let {
                name,
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
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{
        Expression, LetInitialization, Mutability, Parser, PrimitiveType, ScalarLiteral, Type,
    };

    #[test]
    fn test_parse_let_simple_initialized() {
        let input = r###"
let x = 1
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();
        let binding = parser.tree.get(let_id);
        assert_eq!(binding.name, parser.strings.intern("x"));
        assert_eq!(binding.mutability, Mutability::Immutable);
        assert!(binding.r#type.is_none());
        assert_eq!(
            binding.initialization,
            LetInitialization::ExplicitInitialized
        );

        let value_id = binding.value.expect("expected value");
        match parser.tree.get(value_id) {
            Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                _ => panic!("expected integer"),
            },
            _ => panic!("expected scalar literal"),
        }
    }

    #[test]
    fn test_parse_let_typed_initialized() {
        let input = r###"
let x: int32 = 1
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();
        let binding = parser.tree.get(let_id);
        assert_eq!(binding.name, parser.strings.intern("x"));
        assert_eq!(binding.mutability, Mutability::Immutable);
        let ty_id = binding.r#type.expect("expected explicit type");
        match parser.tree.get(ty_id) {
            Type::Primitive(PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 32);
                assert!(int_ty.is_signed);
            }
            _ => panic!("expected primitive int type"),
        }
        assert_eq!(
            binding.initialization,
            LetInitialization::ExplicitInitialized
        );
        let value_id = binding.value.expect("expected value");
        match parser.tree.get(value_id) {
            Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                _ => panic!("expected integer"),
            },
            _ => panic!("expected scalar literal"),
        }
    }

    #[test]
    fn test_parse_var_simple_initialized() {
        let input = r###"
var x = 1
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();
        let binding = parser.tree.get(let_id);
        assert_eq!(binding.name, parser.strings.intern("x"));
        assert_eq!(binding.mutability, Mutability::Mutable);
        assert!(binding.r#type.is_none());
        assert_eq!(
            binding.initialization,
            LetInitialization::ExplicitInitialized
        );
        let value_id = binding.value.expect("expected value");
        match parser.tree.get(value_id) {
            Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                _ => panic!("expected integer"),
            },
            _ => panic!("expected scalar literal"),
        }
    }

    #[test]
    fn test_parse_var_typed_implicit_uninitialized() {
        let input = r###"
var x: int32
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();
        let binding = parser.tree.get(let_id);
        assert_eq!(binding.name, parser.strings.intern("x"));
        assert_eq!(binding.mutability, Mutability::Mutable);
        let ty_id = binding.r#type.expect("expected explicit type");
        match parser.tree.get(ty_id) {
            Type::Primitive(PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 32);
                assert!(int_ty.is_signed);
            }
            _ => panic!("expected primitive int type"),
        }
        assert!(binding.value.is_none());
        assert_eq!(
            binding.initialization,
            LetInitialization::ImplicitUninitialized
        );
    }

    #[test]
    fn test_parse_var_typed_explicit_uninitialized() {
        let input = r###"
var x: [float64; 3] = ...
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();
        let binding = parser.tree.get(let_id);
        assert_eq!(binding.name, parser.strings.intern("x"));
        assert_eq!(binding.mutability, Mutability::Mutable);
        let ty_id = binding.r#type.expect("expected explicit type");
        match parser.tree.get(ty_id) {
            Type::Array {
                element_type,
                count,
            } => {
                // [float64; 3]
                match parser.tree.get(*element_type) {
                    Type::Primitive(PrimitiveType::Float(crate::FloatType::Float64)) => {}
                    _ => panic!("expected float64 element type"),
                }
                match parser.tree.get(*count) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Integer(n, _) => assert_eq!(*n, 3),
                        _ => panic!("expected integer 3"),
                    },
                    _ => panic!("expected scalar literal count"),
                }
            }
            _ => panic!("expected array type"),
        }
        assert!(binding.value.is_none());
        assert_eq!(
            binding.initialization,
            LetInitialization::ExplicitUninitialized
        );
    }
}
