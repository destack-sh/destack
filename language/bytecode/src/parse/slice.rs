use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one slice operation.
    pub(super) fn parse_slice_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let results = self.parse_definitions(Opcode::SLICE_VIEW)?;

        match name {
            "slice.view" => self.parse_slice_view(&results, function),
            _ => Err(ParseError::new("unknown slice operation", token.span)),
        }
    }

    /// Parse one contiguous slice subview.
    fn parse_slice_view(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let source = self.parse_register_span()?;

        // parse the physical element stride and dynamic subrange
        self.eat_token(TokenType::Comma)?;
        let stride = self.parse_u32()?;
        self.eat_token(TokenType::Comma)?;
        let start = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let length = self.parse_register()?;
        // encode the source fat value and scalar subrange
        let mut instruction = InstructionBuilder::new(Opcode::SLICE_VIEW);
        instruction.span(source);
        instruction.u32(stride);
        instruction.register(start);
        instruction.register(length);

        function.emit(instruction, results, self.empty_span())
    }
}
