//! Parse use and with declarations.
use dyst_token::TokenType;

use crate::{AstResult, Keyword, NodeId, Parser, WithClause};

impl<'a> Parser<'a> {
    /// Eat a with context declaration or assignment maybe.
    #[inline]
    pub fn eat_with_maybe(&mut self) -> AstResult<Option<Vec<NodeId<WithClause>>>> {
        if self.peek_keyword(Keyword::With).is_ok() {
            Ok(Some(self.eat_with()?))
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
    /// with !Bar
    /// with (
    ///    !Bar,
    ///    Time<float32> // optional comma
    ///    F: Numeric
    /// )
    /// ```
    pub fn eat_with(&mut self) -> AstResult<Vec<NodeId<WithClause>>> {
        self.eat_keyword(Keyword::With)?;
        let clauses = self.with_options(self.options.in_before_block(), |parser| {
            parser.eat_with_body()
        })?;
        Ok(clauses)
    }

    /// Eat the clauses of a `with` declaration (without the `with` keyword).
    fn eat_with_body(&mut self) -> AstResult<Vec<NodeId<WithClause>>> {
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

        Ok(clauses)
    }

    /// Eat a single with clause. May be a declaration or a assignment.
    fn eat_with_clause(&mut self) -> AstResult<NodeId<WithClause>> {
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
        let right = self.with_options(self.options.in_before_block(), |parser| {
            parser.eat_expression()
        })?;

        // clause
        let clause = self
            .tree
            .allocate(WithClause { alias, right }, self.get_span_from(start));

        Ok(clause)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Expression, TypeLiteral, UnaryOperator, WithClause, assert_expr_path, assert_node,
        assert_path, assert_string,
    };

    #[test]
    fn test_parse_with_type_assertion() {
        let mut test = TestParser::new("with T: int32");
        let mut parser = test.prepare();
        let clauses = parser.eat_with().unwrap();
        // with T: int32
        assert_eq!(clauses.len(), 1);
        assert_node!(parser.tree, clauses[0], WithClause { alias, right } => {
            assert_string!(parser.session, alias.unwrap(), "T");
            assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                assert_eq!(int_ty.width, Some(32));
                assert!(int_ty.is_signed);
            });
        });
    }

    #[test]
    fn test_parse_with_simple_declaration() {
        let mut test = TestParser::new("with Foo");
        let mut parser = test.prepare();
        let clauses = parser.eat_with().unwrap();
        // with Foo
        assert_eq!(clauses.len(), 1);
        assert_node!(parser.tree, clauses[0], WithClause { alias: _, right } => {
            assert_expr_path!(parser.session, parser.tree.get(*right), "Foo");
        });
    }

    #[test]
    fn test_parse_with_path_declaration() {
        let mut test = TestParser::new("with Foo.Bar");
        let mut parser = test.prepare();
        let clauses = parser.eat_with().unwrap();
        // with Foo.Bar
        assert_eq!(clauses.len(), 1);
        assert_node!(parser.tree, clauses[0], WithClause { alias: _, right } => {
            assert_expr_path!(parser.session, parser.tree.get(*right), "Foo.Bar");
        });
    }

    #[test]
    fn test_parse_with_multiple_clauses() {
        let input = "with !Bar, Time, F: Numeric";
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let clauses = parser.eat_with().unwrap();

        assert_eq!(clauses.len(), 3);

        // !Bar
        assert_node!(parser.tree, clauses[0], WithClause { alias: _, right } => {
            assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::Not);
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Bar");
                });
            });
        });

        // Time
        assert_node!(parser.tree, clauses[1], WithClause { alias: _, right } => {
            assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                assert_path!(parser.session, *path, "Time");
                assert!(static_arguments.is_none());
            });
        });

        // F: Numeric
        assert_node!(parser.tree, clauses[2], WithClause { alias, right } => {
            assert_string!(parser.session, alias.unwrap(), "F");
            assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Numeric");
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
        let mut parser = test.prepare();
        let clauses = parser.eat_with().unwrap();

        assert_eq!(clauses.len(), 3);

        // !Bar
        assert_node!(parser.tree, clauses[0], WithClause { alias: _, right } => {
            assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::Not);
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Bar");
                });
            });
        });

        // Time
        assert_node!(parser.tree, clauses[1], WithClause { alias: _, right } => {
            assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                assert_path!(parser.session, *path, "Time");
                assert!(static_arguments.is_none());
            });
        });

        // F: Numeric
        assert_node!(parser.tree, clauses[2], WithClause { alias, right } => {
            assert_string!(parser.session, alias.unwrap(), "F");
            assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Numeric");
            });
        });
    }
}
