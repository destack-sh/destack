use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, ReferenceType, RegisterSpan,
    RelocationTag, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one reference lifetime or storage operation.
    pub(super) fn parse_reference_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "release" => Opcode::RELEASE,
            "free" => Opcode::FREE,
            "drop" => Opcode::DROP,
            "barrier" => Opcode::BARRIER,
            _ => return Err(ParseError::new("unknown reference operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match name {
            "release" | "free" => self.parse_owner(opcode, &results, function),
            "drop" => self.parse_drop(&results, function),
            "barrier" => self.parse_barrier(token, &results, function),
            _ => Err(ParseError::new("invalid reference operation", token.span)),
        }
    }

    /// Parse one operation on an allocation owner.
    fn parse_owner(
        &mut self,
        opcode: Opcode,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let owner = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(owner);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one explicit value destruction.
    fn parse_drop(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;

        // resolve the linked destructor
        let destructor = self.parse_function_id()?;

        // encode the explicit destruction
        let mut instruction = InstructionBuilder::new(Opcode::DROP);
        instruction.span(value);
        instruction.relocation(RelocationTag::FUNCTION, destructor.0);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one managed reference write barrier.
    fn parse_barrier(
        &mut self,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the managed object and changed byte range
        let object = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let offset = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_register()?;
        let reference = self.parse_reference_representation(token)?;

        // encode the write barrier
        let mut instruction = InstructionBuilder::new(Opcode::BARRIER);
        instruction.register(object);
        instruction.reference(reference.kind(), reference.storage());
        instruction.register(offset);
        instruction.register(byte_len);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one trailing reference representation.
    fn parse_reference_representation(&mut self, token: Token) -> ParseResult<ReferenceType> {
        let ty = self.parse_representation()?;

        ty.reference_type()
            .ok_or_else(|| ParseError::new("expected reference representation", token.span))
    }
}
