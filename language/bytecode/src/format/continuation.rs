use destack_fir::format::{FormatError, FormatResult};

use crate::{Opcode, RegisterSpan, RelocationTag};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one continuation operation.
    pub(super) fn format_continuation(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::CONTINUATION_NEW => self.format_continuation_new(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid continuation opcode",
            }),
        }
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
