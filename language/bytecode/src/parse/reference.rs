use crate::{
    Opcode, ParseError, ParseResult, Parser, ReferenceKind, RegisterId, RegisterRange, Scalar,
    Symbol, Token, TokenType, ValueType,
};

use super::builder::InstructionBuilder;
use super::function::FunctionBuilder;

impl Parser<'_> {
    /// Parse one reference lifetime or storage operation.
    pub(super) fn parse_reference_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionBuilder,
    ) -> ParseResult<()> {
        match Opcode::from_name(name) {
            Some(Opcode::NEW_COMPLETE) => self.parse_new_complete(token, results, function),
            Some(Opcode::FREE) => {
                self.parse_reference_lifetime(Opcode::FREE, token, results, function)
            }
            Some(Opcode::DROP) => self.parse_drop(token, results, function),
            Some(opcode @ (Opcode::PIN | Opcode::UNPIN)) => {
                self.parse_reference_lifetime(opcode, token, results, function)
            }
            Some(Opcode::BARRIER) => self.parse_barrier(token, results, function),
            _ => Err(ParseError::new("unknown reference operation", token.span)),
        }
    }

    /// Parse one allocation initialization transition.
    fn parse_new_complete(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionBuilder,
    ) -> ParseResult<()> {
        // derive the initialized form from the source value
        let input = self.parse_register()?;
        let input_type = function
            .value_type(input)
            .filter(|ty| ty.is_uninitialized())
            .ok_or_else(|| {
                ParseError::new(
                    "new.complete requires an uninitialized allocation",
                    token.span,
                )
            })?;
        let result_type = input_type.initialized().ok_or_else(|| {
            ParseError::new("uninitialized value has no initialized form", token.span)
        })?;

        // encode the complete allocation state transition
        let mut instruction = InstructionBuilder::new(Opcode::NEW_COMPLETE);
        instruction.range(RegisterRange::new(input, input_type.word_count()));

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one managed pin transition or unique release.
    fn parse_reference_lifetime(
        &mut self,
        opcode: Opcode,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionBuilder,
    ) -> ParseResult<()> {
        // derive the exact ownership required by this transition
        let value = self.parse_register()?;
        let kind = function
            .value_type(value)
            .filter(|ty| ty.is_initialized_reference())
            .and_then(ValueType::reference_type)
            .map(|reference| reference.kind());
        let is_free = opcode == Opcode::FREE;
        let required = if is_free {
            ReferenceKind::UNIQUE
        } else {
            ReferenceKind::MANAGED
        };
        if kind != Some(required) {
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

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one explicit value destruction.
    fn parse_drop(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionBuilder,
    ) -> ParseResult<()> {
        // match one address or initialized reference
        let value = self.parse_register()?;
        let value_type = function
            .value_type(value)
            .ok_or_else(|| ParseError::new("drop reads an uninitialized value", token.span))?;
        if !value_type.is_address() && !value_type.is_initialized_reference() {
            return Err(ParseError::new(
                "drop requires a native address or initialized reference",
                token.span,
            ));
        }

        // resolve the concrete dropped type
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_type_name()?;

        // encode the explicit destruction
        let mut instruction = InstructionBuilder::new(Opcode::DROP);
        instruction.register(value);
        instruction.symbol(Symbol::ty(ty.0));

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one managed reference write barrier.
    fn parse_barrier(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionBuilder,
    ) -> ParseResult<()> {
        // parse the managed object and changed byte range
        let object = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let offset = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_register()?;

        // match the managed object and byte range types
        let is_managed = function.value_type(object).is_some_and(|ty| {
            ty.is_initialized_reference()
                && ty
                    .reference_type()
                    .is_some_and(|reference| reference.kind() == ReferenceKind::MANAGED)
        });
        let uint64 = ValueType::scalar(Scalar::Uint64);
        if !is_managed || !function.has_type(offset, uint64) || !function.has_type(byte_len, uint64)
        {
            return Err(ParseError::new(
                "write barrier requires a managed reference and uint64 byte range",
                token.span,
            ));
        }

        // encode the write barrier
        let mut instruction = InstructionBuilder::new(Opcode::BARRIER);
        instruction.register(object);
        instruction.register(offset);
        instruction.register(byte_len);

        function.emit(instruction, results, &[], self.empty_span())
    }
}
