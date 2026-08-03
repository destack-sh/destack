use crate::{
    CounterId, InstructionBuilder, Opcode, ParseError, ParseResult, Parser, SamplerId, Token,
    TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one explicit profile operation.
    pub(super) fn parse_profile_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "profile.increment" => Opcode::PROFILE_INCREMENT,
            "profile.sample" => Opcode::PROFILE_SAMPLE,
            _ => return Err(ParseError::new("unknown profile operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;
        let mut instruction = InstructionBuilder::new(opcode);

        // increment one function-local counter
        if opcode == Opcode::PROFILE_INCREMENT {
            let token = self.eat_token(TokenType::Identifier)?;
            let counter = self
                .text(token)
                .strip_prefix('c')
                .and_then(|index| index.parse().ok())
                .map(CounterId)
                .ok_or_else(|| ParseError::new("expected counter id", token.span))?;
            instruction.counter(counter);
        }
        // sample one function-local sampler
        else {
            let token = self.eat_token(TokenType::Identifier)?;
            let sampler = self
                .text(token)
                .strip_prefix('s')
                .and_then(|index| index.parse().ok())
                .map(SamplerId)
                .ok_or_else(|| ParseError::new("expected sampler id", token.span))?;
            instruction.sampler(sampler);
            self.eat_token(TokenType::Comma)?;
            let value = self.parse_register()?;
            instruction.register(value);
        }

        function.emit(instruction, &results, self.empty_span())
    }
}
