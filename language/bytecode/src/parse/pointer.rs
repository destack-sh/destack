use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one pointer operation.
    pub(super) fn parse_pointer_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "pointer.add" => {
                let results = self.parse_definitions(Opcode::POINTER_ADD)?;

                self.parse_pointer_add(token, &results, function)
            }
            "pointer.diff" => {
                let results = self.parse_definitions(Opcode::POINTER_DIFF)?;

                self.parse_pointer_diff(&results, function)
            }
            _ => Err(ParseError::new("unknown pointer operation", token.span)),
        }
    }

    /// Parse one immediate, register, or scaled pointer addition.
    fn parse_pointer_add(
        &mut self,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;

        // encode an immediate byte displacement
        if self.peek_is(TokenType::Integer) {
            let offset = self.parse_i64()?;
            let offset = i32::try_from(offset)
                .map_err(|_| ParseError::new("pointer offset exceeds int32", token.span))?;
            let mut instruction = InstructionBuilder::new(Opcode::POINTER_ADD_IMMEDIATE);
            instruction.register(pointer);
            instruction.i32(offset);

            return function.emit(instruction, results, self.empty_span());
        }

        // parse one signed register byte offset and optional scale
        let offset = self.parse_register()?;
        let scale = if self.eat_token_if(TokenType::Comma) {
            Some(self.parse_u32()?)
        } else {
            None
        };

        // encode the register address calculation
        let opcode = if scale.is_some() {
            Opcode::POINTER_ADD_SCALED
        } else {
            Opcode::POINTER_ADD
        };
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(pointer);
        instruction.register(offset);
        if let Some(scale) = scale {
            instruction.u32(scale);
        }

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one signed byte offset between two pointers.
    fn parse_pointer_diff(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let origin = self.parse_register()?;

        // encode the signed byte offset from the origin
        let mut instruction = InstructionBuilder::new(Opcode::POINTER_DIFF);
        instruction.register(pointer);
        instruction.register(origin);

        function.emit(instruction, results, self.empty_span())
    }
}
