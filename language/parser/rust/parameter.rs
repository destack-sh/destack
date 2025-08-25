use crate::{Parameter, ParseResult, Parser};
use destack_language_lexer::TokenType;

impl<'a> Parser<'a> {
    /// Eat a parameter (e.g., `x: int32` or `Validate: bool = false`).
    pub fn eat_parameter(&mut self) -> ParseResult<Parameter> {
        // name: type
        let name = self.eat_identifier()?;
        self.eat_colon()?;
        let r#type = self.eat_type()?;

        // default value
        if self.peek_next_token(TokenType::Assign).is_ok() {
            // has default value
            self.eat_token(TokenType::Assign)?;
            let value = self.eat_expression()?;
            Ok(Parameter {
                name,
                r#type,
                default: Some(value),
            })
        } else {
            // no default value
            Ok(Parameter {
                name,
                r#type,
                default: None,
            })
        }
    }
}
