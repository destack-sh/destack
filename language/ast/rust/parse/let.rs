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
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{
        Expression, FloatType, LetInitialization, Mutability, Parser, Pattern, PatternField,
        PrimitiveType, ScalarLiteral, Type,
    };

    #[test]
    fn test_parse_let_scalar() {
        let input = r###"
let x: int32 = 1
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();

        // x
        let binding = parser.tree.get(let_id);
        match *parser.tree.get(binding.pattern) {
            Pattern::Identifier(name) => assert_eq!(name, parser.strings.intern("x")),
            _ => panic!("expected identifier pattern"),
        }
        assert_eq!(binding.mutability, Mutability::Immutable);

        // int32
        let ty_id = binding.r#type.expect("expected explicit type");
        match parser.tree.get(ty_id) {
            Type::Primitive(PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 32);
                assert!(int_ty.is_signed);
            }
            _ => panic!("expected int32 type"),
        }
        assert_eq!(binding.initialization, LetInitialization::Explicit);

        // 1
        let value_id = binding.value.expect("expected value");
        match parser.tree.get(value_id) {
            Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                _ => panic!("expected integer"),
            },
            _ => panic!("expected 1"),
        }
    }

    #[test]
    fn test_parse_var_array() {
        let input = r###"
var x: [float64; 3] = --
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();
        let binding = parser.tree.get(let_id);
        // var (mutable)
        assert_eq!(binding.mutability, Mutability::Mutable);

        // pattern: x
        match *parser.tree.get(binding.pattern) {
            Pattern::Identifier(name) => assert_eq!(name, parser.strings.intern("x")),
            _ => panic!("expected identifier pattern"),
        }

        // [float64; 3]
        let ty_id = binding.r#type.expect("expected explicit type");
        match parser.tree.get(ty_id) {
            Type::Array {
                element_type,
                count,
            } => {
                match parser.tree.get(*element_type) {
                    Type::Primitive(PrimitiveType::Float(FloatType::Float64)) => {}
                    _ => panic!("expected float64 element type"),
                }
                match parser.tree.get(*count) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Integer(n, _) => assert_eq!(*n, 3),
                        _ => panic!("expected integer literal count"),
                    },
                    _ => panic!("expected scalar literal count expression"),
                }
            }
            _ => panic!("expected array type"),
        }

        // initialization: explicit uninitialized and no value
        assert_eq!(binding.initialization, LetInitialization::Explicit);
        assert!(binding.value.is_none());
    }

    #[test]
    fn test_parse_let_tuple() {
        let input = r###"
let (x, y) = foo()
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let_or_var().unwrap();
        let binding = parser.tree.get(let_id);

        // (x, y)
        match parser.tree.get(binding.pattern) {
            Pattern::Tuple { fields } => {
                assert_eq!(fields.len(), 2);
                // x
                let field_pattern = parser.tree.get(fields[0]);
                match field_pattern {
                    &PatternField::Named { name, .. } => {
                        assert_eq!(name, parser.strings.intern("x"));
                    }
                    _ => panic!("expected x, got {field_pattern:?}"),
                }
                // y
                let field_pattern = parser.tree.get(fields[1]);
                match field_pattern {
                    &PatternField::Named { name, .. } => {
                        assert_eq!(name, parser.strings.intern("y"));
                    }
                    _ => panic!("expected y, got {field_pattern:?}"),
                }
            }
            _ => panic!("expected tuple pattern, got {binding:?}"),
        }

        // let (immutable), no explicit type
        assert_eq!(binding.mutability, Mutability::Immutable);
        assert!(binding.r#type.is_none());

        // initialized with a value
        assert_eq!(binding.initialization, LetInitialization::Explicit);
        assert!(binding.value.is_some());
    }
}
