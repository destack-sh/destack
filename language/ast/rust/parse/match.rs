use crate::{Match, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a match statement.
    pub fn eat_match(&mut self) -> ParseResult<NodeId<Match>> {
        todo!()
    }
}
