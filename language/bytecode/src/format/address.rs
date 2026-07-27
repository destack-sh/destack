use crate::{Opcode, RegisterSpan};
use destack_fir::format::{FormatError, FormatResult};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one stable frame or global address.
    pub(super) fn format_address(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::FRAME_ADDRESS => self.format_frame_address(),
            Opcode::GLOBAL_ADDRESS => self.format_global_address(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid address opcode",
            }),
        }
    }

    /// Format one frame address.
    fn format_frame_address(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let (register, word_count) = self.register_span_id()?;
        let value = RegisterSpan::new(register, word_count);

        self.write_opcode("frame.address")?;
        self.write_result(result)?;
        self.write_comma()?;
        self.write_span(value)
    }

    /// Format one global address.
    fn format_global_address(&mut self) -> FormatResult<()> {
        self.write_opcode("global.address")?;
        self.result()?;
        let symbol = self.relocation_text()?;

        self.write_comma()?;
        self.write_text(&symbol)
    }
}
