//! Parse use and with declarations.
use crate::{ParseResult, Parser};

use dyst_ast::{Expression, Keyword, LocalNodeId, TokenType, WithClause};

impl<'a> Parser<'a> {
    /// Eat a with context declaration or assignment maybe (including the `with` keyword and an optional body).
    #[inline]
    pub fn eat_with_maybe(&mut self) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if self.peek_keyword(Keyword::With).is_ok() {
            Ok(Some(self.eat_with()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a with context declaration or assignment (including the `with` keyword and an optional body).
    ///
    /// Examples:
    /// ```
    /// with T: int32
    /// with Foo
    /// with Foo, Bar
    /// with Foo.Bar
    ///
    /// with x, y {
    ///   ...
    /// }
    /// ```
    pub fn eat_with(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::With)?;

        // clauses
        let clauses = self.with_options(self.options.in_before_block(), |parser| {
            parser.eat_with_clauses()
        })?;

        // body
        let body = if self.peek_block().is_ok() {
            Some(self.eat_block()?)
        } else {
            None
        };

        // with
        let with_id = self.tree.insert(
            Expression::With { clauses, body },
            self.get_span_from(start),
        );
        Ok(with_id)
    }

    /// Eat a with context declaration or assignment maybe.
    #[inline]
    pub fn eat_with_header_maybe(&mut self) -> ParseResult<Option<Vec<LocalNodeId<WithClause>>>> {
        if self.peek_keyword(Keyword::With).is_ok() {
            Ok(Some(self.eat_with_header()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a with context declaration or assignment.
    ///
    /// With can declare the use of an item in a scope and refine type bounds.
    ///
    /// Examples:
    /// ```
    /// with T: int32
    /// with Foo
    /// with Foo, Bar
    /// with Foo.Bar
    /// with (
    ///    Time<float32> // optional comma
    ///    F: Numeric
    /// )
    /// ```
    pub fn eat_with_header(&mut self) -> ParseResult<Vec<LocalNodeId<WithClause>>> {
        self.eat_keyword(Keyword::With)?;
        let clauses = self.with_options(self.options.in_before_block(), |parser| {
            parser.eat_with_clauses()
        })?;
        Ok(clauses)
    }

    /// Eat the clauses of a `with` declaration (without the `with` keyword).
    /// Separated by commas.
    fn eat_with_clauses(&mut self) -> ParseResult<Vec<LocalNodeId<WithClause>>> {
        let mut clauses: Vec<LocalNodeId<WithClause>> = Vec::new();

        // parenthesized list with newlines
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.eat_token(TokenType::OpenParenthesis)?;
            self.eat_newlines_maybe()?;
            while self.peek().is_ok() {
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
            while self.peek().is_ok() {
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

        Ok(clauses)
    }

    /// Eat a single with clause. May be a declaration or a assignment.
    fn eat_with_clause(&mut self) -> ParseResult<LocalNodeId<WithClause>> {
        let start = self.mark();

        // alias
        let alias =
            if self.peek_identifier().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
                let alias = self.eat_identifier()?;
                self.eat_token(TokenType::Colon)?;
                Some(alias)
            } else {
                None
            };

        // right
        let right = self.with_options(
            self.options.not_in_position().in_static().in_before_block(),
            |parser| parser.eat_expression(),
        )?;

        // clause
        let clause = self
            .tree
            .insert(WithClause { alias, right }, self.get_span_from(start));

        Ok(clause)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{Expression, UnaryOperator, WithClause};

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_with_type_assertion() {
        let mut test = TestParser::new("with T: Something");
        let mut parser = test.prepare();
        let clauses = parser.eat_with_header().unwrap();
        // with T: Something
        assert_eq!(clauses.len(), 1);
        assert_node!(parser.tree, clauses[0], WithClause { alias, right } => {
            assert_string!(parser, alias.unwrap(), "T");
            assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Something");
            });
        });
    }

    #[test]
    fn test_parse_with_simple_declaration() {
        let mut test = TestParser::new("with Foo");
        let mut parser = test.prepare();
        let clauses = parser.eat_with_header().unwrap();
        // with Foo
        assert_eq!(clauses.len(), 1);
        assert_node!(parser.tree, clauses[0], WithClause { alias: _, right } => {
            assert_expression_path!(parser, parser.tree.get(*right), "Foo");
        });
    }

    #[test]
    fn test_parse_with_path_declaration() {
        let mut test = TestParser::new("with Foo.Bar");
        let mut parser = test.prepare();
        let clauses = parser.eat_with_header().unwrap();
        // with Foo.Bar
        assert_eq!(clauses.len(), 1);
        assert_node!(parser.tree, clauses[0], WithClause { alias: _, right } => {
            assert_expression_path!(parser, parser.tree.get(*right), "Foo.Bar");
        });
    }

    #[test]
    fn test_parse_with_multiple_clauses() {
        let input = "with Time, F: Force";
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let clauses = parser.eat_with_header().unwrap();

        assert_eq!(clauses.len(), 2);

        // Time
        assert_node!(parser.tree, clauses[0], WithClause { alias: _, right } => {
            assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Time");
                assert!(static_arguments.is_none());
            });
        });

        // F: Force
        assert_node!(parser.tree, clauses[1], WithClause { alias, right } => {
            assert_string!(parser, alias.unwrap(), "F");
            assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Force");
            });
        });
    }

    #[test]
    fn test_parse_with_parenthesized_multiline() {
        let input = r##"with (
  !Bar
  Time,
  F: Force
)"##;
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let clauses = parser.eat_with_header().unwrap();

        assert_eq!(clauses.len(), 3);

        // !Bar
        assert_node!(parser.tree, clauses[0], WithClause { alias: _, right } => {
            assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::Not);
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Bar");
                });
            });
        });

        // Time
        assert_node!(parser.tree, clauses[1], WithClause { alias: _, right } => {
            assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Time");
                assert!(static_arguments.is_none());
            });
        });

        // F: Force
        assert_node!(parser.tree, clauses[2], WithClause { alias, right } => {
            assert_string!(parser, alias.unwrap(), "F");
            assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Force");
            });
        });
    }

    #[test]
    fn test_parse_with_expression_via_expression_parser() {
        let mut test = TestParser::new("with Context");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // with Context
        assert_node!(parser.tree, expression_id, Expression::With { clauses, body } => {
            assert_eq!(clauses.len(), 1);
            assert!(body.is_none());

            assert_node!(parser.tree, clauses[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });
        });
    }
}
