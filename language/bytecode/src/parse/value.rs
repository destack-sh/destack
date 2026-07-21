use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterId, RegisterRange, Scalar,
    Symbol, Token, TokenType, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one register value operation.
    pub(super) fn parse_value_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match Opcode::from_name(name) {
            Some(Opcode::MOVE) => self.parse_move(token, results, function),
            Some(Opcode::SELECT) => self.parse_select(token, results, function),
            Some(Opcode::TYPE_ID) => self.parse_type_id(results, function),
            Some(Opcode::EQUAL) => self.parse_equal(token, results, function),
            _ => Err(ParseError::new("unknown value operation", token.span)),
        }
    }

    /// Parse one logical value move.
    fn parse_move(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let input = self.parse_register()?;
        let ty = function
            .value_type(input)
            .ok_or_else(|| ParseError::new("move reads an uninitialized register", token.span))?;
        let mut instruction = if ty.word_count() == 1 {
            InstructionBuilder::new(Opcode::MOVE)
        } else {
            InstructionBuilder::new(Opcode::MOVE_RANGE)
        };

        // encode one word directly and wider values as contiguous ranges
        if ty.word_count() == 1 {
            instruction.register(input);
        } else {
            instruction.range(RegisterRange::new(input, ty.word_count()));
        }

        function.emit(instruction, results, &[ty], self.empty_span())
    }

    /// Parse one scalar value selection.
    fn parse_select(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let condition = self.parse_register()?;
        if !function.has_type(condition, ValueType::scalar(Scalar::Boolean)) {
            return Err(ParseError::new(
                "select condition is not boolean",
                token.span,
            ));
        }
        self.eat_token(TokenType::Comma)?;
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        let ty = function
            .value_type(left)
            .ok_or_else(|| ParseError::new("select reads an uninitialized register", token.span))?;
        if !function.has_type(right, ty) {
            return Err(ParseError::new(
                "select values must have the same type",
                token.span,
            ));
        }

        // encode the selected values in source order
        let is_range = ty.word_count() > 1;
        let opcode = if is_range {
            Opcode::SELECT_RANGE
        } else {
            Opcode::SELECT
        };
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(condition);
        if is_range {
            instruction.range(RegisterRange::new(left, ty.word_count()));
            instruction.range(RegisterRange::new(right, ty.word_count()));
        } else {
            instruction.register(left);
            instruction.register(right);
        }

        function.emit(instruction, results, &[ty], self.empty_span())
    }

    /// Parse one linked runtime type identity.
    fn parse_type_id(
        &mut self,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let ty = self.parse_type_name()?;
        let mut instruction = InstructionBuilder::new(Opcode::TYPE_ID);
        instruction.symbol(Symbol::ty(ty.0));

        function.emit(
            instruction,
            results,
            &[ValueType::type_id()],
            self.empty_span(),
        )
    }

    /// Parse exact equality between matching one-register values.
    fn parse_equal(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let left = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        let ty = function
            .value_type(left)
            .ok_or_else(|| ParseError::new("equal reads an uninitialized value", token.span))?;
        if ty.word_count() != 1 || !function.has_type(right, ty) {
            return Err(ParseError::new(
                "equal requires matching one-register values",
                token.span,
            ));
        }

        // encode both exact value operands
        let mut instruction = InstructionBuilder::new(Opcode::EQUAL);
        instruction.register(left);
        instruction.register(right);

        function.emit(
            instruction,
            results,
            &[ValueType::scalar(Scalar::Boolean)],
            self.empty_span(),
        )
    }
}
