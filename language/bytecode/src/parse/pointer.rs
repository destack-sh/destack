use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, RelocationTag,
    Token, TokenType,
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
        let opcode = match name {
            "pointer.frame" => Opcode::POINTER_FRAME,
            "pointer.global" => Opcode::POINTER_GLOBAL,
            "pointer.local" => Opcode::POINTER_LOCAL,
            "pointer.shared" => Opcode::POINTER_SHARED,
            "pointer.add" => Opcode::POINTER_ADD,
            "pointer.byteOffsetFrom" => Opcode::POINTER_BYTE_OFFSET_FROM,
            _ => return Err(ParseError::new("unknown pointer operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match name {
            "pointer.frame" => self.parse_pointer_frame(&results, function),
            "pointer.global" => self.parse_pointer_global(&results, function),
            "pointer.local" | "pointer.shared" => {
                self.parse_pointer_reference(opcode, &results, function)
            }
            "pointer.add" => self.parse_pointer_add(token, &results, function),
            "pointer.byteOffsetFrom" => self.parse_pointer_byte_offset_from(&results, function),
            _ => Err(ParseError::new("invalid pointer operation", token.span)),
        }
    }

    /// Parse one stable reference pointer materialization.
    fn parse_pointer_reference(
        &mut self,
        opcode: Opcode,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let reference = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(reference);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one linked global pointer materialization.
    fn parse_pointer_global(
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

        // encode the linked global identity
        let mut instruction = InstructionBuilder::new(Opcode::POINTER_GLOBAL);
        instruction.relocation(RelocationTag::GLOBAL, global);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one frame pointer materialization.
    fn parse_pointer_frame(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register_span()?;
        let mut instruction = InstructionBuilder::new(Opcode::POINTER_FRAME);
        instruction.span(value);

        function.emit(instruction, results, self.empty_span())
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
    fn parse_pointer_byte_offset_from(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let origin = self.parse_register()?;

        // encode the signed byte offset from the origin
        let mut instruction = InstructionBuilder::new(Opcode::POINTER_BYTE_OFFSET_FROM);
        instruction.register(pointer);
        instruction.register(origin);

        function.emit(instruction, results, self.empty_span())
    }
}
