use crate::{
    InstructionBuilder, New, NewKind, Opcode, ParseError, ParseResult, Parser, RelocationTag,
    Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one `new` instruction.
    pub(super) fn parse_new(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let operation = New::from_name(name)
            .ok_or_else(|| ParseError::new("invalid new operation", token.span))?;
        let opcode = Opcode::new(operation)
            .ok_or_else(|| ParseError::new("invalid new operation", token.span))?;
        let results = self.parse_definitions(opcode)?;

        // encode the direct allocation site identity
        let allocation = self.parse_allocation_id()?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.relocation(RelocationTag::ALLOCATION, allocation);

        // encode the variable slice length
        if operation.kind == NewKind::Slice {
            self.eat_token(TokenType::Comma)?;
            let length = self.parse_register()?;
            instruction.register(length);
        }

        // encode explicit success and failure edges
        if operation.is_fallible {
            self.eat_token(TokenType::FatArrow)?;
            instruction.branch(self.parse_label()?);
            self.eat_token(TokenType::Comma)?;
            instruction.branch(self.parse_label()?);
        }

        function.emit(instruction, &results, self.empty_span())
    }
}
