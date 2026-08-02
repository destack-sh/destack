use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RelocationTag, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one execution context operation.
    pub(super) fn parse_context_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "context.current" => Opcode::CONTEXT_CURRENT,
            "context.replace" => Opcode::CONTEXT_REPLACE,
            "context.bind" => Opcode::CONTEXT_BIND,
            "context.get" => Opcode::CONTEXT_GET,
            _ => return Err(ParseError::new("unknown context operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;
        let mut instruction = InstructionBuilder::new(opcode);

        // parse the operation-specific executable operands
        match opcode {
            Opcode::CONTEXT_CURRENT => {}
            Opcode::CONTEXT_REPLACE => instruction.register(self.parse_register()?),
            Opcode::CONTEXT_BIND => self.parse_context_bind(&mut instruction)?,
            Opcode::CONTEXT_GET => self.parse_context_get(&mut instruction)?,
            _ => return Err(ParseError::new("invalid context operation", token.span)),
        }

        function.emit(instruction, &results, self.empty_span())
    }

    /// Parse one context extension.
    fn parse_context_bind(&mut self, instruction: &mut InstructionBuilder) -> ParseResult<()> {
        self.parse_context_value(instruction)?;
        self.eat_token(TokenType::Comma)?;
        let allocation = self.parse_allocation_id()?;
        instruction.relocation(RelocationTag::ALLOCATION, allocation);
        self.eat_token(TokenType::Comma)?;
        instruction.u32(self.parse_u32()?);

        Ok(())
    }

    /// Parse one context value lookup.
    fn parse_context_get(&mut self, instruction: &mut InstructionBuilder) -> ParseResult<()> {
        self.parse_context_value(instruction)?;
        self.eat_token(TokenType::Comma)?;
        instruction.u32(self.parse_u32()?);

        Ok(())
    }

    /// Parse one context, variable, and value operand group.
    fn parse_context_value(&mut self, instruction: &mut InstructionBuilder) -> ParseResult<()> {
        instruction.register(self.parse_register()?);
        self.eat_token(TokenType::Comma)?;
        instruction.register(self.parse_register()?);
        self.eat_token(TokenType::Comma)?;
        instruction.span(self.parse_register_span()?);

        Ok(())
    }
}
