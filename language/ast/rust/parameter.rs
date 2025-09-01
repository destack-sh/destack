use crate::{Parameter, NodeId, ParseResult, Parser};
use destack_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat a parameter
    ///
    /// Examples:
    /// ```
    /// x: int32
    /// Validate: bool = false
    /// ```
    pub fn eat_parameter(&mut self) -> ParseResult<NodeId<Parameter>> {
        let start = self.mark();

        // name: type
        let name = self.eat_identifier()?;
        self.eat_colon()?;
        let r#type = self.eat_type()?;

        // default value
        if self.peek_next_token(TokenType::Assign).is_ok() {
            // has default value
            self.eat_token(TokenType::Assign)?;
            let value = self.eat_expression()?;
            let parameter_id = self.tree.allocate_from_mark(Parameter {
                name,
                r#type,
                default: Some(value),
            }, start);
            Ok(parameter_id)
        } else {
            // no default value
            let parameter_id = self.tree.allocate_from_mark(Parameter {
                name,
                r#type,
                default: None,
            }, start);
            Ok(parameter_id)
        }
    }

    /// Eat a parameter list.
    ///
    /// Examples:
    /// ```
    /// x: int32
    /// x: int32, y: int32
    /// ```
    pub fn eat_parameters_body(&mut self) -> ParseResult<Vec<NodeId<Parameter>>> {
        let mut parameters: Vec<NodeId<Parameter>> = Vec::new();
        loop {
            let parameter = self.eat_parameter()?;
            parameters.push(parameter);
            if self.peek_next_token(TokenType::Comma).is_ok() {
                self.eat_token(TokenType::Comma)?;
            } else {
                break;
            }
        }
        Ok(parameters)
    }
}

#[cfg(test)]
mod tests {}
