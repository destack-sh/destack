//! Parse loops, for, while, etc.

use crate::{For, Keyword, Loop, NodeId, ParseResult, Parser, While};

impl<'a> Parser<'a> {
    /// Eat a loop (e.g., `loop { ... }`).
    ///
    /// Examples:
    /// ```
    /// loop {
    ///     y = getNext()
    ///     if y < 0 {
    ///         break
    ///     }
    /// }
    /// ```
    pub fn eat_loop(&mut self) -> ParseResult<NodeId<Loop>> {
        self.eat_keyword(Keyword::Loop)?;
        let start = self.mark();
        let block_id = self.eat_block()?;
        let loop_id = self
            .tree
            .allocate(Loop { body: block_id }, self.span_from(start));
        Ok(loop_id)
    }

    /// Eat a for loop (e.g., `for x in items { ... }`).
    ///
    /// Examples:
    /// ```
    /// for x in 1..10 {
    ///     y = 2
    /// }
    ///
    /// for x in zeds a: {
    ///     if y > 5 {
    ///         continue :a
    ///     }
    ///     y = 2
    /// }
    /// ```
    pub fn eat_for(&mut self) -> ParseResult<NodeId<For>> {
        todo!()
    }

    pub fn eat_for_header(&mut self) -> ParseResult<NodeId<For>> {
        todo!()
    }

    pub fn eat_while(&mut self) -> ParseResult<NodeId<While>> {
        todo!()
    }

    pub fn eat_while_header(&mut self) -> ParseResult<NodeId<While>> {
        todo!()
    }
}
