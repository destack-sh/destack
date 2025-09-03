//! Parse expressions. Mostly defers to other parsers.

use crate::{Expression, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat an expression.
    pub fn eat_expression(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();
        let scalar_literal = self.eat_scalar_literal()?;
        let expression_id = self.tree.allocate(
            Expression::ScalarLiteral(scalar_literal),
            self.span_from(start),
        );
        // nocheckin: parse expressions
        Ok(expression_id)
    }
}
