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
            "global.address" => self.parse_global_address(token, function),
            _ => Err(ParseError::new("unknown address operation", token.span)),
        }
    }

    /// Parse one address arithmetic operation.
    pub(super) fn parse_address_arithmetic(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "address.add" => {
                let results = self.parse_definitions(Opcode::ADDRESS_ADD)?;

                self.parse_address_add(token, &results, function)
            }
            "address.diff" => {
                let results = self.parse_definitions(Opcode::ADDRESS_DIFF)?;

                self.parse_address_diff(&results, function)
            }
            "address.pointer" => self.parse_address_rebase(Opcode::ADDRESS_POINTER, function),
            "address.reference" => self.parse_address_rebase(Opcode::ADDRESS_REFERENCE, function),
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
        let mut instruction = InstructionBuilder::new(Opcode::GLOBAL_ADDRESS);
        instruction.global(global);

        function.emit(instruction, &results, operation.span)
    }

    /// Parse one immediate, register, or scaled address addition.
    fn parse_address_add(
        &mut self,
        token: Token,
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
            let mut instruction = InstructionBuilder::new(Opcode::ADDRESS_ADD_IMMEDIATE);
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
        let opcode = match scale {
            None => Opcode::ADDRESS_ADD,
            Some(_) => Opcode::ADDRESS_ADD_SCALED,
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

    /// Parse one rebase between a world reference and a native pointer.
    fn parse_address_rebase(
        &mut self,
        opcode: Opcode,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let results = self.parse_definitions(opcode)?;
        let address = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(address);

        function.emit(instruction, &results, self.empty_span())
    }

    /// Parse one signed byte offset between two addresses.
    fn parse_address_diff(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let address = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let origin = self.parse_register()?;

        // encode the signed byte offset from the origin
        let mut instruction = InstructionBuilder::new(Opcode::ADDRESS_DIFF);
        instruction.register(address);
        instruction.register(origin);

        function.emit(instruction, results, self.empty_span())
    }
}
