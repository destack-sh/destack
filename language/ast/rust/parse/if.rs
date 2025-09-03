use crate::{If, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat an if statement.
    pub fn eat_if(&mut self) -> ParseResult<NodeId<If>> {
        todo!()
    }
}
