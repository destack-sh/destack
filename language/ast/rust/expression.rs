//! Parse expressions. Mostly defers to other parsers.

use crate::{Expression, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat an expression.
    pub fn eat_expression(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();
        // nocheckin

        todo!()
    }
}
