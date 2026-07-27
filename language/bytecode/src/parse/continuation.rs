use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, RelocationTag,
    Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one continuation operation.
    pub(super) fn parse_continuation_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "continuation.new" => Opcode::CONTINUATION_NEW,
            "continuation.destroy" => Opcode::CONTINUATION_DESTROY,
            "continuation.resume" => Opcode::CONTINUATION_RESUME,
            "continuation.complete" => Opcode::CONTINUATION_COMPLETE,
            _ => {
                return Err(ParseError::new(
                    "invalid continuation operation",
                    token.span,
                ));
            }
        };
        let results = self.parse_definitions(opcode)?;

        match opcode {
            Opcode::CONTINUATION_NEW => self.parse_continuation_new(&results, function),
            Opcode::CONTINUATION_DESTROY => self.parse_continuation_destroy(&results, function),
            Opcode::CONTINUATION_RESUME | Opcode::CONTINUATION_COMPLETE => {
                self.parse_continuation_execution(opcode, &results, function)
            }
            _ => unreachable!("continuation opcode selected above"),
        }
    }

    /// Parse destruction of one continuation.
    fn parse_continuation_destroy(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let continuation = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(Opcode::CONTINUATION_DESTROY);
        instruction.register(continuation);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one continuation execution until it yields or returns.
    fn parse_continuation_execution(
        &mut self,
        opcode: Opcode,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let continuation = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register_span()?;
        self.eat_token(TokenType::FatArrow)?;
        let yielded = self.parse_label()?;
        self.eat_token(TokenType::Pipe)?;
        let returned = self.parse_label()?;
        self.eat_token(TokenType::Pipe)?;
        let unwind = self.parse_label()?;

        // encode the continuation, value, and outcome destinations
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(continuation);
        instruction.span(value);
        instruction.branch(yielded);
        instruction.branch(returned);
        instruction.branch(unwind);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one ready continuation.
    fn parse_continuation_new(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let target = self.parse_function_id()?;
        self.eat_token(TokenType::Comma)?;
        let captures = self.parse_continuation_value()?;

        // encode the target and captured arguments
        let mut instruction = InstructionBuilder::new(Opcode::CONTINUATION_NEW);
        instruction.relocation(RelocationTag::FUNCTION, target.0);
        instruction.span(captures);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one optional physical value span.
    fn parse_continuation_value(&mut self) -> ParseResult<RegisterSpan> {
        if self.eat_name_if("_") {
            Ok(RegisterSpan::empty())
        } else {
            self.parse_register_span()
        }
    }
}
