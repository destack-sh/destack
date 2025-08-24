use crate::{Block, ParseResult, Parser, Statement};

impl<'a> Parser<'a> {
    /// Parse a block of statements (without the `{` and `}`)
    pub fn eat_block_content(&mut self) -> ParseResult<Block> {
        todo!()
    }

    /// Parse a statement (without the `;`).
    pub fn eat_statement_content(&mut self) -> ParseResult<Statement> {
        todo!()
    }

    /// Parse a statement (with the `;`).
    pub fn eat_statement(&mut self) -> ParseResult<Statement> {
        todo!()
    }
}
