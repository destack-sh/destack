use crate::{
    CounterId, InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterId, SamplerId,
    Token, TokenType, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one explicit profile operation.
    pub(super) fn parse_profile_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "profile.increment" => Opcode::PROFILE_INCREMENT,
            "profile.sample" => Opcode::PROFILE_SAMPLE,
            _ => return Err(ParseError::new("unknown profile operation", token.span)),
        };
        let mut instruction = InstructionBuilder::new(opcode);

        // increment one function-local counter
        if opcode == Opcode::PROFILE_INCREMENT {
            self.eat_name("counter")?;
            self.eat_token(TokenType::OpenParenthesis)?;
            let counter = CounterId(self.parse_u32()?);
            self.eat_token(TokenType::CloseParenthesis)?;
            instruction
                .counter(counter)
                .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        }
        // sample one function-local sampler
        else {
            self.eat_name("sampler")?;
            self.eat_token(TokenType::OpenParenthesis)?;
            let sampler = SamplerId(self.parse_u32()?);
            self.eat_token(TokenType::CloseParenthesis)?;
            instruction
                .sampler(sampler)
                .map_err(|error| ParseError::new(error.to_string(), token.span))?;
            self.eat_token(TokenType::Comma)?;
            let value = self.parse_register()?;
            let is_profile_value = function
                .value_type(value)
                .is_some_and(ValueType::is_profile_value);
            if !is_profile_value {
                return Err(ParseError::new(
                    "profile sample requires one runtime word value",
                    token.span,
                ));
            }
            instruction.register(value);
        }

        function.emit(instruction, results, &[], self.empty_span())
    }
}
