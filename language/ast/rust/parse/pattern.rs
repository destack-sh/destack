//! Parse patterns.

use destack_language_token::TokenType;

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
        let start = self.mark();

        // wildcard
        if self.peek_identifier_char('_').is_ok() {
            self.bump();
            let pattern = self
                .tree
                .allocate(Pattern::Wildcard, self.get_span_from(start));
            return Ok(pattern);
        }
        // rest
        else if self.peek_token(TokenType::Range).is_ok() {
            self.bump();
            let pattern = self.tree.allocate(Pattern::Rest, self.get_span_from(start));
            return Ok(pattern);
        }

        todo!("patterns")
    }
}
