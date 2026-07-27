use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterSpan, RelocationTag,
    Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one task operation.
    pub(super) fn parse_task_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "task.resolve" => Opcode::TASK_RESOLVE,
            "task.start" => Opcode::TASK_START,
            "task.park" => Opcode::TASK_PARK,
            "task.cancel" => Opcode::TASK_CANCEL,
            "task.detach" => Opcode::TASK_DETACH,
            _ => return Err(ParseError::new("invalid task operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;

        match opcode {
            Opcode::TASK_RESOLVE => self.parse_task_resolve(&results, function),
            Opcode::TASK_START => self.parse_task_start(&results, function),
            Opcode::TASK_PARK => self.parse_task_park(&results, function),
            Opcode::TASK_CANCEL | Opcode::TASK_DETACH => {
                self.parse_task_handle(opcode, &results, function)
            }
            _ => unreachable!("task opcode selected above"),
        }
    }

    /// Parse one already completed task.
    fn parse_task_resolve(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register_span()?;
        self.eat_token(TokenType::Comma)?;
        let ty = self.parse_type_id()?;

        // encode the value through its linked Program type
        let mut instruction = InstructionBuilder::new(Opcode::TASK_RESOLVE);
        instruction.relocation(RelocationTag::TYPE, ty.0);
        instruction.span(value);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one eager task start.
    fn parse_task_start(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let continuation = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(Opcode::TASK_START);
        instruction.register(continuation);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse parking one waiter on a task.
    fn parse_task_park(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let task = self.parse_register()?;
        self.eat_token(TokenType::Comma)?;
        let waiter = self.parse_register()?;

        let mut instruction = InstructionBuilder::new(Opcode::TASK_PARK);
        instruction.register(task);
        instruction.register(waiter);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one task handle operation.
    fn parse_task_handle(
        &mut self,
        opcode: Opcode,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let task = self.parse_register()?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(task);

        function.emit(instruction, results, self.empty_span())
    }
}
