//! Parse unions and enums (which are just sugar for unions).

use destack_language_arena::StringId;
use destack_language_token::{TokenSpan, TokenType};

use crate::{ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Peek an identifier.
    #[inline]
    pub fn peek_identifier(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Identifier)
    }

    /// Eat an identifier.
    #[inline]
    pub fn eat_identifier(&mut self) -> ParseResult<StringId> {
        let token = *self.eat_token(TokenType::Identifier)?;
        let string_id = self.strings.intern(self.get_token_str(token));
        Ok(string_id)
    }
}
