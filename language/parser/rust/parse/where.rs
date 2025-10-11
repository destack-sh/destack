//! Parse use and where declarations.
use crate::TokenType;

use crate::{Keyword, NodeId, Parser, ParserResult, WhereClause};

impl<'a> Parser<'a> {
    /// Eat a where context declaration or assignment maybe.
    ///
    /// Examples:
    /// ```
    /// where T: int32
    /// where Foo
    /// where Foo, Bar
    /// where Foo.Bar
    /// where !Bar
    /// ```
    #[inline]
    pub fn eat_where_maybe(&mut self) -> ParserResult<Option<Vec<NodeId<WhereClause>>>> {
        if self.peek_keyword(Keyword::Where).is_ok() {
            Ok(Some(self.eat_where()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a where context declaration or assignment.
    ///
    /// Examples:
    /// ```
    /// where T: int32
    /// where Foo
    /// where Foo, Bar
    /// where Foo.Bar
    /// where !Bar, Time > Limit, F: Numeric
    /// where (
    ///    !Bar,
    ///    F: Numeric // optional comma
    ///    T > Y
    /// )
    /// ```
    pub fn eat_where(&mut self) -> ParserResult<Vec<NodeId<WhereClause>>> {
        self.eat_keyword(Keyword::Where)?;
        let clauses = self.with_options(self.options.in_before_block(), |parser| {
            parser.eat_where_body()
        })?;
        Ok(clauses)
    }

    /// Eat the clauses of a `where` declaration (without the `where` keyword).
    /// Separated by commas.
    fn eat_where_body(&mut self) -> ParserResult<Vec<NodeId<WhereClause>>> {
        let mut clauses: Vec<NodeId<WhereClause>> = Vec::new();

        // parenthesized list with newlines
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.eat_token(TokenType::OpenParenthesis)?;
            self.eat_newlines_maybe()?;
            loop {
                self.eat_newlines_maybe()?;
                if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                    break;
                }
                let next_clause = self.eat_where_clause()?;
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
                let clause = self.eat_where_clause()?;
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

    /// Eat a single where clause. May be a declaration or a assignment.
    fn eat_where_clause(&mut self) -> ParserResult<NodeId<WhereClause>> {
        let start = self.mark();

        let clause = {
            // assertion
            if self.peek_identifier().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
                let left = self.eat_identifier()?;
                self.eat_token(TokenType::Colon)?;
                let right =
                    self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
                self.tree.insert(
                    WhereClause::Assertion { left, right },
                    self.get_span_from(start),
                )
            }
            // guard
            else {
                let guard = self.eat_expression()?;
                self.tree
                    .insert(WhereClause::Guard { guard }, self.get_span_from(start))
            }
        };

        Ok(clause)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Expression, TypeLiteral, UnaryOperator, WhereClause, assert_expr_path,
        assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_where_type_assertion() {
        let mut test = TestParser::new("where T: int32");
        let mut parser = test.prepare();
        let clauses = parser.eat_where().unwrap();

        // where T: int32
        assert_eq!(clauses.len(), 1);
        assert_node!(parser.tree, clauses[0], WhereClause::Assertion { left, right } => {
            assert_string!(parser, *left, "T");
            assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                assert_eq!(int_ty.width, Some(32));
                assert!(int_ty.is_signed);
            });
        });
    }

    #[test]
    fn test_parse_where_guard_comparison() {
        let mut test = TestParser::new("where T > Y");
        let mut parser = test.prepare();
        let clauses = parser.eat_where().unwrap();

        // where T > Y
        assert_eq!(clauses.len(), 1);
        assert_node!(parser.tree, clauses[0], WhereClause::Guard { guard } => {
            assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                assert_expr_path!(parser, parser.tree.get(*left), "T");
                assert_expr_path!(parser, parser.tree.get(*right), "Y");
            });
        });
    }

    #[test]
    fn test_parse_where_multiple_clauses() {
        let input = "where !Bar, Time > Limit, F: Numeric";
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let clauses = parser.eat_where().unwrap();

        assert_eq!(clauses.len(), 3);

        // !Bar
        assert_node!(parser.tree, clauses[0], WhereClause::Guard { guard } => {
            assert_node!(parser.tree, *guard, Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::Not);
                assert_expr_path!(parser, parser.tree.get(*right), "Bar");
            });
        });

        // Time > Limit
        assert_node!(parser.tree, clauses[1], WhereClause::Guard { guard } => {
            assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                assert_expr_path!(parser, parser.tree.get(*left), "Time");
                assert_expr_path!(parser, parser.tree.get(*right), "Limit");
            });
        });

        // F: Numeric
        assert_node!(parser.tree, clauses[2], WhereClause::Assertion { left, right } => {
            assert_string!(parser, *left, "F");
            assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Numeric");
            });
        });
    }

    #[test]
    fn test_parse_where_parenthesized_multiline() {
        let input = r##"where (
  !Bar
  Time > Limit,
  F: Numeric
)"##;
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let clauses = parser.eat_where().unwrap();

        assert_eq!(clauses.len(), 3);

        // !Bar
        assert_node!(parser.tree, clauses[0], WhereClause::Guard { guard } => {
            assert_node!(parser.tree, *guard, Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::Not);
                assert_expr_path!(parser, parser.tree.get(*right), "Bar");
            });
        });

        // Time > Limit
        assert_node!(parser.tree, clauses[1], WhereClause::Guard { guard } => {
            assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                assert_expr_path!(parser, parser.tree.get(*left), "Time");
                assert_expr_path!(parser, parser.tree.get(*right), "Limit");
            });
        });

        // F: Numeric
        assert_node!(parser.tree, clauses[2], WhereClause::Assertion { left, right } => {
            assert_string!(parser, *left, "F");
            assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Numeric");
            });
        });
    }
}
