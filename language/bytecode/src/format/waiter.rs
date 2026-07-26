use destack_fir::format::{FormatError, FormatResult};

use crate::{Opcode, RegisterSpan};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one waiter operation.
    pub(super) fn format_waiter(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::WAITER_QUEUE => self.format_waiter_queue(),
            Opcode::WAITER_CANCEL => self.format_waiter_cancel(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid waiter opcode",
            }),
        }
    }

    /// Format one waiter queue operation.
    fn format_waiter_queue(&mut self) -> FormatResult<()> {
        let waiter = self.register_id()?;
        let ty = self.relocation_text()?;
        let (start, word_count) = self.register_span_id()?;
        let value = RegisterSpan::new(start, word_count);

        // write the waiter, result value, and its Program type
        self.write_opcode("waiter.queue")?;
        self.write_register(waiter)?;
        self.write_comma()?;
        self.write_span(value)?;
        self.write_comma()?;
        self.write_text(&ty)
    }

    /// Format one waiter cancellation operation.
    fn format_waiter_cancel(&mut self) -> FormatResult<()> {
        let waiter = self.register_id()?;

        self.write_opcode("waiter.cancel")?;
        self.write_register(waiter)
    }
}
