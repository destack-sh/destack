//! Parse use and where declarations.
use dyst_token::TokenType;

use crate::{AstResult, Keyword, NodeId, Parser, WhereClause};

impl<'a> Parser<'a> {
    /// Eat a where context declaration or assignment maybe.
    #[inline]
    pub fn eat_where_maybe(&mut self) -> AstResult<Option<Vec<NodeId<WhereClause>>>> {
        if self.peek_keyword(Keyword::Where).is_ok() {
            Ok(Some(self.eat_where()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a where context declaration or assignment.
    ///
    /// Where can declare the use of an item in a scope and refine type bounds.
    ///
    /// Examples:
    /// ```
    /// where T: int32
    /// where Foo
    /// where Foo, Bar
    /// where Foo.Bar
    /// where !Bar
    /// where (
    ///    !Bar,
    ///    F: Numeric // optional comma
    ///    T > Y
    /// )
    /// ```
    pub fn eat_where(&mut self) -> AstResult<Vec<NodeId<WhereClause>>> {
        self.eat_keyword(Keyword::Where)?;
        let clauses = self.eat_where_body()?;
        Ok(clauses)
    }

    /// Eat the clauses of a `where` declaration (whereout the `where` keyword).
    pub fn eat_where_body(&mut self) -> AstResult<Vec<NodeId<WhereClause>>> {
        let mut clauses: Vec<NodeId<WhereClause>> = Vec::new();

        // parenthesized list where newlines
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
                // optional comma where newlines
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
    pub fn eat_where_clause(&mut self) -> AstResult<NodeId<WhereClause>> {
        let start = self.mark();

        let clause = {
            // assertion
            if self.peek_identifier().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
                let left = self.eat_identifier()?;
                self.eat_token(TokenType::Colon)?;
                let right = self.eat_expression()?;
                self.tree.allocate(
                    WhereClause::Assertion { left, right },
                    self.get_span_from(start),
                )
            }
            // guard
            else {
                let guard = self.eat_expression()?;
                self.tree
                    .allocate(WhereClause::Guard { guard }, self.get_span_from(start))
            }
        };

        Ok(clause)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Expression, TypeLiteral, UnaryOperator, WhereClause, assert_expr_path, assert_node,
        assert_path, assert_string,
    };
}
