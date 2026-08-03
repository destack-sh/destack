use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one stable frame or global address.
    pub(super) fn parse_address(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "frame.address" => {
                let results = self.parse_definitions(Opcode::FRAME_ADDRESS)?;

                self.parse_frame_address(&results, function)
            }
            "global.address.constant" => {
                self.parse_global_address(Opcode::GLOBAL_ADDRESS_CONSTANT, token, function)
            }
            "global.address.local" => {
                self.parse_global_address(Opcode::GLOBAL_ADDRESS_LOCAL, token, function)
            }
            "global.address.shared" => {
                self.parse_global_address(Opcode::GLOBAL_ADDRESS_SHARED, token, function)
            }
            _ => Err(ParseError::new("unknown address operation", token.span)),
        }
    }

    /// Parse one stable frame address.
    fn parse_frame_address(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register_span()?;
        let mut instruction = InstructionBuilder::new(Opcode::FRAME_ADDRESS);
        instruction.span(value);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one stable global address.
    fn parse_global_address(
        &mut self,
        opcode: Opcode,
        operation: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let results = self.parse_results(1, true)?;
        let token = self.eat_token(TokenType::Identifier)?;
        let global = self
            .text(token)
            .strip_prefix('g')
            .and_then(|index| index.parse::<u32>().ok())
            .ok_or_else(|| ParseError::new("expected global id", token.span))?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.global(global);

        function.emit(instruction, &results, operation.span)
    }
}
