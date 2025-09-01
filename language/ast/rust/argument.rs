use crate::{ArgumentNode, ParseResult, Parser};
use destack_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat an argument (e.g., `x: 1` or `y`).
    pub fn eat_argument(&mut self) -> ParseResult<ArgumentNode> {
        if self.peek_next_token(TokenType::Identifier).is_ok()
            && self.peek_next_next_token(TokenType::Colon).is_ok()
        {
            // named argument
            let name = self.eat_identifier()?;
            self.eat_colon()?;
            let value = self.eat_expression()?;
            Ok(ArgumentNode {
                name: Some(name),
                value,
            })
        } else {
            // positional argument
            let value = self.eat_expression()?;
            Ok(ArgumentNode { name: None, value })
        }
    }

    /// Eat an argument list (e.g., `x: 1, y: 2`).
    pub fn eat_arguments_body(&mut self) -> ParseResult<Vec<ArgumentNode>> {
        let mut arguments: Vec<ArgumentNode> = Vec::new();
        loop {
            let argument = self.eat_argument()?;
            arguments.push(argument);
            if self.peek_next_token(TokenType::Comma).is_ok() {
                self.eat_token(TokenType::Comma)?;
            } else {
                break;
            }
        }
        Ok(arguments)
    }
}

#[cfg(test)]
mod tests {}
