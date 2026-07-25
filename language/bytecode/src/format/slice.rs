use crate::{Opcode, RegisterSpan};
use destack_fir::format::{FormatError, FormatResult};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one slice operation.
    pub(super) fn format_slice(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::SLICE_VIEW => self.format_slice_view(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid slice opcode",
            }),
        }
    }

    /// Format one contiguous slice subview.
    fn format_slice_view(&mut self) -> FormatResult<()> {
        self.write_opcode("slice.view")?;

        // decode result and source slice ranges
        let (result, result_word_count) = self.register_span_id()?;
        let (source, source_word_count) = self.register_span_id()?;
        let stride = self.u32()?.to_string();

        // decode the dynamic subrange
        let start = self.register_id()?;
        let length = self.register_id()?;

        // write the complete slice operation
        self.write_span(RegisterSpan::new(result, result_word_count))?;
        self.write_comma()?;
        self.write_span(RegisterSpan::new(source, source_word_count))?;
        self.write_comma()?;
        self.write_text(&stride)?;
        self.write_comma()?;
        self.write_register(start)?;
        self.write_comma()?;
        self.write_register(length)
    }
}
