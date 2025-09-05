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
            .allocate(Loop { body: block_id }, self.get_span_from(start));
        Ok(loop_id)
    }

    /// Eat a for loop (including keyword and header).
    ///
    /// Examples:
    /// ```
    /// for x in 1..10 {
    ///     y = 2
    /// }
    ///
    /// for x in zeds.iter() a: {
    ///     if y > 5 {
    ///         continue :a
    ///     }
    ///     y = 2
    /// }
    /// ```
    pub fn eat_for(&mut self) -> ParseResult<NodeId<For>> {
        let start = self.mark();
        // header
        self.eat_keyword(Keyword::For)?;
        let pattern_id = self.eat_pattern(None)?;
        self.eat_keyword(Keyword::In)?;
        let iterator_id = self.eat_expression(None)?;
        // body
        let block_id = self.eat_block()?;
        // for
        let for_id = self.tree.allocate(
            For {
                pattern: pattern_id,
                iterator: iterator_id,
                body: block_id,
            },
            self.get_span_from(start),
        );
        Ok(for_id)
    }

    /// Eat a while loop (including keyword and header).
    ///
    /// Examples:
    /// ```
    /// while x > 1 {
    ///     y = 2
    /// }
    ///
    /// while y < 10 l: {
    ///     y = 2
    ///     break :l
    /// }
    /// ```
    pub fn eat_while(&mut self) -> ParseResult<NodeId<While>> {
        let start = self.mark();
        // header
        self.eat_keyword(Keyword::While)?;
        let condition_id = self.eat_expression(None)?;
        // body
        let block_id = self.eat_block()?;
        // while
        let while_id = self.tree.allocate(
            While {
                condition: condition_id,
                body: block_id,
            },
            self.get_span_from(start),
        );
        Ok(while_id)
    }
}
