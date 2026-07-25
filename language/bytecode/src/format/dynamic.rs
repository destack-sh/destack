use crate::{Opcode, RegisterSpan};
use destack_fir::format::{FormatError, FormatResult};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one dynamic value operation.
    pub(super) fn format_dynamic(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::DYNAMIC_BIND => self.format_dynamic_bind(),
            Opcode::DYNAMIC_TYPE => self.format_dynamic_type(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid dynamic opcode",
            }),
        }
    }

    /// Format one dynamic value construction.
    fn format_dynamic_bind(&mut self) -> FormatResult<()> {
        // decode result, payload, and dispatch table
        let (result, word_count) = self.register_span_id()?;
        let value = self.register_id()?;
        let table = self.relocation_text()?;

        // write the dynamic construction
        self.write_opcode("dynamic.bind")?;
        self.write_span(RegisterSpan::new(result, word_count))?;
        self.write_comma()?;
        self.write_register(value)?;
        self.write_comma()?;
        self.write_text(&table)
    }

    /// Format one dynamic runtime type access.
    fn format_dynamic_type(&mut self) -> FormatResult<()> {
        // decode one complete dynamic value
        let result = self.register_id()?;
        let (dynamic, word_count) = self.register_span_id()?;

        // write the runtime type projection
        self.write_opcode("dynamic.type")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_span(RegisterSpan::new(dynamic, word_count))
    }
}
