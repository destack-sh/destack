use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, Placement, RegisterSpan,
    RelocationTag, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one packed value or variant operation.
    pub(super) fn parse_aggregate_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "aggregate" => Opcode::AGGREGATE,
            "extract" => Opcode::EXTRACT,
            "insert" => Opcode::INSERT,
            "variant.new" => Opcode::VARIANT_NEW,
            "variant.tag" => Opcode::VARIANT_TAG,
            _ => return Err(ParseError::new("unknown aggregate operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match name {
            "aggregate" => self.parse_aggregate(token, &results, function),
            "extract" => self.parse_extract(&results, function),
            "insert" => self.parse_insert(&results, function),
            "variant.new" => self.parse_variant_new(&results, function),
            "variant.tag" => self.parse_variant_tag(&results, function),
            _ => Err(ParseError::new("invalid aggregate operation", token.span)),
        }
    }

    /// Parse one packed aggregate construction.
    fn parse_aggregate(
        &mut self,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut placements = Vec::new();

        // parse each physical source placement
        while !self.eat_token_if(TokenType::CloseBracket) {
            placements.push(self.parse_placement()?);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseBracket)?;

                break;
            }
        }

        // encode the exact packed byte layout
        let mut instruction = InstructionBuilder::new(Opcode::AGGREGATE);
        instruction
            .placements(&placements)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one value extraction by byte range.
    fn parse_extract(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let source = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let byte_offset = self.parse_u32()?;
        self.eat_token(TokenType::Colon)?;
        let byte_len = self.parse_u32()?;

        // encode the exact copied byte range
        let mut instruction = InstructionBuilder::new(Opcode::EXTRACT);
        instruction.span(source);
        instruction.u32(byte_offset);
        instruction.u32(byte_len);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one persistent value insertion by byte range.
    fn parse_insert(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let aggregate = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let byte_offset = self.parse_u32()?;
        self.eat_token(TokenType::Colon)?;
        let byte_len = self.parse_u32()?;
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register_span()?;

        // encode the copy and exact replacement byte range
        let mut instruction = InstructionBuilder::new(Opcode::INSERT);
        instruction.span(aggregate);
        instruction.u32(byte_offset);
        instruction.u32(byte_len);
        instruction.span(value);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one variant construction with an optional payload.
    fn parse_variant_new(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let layout = self.parse_layout_id()?;
        self.eat_token(TokenType::Comma)?;
        let case = self.parse_u32()?;
        let payload = if self.eat_token_if(TokenType::Comma) {
            self.parse_register_span()?
        } else {
            RegisterSpan::empty()
        };

        // encode the selected layout, case, and optional payload
        let mut instruction = InstructionBuilder::new(Opcode::VARIANT_NEW);
        instruction.relocation(RelocationTag::LAYOUT, layout.0);
        instruction.u32(case);
        instruction.span(payload);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one variant discriminant extraction.
    fn parse_variant_tag(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let variant = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let layout = self.parse_layout_id()?;

        // encode the exact linked variant layout
        let mut instruction = InstructionBuilder::new(Opcode::VARIANT_TAG);
        instruction.span(variant);
        instruction.relocation(RelocationTag::LAYOUT, layout.0);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one physical aggregate placement.
    fn parse_placement(&mut self) -> ParseResult<Placement> {
        let registers = self.parse_register_span()?;
        self.eat_token(TokenType::At)?;
        let byte_offset = self.parse_u32()?;
        self.eat_token(TokenType::Colon)?;
        let byte_len = self.parse_u32()?;

        Ok(Placement::new(registers, byte_offset, byte_len))
    }
}
