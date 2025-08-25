use crate::{Argument, ParseResult, Parser};
use destack_language_lexer::TokenType;

impl<'a> Parser<'a> {
    /// Eat an argument (e.g., `x: 1` or `y`).
    pub fn eat_argument(&mut self) -> ParseResult<Argument> {
        if self.peek_next_token(TokenType::Identifier).is_ok()
            && self.peek_next_next_token(TokenType::Colon).is_ok()
        {
            // named argument
            let name = self.eat_identifier()?;
            self.eat_colon()?;
            let value = self.eat_expression()?;
            Ok(Argument {
                name: Some(name),
                value,
            })
        } else {
            // positional argument
            let value = self.eat_expression()?;
            Ok(Argument { name: None, value })
        }
    }
}
