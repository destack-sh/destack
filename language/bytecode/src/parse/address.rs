use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, RelocationTag,
    Token, TokenType,
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
        let opcode = match name {
            "frame.address" => Opcode::FRAME_ADDRESS,
            "global.address" => Opcode::GLOBAL_ADDRESS,
            _ => return Err(ParseError::new("unknown address operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match opcode {
            Opcode::FRAME_ADDRESS => self.parse_frame_address(&results, function),
            Opcode::GLOBAL_ADDRESS => self.parse_global_address(&results, function),
            _ => Err(ParseError::new("invalid address operation", token.span)),
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
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let token = self.eat_token(TokenType::Identifier)?;
        let global = self
            .text(token)
            .strip_prefix('g')
            .and_then(|index| index.parse::<u32>().ok())
            .ok_or_else(|| ParseError::new("expected global id", token.span))?;
        let mut instruction = InstructionBuilder::new(Opcode::GLOBAL_ADDRESS);
        instruction.relocation(RelocationTag::GLOBAL, global);

        function.emit(instruction, results, self.empty_span())
    }
}
