use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one dynamic value operation.
    pub(super) fn parse_dynamic_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "dynamic.bind" => Opcode::DYNAMIC_BIND,
            "dynamic.bind.virtual" => Opcode::DYNAMIC_BIND_VIRTUAL,
            "dynamic.read" => Opcode::DYNAMIC_READ,
            "type.of.dynamic" => Opcode::TYPE_OF_DYNAMIC,
            "type.of.object" => Opcode::TYPE_OF_OBJECT,
            _ => return Err(ParseError::new("unknown dynamic operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match name {
            "dynamic.bind" => self.parse_dynamic_bind(&results, function),
            "dynamic.bind.virtual" => self.parse_dynamic_bind_virtual(&results, function),
            "dynamic.read" => self.parse_dynamic_read(&results, function),
            "type.of.dynamic" => self.parse_type_of_dynamic(&results, function),
            "type.of.object" => self.parse_type_of_object(&results, function),
            _ => Err(ParseError::new("invalid dynamic operation", token.span)),
        }
    }

    /// Parse one dynamic value construction.
    fn parse_dynamic_bind(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the erased payload and exact dispatch identity
        let payload = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let token = self.eat_token(TokenType::Identifier)?;
        let table = self
            .text(token)
            .strip_prefix('d')
            .and_then(|index| index.parse::<u32>().ok())
            .ok_or_else(|| ParseError::new("expected dynamic table id", token.span))?;

        // encode the erased payload and linked dispatch table
        let mut instruction = InstructionBuilder::new(Opcode::DYNAMIC_BIND);
        instruction.register(payload);
        instruction.dynamic_table(table);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one class object's dynamic value construction.
    fn parse_dynamic_bind_virtual(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let payload = self.parse_register()?;
        self.eat_token(TokenType::OpenBracket)?;
        let slot = self.parse_u16()?;
        self.eat_token(TokenType::CloseBracket)?;

        // encode the class payload and its conformance slot
        let mut instruction = InstructionBuilder::new(Opcode::DYNAMIC_BIND_VIRTUAL);
        instruction.register(payload);
        instruction.u16(slot);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one dynamic field read.
    fn parse_dynamic_read(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let dynamic = self.parse_register_span()?;
        self.eat_token(TokenType::OpenBracket)?;
        let slot = self.parse_u16()?;
        self.eat_token(TokenType::CloseBracket)?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_u32()?;

        // encode the dynamic value, field slot, and exact result width
        let mut instruction = InstructionBuilder::new(Opcode::DYNAMIC_READ);
        instruction.span(dynamic);
        instruction.u16(slot);
        instruction.u32(byte_len);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one dynamic runtime type access.
    fn parse_type_of_dynamic(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let dynamic = self.parse_register_span()?;

        // encode the runtime type projection
        let mut instruction = InstructionBuilder::new(Opcode::TYPE_OF_DYNAMIC);
        instruction.span(dynamic);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one class object runtime type access.
    fn parse_type_of_object(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the object reference
        let object = self.parse_register()?;

        // encode the runtime type projection
        let mut instruction = InstructionBuilder::new(Opcode::TYPE_OF_OBJECT);
        instruction.register(object);

        function.emit(instruction, results, self.empty_span())
    }
}
