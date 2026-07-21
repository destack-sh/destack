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
        let opcode = Opcode::from_name(name)
            .ok_or_else(|| ParseError::new("unknown profile operation", token.span))?;
        let mut instruction = InstructionBuilder::new(opcode);

        // increment one function-local counter
        if opcode == Opcode::PROFILE_INCREMENT {
            self.eat_name("counter")?;
            self.eat_token(TokenType::OpenParenthesis)?;
            let counter = CounterId(self.parse_u32()?);
            self.eat_token(TokenType::CloseParenthesis)?;
            instruction.counter(counter);
        }
        // sample one function-local sampler
        else {
            self.eat_name("sampler")?;
            self.eat_token(TokenType::OpenParenthesis)?;
            let sampler = SamplerId(self.parse_u32()?);
            self.eat_token(TokenType::CloseParenthesis)?;
            instruction.sampler(sampler);
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
