//! Parse use and with declarations.
use dyst_language_token::TokenType;

use crate::{
    Expression, Keyword, NodeId, ParseResult, Parser, Use, UseClause, UseItem, Visibility, With,
    WithClause,
};

impl<'a> Parser<'a> {
    /// Eat a with declaration.
    ///
    /// With can declare the use of an item in a scope and refine type bounds.
    ///
    /// Examples:
    /// ```
    /// with T: int32
    /// with Foo
    /// with Foo as Bar
    /// with Foo, Bar
    /// with Foo.Bar
    /// with !Bar
    /// with (
    ///    !Bar,
    ///    Time[float32] // optional comma
    ///    F: Numeric
    /// )
    /// ```
    pub fn eat_with(&mut self) -> ParseResult<NodeId<With>> {
        self.eat_keyword(Keyword::With)?;
        let with = self.eat_with_body()?;
        Ok(with)
    }

    /// Eat the clauses of a `with` declaration (without the `with` keyword).
    pub fn eat_with_body(&mut self) -> ParseResult<NodeId<With>> {
        let start = self.mark();
        let mut clauses: Vec<NodeId<WithClause>> = Vec::new();

        // parenthesized list with newlines
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.eat_token(TokenType::OpenParenthesis)?;
            self.eat_newlines_maybe()?;
            loop {
                self.eat_newlines_maybe()?;
                if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                    break;
                }
                let next_clause = self.eat_with_clause()?;
                clauses.push(next_clause);
                // optional comma with newlines
                if self.peek_token(TokenType::Comma).is_ok() {
                    self.eat_token(TokenType::Comma)?;
                }
            }
            self.eat_token(TokenType::CloseParenthesis)?;
        }
        // plain list separated by commas
        else {
            loop {
                let clause = self.eat_with_clause()?;
                clauses.push(clause);
                // required comma
                if self.peek_token(TokenType::Comma).is_ok() {
                    self.eat_token(TokenType::Comma)?;
                } else {
                    break;
                }
            }
        }

        let with = self
            .tree
            .allocate(With { clauses }, self.get_span_from(start));
        Ok(with)
    }

    /// Eat a single with clause.
    ///
    /// A clause can be a declaration (`Foo`, `Foo as Bar`, `Foo.Bar as Baz`)
    /// or an assertion (`T: int32`, `Self: geom.Mesh<T>`, `T.Item: Copy`).
    pub fn eat_with_clause(&mut self) -> ParseResult<NodeId<WithClause>> {
        let start = self.mark();

        // first parse the left-hand side type target
        let left = self.eat_type()?;

        // assertion: `T: SomeType`
        if self.peek_colon().is_ok() {
            self.eat_colon()?;
            let right = self.eat_type()?;
            let clause = self.tree.allocate(
                WithClause::Assertion {
                    target: left,
                    assertion: right,
                },
                self.get_span_from(start),
            );
            Ok(clause)
        }
        // declaration: optional alias `as Ident`
        else {
            let alias = if let Ok(next) = self.peek_token(TokenType::Identifier) {
                if self.get_token_str(*next) == Keyword::As.as_str() {
                    self.bump(); // eat as keyword
                    Some(self.eat_identifier()?)
                } else {
                    None
                }
            } else {
                None
            };
            let clause = self.tree.allocate(
                WithClause::Declaration {
                    target: left,
                    alias,
                },
                self.get_span_from(start),
            );
            Ok(clause)
        }
    }

    /// Eat a use declaration.
    ///
    /// Examples:
    /// ```
    /// use foo
    /// use foo, bar
    /// use foo.bar
    /// use foo.{bar, baz}
    /// use foo.{} // valid but linted
    /// use foo as baz
    /// ```
    pub fn eat_use(&mut self, visibility: Option<Visibility>) -> ParseResult<NodeId<Use>> {
        self.eat_keyword(Keyword::Use)?;
        let using = self.eat_use_header(visibility)?;
        Ok(using)
    }

    /// Eat the content of a use declaration (without the `use` keyword).
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo, bar
    /// foo.bar
    /// foo.{bar, baz}
    /// foo.{} // valid but linted
    /// foo as baz
    /// ```
    fn eat_use_header(&mut self, visibility: Option<Visibility>) -> ParseResult<NodeId<Use>> {
        let start = self.mark();

        // parse one or more clauses separated by commas
        let mut clauses: Vec<NodeId<UseClause>> = Vec::new();
        let clause = self.eat_use_clause()?;
        clauses.push(clause);
        loop {
            if self.peek_token(TokenType::Comma).is_ok() {
                self.eat_token(TokenType::Comma)?;
                // allow trailing comma before stop
                if self.peek_statement_stop().is_ok() {
                    break;
                }
                let next_clause = self.eat_use_clause()?;
                clauses.push(next_clause);
                continue;
            }
            break;
        }

        let using = self.tree.allocate(
            Use {
                clauses,
                body: None,
                visibility,
            },
            self.get_span_from(start),
        );
        Ok(using)
    }

    /// Eat a single use clause (like `foo`, `foo as bar`, `foo.{a, b}`).
    ///
    /// Examples:
    /// ```
    /// foo
    /// foo as bar
    /// foo.{a, b}
    /// ```
    pub fn eat_use_clause(&mut self) -> ParseResult<NodeId<UseClause>> {
        let start = self.mark();
        let path = self.eat_path()?;

        // try grouped items first: `. { ... }`
        let items = if self.peek_token(TokenType::Dot).is_ok() {
            self.eat_token(TokenType::Dot)?;
            self.eat_token(TokenType::OpenBrace)?;

            // parse zero or more items
            // (empty group `.{}` is valid)
            let mut items: Vec<NodeId<UseItem>> = Vec::new();
            if self.peek_token(TokenType::CloseBrace).is_err() {
                loop {
                    let item = self.eat_use_item()?;
                    items.push(item);
                    if self.peek_token(TokenType::Comma).is_ok() {
                        self.eat_token(TokenType::Comma)?;
                        // allow trailing comma
                        if self.peek_token(TokenType::CloseBrace).is_ok() {
                            break;
                        }
                        continue;
                    }
                    break;
                }
            }

            self.eat_token(TokenType::CloseBrace)?;
            Some(items)
        } else {
            None
        };

        // optional alias `as Ident` (only when no grouped items were present)
        let alias = if items.is_none() {
            if let Ok(next) = self.peek_token(TokenType::Identifier) {
                if self.get_token_str(*next) == Keyword::As.as_str() {
                    self.eat_keyword(Keyword::As)?;
                    Some(self.eat_identifier()?)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // allocate the path expression for the target
        let span = self.get_span_from(start);
        let target = self.tree.allocate(Expression::Path(path), span);

        let clause = self.tree.allocate(
            UseClause {
                target,
                alias,
                items,
            },
            span,
        );
        Ok(clause)
    }

    /// Eat a use item (like `geometry` or `geometry as geom`).
    ///
    /// Examples:
    /// ```
    /// geometry
    /// geometry as geom
    /// ```
    pub fn eat_use_item(&mut self) -> ParseResult<NodeId<UseItem>> {
        let start = self.mark();
        let name = self.eat_identifier()?;
        let alias = if let Ok(next) = self.peek_token(TokenType::Identifier) {
            if self.get_token_str(*next) == Keyword::As.as_str() {
                self.eat_keyword(Keyword::As)?;
                Some(self.eat_identifier()?)
            } else {
                None
            }
        } else {
            None
        };

        let item = self
            .tree
            .allocate(UseItem { name, alias }, self.get_span_from(start));
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Expression, PrimitiveType, Type, Use, UseClause, UseItem, With, WithClause, assert_node,
        assert_path, assert_string,
    };

    #[test]
    fn test_parse_with_type_assertion() {
        let mut test = TestParser::new("with T: int32");
        let mut parser = test.parser();
        let with_id = parser.eat_with().unwrap();
        // with T: int32
        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], WithClause::Assertion { target, assertion } => {
                assert_node!(parser.tree, *target, Type::Path { path, .. } => {
                    assert_path!(parser.session, *path, "T");
                });
                assert_node!(parser.tree, *assertion, Type::Primitive(PrimitiveType::Int(int_ty)) => {
                    assert_eq!(int_ty.width, 32);
                    assert!(int_ty.is_signed);
                });
            });
        });
    }

    #[test]
    fn test_parse_with_simple_declaration() {
        let mut test = TestParser::new("with Foo");
        let mut parser = test.parser();
        let with_id = parser.eat_with().unwrap();
        // with Foo
        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, alias } => {
                assert!(alias.is_none());
                assert_node!(parser.tree, *target, Type::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Foo");
                });
            });
        });
    }

    #[test]
    fn test_parse_with_aliased_declaration() {
        let mut test = TestParser::new("with Foo as Bar");
        let mut parser = test.parser();
        let with_id = parser.eat_with().unwrap();
        // with Foo as Bar
        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, alias } => {
                assert_string!(parser.session, alias.unwrap(), "Bar");
                assert_node!(parser.tree, *target, Type::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Foo");
                });
            });
        });
    }

    #[test]
    fn test_parse_with_path_declaration() {
        let mut test = TestParser::new("with Foo.Bar");
        let mut parser = test.parser();
        let with_id = parser.eat_with().unwrap();
        // with Foo.Bar
        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, alias } => {
                assert!(alias.is_none());
                assert_node!(parser.tree, *target, Type::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Foo.Bar");
                });
            });
        });
    }

    #[test]
    fn test_parse_with_negated_declaration() {
        let mut test = TestParser::new("with !Bar");
        let mut parser = test.parser();
        let with_id = parser.eat_with().unwrap();
        // with !Bar
        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, alias } => {
                assert!(alias.is_none());
                assert_node!(parser.tree, *target, Type::Not(inner_id) => {
                    assert_node!(parser.tree, *inner_id, Type::Path { path, .. } => {
                        assert_path!(parser.session, *path, "Bar");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_with_multiple_clauses() {
        let input = "with !Bar, Time, F: Numeric";
        let mut test = TestParser::new(input);
        let mut parser = test.parser();
        let with_id = parser.eat_with().unwrap();

        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 3);

            // !Bar
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Type::Not(inner_id) => {
                    assert_node!(parser.tree, *inner_id, Type::Path { path, .. } => {
                        assert_path!(parser.session, *path, "Bar");
                    });
                });
            });

            // Time
            assert_node!(parser.tree, clauses[1], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Type::Path { path, static_arguments } => {
                    assert_path!(parser.session, *path, "Time");
                    assert!(static_arguments.is_none());
                });
            });

            // F: Numeric
            assert_node!(parser.tree, clauses[2], WithClause::Assertion { target, assertion } => {
                assert_node!(parser.tree, *target, Type::Path { path, .. } => {
                    assert_path!(parser.session, *path, "F");
                });
                assert_node!(parser.tree, *assertion, Type::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Numeric");
                });
            });
        });
    }

    #[test]
    fn test_parse_with_parenthesized_multiline() {
        let input = r##"with (
  !Bar
  Time,
  F: Numeric
)"##;
        let mut test = TestParser::new(input);
        let mut parser = test.parser();
        let with_id = parser.eat_with().unwrap();

        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 3);

            // !Bar
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Type::Not(inner_id) => {
                    assert_node!(parser.tree, *inner_id, Type::Path { path, .. } => {
                        assert_path!(parser.session, *path, "Bar");
                    });
                });
            });

            // Time
            assert_node!(parser.tree, clauses[1], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Type::Path { path, static_arguments } => {
                    assert_path!(parser.session, *path, "Time");
                    assert!(static_arguments.is_none());
                });
            });

            // F: Numeric
            assert_node!(parser.tree, clauses[2], WithClause::Assertion { target, assertion } => {
                assert_node!(parser.tree, *target, Type::Path { path, .. } => {
                    assert_path!(parser.session, *path, "F");
                });
                assert_node!(parser.tree, *assertion, Type::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Numeric");
                });
            });
        });
    }

    #[test]
    fn test_parse_use_simple() {
        // use dyst
        let mut test = TestParser::new("use dyst");
        let mut parser = test.parser();
        let use_id = parser.eat_use(None).unwrap();

        // use
        assert_node!(parser.tree, use_id, Use { body, visibility, clauses } => {
            assert_eq!(*body, None);
            assert_eq!(*visibility, None);
            assert_eq!(clauses.len(), 1);
            // use dyst
            assert_node!(parser.tree, clauses[0], UseClause { target, alias, items } => {
                assert!(alias.is_none());
                assert!(items.is_none());
                assert_node!(parser.tree, *target, Expression::Path(path) => {
                    assert_path!(parser.session, *path, "dyst");
                });
            });
        });
    }

    #[test]
    fn test_parse_use_path() {
        let mut test = TestParser::new("use dyst.geometry");
        let mut parser = test.parser();
        let use_id = parser.eat_use(None).unwrap();

        // use dyst.geometry
        assert_node!(parser.tree, use_id, Use { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            // use dyst.geometry
            assert_node!(parser.tree, clauses[0], UseClause { target, alias, items } => {
                assert!(alias.is_none());
                assert!(items.is_none());
                assert_node!(parser.tree, *target, Expression::Path(path) => {
                    assert_path!(parser.session, *path, "dyst.geometry");
                });
            });
        });
    }

    #[test]
    fn test_parse_use_with_alias() {
        let mut test = TestParser::new("use dyst as ds");
        let mut parser = test.parser();
        let use_id = parser.eat_use(None).unwrap();

        // use dyst as ds
        assert_node!(parser.tree, use_id, Use { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            // use dyst as ds
            assert_node!(parser.tree, clauses[0], UseClause { target, alias, items } => {
                assert_string!(parser.session, alias.unwrap(), "ds");
                assert!(items.is_none());
                assert_node!(parser.tree, *target, Expression::Path(path) => {
                    assert_path!(parser.session, *path, "dyst");
                });
            });
        });
    }

    #[test]
    fn test_parse_use_with_items() {
        let mut test = TestParser::new("use ds.geometry.{Vector2, Vector3 as V3}");
        let mut parser = test.parser();
        let use_id = parser.eat_use(None).unwrap();

        // use ds.geometry.{Vector2, Vector3 as V3}
        assert_node!(parser.tree, use_id, Use { clauses, .. } => {
            assert_eq!(clauses.len(), 1);
            // use ds.geometry.{Vector2, Vector3 as V3}
            assert_node!(parser.tree, clauses[0], UseClause { target, alias, items } => {
                assert!(alias.is_none());
                let items = items.as_ref().expect("expected items");
                assert_eq!(items.len(), 2);
                // Vector2
                assert_node!(parser.tree, items[0], UseItem { name, alias } => {
                    assert_string!(parser.session, *name, "Vector2");
                    assert_eq!(*alias, None);
                });
                // Vector3 as V3
                assert_node!(parser.tree, items[1], UseItem { name, alias } => {
                    assert_string!(parser.session, *name, "Vector3");
                    assert_string!(parser.session, alias.unwrap(), "V3");
                });
                // ds.geometry
                assert_node!(parser.tree, *target, Expression::Path(path) => {
                    assert_path!(parser.session, *path, "ds.geometry");
                });
            });
        });
    }

    #[test]
    fn test_parse_use_multiple_clauses() {
        let mut test = TestParser::new("use dyst, dyst");
        let mut parser = test.parser();
        let use_id = parser.eat_use(None).unwrap();
        // use dyst, dyst
        assert_node!(parser.tree, use_id, Use { body, clauses, .. } => {
            assert_eq!(*body, None);
            assert_eq!(clauses.len(), 2);
            // use dyst
            assert_node!(parser.tree, clauses[0], UseClause { target, .. } => {
                assert_node!(parser.tree, *target, Expression::Path(path) => {
                    assert_path!(parser.session, *path, "dyst");
                });
            });
            // use dyst
            assert_node!(parser.tree, clauses[1], UseClause { target, .. } => {
                assert_node!(parser.tree, *target, Expression::Path(path) => {
                    assert_path!(parser.session, *path, "dyst");
                });
            });
        });
    }
}
