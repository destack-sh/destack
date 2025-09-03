//! Parse patterns.

use crate::{NodeId, ParseResult, Parser, Pattern};

impl<'a> Parser<'a> {
    /// Eat a pattern.
    pub fn eat_pattern(&mut self) -> ParseResult<NodeId<Pattern>> {
        todo!()
    }
}
