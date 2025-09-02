//! Parse expressions. Mostly defers to other parsers.

use crate::{Expression, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat an expression.
    pub fn eat_expression(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();
        let literal_id = self.eat_scalar_literal()?;
        let expression_id = self
            .tree
            .allocate(Expression::ScalarLiteral(literal_id), self.span_from(start));
        Ok(expression_id)
    }
}
