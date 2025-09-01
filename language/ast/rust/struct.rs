//! Parse structs.

use crate::{NodeId, ParseResult, Parser, Struct};

impl<'a> Parser<'a> {
    /// Eat a struct.
    pub fn eat_struct(&mut self) -> ParseResult<NodeId<Struct>> {
        todo!()
    }

    // Eat a struct body (without the header or `{` and `}`)
    pub fn eat_struct_body(&mut self) -> ParseResult<NodeId<Struct>> {
        todo!()
    }
}
