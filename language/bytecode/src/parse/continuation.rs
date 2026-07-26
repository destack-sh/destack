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
            _ => unreachable!("continuation opcode selected above"),
        }
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
