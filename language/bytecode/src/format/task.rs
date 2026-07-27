use destack_fir::format::{FormatError, FormatResult};

use crate::{Opcode, RegisterSpan};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one task operation.
    pub(super) fn format_task(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::TASK_RESOLVE => self.format_task_resolve(),
            Opcode::TASK_START => self.format_task_start(),
            Opcode::TASK_PARK => self.format_task_park(),
            Opcode::TASK_CANCEL | Opcode::TASK_DETACH => self.format_task_handle(opcode),
            _ => Err(FormatError::SyntaxError {
                message: "invalid task opcode",
            }),
        }
    }

    /// Format one already completed task.
    fn format_task_resolve(&mut self) -> FormatResult<()> {
        let task = self.register_id()?;
        let ty = self.relocation_text()?;
        let (start, word_count) = self.register_span_id()?;
        let value = RegisterSpan::new(start, word_count);

        self.write_opcode("task.resolve")?;
        self.write_register(task)?;
        self.write_comma()?;
        self.write_span(value)?;
        self.write_comma()?;
        self.write_text(&ty)
    }

    /// Format one eager task start.
    fn format_task_start(&mut self) -> FormatResult<()> {
        let task = self.register_id()?;
        let continuation = self.register_id()?;

        self.write_opcode("task.start")?;
        self.write_register(task)?;
        self.write_comma()?;
        self.write_register(continuation)
    }

    /// Format parking one waiter on a task.
    fn format_task_park(&mut self) -> FormatResult<()> {
        let task = self.register_id()?;
        let waiter = self.register_id()?;

        self.write_opcode("task.park")?;
        self.write_register(task)?;
        self.write_comma()?;
        self.write_register(waiter)
    }

    /// Format one task handle operation.
    fn format_task_handle(&mut self, opcode: Opcode) -> FormatResult<()> {
        let task = self.register_id()?;
        let name = opcode.name().ok_or(FormatError::SyntaxError {
            message: "task opcode has no text form",
        })?;

        self.write_opcode(name)?;
        self.write_register(task)
    }
}
