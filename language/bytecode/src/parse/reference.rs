use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, ReferenceKind, RegisterId, Scalar,
    Symbol, Token, TokenType, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one reference lifetime or storage operation.
    pub(super) fn parse_reference_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "free" => self.parse_reference_lifetime(Opcode::FREE, token, results, function),
            "drop" => self.parse_drop(token, results, function),
            "pin" => self.parse_reference_lifetime(Opcode::PIN, token, results, function),
            "unpin" => self.parse_reference_lifetime(Opcode::UNPIN, token, results, function),
            "barrier" => self.parse_barrier(token, results, function),
            _ => Err(ParseError::new("unknown reference operation", token.span)),
        }
    }

    /// Parse one managed pin transition or unique release.
    fn parse_reference_lifetime(
        &mut self,
        opcode: Opcode,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // derive the exact ownership required by this transition
        let value = self.parse_register()?;
        let reference = function
            .value_type(value)
            .filter(|ty| ty.is_initialized_reference())
            .and_then(ValueType::reference_type)
            .ok_or_else(|| ParseError::new("operation requires a reference", token.span))?;
        let is_free = opcode == Opcode::FREE;
        let required = if is_free {
            ReferenceKind::UNIQUE
        } else {
            ReferenceKind::MANAGED
        };
        if reference.kind() != required {
            return Err(ParseError::new(
                if is_free {
                    "free requires a unique reference"
                } else {
                    "pin operation requires a managed reference"
                },
                token.span,
            ));
        }

        // encode the lifetime transition
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(value);
        instruction.reference(reference.kind(), reference.space());

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one explicit value destruction.
    fn parse_drop(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // match one pointer or initialized reference
        let value = self.parse_register()?;
        let value_type = function
            .value_type(value)
            .ok_or_else(|| ParseError::new("drop reads an uninitialized value", token.span))?;
        if !value_type.is_pointer() && !value_type.is_initialized_reference() {
            return Err(ParseError::new(
                "drop requires a native pointer or initialized reference",
                token.span,
            ));
        }

        // resolve the concrete dropped type
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_type_name()?;

        // encode the explicit destruction
        let mut instruction = InstructionBuilder::new(Opcode::DROP);
        instruction.register(value);
        instruction.value_type(value_type);
        instruction.symbol(Symbol::ty(ty.0));

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one managed reference write barrier.
    fn parse_barrier(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the managed object and changed byte range
        let object = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let offset = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_register()?;

        // match the managed object and byte range types
        let reference = function
            .value_type(object)
            .filter(|ty| ty.is_initialized_reference())
            .and_then(ValueType::reference_type)
            .ok_or_else(|| {
                ParseError::new(
                    "write barrier requires a managed reference and uint64 byte range",
                    token.span,
                )
            })?;
        let uint64 = ValueType::scalar(Scalar::Uint64);
        if reference.kind() != ReferenceKind::MANAGED
            || !function.has_type(offset, uint64)
            || !function.has_type(byte_len, uint64)
        {
            return Err(ParseError::new(
                "write barrier requires a managed reference and uint64 byte range",
                token.span,
            ));
        }

        // encode the write barrier
        let mut instruction = InstructionBuilder::new(Opcode::BARRIER);
        instruction.register(object);
        instruction.reference(reference.kind(), reference.space());
        instruction.register(offset);
        instruction.register(byte_len);

        function.emit(instruction, results, &[], self.empty_span())
    }
}
