use crate::{
    Address, InstructionBuilder, MemoryOperation, Opcode, ParseError, ParseResult, Parser,
    Prefetch, RegisterId, Scalar, Token, TokenType, Transfer,
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
            "memory.load" => self.parse_memory_range(MemoryOperation::Load, false, token, function),
            "memory.store" => {
                self.parse_memory_range(MemoryOperation::Store, false, token, function)
            }
            "memory.load.volatile" => {
                self.parse_memory_range(MemoryOperation::Load, true, token, function)
            }
            "memory.store.volatile" => {
                self.parse_memory_range(MemoryOperation::Store, true, token, function)
            }
            "memory.copy" => self.parse_transfer(Transfer::Copy, token, function),
            "memory.move" => self.parse_transfer(Transfer::Move, token, function),
            "memory.fill" => self.parse_fill(token, function),
            "memory.compare" => self.parse_compare(token, function),
            "prefetch.read" => self.parse_prefetch(Prefetch::Read, token, function),
            "prefetch.write" => self.parse_prefetch(Prefetch::Write, token, function),
            _ => Err(ParseError::new("unknown memory operation", token.span)),
        }
    }

    /// Parse one scalar memory operation.
    pub(super) fn parse_scalar_memory(
        &mut self,
        scalar: Scalar,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let (operation, name) = if let Some(name) = name.strip_prefix("load") {
            (MemoryOperation::Load, name)
        } else if let Some(name) = name.strip_prefix("store") {
            (MemoryOperation::Store, name)
        } else {
            return Err(ParseError::new(
                "unknown scalar memory operation",
                token.span,
            ));
        };
        let is_volatile = match name {
            "" => false,
            ".volatile" => true,
            _ => {
                return Err(ParseError::new(
                    "unknown scalar memory operation",
                    token.span,
                ));
            }
        };

        self.parse_memory(operation, scalar, is_volatile, token, function)
    }

    /// Parse one packed value load or store.
    fn parse_memory_range(
        &mut self,
        operation: MemoryOperation,
        is_volatile: bool,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        if operation == MemoryOperation::Load {
            let results = self.parse_results(1, true)?;
            let (address, register) = self.parse_address_register()?;
            self.eat_token(TokenType::Comma)?;
            let byte_len = self.parse_u32()?;
            let opcode = Opcode::memory_range(MemoryOperation::Load, address, is_volatile);
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.register(register);
            instruction.u32(byte_len);

            function.emit(instruction, &results, token.span)
        } else {
            let (address, register) = self.parse_address_register()?;
            self.eat_token(TokenType::Comma)?;
            let value = self.parse_register_span()?;
            self.eat_token(TokenType::Comma)?;
            let byte_len = self.parse_u32()?;
            let opcode = Opcode::memory_range(MemoryOperation::Store, address, is_volatile);
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.register(register);
            instruction.span(value);
            instruction.u32(byte_len);

            function.emit(instruction, &[], token.span)
        }
    }

    /// Parse one scalar load or store.
    fn parse_memory(
        &mut self,
        operation: MemoryOperation,
        scalar: Scalar,
        is_volatile: bool,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        if operation == MemoryOperation::Load {
            let results = self.parse_results(1, true)?;
            let (address, register) = self.parse_address_register()?;
            let opcode = Opcode::memory(operation, address, scalar, is_volatile);
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.register(register);

            function.emit(instruction, &results, token.span)
        } else {
            let (address, register) = self.parse_address_register()?;
            self.eat_token(TokenType::Comma)?;
            let value = self.parse_register()?;
            let opcode = Opcode::memory(operation, address, scalar, is_volatile);
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.register(register);
            instruction.register(value);

            function.emit(instruction, &[], token.span)
        }
    }

    /// Parse one byte copy or move.
    fn parse_transfer(
        &mut self,
        operation: Transfer,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let (target_address, target) = self.parse_address_register()?;
        self.eat_token(TokenType::Comma)?;
        let (source_address, source) = self.parse_address_register()?;
        self.eat_token(TokenType::Comma)?;
        let length = self.parse_length()?;
        let is_immediate = matches!(length, Length::Immediate(_));
        let opcode = Opcode::transfer(operation, target_address, source_address, is_immediate);

        // encode target then source for direct execution
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(target);
        instruction.register(source);
        length.encode(&mut instruction);

        function.emit(instruction, &[], token.span)
    }

    /// Parse one byte fill.
    fn parse_fill(&mut self, token: Token, function: &mut FunctionParser) -> ParseResult<()> {
        let (address, target) = self.parse_address_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let length = self.parse_length()?;
        let is_immediate = matches!(length, Length::Immediate(_));
        let opcode = Opcode::fill(address, is_immediate);

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
        let (left_address, left) = self.parse_address_register()?;
        self.eat_token(TokenType::Comma)?;
        let (right_address, right) = self.parse_address_register()?;
        self.eat_token(TokenType::Comma)?;
        let length = self.parse_length()?;
        let is_immediate = matches!(length, Length::Immediate(_));
        let opcode = Opcode::compare(left_address, right_address, is_immediate);

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
        operation: Prefetch,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let (address, register) = self.parse_address_register()?;
        let opcode = Opcode::prefetch(operation, address);
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(register);

        function.emit(instruction, &[], token.span)
    }

    /// Parse one relative reference or process-local pointer register.
    pub(super) fn parse_address_register(&mut self) -> ParseResult<(Address, RegisterId)> {
        let address = if self.eat_name_if("pointer") {
            Address::Pointer
        } else {
            Address::Reference
        };
        let register = self.parse_register()?;

        Ok((address, register))
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
