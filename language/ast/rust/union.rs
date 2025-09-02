//! Parse unions and enums (which are just sugar for unions).

use crate::{NodeId, ParseResult, Parser, Union};

impl<'a> Parser<'a> {
    pub fn eat_union_or_enum(&mut self) -> ParseResult<NodeId<Union>> {
        todo!()
    }

    pub fn eat_union_or_enum_body(&mut self) -> ParseResult<NodeId<Union>> {
        todo!()
    }
}
