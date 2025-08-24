use crate::{ParseResult, Parser};
use destack_language_lexer::TokenType;

pub type Identifier = String;

impl<'a> Parser<'a> {
    /// Eat an identifier.
    pub fn eat_identifier(&mut self) -> ParseResult<Identifier> {
        let token = *self.eat_token_type(TokenType::Identifier)?;
        Ok(self.get_token_str(token).to_string())
    }
}
