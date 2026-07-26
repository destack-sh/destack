use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, RelocationTag,
    Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one waiter operation.
    pub(super) fn parse_waiter_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "waiter.queue" => Opcode::WAITER_QUEUE,
            "waiter.cancel" => Opcode::WAITER_CANCEL,
            _ => return Err(ParseError::new("invalid waiter operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match opcode {
            Opcode::WAITER_QUEUE => self.parse_waiter_queue(&results, function),
            Opcode::WAITER_CANCEL => self.parse_waiter_cancel(&results, function),
            _ => unreachable!("waiter opcode selected above"),
        }
    }

    /// Parse one waiter queue operation.
    fn parse_waiter_queue(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let waiter = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let ty = self.parse_type_id()?;

        // encode the waiter, result value, and its Program type
        let mut instruction = InstructionBuilder::new(Opcode::WAITER_QUEUE);
        instruction.register(waiter);
        instruction.relocation(RelocationTag::TYPE, ty.0);
        instruction.span(value);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one waiter cancellation operation.
    fn parse_waiter_cancel(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let waiter = self.parse_register()?;

        // encode the consumed waiter
        let mut instruction = InstructionBuilder::new(Opcode::WAITER_CANCEL);
        instruction.register(waiter);

        function.emit(instruction, results, self.empty_span())
    }
}
