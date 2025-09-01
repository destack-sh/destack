use crate::{ParseResult, Parser, StringId};
use destack_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat an identifier.
    pub fn eat_identifier(&mut self) -> ParseResult<StringId> {
        let token = *self.eat_token(TokenType::Identifier)?;
        let identifier_id = self.identifiers.intern(self.get_token_str(token));
        Ok(identifier_id)
    }
}
