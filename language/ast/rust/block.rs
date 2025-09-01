use crate::{Block, ParseResult, Parser};
use destack_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat a block (including the label, `{`, and `}`).
    pub fn eat_block(&mut self) -> ParseResult<Block> {
        self.eat_token(TokenType::OpenBrace)?;
        let block = self.eat_block_body()?;
        self.eat_token(TokenType::CloseBrace)?;
        Ok(block)
    }

    /// Parse a block of statements (without the label, `{`, and `}`)
    pub fn eat_block_body(&mut self) -> ParseResult<Block> {
        todo!()
    }
}
