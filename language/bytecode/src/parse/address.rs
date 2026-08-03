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

    /// Parse one relative reference or native pointer arithmetic operation.
    pub(super) fn parse_address_arithmetic(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let is_reference = name.starts_with("reference.");
        match name {
            "reference.add" | "pointer.add" => {
                let opcode = if is_reference {
                    Opcode::REFERENCE_ADD
                } else {
                    Opcode::POINTER_ADD
                };
                let results = self.parse_definitions(opcode)?;

                self.parse_address_add(token, is_reference, &results, function)
            }
            "reference.diff" | "pointer.diff" => {
                let opcode = if is_reference {
                    Opcode::REFERENCE_DIFF
                } else {
                    Opcode::POINTER_DIFF
                };
                let results = self.parse_definitions(opcode)?;

                self.parse_address_diff(opcode, &results, function)
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

    /// Parse one immediate, register, or scaled address addition.
    fn parse_address_add(
        &mut self,
        token: Token,
        is_reference: bool,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let address = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;

        // encode an immediate byte displacement
        if self.peek_is(TokenType::Integer) {
            let offset = self.parse_i64()?;
            let offset = i32::try_from(offset)
                .map_err(|_| ParseError::new("address offset exceeds int32", token.span))?;
            let opcode = if is_reference {
                Opcode::REFERENCE_ADD_IMMEDIATE
            } else {
                Opcode::POINTER_ADD_IMMEDIATE
            };
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.register(address);
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
        let opcode = match (is_reference, scale.is_some()) {
            (true, false) => Opcode::REFERENCE_ADD,
            (true, true) => Opcode::REFERENCE_ADD_SCALED,
            (false, false) => Opcode::POINTER_ADD,
            (false, true) => Opcode::POINTER_ADD_SCALED,
        };

        // encode the register address calculation
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(address);
        instruction.register(offset);
        if let Some(scale) = scale {
            instruction.u32(scale);
        }

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one signed byte offset between two addresses.
    fn parse_address_diff(
        &mut self,
        opcode: Opcode,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let address = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let origin = self.parse_register()?;

        // encode the signed byte offset from the origin
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(address);
        instruction.register(origin);

        function.emit(instruction, results, self.empty_span())
    }
}
