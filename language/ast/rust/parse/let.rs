use crate::Expression;
use dyst_source::PathId;
use dyst_token::TokenType;

use crate::{Keyword, Mutability, NodeId, AstResult, Parser, ScopedMutability, Visibility};

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
    pub fn eat_scoped_mutability(&mut self) -> AstResult<ScopedMutability> {
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
    ///
    /// let t? = foo() else { return }
    /// let t = foo() ?? return;
    /// ```
    pub fn eat_let(&mut self, visibility: Option<Visibility>) -> AstResult<NodeId<Expression>> {
        let start = self.mark();

        // mutability
        let mutability = if self.peek_keyword(Keyword::Var).is_ok() {
            self.eat_scoped_mutability()?
        } else {
            self.bump();
            ScopedMutability::Unscoped {
                mutability: Mutability::Immutable,
            }
        };

        // pattern
        let pattern = self.eat_pattern()?;

        // type
        let r#type = if self.peek_colon().is_ok() {
            self.bump(); // eat colon
            let r#type =
                self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
            Some(r#type)
        } else {
            None
        };

        // value
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.bump(); // eat assign
            self.eat_newlines_maybe()?;
            Some(self.eat_expression()?)
        } else {
            // implicitly uninitialized
            None
        };

        // let
        let let_id = self.tree.allocate(
            Expression::Let {
                pattern,
                mutability,
                visibility,
                r#type,
                value,
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
        Expression, Mutability, Pattern, PatternField, ScalarLiteral, ScopedMutability,
        TypeLiteral, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_var_with_scoped_mutability() {
        let mut test = TestParser::new(
            r###"
var(x, y) pos: Vector4
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let let_id = parser.eat_let(None).unwrap();

        // var(x, y) pos: Vector2 = --
        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, r#type: ty, value: _, .. } => {
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
            assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Vector4");
            });
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

        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, r#type, value, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser.session, *name, "x");
            });
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            // int32
            let ty_id = r#type.expect("expected explicit type");
            assert_node!(parser.tree, ty_id, Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                assert_eq!(int_ty.width, Some(32));
                assert!(int_ty.is_signed);
            });

            // 1
            let value_id = value.expect("expected value");
            assert_node!(parser.tree, value_id, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    }

    #[test]
    fn test_parse_var_array_undefined() {
        let mut test = TestParser::new(
            r###"
var x: float64[3] = undefined
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let(None).unwrap();
        let x = parser.intern_string("x");

        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, r#type,  .. } => {
            // var (mutable)
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });

            // pattern: x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_eq!(*name, x);
            });

            // float64[3]
            let ty_id = r#type.expect("expected explicit type");
            assert_node!(parser.tree, ty_id, Expression::Index { receiver, index } => {
                assert_node!(parser.tree, *receiver, Expression::TypeLiteral(TypeLiteral::Float(float_ty)) => {
                    assert_eq!(float_ty.width, Some(64));
                });
                assert_node!(parser.tree, index.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
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

        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, r#type, value, .. } => {
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
            assert!(value.is_some());
        });
    }

    #[test]
    fn test_parse_let_implicit_undefined() {
        let mut test = TestParser::new("let x: int32");
        let mut parser = test.prepare();

        let let_id = parser.eat_let(None).unwrap();
        let x = parser.intern_string("x");

        // let x: int32
        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, r#type, value, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_eq!(*name, x);
            });
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
            // int32
            assert!(r#type.is_some());
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_parse_let_multiline_value() {
        let mut test = TestParser::new(
            r###"
let x = 
    foo.parse()
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let(None).unwrap();
        let x = parser.intern_string("x");

        // let x = foo.parse()
        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, value, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_eq!(*name, x);
            });
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            // foo.parse()
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::Call { runtime, receiver, dynamic_arguments: _ } => {
                assert_eq!(*runtime, None);
                assert_node!(parser.tree, *receiver, Expression::Path { path, static_arguments: _ } => {
                    assert_path!(parser.session, *path, "foo.parse");
                });
            });
        });
    }
}
