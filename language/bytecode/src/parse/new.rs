use crate::{
    InstructionBuilder, New, NewKind, Opcode, ParseError, ParseResult, Parser, RegisterId, Scalar,
    Symbol, Token, TokenType, ValueType,
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
}
