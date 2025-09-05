//! Parse use and with declarations.
use destack_language_token::TokenType;

use crate::{
    Expression, Keyword, NodeId, ParseResult, Parser, Use, UseClause, UseItem, With, WithClause,
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
    /// with Foo, Bar
    /// with Foo.Bar
    /// with !Bar
    /// with (
    ///    !Bar,
    ///    Time[float32],
    ///    F: Numeric
    /// )
    /// ```
    pub fn eat_with(&mut self) -> ParseResult<NodeId<With>> {
        self.eat_keyword(Keyword::With)?;
        let with = self.eat_with_header()?;
        self.eat_statement_stop()?;
        Ok(with)
    }

    /// Eat the content of a with declaration (without the `with` keyword).
    pub fn eat_with_header(&mut self) -> ParseResult<NodeId<With>> {
        let start = self.mark();

        // parse one or more clauses separated by commas
        let mut clauses: Vec<NodeId<WithClause>> = Vec::new();
        let clause = self.eat_with_clause()?;
        clauses.push(clause);
        loop {
            if self.peek_token(TokenType::Comma).is_ok() {
                self.eat_token(TokenType::Comma)?;
                // allow trailing comma before stop
                if self.peek_statement_stop().is_ok() {
                    break;
                }
                let next_clause = self.eat_with_clause()?;
                clauses.push(next_clause);
                continue;
            }
            break;
        }

        let with = self
            .tree
            .allocate(With { clauses }, self.get_span_from(start));
        Ok(with)
    }

    /// Eat a single with clause.
    ///
    /// A clause can be a declaration (`Foo`, `Foo as Bar`, `Foo.Bar as Baz`)
    /// or an assertion (`T: int32`, `Self: geom.Mesh[T]`, `T.Item: Copy`).
    pub fn eat_with_clause(&mut self) -> ParseResult<NodeId<WithClause>> {
        let start = self.mark();

        // first parse the left-hand side type target
        let lhs = self.eat_type()?;

        // assertion: `T: SomeType`
        if self.peek_colon().is_ok() {
            self.eat_colon()?;
            let rhs = self.eat_type()?;
            let clause = self.tree.allocate(
                WithClause::Assertion {
                    target: lhs,
                    assertion: rhs,
                },
                self.get_span_from(start),
            );
            Ok(clause)
        }
        // declaration: optional alias `as Ident`
        else {
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

            let clause = self.tree.allocate(
                WithClause::Declaration { target: lhs, alias },
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
    pub fn eat_use(&mut self) -> ParseResult<NodeId<Use>> {
        self.eat_keyword(Keyword::Use)?;
        let using = self.eat_use_header()?;
        self.eat_statement_stop()?;
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
    pub fn eat_use_header(&mut self) -> ParseResult<NodeId<Use>> {
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
        let target = self.tree.allocate(Expression::Path { path }, span);

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
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Expression, Parser, PrimitiveType, Type, UseItem, WithClause};

    #[test]
    fn test_parse_with() {
        let input = r##"
with T: int32
with Foo
with Foo as Bar
with Foo.Bar
with !Bar
with !Bar, Time[float32], F: Numeric
"##;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // with T: int32
        let with_id = parser.eat_with().unwrap();
        let with = parser.tree.get(with_id);
        assert_eq!(with.clauses.len(), 1);
        match parser.tree.get(with.clauses[0]) {
            WithClause::Assertion { target, assertion } => {
                match parser.tree.get(*target) {
                    Type::Path { path, .. } => {
                        assert_eq!(*path, parser.paths.intern(vec![parser.strings.intern("T")]));
                    }
                    _ => panic!("expected path type for target"),
                }
                match parser.tree.get(*assertion) {
                    Type::Primitive(PrimitiveType::Int(int_ty)) => {
                        assert_eq!(int_ty.width, 32);
                        assert!(int_ty.is_signed);
                    }
                    _ => panic!("expected primitive int type for assertion"),
                }
            }
            _ => panic!("expected assertion clause"),
        }

        // with Foo
        let with_id = parser.eat_with().unwrap();
        let with = parser.tree.get(with_id);
        assert_eq!(with.clauses.len(), 1);
        match parser.tree.get(with.clauses[0]) {
            WithClause::Declaration { target, alias } => {
                assert!(alias.is_none());
                match parser.tree.get(*target) {
                    Type::Path { path, .. } => {
                        assert_eq!(
                            *path,
                            parser.paths.intern(vec![parser.strings.intern("Foo")])
                        );
                    }
                    _ => panic!("expected path type"),
                }
            }
            _ => panic!("expected declaration clause"),
        }

        // with Foo as Bar
        let with_id = parser.eat_with().unwrap();
        let with = parser.tree.get(with_id);
        assert_eq!(with.clauses.len(), 1);
        match parser.tree.get(with.clauses[0]) {
            WithClause::Declaration { target, alias } => {
                assert_eq!(*alias, Some(parser.strings.intern("Bar")));
                match parser.tree.get(*target) {
                    Type::Path { path, .. } => {
                        assert_eq!(
                            *path,
                            parser.paths.intern(vec![parser.strings.intern("Foo")])
                        );
                    }
                    _ => panic!("expected path type"),
                }
            }
            _ => panic!("expected declaration clause"),
        }

        // with Foo.Bar
        let with_id = parser.eat_with().unwrap();
        let with = parser.tree.get(with_id);
        assert_eq!(with.clauses.len(), 1);
        match parser.tree.get(with.clauses[0]) {
            WithClause::Declaration { target, alias } => {
                assert!(alias.is_none());
                match parser.tree.get(*target) {
                    Type::Path { path, .. } => {
                        assert_eq!(
                            *path,
                            parser.paths.intern(vec![
                                parser.strings.intern("Foo"),
                                parser.strings.intern("Bar")
                            ])
                        );
                    }
                    _ => panic!("expected path type"),
                }
            }
            _ => panic!("expected declaration clause"),
        }

        // with !Bar
        let with_id = parser.eat_with().unwrap();
        let with = parser.tree.get(with_id);
        assert_eq!(with.clauses.len(), 1);
        match parser.tree.get(with.clauses[0]) {
            WithClause::Declaration { target, alias } => {
                assert!(alias.is_none());
                match parser.tree.get(*target) {
                    Type::Not(inner_id) => match parser.tree.get(*inner_id) {
                        Type::Path { path, .. } => {
                            assert_eq!(
                                *path,
                                parser.paths.intern(vec![parser.strings.intern("Bar")])
                            );
                        }
                        _ => panic!("expected path type inside !"),
                    },
                    _ => panic!("expected Not type"),
                }
            }
            _ => panic!("expected declaration clause"),
        }

        // with !Bar, Time[float32], F: Numeric
        let with_id = parser.eat_with().unwrap();
        let with = parser.tree.get(with_id);
        assert_eq!(with.clauses.len(), 3);
        // !Bar
        match parser.tree.get(with.clauses[0]) {
            WithClause::Declaration { target, .. } => match parser.tree.get(*target) {
                Type::Not(inner_id) => match parser.tree.get(*inner_id) {
                    Type::Path { path, .. } => {
                        assert_eq!(
                            *path,
                            parser.paths.intern(vec![parser.strings.intern("Bar")])
                        );
                    }
                    _ => panic!("expected path type inside !"),
                },
                _ => panic!("expected Not type"),
            },
            _ => panic!("expected declaration clause"),
        }
        // Time[float32]
        match parser.tree.get(with.clauses[1]) {
            WithClause::Declaration { target, .. } => match parser.tree.get(*target) {
                Type::Path {
                    path,
                    static_arguments,
                } => {
                    assert_eq!(
                        *path,
                        parser.paths.intern(vec![parser.strings.intern("Time")])
                    );
                    let args = static_arguments.as_ref().expect("expected static args");
                    assert_eq!(args.len(), 1);
                }
                _ => panic!("expected path type with static args"),
            },
            _ => panic!("expected declaration clause"),
        }
        // F: Numeric
        match parser.tree.get(with.clauses[2]) {
            WithClause::Assertion { target, assertion } => {
                match parser.tree.get(*target) {
                    Type::Path { path, .. } => {
                        assert_eq!(*path, parser.paths.intern(vec![parser.strings.intern("F")]));
                    }
                    _ => panic!("expected path type for target"),
                }
                match parser.tree.get(*assertion) {
                    Type::Path { path, .. } => {
                        assert_eq!(
                            *path,
                            parser.paths.intern(vec![parser.strings.intern("Numeric")])
                        );
                    }
                    _ => panic!("expected path type for assertion"),
                }
            }
            _ => panic!("expected assertion clause"),
        }
    }

    #[test]
    fn test_parse_using() {
        let input = r##"
use dyst
use dyst.geometry
use dyst as ds
use ds.geometry as geom
use ds.geometry.{Vector2, Vector3 as V3}
use dyst, dyst
"##;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // use dyst
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.body, None);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert!(clause.alias.is_none());
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![parser.strings.intern("dyst")])
            ),
            _ => panic!("expected path expression"),
        }

        // use dyst.geometry
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert!(clause.alias.is_none());
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![
                    parser.strings.intern("dyst"),
                    parser.strings.intern("geometry")
                ])
            ),
            _ => panic!("expected path expression"),
        }

        // use dyst as ds
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert_eq!(clause.alias, Some(parser.strings.intern("ds")));
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![parser.strings.intern("dyst")]),
            ),
            _ => panic!("expected path expression"),
        }

        // use ds.geometry as geom
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert_eq!(clause.alias, Some(parser.strings.intern("geom")));
        assert!(clause.items.is_none());
        match parser.tree.get(clause.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![
                    parser.strings.intern("ds"),
                    parser.strings.intern("geometry")
                ]),
            ),
            _ => panic!("expected path expression"),
        }

        // use ds.geometry.{Vector2, Vector3 as V3}
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.clauses.len(), 1);
        let clause = parser.tree.get(using.clauses[0]);
        assert!(clause.alias.is_none());
        let items = clause.items.as_ref().expect("expected items");
        assert_eq!(items.len(), 2);
        let item0 = parser.tree.get(items[0]);
        assert_eq!(
            *item0,
            UseItem {
                name: parser.strings.intern("Vector2"),
                alias: None,
            }
        );
        let item1 = parser.tree.get(items[1]);
        assert_eq!(
            *item1,
            UseItem {
                name: parser.strings.intern("Vector3"),
                alias: Some(parser.strings.intern("V3")),
            }
        );

        // use dyst, dyst
        let use_id = parser.eat_use().unwrap();
        let using = parser.tree.get(use_id);
        assert_eq!(using.body, None);
        assert_eq!(using.clauses.len(), 2);
        let clause0 = parser.tree.get(using.clauses[0]);
        match parser.tree.get(clause0.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![parser.strings.intern("dyst")]),
            ),
            _ => panic!("expected path expression"),
        }
        let clause1 = parser.tree.get(using.clauses[1]);
        match parser.tree.get(clause1.target) {
            Expression::Path { path } => assert_eq!(
                *path,
                parser.paths.intern(vec![parser.strings.intern("dyst")]),
            ),
            _ => panic!("expected path expression"),
        }
    }
}
