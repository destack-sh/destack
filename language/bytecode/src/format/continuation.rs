use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Opcode, RegisterSpan, RelocationTag};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one continuation operation.
    pub(super) fn format_continuation(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::CONTINUATION_NEW => self.format_continuation_new(),
            Opcode::CONTINUATION_DESTROY => self.format_continuation_destroy(),
            Opcode::CONTINUATION_RESUME | Opcode::CONTINUATION_COMPLETE => {
                self.format_continuation_execution(opcode)
            }
            _ => Err(FormatError::SyntaxError {
                message: "invalid continuation opcode",
            }),
        }
    }

    /// Format destruction of one continuation.
    fn format_continuation_destroy(&mut self) -> FormatResult<()> {
        let continuation = self.register_id()?;
        self.write_opcode("continuation.destroy")?;

        self.write_register(continuation)
    }

    /// Format one continuation execution until it yields or returns.
    fn format_continuation_execution(&mut self, opcode: Opcode) -> FormatResult<()> {
        // decode results, inputs, and branch destinations
        let (yielded, yielded_count) = self.register_span_id()?;
        let yielded = RegisterSpan::new(yielded, yielded_count);
        let replacement = self.register_id()?;
        let (returned, returned_count) = self.register_span_id()?;
        let returned = RegisterSpan::new(returned, returned_count);
        let continuation = self.register_id()?;
        let (value, value_count) = self.register_span_id()?;
        let value = RegisterSpan::new(value, value_count);
        let yielded_target = self.branch()?;
        let returned_target = self.branch()?;
        let unwind = self.branch()?;

        // write destinations followed by consumed values
        self.write_opcode(opcode.name().ok_or(FormatError::SyntaxError {
            message: "continuation opcode has no text form",
        })?)?;
        self.write_span(yielded)?;
        self.write_comma()?;
        self.write_result(replacement)?;
        self.write_comma()?;
        self.write_span(returned)?;
        self.write_comma()?;
        self.write_register(continuation)?;
        self.write_comma()?;
        self.write_span(value)?;
        write!(self.formatter, [space(), token("=>"), space()])?;
        self.write_label(yielded_target)?;
        write!(self.formatter, [space(), token("|"), space()])?;
        self.write_label(returned_target)?;
        write!(self.formatter, [space(), token("|"), space()])?;
        self.write_label(unwind)
    }

    /// Format one ready continuation.
    fn format_continuation_new(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let (target, relocation) = self.relocation_with_text()?;
        let captures = self.register_span()?;
        if relocation.tag != RelocationTag::FUNCTION {
            return Err(FormatError::SyntaxError {
                message: "continuation does not reference a function",
            });
        }

        // write the target and captured arguments
        self.write_opcode("continuation.new")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_text(&target)?;
        self.write_comma()?;
        self.write_span(captures)
    }

    /// Read one physical value span.
    fn register_span(&mut self) -> FormatResult<RegisterSpan> {
        let (start, word_count) = self.register_span_id()?;

        Ok(RegisterSpan::new(start, word_count))
    }
}
