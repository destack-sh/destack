use crate::{Opcode, RegisterSpan, ValueType};
use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one dynamic value operation.
    pub(super) fn format_dynamic(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::DYNAMIC_BIND => self.format_dynamic_bind(),
            Opcode::DYNAMIC_READ => self.format_dynamic_read(),
            Opcode::TYPE_OF_DYNAMIC => self.format_type_of_dynamic(),
            Opcode::TYPE_OF_OBJECT => self.format_type_of_object(),
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

    /// Format one dynamic field read.
    fn format_dynamic_read(&mut self) -> FormatResult<()> {
        let (result, result_word_count) = self.register_span_id()?;
        let (dynamic, dynamic_word_count) = self.register_span_id()?;
        let slot = self.u16()?;
        let byte_len = self.u32()?;

        // write the selected field and exact result width
        self.write_opcode("dynamic.read")?;
        self.write_span(RegisterSpan::new(result, result_word_count))?;
        self.write_comma()?;
        self.write_span(RegisterSpan::new(dynamic, dynamic_word_count))?;
        self.write_text("[")?;
        self.write_text(&slot.to_string())?;
        self.write_text("]")?;
        self.write_comma()?;
        self.write_text(&byte_len.to_string())
    }

    /// Format one dynamic runtime type access.
    fn format_type_of_dynamic(&mut self) -> FormatResult<()> {
        // decode one complete dynamic value
        let result = self.register_id()?;
        let (dynamic, word_count) = self.register_span_id()?;

        // write the runtime type projection
        self.write_opcode("type.of.dynamic")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_span(RegisterSpan::new(dynamic, word_count))
    }

    /// Format one class object runtime type access.
    fn format_type_of_object(&mut self) -> FormatResult<()> {
        // decode the object reference and the dispatch field holding its table
        let result = self.register_id()?;
        let object = self.register_id()?;
        let reference = self.reference()?;
        let dispatch_offset = self.u32()?.to_string();

        // write the runtime type projection
        self.write_opcode("type.of.object")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(object)?;
        self.write_token(":")?;
        write!(self.formatter, [space()])?;
        write!(
            self.formatter,
            [&ValueType::reference(reference.kind(), reference.storage())]
        )?;
        self.write_token("[")?;
        self.write_text(&dispatch_offset)?;
        self.write_token("]")
    }
}
