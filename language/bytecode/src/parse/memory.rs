use crate::{
    Address, InstructionBuilder, MemoryOperation, Opcode, ParseError, ParseResult, Parser,
    Prefetch, RegisterId, Token, TokenType, Transfer,
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
            "load" => self.parse_load(Address::Memory, false, token, function),
            "load.constant" => self.parse_load(Address::Constant, false, token, function),
            "load.pointer" => self.parse_load(Address::Pointer, false, token, function),
            "store" => self.parse_store(Address::Memory, false, token, function),
            "store.pointer" => self.parse_store(Address::Pointer, false, token, function),
            "load.volatile" => self.parse_load(Address::Memory, true, token, function),
            "load.volatile.pointer" => self.parse_load(Address::Pointer, true, token, function),
            "store.volatile" => self.parse_store(Address::Memory, true, token, function),
            "store.volatile.pointer" => self.parse_store(Address::Pointer, true, token, function),
            _ if name.starts_with("memory.copy") => {
                self.parse_transfer(name, Transfer::Copy, token, function)
            }
            _ if name.starts_with("memory.move") => {
                self.parse_transfer(name, Transfer::Move, token, function)
            }
            _ if name.starts_with("memory.fill") => self.parse_fill(name, token, function),
            _ if name.starts_with("memory.compare") => self.parse_compare(name, token, function),
            _ if name.starts_with("prefetch.read") => {
                self.parse_prefetch(name, Prefetch::Read, token, function)
            }
            _ if name.starts_with("prefetch.write") => {
                self.parse_prefetch(name, Prefetch::Write, token, function)
            }
            _ => Err(ParseError::new("unknown memory operation", token.span)),
        }
    }

    /// Parse one scalar or packed value load.
    fn parse_load(
        &mut self,
        address: Address,
        is_volatile: bool,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let results = self.parse_results(1, true)?;
        let pointer = self.parse_register()?;

        // select a packed byte range or scalar representation
        if self.eat_token_if(TokenType::Comma) {
            let byte_len = self.parse_u32()?;
            let opcode = Opcode::memory_range(MemoryOperation::Load, address, is_volatile)
                .ok_or_else(|| ParseError::new("invalid load address", token.span))?;
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.register(pointer);
            instruction.u32(byte_len);

            function.emit(instruction, &results, token.span)
        } else {
            let scalar = self.parse_scalar_representation()?;
            let opcode = Opcode::memory(MemoryOperation::Load, address, scalar, is_volatile)
                .ok_or_else(|| ParseError::new("invalid load address", token.span))?;
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.register(pointer);

            function.emit(instruction, &results, token.span)
        }
    }

    /// Parse one scalar or packed value store.
    fn parse_store(
        &mut self,
        address: Address,
        is_volatile: bool,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register_span()?;

        // select a packed byte range or scalar representation
        if self.eat_token_if(TokenType::Comma) {
            let byte_len = self.parse_u32()?;
            let opcode = Opcode::memory_range(MemoryOperation::Store, address, is_volatile)
                .ok_or_else(|| ParseError::new("invalid store address", token.span))?;
            let mut instruction = InstructionBuilder::new(opcode);
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
            let opcode = Opcode::memory(MemoryOperation::Store, address, scalar, is_volatile)
                .ok_or_else(|| ParseError::new("invalid store address", token.span))?;
            let mut instruction = InstructionBuilder::new(opcode);
            instruction.register(pointer);
            instruction.register(value.start);

            function.emit(instruction, &[], token.span)
        }
    }

    /// Parse one byte copy or move.
    fn parse_transfer(
        &mut self,
        name: &str,
        operation: Transfer,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let prefix = format!("memory.{}", operation.name());
        let (target_address, source_address) = self.parse_address_pair(name, &prefix, token)?;
        let target = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let source = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let length = self.parse_length()?;
        let is_immediate = matches!(length, Length::Immediate(_));
        let opcode = Opcode::transfer(operation, target_address, source_address, is_immediate)
            .ok_or_else(|| ParseError::new("invalid transfer address", token.span))?;

        // encode target then source for direct execution
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(target);
        instruction.register(source);
        length.encode(&mut instruction);

        function.emit(instruction, &[], token.span)
    }

    /// Parse one byte fill.
    fn parse_fill(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let address = self.parse_address_suffix(name, "memory.fill", token)?;
        let target = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let length = self.parse_length()?;
        let is_immediate = matches!(length, Length::Immediate(_));
        let opcode = Opcode::fill(address, is_immediate)
            .ok_or_else(|| ParseError::new("invalid fill address", token.span))?;

        // encode the complete fill range
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(target);
        instruction.register(byte);
        length.encode(&mut instruction);

        function.emit(instruction, &[], token.span)
    }

    /// Parse one byte comparison.
    fn parse_compare(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let (left_address, right_address) =
            self.parse_address_pair(name, "memory.compare", token)?;
        let results = self.parse_results(1, true)?;
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
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
        name: &str,
        operation: Prefetch,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let prefix = format!("prefetch.{}", operation.name());
        let address = self.parse_address_suffix(name, &prefix, token)?;
        let opcode = Opcode::prefetch(operation, address)
            .ok_or_else(|| ParseError::new("invalid prefetch address", token.span))?;
        let pointer = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(pointer);

        function.emit(instruction, &[], token.span)
    }

    /// Parse one optional memory address suffix.
    fn parse_address_suffix(&self, name: &str, prefix: &str, token: Token) -> ParseResult<Address> {
        if name == prefix {
            return Ok(Address::Memory);
        }

        let name = name
            .strip_prefix(prefix)
            .and_then(|name| name.strip_prefix('.'))
            .ok_or_else(|| ParseError::new("invalid memory operation", token.span))?;

        Address::from_name(name)
            .ok_or_else(|| ParseError::new("expected memory address", token.span))
    }

    /// Parse one optional pair of memory address suffixes.
    fn parse_address_pair(
        &self,
        name: &str,
        prefix: &str,
        token: Token,
    ) -> ParseResult<(Address, Address)> {
        if name == prefix {
            return Ok((Address::Memory, Address::Memory));
        }

        let name = name
            .strip_prefix(prefix)
            .and_then(|name| name.strip_prefix('.'))
            .ok_or_else(|| ParseError::new("invalid memory operation", token.span))?;
        let (first, second) = name
            .split_once('.')
            .ok_or_else(|| ParseError::new("expected two memory addresses", token.span))?;
        let first = Address::from_name(first)
            .ok_or_else(|| ParseError::new("expected memory address", token.span))?;
        let second = Address::from_name(second)
            .ok_or_else(|| ParseError::new("expected memory address", token.span))?;

        Ok((first, second))
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
