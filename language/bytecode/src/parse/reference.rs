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
            "free" => Opcode::FREE,
            "drop" => Opcode::DROP,
            "pin" => Opcode::PIN,
            "unpin" => Opcode::UNPIN,
            "barrier" => Opcode::BARRIER,
            _ => return Err(ParseError::new("unknown reference operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match name {
            "free" | "pin" | "unpin" => {
                self.parse_reference_lifetime(opcode, token, &results, function)
            }
            "drop" => self.parse_drop(&results, function),
            "barrier" => self.parse_barrier(token, &results, function),
            _ => Err(ParseError::new("invalid reference operation", token.span)),
        }
    }

    /// Parse one managed pin transition or unique release.
    fn parse_reference_lifetime(
        &mut self,
        opcode: Opcode,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register()?;
        let reference = self.parse_reference_representation(token)?;

        // encode the lifetime transition
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(value);
        instruction.reference(reference.kind(), reference.storage());

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one explicit value destruction.
    fn parse_drop(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register_span()?;

        // resolve the linked destructor
        self.eat_token(TokenType::Comma)?;
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
