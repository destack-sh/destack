//! Parse patterns.

use crate::{NodeId, ParseResult, Parser, Pattern};

impl<'a> Parser<'a> {
    /// Eat a pattern.
    ///
    /// Examples:
    /// ```
    /// _
    /// 1
    /// 2 | 3
    /// 4..6
    /// (x, 0, ..)
		/// x, y
		/// y, x, ..
    /// Vector2 { x: 0, y }
    /// Point(x, y: new_y)
    /// ```
    pub fn eat_pattern(&mut self) -> ParseResult<NodeId<Pattern>> {
        todo!()
    }
}
