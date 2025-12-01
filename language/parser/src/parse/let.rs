use crate::{ParseError, ParseResult, Parser};

use destack_ast::{DeclarationDescriptor, Expression, Keyword, LocalNodeId, Mutability, TokenType};

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

    /// Eat a mutability modifier.
    pub fn eat_mutability(&mut self) -> ParseResult<Mutability> {
        let keyword = self.peek_any_keyword()?;
        // mutable
        if keyword == Keyword::Let || keyword == Keyword::Var || keyword == Keyword::Mut {
            self.bump(); // eat mutability
            Ok(Mutability::Mutable)
        }
        // immutable
        else if keyword == Keyword::Const || keyword == Keyword::Readonly {
            self.bump(); // eat readonly
            Ok(Mutability::Immutable)
        }
        // nothing
        else {
            Err(ParseError::expected(
                self.peek_token(TokenType::Identifier)?.span,
                TokenType::Identifier,
            ))
        }
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

        // mutability
        let mutability = self.eat_mutability()?;

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

        // let
        let let_id = self.tree.insert(
            Expression::Let {
                descriptor,
                pattern: pattern_id,
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
    use destack_ast::{
        Argument, DeclarationDescriptor, Expression, IntType, Key, Mutability, Name, Pattern,
        PatternField, Property, ScalarLiteral, TypeLiteral,
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

        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, ty, value, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert_eq!(*mutability, Mutability::Immutable);

            // int32
            let ty_id = ty.expect("expected explicit type");
            assert_node!(parser.tree, ty_id, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));

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

        let let_id = parser.eat_let(DeclarationDescriptor::default()).unwrap();

        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, ty,  .. } => {
            // var (mutable)
            assert_eq!(*mutability, Mutability::Mutable);

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
            assert_eq!(*mutability, Mutability::Immutable);
            assert!(ty.is_none());

            // foo()
            assert!(value.is_some());
        });
    }

    #[test]
    fn test_parse_let_implicit_undefined() {
        let mut test = TestParser::new("const x: int32");
        let mut parser = test.prepare();

        let let_id = parser.eat_let(DeclarationDescriptor::default()).unwrap();

        // let x: int32
        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, ty, value, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert_eq!(*mutability, Mutability::Immutable);
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

        let let_id = parser.eat_let(DeclarationDescriptor::default()).unwrap();

        // const x = foo.parse()
        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, value, .. } => {
            // x
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert_eq!(*mutability, Mutability::Immutable);

            // foo.parse()
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _,  left, static_arguments: _, dynamic_arguments: _ } => {
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
> = new Map()
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let let_id = parser.eat_expression().unwrap();

        // const renderCounter
        assert_node!(parser.tree, let_id, Expression::Let { pattern, mutability, ty, value, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
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
    }
}
