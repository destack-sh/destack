use crate::{
    InstructionBuilder, MemoryOperation, Opcode, ParseError, ParseResult, Parser, RegisterId,
    Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one memory operation.
    pub(super) fn parse_memory_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "load" => self.parse_load(token, function),
            "store" => self.parse_store(token, function),
            "memory.copy" => self.parse_transfer(false, token, function),
            "memory.move" => self.parse_transfer(true, token, function),
            "memory.fill" => self.parse_fill(token, function),
            "memory.compare" => self.parse_compare(token, function),
            "prefetch.read" => self.parse_prefetch(Opcode::PREFETCH_READ, token, function),
            "prefetch.write" => self.parse_prefetch(Opcode::PREFETCH_WRITE, token, function),
            _ => Err(ParseError::new("unknown memory operation", token.span)),
        }
    }

    /// Parse one scalar or packed value load.
    fn parse_load(&mut self, token: Token, function: &mut FunctionParser) -> ParseResult<()> {
        let results = self.parse_results(1, true)?;
        let pointer = self.parse_register()?;

        // select a packed byte range or scalar representation
        if self.eat_token_if(TokenType::Comma) {
            let byte_len = self.parse_u32()?;
            let mut instruction = InstructionBuilder::new(Opcode::LOAD);
            instruction.register(pointer);
            instruction.u32(byte_len);

            function.emit(instruction, &results, token.span)
        } else {
            let scalar = self.parse_scalar_representation()?;
            let opcode = Opcode::memory(MemoryOperation::Load, scalar);
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.register(pointer);

            function.emit(instruction, &results, token.span)
        }
    }

    /// Parse one scalar or packed value store.
    fn parse_store(&mut self, token: Token, function: &mut FunctionParser) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register_span()?;

        // select a packed byte range or scalar representation
        if self.eat_token_if(TokenType::Comma) {
            let byte_len = self.parse_u32()?;
            let mut instruction = InstructionBuilder::new(Opcode::STORE);
            instruction.register(pointer);
            instruction.span(value);
            instruction.u32(byte_len);

            function.emit(instruction, &[], token.span)
        } else {
            let scalar = self.parse_scalar_representation()?;
            if value.word_count != 1 {
                return Err(ParseError::new(
                    "scalar store requires one value register",
                    token.span,
                ));
            }
            let opcode = Opcode::memory(MemoryOperation::Store, scalar);
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.register(pointer);
            instruction.register(value.start);

            function.emit(instruction, &[], token.span)
        }
    }

    /// Parse one byte copy or move.
    fn parse_transfer(
        &mut self,
        is_move: bool,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let target = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let source = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let length = self.parse_length()?;
        let opcode = match (is_move, length) {
            (false, Length::Register(_)) => Opcode::MEMORY_COPY,
            (true, Length::Register(_)) => Opcode::MEMORY_MOVE,
            (false, Length::Immediate(_)) => Opcode::MEMORY_COPY_IMMEDIATE,
            (true, Length::Immediate(_)) => Opcode::MEMORY_MOVE_IMMEDIATE,
        };

        // encode target then source for direct execution
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(target);
        instruction.register(source);
        length.encode(&mut instruction);

        function.emit(instruction, &[], token.span)
    }

    /// Parse one byte fill.
    fn parse_fill(&mut self, token: Token, function: &mut FunctionParser) -> ParseResult<()> {
        let target = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let length = self.parse_length()?;
        let opcode = if matches!(length, Length::Immediate(_)) {
            Opcode::MEMORY_FILL_IMMEDIATE
        } else {
            Opcode::MEMORY_FILL
        };

        // encode the complete fill range
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(target);
        instruction.register(byte);
        length.encode(&mut instruction);

        function.emit(instruction, &[], token.span)
    }

    /// Parse one byte comparison.
    fn parse_compare(&mut self, token: Token, function: &mut FunctionParser) -> ParseResult<()> {
        let results = self.parse_results(1, true)?;
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let length = self.parse_length()?;
        let opcode = if matches!(length, Length::Immediate(_)) {
            Opcode::MEMORY_COMPARE_IMMEDIATE
        } else {
            Opcode::MEMORY_COMPARE
        };

        // encode the signed comparison result
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(left);
        instruction.register(right);
        length.encode(&mut instruction);

        function.emit(instruction, &results, token.span)
    }

    /// Parse one register or immediate byte length.
    fn parse_length(&mut self) -> ParseResult<Length> {
        if self.peek_is(TokenType::Integer) {
            Ok(Length::Immediate(self.parse_u32()?))
        } else {
            Ok(Length::Register(self.parse_register()?))
        }
    }

    /// Parse one prefetch hint.
    fn parse_prefetch(
        &mut self,
        opcode: Opcode,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(pointer);

        function.emit(instruction, &[], token.span)
    }
}

/// One byte length encoded directly or read from a register.
#[derive(Clone, Copy, Debug)]
enum Length {
    /// One register containing the byte length.
    Register(RegisterId),
    /// One instruction-local byte length.
    Immediate(u32),
}

impl Length {
    /// Append this byte length to one instruction.
    fn encode(self, instruction: &mut InstructionBuilder) {
        match self {
            Self::Register(register) => instruction.register(register),
            Self::Immediate(length) => instruction.u32(length),
        }
    }
}
