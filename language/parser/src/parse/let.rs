use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    DeclarationDescriptor, Declarator, Expression, Keyword, LetKind, LocalNodeId, Mutability,
    TokenType,
};

impl Parser {
    /// Peek a mutability modifier.
    pub fn peek_mutability(&mut self) -> ParseResult<()> {
        let keyword = self.peek_any_keyword()?;
        if keyword == Keyword::Var
            || keyword == Keyword::Mut
            || keyword == Keyword::Const
            || keyword == Keyword::Readonly
        {
            Ok(())
        } else {
            Err(ParseError::expected(
                self.peek_token(TokenType::Identifier)?.span,
                TokenType::Identifier,
            ))
        }
    }

    /// Eat a let/var/const keyword and return the kind and mutability.
    pub fn eat_let_kind(&mut self) -> ParseResult<(LetKind, Mutability)> {
        let keyword = self.peek_any_keyword()?;
        match keyword {
            Keyword::Let => {
                self.bump();
                Ok((LetKind::Let, Mutability::Mutable))
            }
            Keyword::Var | Keyword::Mut => {
                self.bump();
                Ok((LetKind::Var, Mutability::Mutable))
            }
            Keyword::Const | Keyword::Readonly => {
                self.bump();
                Ok((LetKind::Const, Mutability::Immutable))
            }
            _ => Err(ParseError::expected(
                self.peek_token(TokenType::Identifier)?.span,
                TokenType::Identifier,
            )),
        }
    }

    /// Eat a mutability modifier.
    pub fn eat_mutability(&mut self) -> ParseResult<Mutability> {
        let (_, mutability) = self.eat_let_kind()?;
        Ok(mutability)
    }

    /// Eat a mutability modifier maybe.
    pub fn eat_mutability_maybe(&mut self) -> ParseResult<Option<Mutability>> {
        let Ok(keyword) = self.peek_any_keyword() else {
            return Ok(None);
        };
        // mutable
        if keyword == Keyword::Var || keyword == Keyword::Mut {
            self.bump(); // eat mutability
            Ok(Some(Mutability::Mutable))
        }
        // immutable
        else if keyword == Keyword::Let
            || keyword == Keyword::Const
            || keyword == Keyword::Readonly
        {
            self.bump(); // eat readonly
            Ok(Some(Mutability::Immutable))
        }
        // nothing
        else {
            Ok(None)
        }
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
    /// let a: T1 = v1, b: T2  // multiple declarators
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
    pub fn eat_let(
        &mut self,
        descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // kind and mutability
        let (kind, mutability) = self.eat_let_kind()?;

        // Parse declarators (comma-separated list)
        let mut declarators = Vec::new();
        loop {
            let declarator_id = self.eat_declarator()?;
            declarators.push(declarator_id);

            // Check for comma to continue parsing more declarators
            if self.peek_token(TokenType::Comma).is_ok() {
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
            } else {
                break;
            }
        }

        // let
        let let_id = self.tree.insert(
            Expression::Let {
                kind,
                descriptor,
                mutability,
                declarators,
            },
            self.get_span_from(start),
        );
        Ok(let_id)
    }

    /// Eat a single declarator (pattern, optional type, optional value).
    fn eat_declarator(&mut self) -> ParseResult<LocalNodeId<Declarator>> {
        let start = self.mark();

        // pattern
        let pattern_id = self
            .with_options(self.options.not_in_position().in_before_type(), |parser| {
                parser.eat_pattern()
            })?;

        // type
        let ty = if self.peek_colon().is_ok() {
            self.bump(); // eat colon
            let ty = self.with_options(self.options.not_in_position().in_type(), |parser| {
                parser.eat_expression()
            })?;
            Some(ty)
        } else {
            None
        };

        // value
        let value = if self.peek_token(TokenType::Assign).is_ok() {
            self.bump(); // eat assign
            self.eat_newlines_maybe()?;
            Some(self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?)
        } else {
            None
        };

        // declarator
        let declarator_id = self.tree.insert(
            Declarator {
                pattern: pattern_id,
                ty,
                value,
            },
            self.get_span_from(start),
        );
        Ok(declarator_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, DeclarationDescriptor, Declarator, Expression, IntType, Key, Mutability, Name,
        Pattern, PatternField, Property, ScalarLiteral, TypeLiteral,
    };

    use crate::{TestParser, assert_name, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_let_scalar() {
        let mut test = TestParser::new(
            r###"
const x: int32 = 1
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_let(DeclarationDescriptor::default()).unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                // x
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });

                // int32
                let ty_id = ty.expect("expected explicit type");
                assert_node!(parser.tree, ty_id, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));

                // 1
                let value_id = value.expect("expected value");
                assert_node!(parser.tree, value_id, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
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

        let let_id = parser.eat_let(DeclarationDescriptor::default()).unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            // var (mutable)
            assert_eq!(*mutability, Mutability::Mutable);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, .. } => {
                // x
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

        let let_id = parser.eat_let(DeclarationDescriptor::default()).unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            // let (immutable)
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
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

                // no explicit type
                assert!(ty.is_none());

                // foo()
                assert!(value.is_some());
            });
        });
    }

    #[test]
    fn test_parse_let_implicit_undefined() {
        let mut test = TestParser::new("const x: int32");
        let mut parser = test.prepare();

        let let_id = parser.eat_let(DeclarationDescriptor::default()).unwrap();

        // let x: int32
        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                // x
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });
                // int32
                assert!(ty.is_some());
                assert!(value.is_none());
            });
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

        let let_id = parser.eat_let(DeclarationDescriptor::default()).unwrap();

        // const x = foo.parse()
        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                // x
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });
                // foo.parse()
                assert!(value.is_some());
                assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _,  left, static_arguments: _, dynamic_arguments: _ } => {
                    assert_node!(parser.tree, *left, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser, *path, "foo.parse");
                    });
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
> = new Map()
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_expression().unwrap();

        // const renderCounter
        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);

            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
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
                    assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { value } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                    });
                    // Set<{count: number}>
                    assert_node!(parser.tree, static_arguments.as_ref().unwrap()[1], Argument::Positional { value } => {
                        assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                            // Set
                            assert_path!(parser, *path, "Set");
                            // <{count: number}>
                            assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { value } => {
                                assert_node!(parser.tree, *value, Expression::ObjectExpression { ty: None, properties, .. } => {
                                    assert_eq!(properties.len(), 1);
                                    assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), .. } => {
                                        assert_string!(parser, *name, "count");
                                    });
                                });
                            });
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_let_multiple_declarators() {
        let mut test = TestParser::new("let a: int32 = 1, b: string = \"hello\"");
        let mut parser = test.prepare();
        let let_id = parser.eat_let(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, let_id, Expression::Let { declarators, mutability, .. } => {
            assert_eq!(*mutability, Mutability::Mutable);
            assert_eq!(declarators.len(), 2);
            // a: int32 = 1
            assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "a");
                });
                assert!(ty.is_some());
                assert!(value.is_some());
            });
            // b: string = "hello"
            assert_node!(parser.tree, declarators[1], Declarator { pattern, ty, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "b");
                });
                assert!(ty.is_some());
                assert!(value.is_some());
            });
        });
    }
}
