use crate::{
    InstructionBuilder, New, NewKind, Opcode, ParseError, ParseResult, Parser, RegisterId,
    RegisterRange, Scalar, Symbol, Token, TokenType, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one `new` instruction.
    pub(super) fn parse_new(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        if name == "new.complete" {
            return self.parse_new_complete(token, results, result_types, function);
        }

        // match the result reference against the operation
        let operation = New::from_name(name)
            .ok_or_else(|| ParseError::new("invalid new operation", token.span))?;
        let result_type = result_types
            .first()
            .copied()
            .ok_or_else(|| ParseError::new("new requires a reference result", token.span))?;
        let result_reference = if operation.kind == NewKind::Slice {
            result_type.slice_reference()
        } else {
            result_type.reference_type()
        };
        if result_reference != Some(operation.reference()) {
            return Err(ParseError::new(
                "new result does not match its reference type",
                token.span,
            ));
        }

        // match the result against the new type and initialization
        let ty = self.parse_type_name()?;
        let expected_type = operation.result_type(ty);
        if result_type != expected_type {
            return Err(ParseError::new(
                "new result does not match its initialization",
                token.span,
            ));
        }

        // encode the relocatable runtime type
        let opcode = Opcode::new(operation)
            .ok_or_else(|| ParseError::new("invalid new operation", token.span))?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.symbol(Symbol::ty(ty.0));

        // encode the variable slice length
        if operation.kind == NewKind::Slice {
            self.eat_token(TokenType::Comma)?;
            let length = self.parse_register()?;
            if !function.has_type(length, ValueType::scalar(Scalar::Uint64)) {
                return Err(ParseError::new(
                    "new slice length is not uint64",
                    token.span,
                ));
            }
            instruction.register(length);
        }

        // encode explicit success and failure edges
        if operation.is_fallible {
            self.eat_token(TokenType::FatArrow)?;
            instruction.branch(self.parse_label()?);
            self.eat_token(TokenType::Comma)?;
            instruction.branch(self.parse_label()?);
        }

        function.emit(instruction, results, &[expected_type], self.empty_span())
    }

    /// Parse one allocation initialization transition.
    fn parse_new_complete(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
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
        if result_types != [result_type] {
            return Err(ParseError::new(
                "new.complete result does not match its allocation",
                token.span,
            ));
        }

        // encode the initialization state transition
        let mut instruction = InstructionBuilder::new(Opcode::NEW_COMPLETE);
        instruction.range(RegisterRange::new(input, input_type.word_count()));

        function.emit(instruction, results, &[result_type], self.empty_span())
    }
}
