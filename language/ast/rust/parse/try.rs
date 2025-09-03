//! Parse try and catch statements.

use crate::{NodeId, ParseResult, Parser, Try};

impl<'a> Parser<'a> {
    /// Eat a try statement.
    pub fn eat_try_catch(&mut self) -> ParseResult<NodeId<Try>> {
        todo!()
    }
}
