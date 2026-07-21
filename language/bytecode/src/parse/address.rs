use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterId, Scalar, Symbol, Token,
    TokenType, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one address operation.
    pub(super) fn parse_address_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match Opcode::from_name(name) {
            Some(Opcode::GLOBAL_ADDRESS) => self.parse_global_address(results, function),
            Some(Opcode::FRAME_ADDRESS) => self.parse_frame_address(results, function),
            Some(Opcode::ADDRESS_OFFSET) => self.parse_address_offset(token, results, function),
            Some(Opcode::ADDRESS_ELEMENT) => self.parse_address_element(token, results, function),
            Some(Opcode::ADDRESS_DISTANCE) => self.parse_address_distance(token, results, function),
            _ => Err(ParseError::new("unknown address operation", token.span)),
        }
    }

    /// Parse one linked global address.
    fn parse_global_address(
        &mut self,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // resolve the linked global
        let name = self.eat_token(TokenType::Identifier)?;
        let global = self
            .symbols
            .globals
            .get(self.text(name))
            .copied()
            .ok_or_else(|| ParseError::new("unknown global", name.span))?;

        // encode the global relocation
        let mut instruction = InstructionBuilder::new(Opcode::GLOBAL_ADDRESS);
        instruction.symbol(Symbol::global(global.0));

        function.emit(
            instruction,
            results,
            &[ValueType::address()],
            self.empty_span(),
        )
    }

    /// Parse one frame-slot address.
    fn parse_frame_address(
        &mut self,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the dense frame slot
        let slot = self.eat_token(TokenType::Identifier)?;
        let index = self
            .text(slot)
            .strip_prefix('s')
            .ok_or_else(|| ParseError::new("expected frame slot", slot.span))?;
        let index = index
            .parse::<u32>()
            .map_err(|_| ParseError::new("expected frame slot", slot.span))?;
        if index >= function.frame_slot_count {
            return Err(ParseError::new("unknown frame slot", slot.span));
        }

        // encode the slot index
        let mut instruction = InstructionBuilder::new(Opcode::FRAME_ADDRESS);
        instruction.u32(index);

        function.emit(
            instruction,
            results,
            &[ValueType::address()],
            self.empty_span(),
        )
    }

    /// Parse one constant byte offset from an address.
    fn parse_address_offset(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the base address
        let address = self.parse_register()?;
        if !function.has_type(address, ValueType::address()) {
            return Err(ParseError::new(
                "address offset requires a native address",
                token.span,
            ));
        }

        // parse the signed byte displacement
        self.eat_token(TokenType::Comma)?;
        let offset = self.parse_i64()?;
        let offset = i32::try_from(offset)
            .map_err(|_| ParseError::new("address offset exceeds int32", token.span))?;

        // encode the address calculation
        let mut instruction = InstructionBuilder::new(Opcode::ADDRESS_OFFSET);
        instruction.register(address);
        instruction.i32(offset);

        function.emit(
            instruction,
            results,
            &[ValueType::address()],
            self.empty_span(),
        )
    }

    /// Parse one scaled element address.
    fn parse_address_element(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the base and element index
        let address = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let index = self.parse_register()?;
        if !function.has_type(address, ValueType::address())
            || !function.has_type(index, ValueType::scalar(Scalar::Uint64))
        {
            return Err(ParseError::new(
                "element address requires a native address and uint64 index",
                token.span,
            ));
        }

        // parse the static element stride
        self.eat_token(TokenType::Comma)?;
        self.eat_name("stride")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let stride = self.parse_u32()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        // encode the scaled address calculation
        let mut instruction = InstructionBuilder::new(Opcode::ADDRESS_ELEMENT);
        instruction.register(address);
        instruction.register(index);
        instruction.u32(stride);

        function.emit(
            instruction,
            results,
            &[ValueType::address()],
            self.empty_span(),
        )
    }

    /// Parse one signed distance between two addresses.
    fn parse_address_distance(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse both addresses
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        if !function.has_type(left, ValueType::address())
            || !function.has_type(right, ValueType::address())
        {
            return Err(ParseError::new(
                "address distance requires two native addresses",
                token.span,
            ));
        }

        // encode the signed distance
        let mut instruction = InstructionBuilder::new(Opcode::ADDRESS_DISTANCE);
        instruction.register(left);
        instruction.register(right);

        function.emit(
            instruction,
            results,
            &[ValueType::scalar(Scalar::Int64)],
            self.empty_span(),
        )
    }
}
