//! Parse loops, for, while, etc.

use crate::{For, Loop, ParseResult, Parser, While};

impl<'a> Parser<'a> {
    pub fn eat_loop(&mut self) -> ParseResult<Loop> {
        todo!()
    }

    pub fn eat_for(&mut self) -> ParseResult<For> {
        todo!()
    }

    pub fn eat_for_header(&mut self) -> ParseResult<For> {
        todo!()
    }

    pub fn eat_while(&mut self) -> ParseResult<While> {
        todo!()
    }

    pub fn eat_while_header(&mut self) -> ParseResult<While> {
        todo!()
    }
}
