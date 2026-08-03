use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one register value operation.
    pub(super) fn parse_value_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "move" => self.parse_move(function),
            "select" => self.parse_select(function),
            "equal" => self.parse_equal(function),
            "equal.bytes" => self.parse_equal_bytes(function),
            _ => Err(ParseError::new("unknown value operation", token.span)),
        }
    }

    /// Parse one logical value move.
    fn parse_move(&mut self, function: &mut FunctionParser) -> ParseResult<()> {
        let result = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let input = self.parse_register_span()?;
        let mut instruction = if input.word_count == 1 {
            InstructionBuilder::new(Opcode::MOVE)
        } else {
            InstructionBuilder::new(Opcode::MOVE_RANGE)
        };

        // encode one word directly and wider values as contiguous ranges
        if input.word_count == 1 {
            instruction.register(input.start);
        } else {
            instruction.span(input);
        }

        function.emit(instruction, &[result], self.empty_span())
    }

    /// Parse one scalar value selection.
    fn parse_select(&mut self, function: &mut FunctionParser) -> ParseResult<()> {
        let result = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let condition = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let left = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register_span()?;

        // encode the selected values in source order
        let is_range = left.word_count > 1;
        let opcode = if is_range {
            Opcode::SELECT_RANGE
        } else {
            Opcode::SELECT
        };
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(condition);
        if is_range {
            instruction.span(left);
            instruction.span(right);
        } else {
            instruction.register(left.start);
            instruction.register(right.start);
        }

        function.emit(instruction, &[result], self.empty_span())
    }

    /// Parse exact equality between matching one-register values.
    fn parse_equal(&mut self, function: &mut FunctionParser) -> ParseResult<()> {
        let result = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        // encode both exact value operands
        let mut instruction = InstructionBuilder::new(Opcode::EQUAL);
        instruction.register(left);
        instruction.register(right);

        let result = RegisterSpan::new(result, 1);

        function.emit(instruction, &[result], self.empty_span())
    }

    /// Parse raw byte equality between matching values.
    fn parse_equal_bytes(&mut self, function: &mut FunctionParser) -> ParseResult<()> {
        let result = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let left = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let byte_len = self.parse_u32()?;

        // encode both values and their exact represented byte length
        let mut instruction = InstructionBuilder::new(Opcode::EQUAL_BYTES);
        instruction.span(left);
        instruction.span(right);
        instruction.u32(byte_len);
        let result = RegisterSpan::new(result, 1);

        function.emit(instruction, &[result], self.empty_span())
    }
}
