//! Parse unions and enums (which are just sugar for unions).

use crate::{Enum, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    pub fn eat_enum(&mut self) -> ParseResult<NodeId<Enum>> {
        todo!()
    }

    pub fn eat_enum_body(&mut self) -> ParseResult<NodeId<Enum>> {
        todo!()
    }
}
