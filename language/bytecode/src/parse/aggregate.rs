use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterId, RegisterRange, Symbol,
    Token, TokenType, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one aggregate or variant value operation.
    pub(super) fn parse_aggregate_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "aggregate" => self.parse_aggregate(token, results, result_types, function),
            "field.get" => {
                self.parse_projection(Opcode::FIELD_GET, token, results, result_types, function)
            }
            "field.set" => {
                self.parse_update(Opcode::FIELD_SET, token, results, result_types, function)
            }
            "element.get" => {
                self.parse_projection(Opcode::ELEMENT_GET, token, results, result_types, function)
            }
            "element.set" => {
                self.parse_update(Opcode::ELEMENT_SET, token, results, result_types, function)
            }
            "variant.new" => self.parse_variant_new(token, results, result_types, function),
            "variant.tag" => self.parse_variant_tag(token, results, result_types, function),
            "variant.payload" => self.parse_projection(
                Opcode::VARIANT_PAYLOAD,
                token,
                results,
                result_types,
                function,
            ),
            _ => Err(ParseError::new("unknown aggregate operation", token.span)),
        }
    }

    /// Parse one aggregate construction.
    fn parse_aggregate(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let result_type = Self::one_result(token, result_types)?;
        let ty = self.parse_type_name()?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let fields = self.parse_register_list(TokenType::CloseParenthesis, token, function)?;

        // encode logical field starts in source order
        let mut instruction = InstructionBuilder::new(Opcode::AGGREGATE);
        instruction.symbol(Symbol::ty(ty.0));
        instruction
            .registers(&fields)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one field, element, or variant payload projection.
    fn parse_projection(
        &mut self,
        opcode: Opcode,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let result_type = Self::one_result(token, result_types)?;
        let source = self.parse_value_range(token, function)?;
        self.eat_token(TokenType::Comma)?;
        let ty = self.parse_type_name()?;
        self.eat_token(TokenType::Comma)?;
        let index = self.parse_u32()?;

        // encode one logical projection against the linked memory layout
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.range(source);
        instruction.symbol(Symbol::ty(ty.0));
        instruction.u32(index);

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one persistent aggregate field or element update.
    fn parse_update(
        &mut self,
        opcode: Opcode,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let result_type = Self::one_result(token, result_types)?;
        let source = self.parse_value_range(token, function)?;
        self.eat_token(TokenType::Comma)?;
        let ty = self.parse_type_name()?;
        self.eat_token(TokenType::Comma)?;
        let index = self.parse_u32()?;
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_value_range(token, function)?;

        // retain the source and replace one logical component
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.range(source);
        instruction.symbol(Symbol::ty(ty.0));
        instruction.u32(index);
        instruction.range(value);

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one variant construction with an optional payload.
    fn parse_variant_new(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let result_type = Self::one_result(token, result_types)?;
        let ty = self.parse_type_name()?;
        self.eat_token(TokenType::Comma)?;
        let case = self.parse_u32()?;
        let payload = if self.eat_token_if(TokenType::Comma) {
            self.parse_value_range(token, function)?
        } else {
            RegisterRange::empty()
        };

        // encode the case and optional packed payload
        let mut instruction = InstructionBuilder::new(Opcode::VARIANT_NEW);
        instruction.symbol(Symbol::ty(ty.0));
        instruction.u32(case);
        instruction.range(payload);

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one variant discriminant projection.
    fn parse_variant_tag(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let result_type = Self::one_result(token, result_types)?;
        let source = self.parse_value_range(token, function)?;
        self.eat_token(TokenType::Comma)?;
        let ty = self.parse_type_name()?;

        // retain the linked variant layout used to decode the tag
        let mut instruction = InstructionBuilder::new(Opcode::VARIANT_TAG);
        instruction.range(source);
        instruction.symbol(Symbol::ty(ty.0));

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one complete logical value range.
    fn parse_value_range(
        &mut self,
        token: Token,
        function: &FunctionParser,
    ) -> ParseResult<RegisterRange> {
        let register = self.parse_register()?;
        let ty = function
            .value_type(register)
            .ok_or_else(|| ParseError::new("operation reads an uninitialized value", token.span))?;

        Ok(RegisterRange::new(register, ty.word_count()))
    }

    /// Parse one comma-separated register list.
    fn parse_register_list(
        &mut self,
        close: TokenType,
        token: Token,
        function: &FunctionParser,
    ) -> ParseResult<Vec<RegisterId>> {
        let mut registers = Vec::new();
        while !self.eat_token_if(close) {
            let register = self.parse_register()?;
            if function.value_type(register).is_none() {
                return Err(ParseError::new(
                    "aggregate field must begin an initialized value",
                    token.span,
                ));
            }
            registers.push(register);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(close)?;

                break;
            }
        }

        Ok(registers)
    }

    /// Return the one declared result type required by an aggregate operation.
    fn one_result(token: Token, result_types: &[ValueType]) -> ParseResult<ValueType> {
        match result_types {
            [ty] => Ok(*ty),
            _ => Err(ParseError::new(
                "aggregate operation requires one result",
                token.span,
            )),
        }
    }
}
