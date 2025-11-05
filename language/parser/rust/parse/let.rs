use dyst_ast::DefinitionMeta;

use crate::{Expression, ParserError, Path, TokenType};

use crate::{Keyword, Mutability, NodeId, Parser, ParserResult, ScopedMutability};

// TODO #Incomplete: handle scoped mutability better (mut? readonly? const? scopes?)
//  (and how does this interact with let/const expressions, bindings/arguments/parameters/fields, ..)

impl<'a> Parser<'a> {
    /// Peek a mutability modifier.
    pub fn peek_mutability(&mut self) -> ParserResult<()> {
        let keyword = self.peek_any_keyword()?;
        if keyword == Keyword::Var
            || keyword == Keyword::Mut
            || keyword == Keyword::Const
            || keyword == Keyword::Readonly
        {
            Ok(())
        } else {
            Err(ParserError::expected(
                self.peek_token(TokenType::Identifier)?.span,
                TokenType::Identifier,
            ))
        }
    }

    /// Eat a scoped mutability modifier maybe.
    /// Instead of defaulting to immutable returns None.
    pub fn eat_scoped_mutability_maybe(&mut self) -> ParserResult<Option<ScopedMutability>> {
        if self.peek_mutability().is_ok() {
            Ok(Some(self.eat_scoped_mutability()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a scoped mutability modifier.
    /// Allows empty, defaulting to unscoped immutable.
    ///
    /// Examples:
    /// ```
    /// mut
    /// readonly
    /// mut(x, y)
    /// readonly(session.source)
    /// ```
    pub fn eat_scoped_mutability(&mut self) -> ParserResult<ScopedMutability> {
        // mutability
        let mutability = {
            if self.peek_keyword(Keyword::Var).is_ok()
                || self.peek_keyword(Keyword::Mut).is_ok()
                || self.peek_keyword(Keyword::Let).is_ok()
            {
                self.bump(); // eat var or mut
                Mutability::Mutable
            } else if self.peek_keyword(Keyword::Const).is_ok()
                || self.peek_keyword(Keyword::Readonly).is_ok()
            {
                self.bump(); // eat const or readonly
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
                let mut scopes: Vec<Path> = Vec::new();
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
    /// const x = 1
    /// const x: int32 = 1
    /// var x = 1
    /// var x: int32 = 1
    /// var x: int32 // implicitly uninitialized, must be set before use
    ///
    /// const Some(x) = someFunction()
    /// var Point { x, .. } = someFunction()
    /// const t = foo() ?? return;
    ///
    /// if const Some(x) = someFunction() {
    ///     ...
    /// }
    /// if const Some(x) = someFunction() {
    ///     ...
    /// }
    /// ```
    pub fn eat_let(&mut self, meta: DefinitionMeta) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();

        // mutability
        let mutability =
            // var or mut
            if self.peek_keyword(Keyword::Var).is_ok()
                || self.peek_keyword(Keyword::Mut).is_ok()
                || self.peek_keyword(Keyword::Let).is_ok()
            {
                self.eat_scoped_mutability()?
            }
            // let or const 
            else if self.peek_keyword(Keyword::Const).is_ok()
                || self.peek_keyword(Keyword::Readonly).is_ok()
            {
                self.bump(); // eat let or const
                // also support `let mut` or `let var` as an alias for #Compatibility
                if self.peek_keyword(Keyword::Var).is_ok() || self.peek_keyword(Keyword::Mut).is_ok() {
                    self.eat_scoped_mutability()?
                } else {
                    ScopedMutability::Unscoped {
                        mutability: Mutability::Immutable,
                    }
                }
            } else {
                return Err(ParserError::expected(
                    self.peek_token(TokenType::Identifier)?.span,
                    TokenType::Identifier,
                ));
            };

        // pattern
        let pattern =
            self.with_options(self.options.in_before_type(), |parser| parser.eat_pattern())?;

        // type
        let ty = if self.peek_colon().is_ok() {
            self.bump(); // eat colon
            let ty = self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
            Some(ty)
        } else {
            None
        };

        // value
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.bump(); // eat assign
            self.eat_newlines_maybe()?;
            Some(self.eat_expression()?)
        } else {
            None
        };

        // let
        let let_id = self.tree.insert(
            Expression::Let {
                meta,
                pattern,
                mutability,
                ty,
                value,
            },
            self.get_span_from(start),
        );
        Ok(let_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{Argument, Name};

    use crate::parse::tests::TestParser;
    use crate::{
        DefinitionMeta, Expression, Mutability, Pattern, PatternField, ScalarLiteral,
        ScopedMutability, TypeLiteral, assert_name, assert_node, assert_path, assert_string,
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
        let let_id = parser.eat_let(DefinitionMeta::default()).unwrap();

        // var(x, y) pos: Vector2 = --
        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, ty, value: _, .. } => {
            // var(x, y)
            match mutability {
                ScopedMutability::Scoped { mutability, scopes } => {
                    assert_eq!(*mutability, Mutability::Mutable);
                    assert_eq!(scopes.len(), 2);
                    assert_path!(parser, scopes[0], "x");
                    assert_path!(parser, scopes[1], "y");
                }
                _ => panic!("expected ScopedMutability::Scoped"),
            }

            // pos: Vector4
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "pos");
            });
            assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Vector4");
            });
        });
    }

    #[test]
    fn test_parse_let_scalar() {
        let mut test = TestParser::new(
            r###"
const x: int32 = 1
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let(DefinitionMeta::default()).unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, ty, value, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            // int32
            let ty_id = ty.expect("expected explicit type");
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

        let let_id = parser.eat_let(DefinitionMeta::default()).unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, ty,  .. } => {
            // var (mutable)
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });

            // pattern: x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });

            // float64[3]
            let ty_id = ty.expect("expected explicit type");
            assert_node!(parser.tree, ty_id, Expression::Index { position: _, left, index } => {
                assert_node!(parser.tree, *left, Expression::TypeLiteral(TypeLiteral::Float(float_ty)) => {
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
const (x, y) = foo()
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let(DefinitionMeta::default()).unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, ty, value, .. } => {
            // (x, y)
            assert_node!(parser.tree, *pattern, Pattern::Tuple { fields, .. } => {
                assert_eq!(fields.len(), 2);
                // x
                assert_node!(parser.tree, fields[0], PatternField::Named { name, .. } => {
                    assert_name!(parser, *name, "x");
                });
                // y
                assert_node!(parser.tree, fields[1], PatternField::Named { name, .. } => {
                    assert_name!(parser, *name, "y");
                });
            });

            // let (immutable), no explicit type
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
            assert!(ty.is_none());

            // foo()
            assert!(value.is_some());
        });
    }

    #[test]
    fn test_parse_let_implicit_undefined() {
        let mut test = TestParser::new("const x: int32");
        let mut parser = test.prepare();

        let let_id = parser.eat_let(DefinitionMeta::default()).unwrap();

        // let x: int32
        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, ty, value, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
            // int32
            assert!(ty.is_some());
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_parse_let_multiline_value() {
        let mut test = TestParser::new(
            r###"
const x = 
    foo.parse()
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let(DefinitionMeta::default()).unwrap();

        // const x = foo.parse()
        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, value, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            // foo.parse()
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _,  left, dynamic_arguments: _ } => {
                assert_node!(parser.tree, *left, Expression::Path { path, static_arguments: _ } => {
                    assert_path!(parser, *path, "foo.parse");
                });
            });
        });
    }

    #[test]
    fn test_parse_let_multiline_with_static_arguments() {
        let mut test = TestParser::new(
            r###"
const registry: Map<
  string,
  Set<{count: number}>
> = new Map();
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_expression().unwrap();

        // const renderCounter
        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, ty, value, .. } => {
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
            assert!(ty.is_some());
            assert!(value.is_some());

            // registry
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "registry");
            });

            // Map<string, Set<{count: number}>>
            assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, static_arguments } => {
                // Map
                assert_path!(parser, *path, "Map");
                // <string, Set<{count: number}>>
                // string
                assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                });
                // Set<{count: number}>
                assert_node!(parser.tree, static_arguments.as_ref().unwrap()[1], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                        // Set
                        assert_path!(parser, *path, "Set");
                        // <{count: number}>
                        assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                            assert_node!(parser.tree, *value, Expression::StructLiteral { ty: None, fields, .. } => {
                                assert_eq!(fields.len(), 1);
                                assert_node!(parser.tree, fields[0], Argument::Named { name: Name::Identifier(name), ..} => {
                                    assert_string!(parser, *name, "count");
                                });
                            });
                        });
                    });
                });
            });
        });
    }
}
