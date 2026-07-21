use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterId, Scalar, Symbol, Token,
    TokenType, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one pointer operation.
    pub(super) fn parse_pointer_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "global.address" => self.parse_global_address(results, function),
            "frame.address" => self.parse_frame_address(results, function),
            "pointer.offset" => self.parse_pointer_offset(token, results, function),
            "pointer.index" => self.parse_pointer_index(token, results, function),
            "pointer.distance" => self.parse_pointer_distance(token, results, function),
            "reference.pointer" => self.parse_reference_pointer(token, results, function),
            _ => Err(ParseError::new("unknown pointer operation", token.span)),
        }
    }

    /// Parse one stable heap reference projection.
    fn parse_reference_pointer(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let reference = self.parse_register()?;
        let reference_type = function
            .value_type(reference)
            .filter(|ty| ty.is_initialized_reference())
            .and_then(ValueType::reference_type)
            .ok_or_else(|| {
                ParseError::new(
                    "reference pointer requires an initialized reference",
                    token.span,
                )
            })?;

        // retain the space required to resolve the stable offset
        let mut instruction = InstructionBuilder::new(Opcode::REFERENCE_POINTER);
        instruction.register(reference);
        instruction.reference(reference_type.kind(), reference_type.space());

        function.emit(
            instruction,
            results,
            &[ValueType::pointer()],
            self.empty_span(),
        )
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
            &[ValueType::pointer()],
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
        if !function.contains_frame_slot(index) {
            return Err(ParseError::new("unknown frame slot", slot.span));
        }

        // encode the slot index
        let mut instruction = InstructionBuilder::new(Opcode::FRAME_ADDRESS);
        instruction.u32(index);

        function.emit(
            instruction,
            results,
            &[ValueType::pointer()],
            self.empty_span(),
        )
    }

    /// Parse one constant byte offset from a pointer.
    fn parse_pointer_offset(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the base address
        let pointer = self.parse_register()?;
        if !function.has_type(pointer, ValueType::pointer()) {
            return Err(ParseError::new(
                "pointer offset requires a native pointer",
                token.span,
            ));
        }

        // parse the signed byte displacement
        self.eat_token(TokenType::Comma)?;
        let offset = self.parse_i64()?;
        let offset = i32::try_from(offset)
            .map_err(|_| ParseError::new("pointer offset exceeds int32", token.span))?;

        // encode the address calculation
        let mut instruction = InstructionBuilder::new(Opcode::POINTER_OFFSET);
        instruction.register(pointer);
        instruction.i32(offset);

        function.emit(
            instruction,
            results,
            &[ValueType::pointer()],
            self.empty_span(),
        )
    }

    /// Parse one scaled pointer index.
    fn parse_pointer_index(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the base and element index
        let pointer = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let index = self.parse_register()?;
        if !function.has_type(pointer, ValueType::pointer())
            || !function.has_type(index, ValueType::scalar(Scalar::Uint64))
        {
            return Err(ParseError::new(
                "pointer index requires a native pointer and uint64 index",
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
        let mut instruction = InstructionBuilder::new(Opcode::POINTER_INDEX);
        instruction.register(pointer);
        instruction.register(index);
        instruction.u32(stride);

        function.emit(
            instruction,
            results,
            &[ValueType::pointer()],
            self.empty_span(),
        )
    }

    /// Parse one signed distance between two pointers.
    fn parse_pointer_distance(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse both addresses
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        if !function.has_type(left, ValueType::pointer())
            || !function.has_type(right, ValueType::pointer())
        {
            return Err(ParseError::new(
                "pointer distance requires two native pointers",
                token.span,
            ));
        }

        // encode the signed distance
        let mut instruction = InstructionBuilder::new(Opcode::POINTER_DISTANCE);
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
