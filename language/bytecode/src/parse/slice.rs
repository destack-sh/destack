use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterId, RegisterRange, Scalar,
    Symbol, Token, TokenType, ValueTag, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one slice operation.
    pub(super) fn parse_slice_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "slice.view" => self.parse_slice_view(token, results, result_types, function),
            "slice.length" => self.parse_slice_length(token, results, function),
            _ => Err(ParseError::new("unknown slice operation", token.span)),
        }
    }

    /// Parse one contiguous slice subview.
    fn parse_slice_view(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // match the initialized slice result
        let result_type = result_types
            .first()
            .copied()
            .filter(|ty| ty.tag() == ValueTag::SLICE)
            .ok_or_else(|| {
                ParseError::new(
                    "slice.view requires an initialized slice result",
                    token.span,
                )
            })?;
        let element = result_type.slice_element().ok_or_else(|| {
            ParseError::new(
                "slice.view requires an initialized slice result",
                token.span,
            )
        })?;

        // match the initialized slice source
        let source = self.parse_register()?;
        let source_type = function.value_type(source).ok_or_else(|| {
            ParseError::new("slice.view reads an uninitialized slice", token.span)
        })?;
        let is_matching_slice = source_type.tag() == ValueTag::SLICE
            && source_type.slice_element() == result_type.slice_element()
            && source_type.slice_reference() == result_type.slice_reference();
        if !is_matching_slice {
            return Err(ParseError::new(
                "slice.view source does not match its result type",
                token.span,
            ));
        }

        // parse the dynamic subrange
        self.eat_token(TokenType::Comma)?;
        let start = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let length = self.parse_register()?;
        let uint64 = ValueType::scalar(Scalar::Uint64);
        if !function.has_type(start, uint64) || !function.has_type(length, uint64) {
            return Err(ParseError::new(
                "slice.view requires uint64 start and length values",
                token.span,
            ));
        }

        // encode the source fat value and scalar subrange
        let mut instruction = InstructionBuilder::new(Opcode::SLICE_VIEW);
        instruction.range(RegisterRange::new(source, source_type.word_count()));
        instruction.symbol(Symbol::ty(element.0));
        instruction.register(start);
        instruction.register(length);

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one slice element count.
    fn parse_slice_length(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // match the initialized slice input
        let slice = self.parse_register()?;
        let slice_type = function
            .value_type(slice)
            .filter(|ty| ty.tag() == ValueTag::SLICE)
            .ok_or_else(|| {
                ParseError::new("slice.length requires an initialized slice", token.span)
            })?;

        // encode the length projection
        let mut instruction = InstructionBuilder::new(Opcode::SLICE_LENGTH);
        instruction.range(RegisterRange::new(slice, slice_type.word_count()));

        function.emit(
            instruction,
            results,
            &[ValueType::scalar(Scalar::Uint64)],
            self.empty_span(),
        )
    }
}
