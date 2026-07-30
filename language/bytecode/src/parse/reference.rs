use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, ReferenceKind, ReferenceType,
    RegisterSpan, RelocationTag, Space, Storage, Token, TokenType,
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
        let (operation, reference) = if name == "drop" {
            (name, None)
        } else {
            let (operation, reference) = self.parse_reference_name(name, token)?;

            (operation, Some(reference))
        };
        let opcode = match operation {
            "free" => Opcode::FREE,
            "drop" => Opcode::DROP,
            "pin" => Opcode::PIN,
            "unpin" => Opcode::UNPIN,
            "barrier" => Opcode::BARRIER,
            _ => return Err(ParseError::new("unknown reference operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match (operation, reference) {
            ("free" | "pin" | "unpin", Some(reference)) => {
                self.parse_reference_lifetime(opcode, reference, &results, function)
            }
            ("drop", None) => self.parse_drop(&results, function),
            ("barrier", Some(reference)) => self.parse_barrier(reference, &results, function),
            _ => Err(ParseError::new("invalid reference operation", token.span)),
        }
    }

    /// Parse one managed pin transition or unique release.
    fn parse_reference_lifetime(
        &mut self,
        opcode: Opcode,
        reference: ReferenceType,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register()?;

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
        reference: ReferenceType,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the managed object and changed byte range
        let object = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let offset = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_register()?;

        // encode the write barrier
        let mut instruction = InstructionBuilder::new(Opcode::BARRIER);
        instruction.register(object);
        instruction.reference(reference.kind(), reference.storage());
        instruction.register(offset);
        instruction.register(byte_len);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one operation name selected by reference space and ownership.
    pub(super) fn parse_reference_name<'a>(
        &self,
        name: &'a str,
        token: Token,
    ) -> ParseResult<(&'a str, ReferenceType)> {
        let (name, kind) = name
            .rsplit_once('.')
            .ok_or_else(|| ParseError::new("reference operation has no ownership", token.span))?;
        let (operation, space) = name
            .rsplit_once('.')
            .ok_or_else(|| ParseError::new("reference operation has no space", token.span))?;
        let kind = ReferenceKind::from_name(kind)
            .ok_or_else(|| ParseError::new("unknown reference ownership", token.span))?;
        let space = Space::from_name(space)
            .ok_or_else(|| ParseError::new("unknown reference space", token.span))?;

        Ok((operation, ReferenceType::new(kind, Storage::heap(space))))
    }
}
