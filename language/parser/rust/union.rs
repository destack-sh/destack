//! Parse unions and enums (which are just sugar for unions).

use crate::{ParseResult, Parser, Type};

impl<'a> Parser<'a> {
    pub fn eat_union_or_enum(&mut self) -> ParseResult<Type> {
        todo!()
    }

    pub fn eat_union_or_enum_body(&mut self) -> ParseResult<Type> {
        todo!()
    }
}
