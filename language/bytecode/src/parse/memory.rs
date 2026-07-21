use crate::{
    InstructionBuilder, MemoryOperation, Opcode, ParseError, ParseResult, Parser, RegisterId,
    RegisterRange, Scalar, Token, TokenType, ValueType, VectorOperation,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one memory operation.
    pub(super) fn parse_memory_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // load one reference or vector from its declared result type
        if name == "load" {
            return self.parse_load(token, results, result_types, function);
        }

        // store one reference or vector from its input type
        if name == "store" {
            return self.parse_store(token, results, function);
        }

        let scalar = name
            .split_once('.')
            .filter(|(operation, _)| matches!(*operation, "load" | "store"))
            .and_then(|(_, scalar)| Scalar::from_name(scalar));

        // scalar load or store
        if let Some(scalar) = scalar {
            return self.parse_scalar_memory(name, scalar, token, results, result_types, function);
        }

        // byte range and prefetch operations
        match name {
            "copy.bytes" => self.parse_byte_transfer(Opcode::COPY_BYTES, token, results, function),
            "move.bytes" => self.parse_byte_transfer(Opcode::MOVE_BYTES, token, results, function),
            "fill.bytes" => self.parse_byte_fill(token, results, function),
            "compare.bytes" => self.parse_byte_compare(token, results, function),
            "prefetch.read" => self.parse_prefetch(Opcode::PREFETCH_READ, token, results, function),
            "prefetch.write" => {
                self.parse_prefetch(Opcode::PREFETCH_WRITE, token, results, function)
            }
            _ => Err(ParseError::new("unknown memory operation", token.span)),
        }
    }

    /// Parse one reference or vector load.
    fn parse_load(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let Some(ty) = result_types.first().copied() else {
            return Err(ParseError::new("load requires one result", token.span));
        };

        // load a reference with its encoded ownership and space
        if let Some(reference) = ty
            .reference_type()
            .filter(|_| ty.is_initialized_reference())
        {
            let pointer = self.parse_register()?;
            if !function.has_type(pointer, ValueType::pointer()) {
                return Err(ParseError::new(
                    "reference load requires a native pointer",
                    token.span,
                ));
            }

            let mut instruction = InstructionBuilder::new(Opcode::LOAD);
            instruction.register(pointer);
            instruction.reference(reference.kind(), reference.space());

            function.emit(instruction, results, &[ty], self.empty_span())
        }
        // load one fixed width vector
        else if ty.vector_type().is_some() {
            self.parse_vector_operation("load", token, results, result_types, function)
        }
        // require scalar loads to carry their representation suffix
        else {
            Err(ParseError::new(
                "bare load requires a reference or vector result",
                token.span,
            ))
        }
    }

    /// Parse one reference or vector store.
    fn parse_store(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        if !function.has_type(pointer, ValueType::pointer()) {
            return Err(ParseError::new(
                "store requires a native pointer",
                token.span,
            ));
        }

        // read the stored logical value
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register()?;
        let ty = function
            .value_type(value)
            .ok_or_else(|| ParseError::new("store reads an uninitialized value", token.span))?;

        // store one reference word
        if ty.is_initialized_reference() {
            let mut instruction = InstructionBuilder::new(Opcode::STORE);
            instruction.register(pointer);
            instruction.register(value);

            function.emit(instruction, results, &[], self.empty_span())
        }
        // store one fixed width vector range
        else if let Some(vector) = ty.vector_type() {
            let mut instruction = InstructionBuilder::new(Opcode::vector(VectorOperation::Store));
            instruction.register(pointer);
            instruction.range(RegisterRange::new(value, vector.word_count()));
            instruction.vector_type(vector);

            function.emit(instruction, results, &[], self.empty_span())
        }
        // require scalar stores to carry their representation suffix
        else {
            Err(ParseError::new(
                "bare store requires a reference or vector value",
                token.span,
            ))
        }
    }

    /// Parse one scalar load or store.
    fn parse_scalar_memory(
        &mut self,
        name: &str,
        scalar: Scalar,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // select the scalar memory operation
        let operation = if name.starts_with("load.") {
            MemoryOperation::Load
        } else {
            MemoryOperation::Store
        };

        // require one native target pointer
        let pointer = self.parse_register()?;
        if !function.has_type(pointer, ValueType::pointer()) {
            return Err(ParseError::new(
                "memory operation requires a native pointer",
                token.span,
            ));
        }

        // prepare the typed operation
        let mut instruction = InstructionBuilder::new(Opcode::memory(operation, scalar));
        let ty = ValueType::scalar(scalar);

        // load one scalar value
        if operation == MemoryOperation::Load {
            if result_types.first() != Some(&ty) {
                return Err(ParseError::new(
                    "load result does not match its scalar type",
                    token.span,
                ));
            }
            instruction.register(pointer);

            function.emit(instruction, results, &[ty], self.empty_span())
        }
        // store one scalar value
        else {
            self.eat_token(TokenType::Comma)?;
            let value = self.parse_register()?;
            if !function.has_type(value, ty) {
                return Err(ParseError::new(
                    "store value does not match its scalar type",
                    token.span,
                ));
            }
            instruction.register(pointer);
            instruction.register(value);

            function.emit(instruction, results, &[], self.empty_span())
        }
    }

    /// Parse one byte copy or move.
    fn parse_byte_transfer(
        &mut self,
        opcode: Opcode,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse source, target, and byte length
        let source = self.parse_register()?;
        self.eat_token(TokenType::Arrow)?;
        let target = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_register()?;

        // match both pointers and the byte length
        let are_pointers_valid = function.has_type(source, ValueType::pointer())
            && function.has_type(target, ValueType::pointer());
        if !are_pointers_valid || !function.has_type(byte_len, ValueType::scalar(Scalar::Uint64)) {
            return Err(ParseError::new(
                "byte transfer requires pointers and a uint64 byte length",
                token.span,
            ));
        }

        // encode operands in target then source order
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(target);
        instruction.register(source);
        instruction.register(byte_len);

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one byte fill.
    fn parse_byte_fill(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse target, fill byte, and byte length
        let target = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_register()?;
        if !function.has_type(target, ValueType::pointer())
            || !function.has_type(byte, ValueType::scalar(Scalar::Uint8))
            || !function.has_type(byte_len, ValueType::scalar(Scalar::Uint64))
        {
            return Err(ParseError::new(
                "byte fill requires a pointer, uint8 byte, and uint64 byte length",
                token.span,
            ));
        }

        // encode the complete fill range
        let mut instruction = InstructionBuilder::new(Opcode::FILL_BYTES);
        instruction.register(target);
        instruction.register(byte);
        instruction.register(byte_len);

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one byte comparison.
    fn parse_byte_compare(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse both pointers and compared byte length
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_register()?;

        // match both pointers and the byte length
        let are_pointers_valid = function.has_type(left, ValueType::pointer())
            && function.has_type(right, ValueType::pointer());
        if !are_pointers_valid || !function.has_type(byte_len, ValueType::scalar(Scalar::Uint64)) {
            return Err(ParseError::new(
                "byte comparison requires pointers and a uint64 byte length",
                token.span,
            ));
        }

        // encode the signed comparison result
        let mut instruction = InstructionBuilder::new(Opcode::COMPARE_BYTES);
        instruction.register(left);
        instruction.register(right);
        instruction.register(byte_len);

        function.emit(
            instruction,
            results,
            &[ValueType::scalar(Scalar::Int32)],
            self.empty_span(),
        )
    }

    /// Parse one prefetch hint.
    fn parse_prefetch(
        &mut self,
        opcode: Opcode,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // require one native pointer
        let pointer = self.parse_register()?;
        if !function.has_type(pointer, ValueType::pointer()) {
            return Err(ParseError::new("prefetch requires a pointer", token.span));
        }

        // encode the prefetch hint
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(pointer);

        function.emit(instruction, results, &[], self.empty_span())
    }
}
