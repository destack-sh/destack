use crate::{Argument, NodeId, ParseResult, Parser};
use destack_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat an argument (e.g., `x: 1` or `y`).
    pub fn eat_argument(&mut self) -> ParseResult<NodeId<Argument>> {
        let start = self.mark();
        // named argument
        if self.peek_next_token(TokenType::Identifier).is_ok()
            && self.peek_next_next_token(TokenType::Colon).is_ok()
        {
            let name = self.eat_identifier()?;
            self.eat_colon()?;
            let value = self.eat_expression()?;
            let argument_id = self.tree.allocate(
                Argument {
                    name: Some(name),
                    value,
                },
                self.span_from(start),
            );
            Ok(argument_id)
        }
        // positional argument
        else {
            let value = self.eat_expression()?;
            let argument_id = self
                .tree
                .allocate(Argument { name: None, value }, self.span_from(start));
            Ok(argument_id)
        }
    }

    /// Eat an argument list (e.g., `x: 1, y: 2`).
    pub fn eat_arguments_body(&mut self) -> ParseResult<Vec<NodeId<Argument>>> {
        let mut arguments: Vec<NodeId<Argument>> = Vec::new();
        loop {
            let argument_id = self.eat_argument()?;
            arguments.push(argument_id);
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
