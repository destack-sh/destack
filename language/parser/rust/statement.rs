//! Parse statements (and blocks).
//! Like with expressions, this mostly defers to other parsers.

use destack_language_lexer::TokenType;

use crate::{Block, ParseResult, Parser, Statement};

impl<'a> Parser<'a> {
    /// Eat a block (including the `{` and `}`).
    pub fn eat_block(&mut self) -> ParseResult<Block> {
        self.eat_token(TokenType::OpenBrace)?;
        let block = self.eat_block_body()?;
        self.eat_token(TokenType::CloseBrace)?;
        Ok(block)
    }

    /// Parse a block of statements (without the `{` and `}`)
    pub fn eat_block_body(&mut self) -> ParseResult<Block> {
        todo!()
    }

    /// Parse a statement (without the `;`).
    pub fn eat_statement_body(&mut self) -> ParseResult<Statement> {
        todo!()
    }

    /// Parse a statement (with the `;`).
    pub fn eat_statement(&mut self) -> ParseResult<Statement> {
        todo!()
    }
}
