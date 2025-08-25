//! Parse try and catch statements.

use crate::{ParseResult, Parser, Try};

impl<'a> Parser<'a> {
    /// Eat a try statement.
    pub fn eat_try_catch(&mut self) -> ParseResult<Try> {
        todo!()
    }
}
