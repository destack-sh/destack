//! Parse loops, for, while, etc.

use crate::{Expression, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    pub fn eat_loop(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }

    pub fn eat_for(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }

    pub fn eat_for_header(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }

    pub fn eat_while(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }

    pub fn eat_while_header(&mut self) -> ParseResult<NodeId<Expression>> {
        todo!()
    }
}
