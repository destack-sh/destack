//! Parse `as` expressions.

use crate::{Expression, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    pub fn eat_as(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }
}
