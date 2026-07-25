use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, RelocationTag,
    Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one pointer operation.
    pub(super) fn parse_pointer_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let operation = if name.starts_with("reference.pointer.") {
            "reference.pointer"
        } else {
            name
        };
        let opcode = match operation {
            "address" => Opcode::ADDRESS,
            "global.address" => Opcode::GLOBAL_ADDRESS,
            "pointer.add" => Opcode::POINTER_ADD,
            "pointer.distance" => Opcode::POINTER_DISTANCE,
            "reference.pointer" => Opcode::REFERENCE_POINTER,
            _ => return Err(ParseError::new("unknown pointer operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match operation {
            "address" => self.parse_address(&results, function),
            "global.address" => self.parse_global_address(&results, function),
            "pointer.add" => self.parse_pointer_add(token, &results, function),
            "pointer.distance" => self.parse_pointer_distance(&results, function),
            "reference.pointer" => self.parse_reference_pointer(name, token, &results, function),
            _ => Err(ParseError::new("invalid pointer operation", token.span)),
        }
    }

    /// Parse one stable heap reference projection.
    fn parse_reference_pointer(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let (operation, reference_type) = self.parse_reference_name(name, token)?;
        if operation != "reference.pointer" {
            return Err(ParseError::new("invalid reference operation", token.span));
        }
        let reference = self.parse_register()?;

        // retain the representation required to resolve the stable offset
        let mut instruction = InstructionBuilder::new(Opcode::REFERENCE_POINTER);
        instruction.register(reference);
        instruction.reference(reference_type.kind(), reference_type.space());

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one linked global address.
    fn parse_global_address(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let token = self.eat_token(TokenType::Identifier)?;
        let global = self
            .text(token)
            .strip_prefix('g')
            .and_then(|index| index.parse::<u32>().ok())
            .ok_or_else(|| ParseError::new("expected global id", token.span))?;

        // encode the linked global identity
        let mut instruction = InstructionBuilder::new(Opcode::GLOBAL_ADDRESS);
        instruction.relocation(RelocationTag::GLOBAL, global);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse the stable address of one register value.
    fn parse_address(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register_span()?;
        let mut instruction = InstructionBuilder::new(Opcode::ADDRESS);
        instruction.span(value);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one immediate, register, or scaled pointer addition.
    fn parse_pointer_add(
        &mut self,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let pointer = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;

        // encode an immediate byte displacement
        if self.peek_is(TokenType::Integer) {
            let offset = self.parse_i64()?;
            let offset = i32::try_from(offset)
                .map_err(|_| ParseError::new("pointer offset exceeds int32", token.span))?;
            let mut instruction = InstructionBuilder::new(Opcode::POINTER_ADD_IMMEDIATE);
            instruction.register(pointer);
            instruction.i32(offset);

            return function.emit(instruction, results, self.empty_span());
        }

        // parse one signed register byte offset and optional scale
        let offset = self.parse_register()?;
        let scale = if self.eat_token_if(TokenType::Comma) {
            Some(self.parse_u32()?)
        } else {
            None
        };

        // encode the register address calculation
        let opcode = if scale.is_some() {
            Opcode::POINTER_ADD_SCALED
        } else {
            Opcode::POINTER_ADD
        };
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(pointer);
        instruction.register(offset);
        if let Some(scale) = scale {
            instruction.u32(scale);
        }

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one signed distance between two pointers.
    fn parse_pointer_distance(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        // encode the signed byte distance
        let mut instruction = InstructionBuilder::new(Opcode::POINTER_DISTANCE);
        instruction.register(left);
        instruction.register(right);

        function.emit(instruction, results, self.empty_span())
    }
}
