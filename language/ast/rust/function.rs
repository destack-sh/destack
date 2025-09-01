//! Parse functions and closures.

use crate::{Expression, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    pub fn eat_function(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }
}
