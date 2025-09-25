use crate::parse::prelude::*;
use dyst_source::PathId;
use dyst_token::TokenType;

use crate::parse::expression::ExpressionParserOptions;
use crate::{
    Keyword, Let, LetInitialization, Mutability, NodeId, NodeType, ParseResult, Parser,
    ScopedMutability, TypeParserOptions, Visibility,
};

impl Mutability {
    /// Get the keyword for this mutability.
    #[inline]
    pub fn to_keyword(&self) -> Keyword {
        match self {
            Mutability::Immutable => Keyword::Const,
            Mutability::Mutable => Keyword::Var,
        }
    }
}

impl ScopedMutability {
    /// Whether the mutability is mutable.
    #[inline]
    pub fn is_mutable(&self) -> bool {
        match self {
            ScopedMutability::Unscoped { mutability } => *mutability == Mutability::Mutable,
            ScopedMutability::Scoped { mutability, .. } => *mutability == Mutability::Mutable,
        }
    }

    /// Whether the mutability is immutable.
    #[inline]
    pub fn is_immutable(&self) -> bool {
        match self {
            ScopedMutability::Unscoped { mutability } => *mutability == Mutability::Immutable,
            ScopedMutability::Scoped { mutability, .. } => *mutability == Mutability::Immutable,
        }
    }
}

impl<'a> Parser<'a> {
    /// Eat a scoped mutability modifier. Allows nothing.
    ///
    /// Examples:
    /// ```
    ///  // nothing is unscoped const!
    /// var
    /// const
    /// var(x, y)
    /// const(session.source)
    /// ```
    pub fn eat_scoped_mutability(&mut self) -> ParseResult<ScopedMutability> {
        // mutability
        let mutability = {
            if self.peek_keyword(Keyword::Var).is_ok() {
                self.bump();
                Mutability::Mutable
            } else if self.peek_keyword(Keyword::Const).is_ok() {
                self.bump();
                Mutability::Immutable
            } else {
                // nothing means unscoped const
                return Ok(ScopedMutability::Unscoped {
                    mutability: Mutability::Immutable,
                });
            }
        };

        // scopes
        let scoped_mutability = {
            if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open parenthesis
                let mut scopes: Vec<PathId> = Vec::new();
                loop {
                    // break on close parenthesis
                    if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                        break;
                    }
                    // eat any stops
                    else if self.peek_item_stop().is_ok() {
                        self.eat_item_stop_with_newlines()?;
                    }
                    // eat path
                    else {
                        let scope = self.eat_path()?;
                        scopes.push(scope);
                    }
                }
                self.eat_token(TokenType::CloseParenthesis)?;
                ScopedMutability::Scoped { mutability, scopes }
            } else {
                ScopedMutability::Unscoped { mutability }
            }
        };
        Ok(scoped_mutability)
    }

    /// Eat a let or var binding (incl. `let` or `var` keyword).
    ///
    /// Examples:
    /// ```
    /// let x = 1
    /// let x: int32 = 1
    /// var x = 1
    /// var x: int32 = 1
    /// var x: int32 // implicitly uninitialized, must be set before use
    /// var x: [3]float64 = -- // explicitly uninitialized, can do whatever
    ///
    /// let Some(x) = someFunction()
    /// var Point { x, .. } = someFunction()
    /// ```
    pub fn eat_let(&mut self, visibility: Option<Visibility>) -> ParseResult<NodeId<Let>> {
        let start = self.mark();
        // mutability
        let mutability = if self.peek_keyword(Keyword::Var).is_ok() {
            self.eat_scoped_mutability().for_node_type(NodeType::Let)?
        } else {
            self.bump();
            ScopedMutability::Unscoped {
                mutability: Mutability::Immutable,
            }
        };
        // pattern
        let pattern = self
            .eat_pattern(ExpressionParserOptions::default())
            .for_node_type(NodeType::Let)?;
        // type
        let r#type = if self.peek_colon().is_ok() {
            self.bump(); // eat colon
            let r#type = self
                .eat_type(TypeParserOptions::default())
                .for_node_type(NodeType::Let)?;
            Some(r#type)
        } else {
            None
        };
        // value
        let (value, initialization) = if self.peek_token(TokenType::Assign).is_ok() {
            self.bump(); // eat assign
            // explicitly uninitialized
            if self.peek_empty().is_ok() {
                self.bump();
                (None, LetInitialization::Explicit)
            }
            // explicitly initialized
            else {
                (
                    Some(
                        self.eat_expression(ExpressionParserOptions::default())
                            .for_node_type(NodeType::Let)?,
                    ),
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
                visibility,
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
    use crate::parse::tests::TestParser;
    use crate::{
        Expression, FloatType, Let, LetInitialization, Mutability, Pattern, PatternField,
        PrimitiveType, ScopedMutability, Type, assert_int, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_var_with_scoped_mutability() {
        let mut test = TestParser::new(
            r###"
var(x, y) pos: Vector4 = --
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let let_id = parser.eat_let(None).unwrap();

        // var(x, y) pos: Vector2 = --
        assert_node!(parser.tree, let_id, Let { pattern, mutability, r#type: ty, value, initialization, .. } => {
            // var(x, y)
            match mutability {
                ScopedMutability::Scoped { mutability, scopes } => {
                    assert_eq!(*mutability, Mutability::Mutable);
                    assert_eq!(scopes.len(), 2);
                    assert_path!(parser.session, scopes[0], "x");
                    assert_path!(parser.session, scopes[1], "y");
                }
                _ => panic!("expected ScopedMutability::Scoped"),
            }

            // pos: Vector4
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser.session, *name, "pos");
            });
            assert_node!(parser.tree, ty.unwrap(), Type::Path { path, .. } => {
                assert_path!(parser.session, *path, "Vector4");
            });

            // = --
            assert!(value.is_none());
            assert_eq!(*initialization, LetInitialization::Explicit);
        });
    }

    #[test]
    fn test_parse_let_scalar() {
        let mut test = TestParser::new(
            r###"
let x: int32 = 1
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let(None).unwrap();

        assert_node!(parser.tree, let_id, Let { pattern, mutability, r#type, value, initialization, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser.session, *name, "x");
            });
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

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
        let mut test = TestParser::new(
            r###"
var x: [3]float64 = --
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let(None).unwrap();
        let x = parser.intern_string("x");

        assert_node!(parser.tree, let_id, Let { pattern, mutability, r#type, value, initialization, .. } => {
            // var (mutable)
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });

            // pattern: x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_eq!(*name, x);
            });

            // [3]float64
            let ty_id = r#type.expect("expected explicit type");
            assert_node!(parser.tree, ty_id, Type::Array { element, count } => {
                assert_node!(parser.tree, *element, Type::Primitive(PrimitiveType::Float(FloatType::Float64)));
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
        let mut test = TestParser::new(
            r###"
let (x, y) = foo()
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let(None).unwrap();
        let x = parser.intern_string("x");
        let y = parser.intern_string("y");

        assert_node!(parser.tree, let_id, Let { pattern, mutability, r#type, value, initialization, .. } => {
            // (x, y)
            assert_node!(parser.tree, *pattern, Pattern::Tuple { fields, .. } => {
                assert_eq!(fields.len(), 2);
                // x
                assert_node!(parser.tree, fields[0], PatternField::Named { name, .. } => {
                    assert_eq!(*name, x);
                });
                // y
                assert_node!(parser.tree, fields[1], PatternField::Named { name, .. } => {
                    assert_eq!(*name, y);
                });
            });

            // let (immutable), no explicit type
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
            assert!(r#type.is_none());

            // foo()
            assert_eq!(*initialization, LetInitialization::Explicit);
            assert!(value.is_some());
        });
    }

    #[test]
    fn test_parse_let_implicit_uninitialized() {
        let mut test = TestParser::new("let x: int32");
        let mut parser = test.prepare();

        let let_id = parser.eat_let(None).unwrap();
        let x = parser.intern_string("x");

        // let x: int32
        assert_node!(parser.tree, let_id, Let { pattern, mutability, r#type, value, initialization, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_eq!(*name, x);
            });
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
            // int32
            assert!(r#type.is_some());
            assert!(value.is_none());
            assert_eq!(*initialization, LetInitialization::Implicit);
        });
    }
}
