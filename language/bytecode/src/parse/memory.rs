use crate::{
    InstructionBuilder, MemoryOperation, Opcode, ParseError, ParseResult, Parser, RegisterSpan,
    Scalar, Token, TokenType,
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
        // load one physical byte range
        if name == "load" {
            let results = self.parse_definitions(Opcode::LOAD)?;

            return self.parse_load(&results, function);
        }

        // store one physical byte range
        if name == "store" {
            let results = self.parse_definitions(Opcode::STORE)?;

            return self.parse_store(&results, function);
        }

        let scalar = name
            .split_once('.')
            .filter(|(operation, _)| matches!(*operation, "load" | "store"))
            .and_then(|(_, scalar)| Scalar::from_name(scalar));

        // scalar load or store
        if let Some(scalar) = scalar {
            let operation = if name.starts_with("load.") {
                MemoryOperation::Load
            } else {
                MemoryOperation::Store
            };
            let opcode = Opcode::memory(operation, scalar);
            let results = self.parse_definitions(opcode)?;

            return self.parse_scalar_memory(name, scalar, &results, function);
        }

        // byte range and prefetch operations
        match name {
            "copy.bytes" => {
                let results = self.parse_definitions(Opcode::COPY_BYTES)?;

                self.parse_byte_transfer(Opcode::COPY_BYTES, &results, function)
            }
            "move.bytes" => {
                let results = self.parse_definitions(Opcode::MOVE_BYTES)?;

                self.parse_byte_transfer(Opcode::MOVE_BYTES, &results, function)
            }
            "fill.bytes" => {
                let results = self.parse_definitions(Opcode::FILL_BYTES)?;

                self.parse_byte_fill(&results, function)
            }
            "compare.bytes" => {
                let results = self.parse_definitions(Opcode::COMPARE_BYTES)?;

                self.parse_byte_compare(&results, function)
            }
            "prefetch.read" => {
                let results = self.parse_definitions(Opcode::PREFETCH_READ)?;

                self.parse_prefetch(Opcode::PREFETCH_READ, &results, function)
            }
            "prefetch.write" => {
                let results = self.parse_definitions(Opcode::PREFETCH_WRITE)?;

                self.parse_prefetch(Opcode::PREFETCH_WRITE, &results, function)
            }
            _ => Err(ParseError::new("unknown memory operation", token.span)),
        }
    }

    /// Parse one packed value or vector load.
    fn parse_load(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_u32()?;
        let mut instruction = InstructionBuilder::new(Opcode::LOAD);
        instruction.register(pointer);
        instruction.u32(byte_len);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one packed value or vector store.
    fn parse_store(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_u32()?;
        let mut instruction = InstructionBuilder::new(Opcode::STORE);
        instruction.register(pointer);
        instruction.span(value);
        instruction.u32(byte_len);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one scalar load or store.
    fn parse_scalar_memory(
        &mut self,
        name: &str,
        scalar: Scalar,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // select the scalar memory operation
        let operation = if name.starts_with("load.") {
            MemoryOperation::Load
        } else {
            MemoryOperation::Store
        };

        let pointer = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(Opcode::memory(operation, scalar));

        // load one scalar value
        if operation == MemoryOperation::Load {
            instruction.register(pointer);

            function.emit(instruction, results, self.empty_span())
        }
        // store one scalar value
        else {
            self.eat_token(TokenType::Comma)?;
            let value = self.parse_register()?;
            instruction.register(pointer);
            instruction.register(value);

            function.emit(instruction, results, self.empty_span())
        }
    }

    /// Parse one byte copy or move.
    fn parse_byte_transfer(
        &mut self,
        opcode: Opcode,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse source, target, and byte length
        let source = self.parse_register()?;
        self.eat_token(TokenType::Arrow)?;
        let target = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_register()?;

        // encode operands in target then source order
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(target);
        instruction.register(source);
        instruction.register(byte_len);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one byte fill.
    fn parse_byte_fill(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse target, fill byte, and byte length
        let target = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_register()?;
        // encode the complete fill range
        let mut instruction = InstructionBuilder::new(Opcode::FILL_BYTES);
        instruction.register(target);
        instruction.register(byte);
        instruction.register(byte_len);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one byte comparison.
    fn parse_byte_compare(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse both pointers and compared byte length
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_register()?;

        // encode the signed comparison result
        let mut instruction = InstructionBuilder::new(Opcode::COMPARE_BYTES);
        instruction.register(left);
        instruction.register(right);
        instruction.register(byte_len);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one prefetch hint.
    fn parse_prefetch(
        &mut self,
        opcode: Opcode,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(pointer);

        function.emit(instruction, results, self.empty_span())
    }
}
