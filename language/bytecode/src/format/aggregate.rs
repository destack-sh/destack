use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::Opcode;

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one aggregate or variant value operation.
    pub(super) fn format_aggregate(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::AGGREGATE => self.format_aggregate_new(),
            Opcode::FIELD_GET | Opcode::ELEMENT_GET | Opcode::VARIANT_PAYLOAD => {
                self.format_projection(opcode)
            }
            Opcode::FIELD_SET | Opcode::ELEMENT_SET => self.format_update(opcode),
            Opcode::VARIANT_NEW => self.format_variant_new(),
            Opcode::VARIANT_TAG => self.format_variant_tag(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid aggregate opcode",
            }),
        }
    }

    /// Format one aggregate construction.
    fn format_aggregate_new(&mut self) -> FormatResult<()> {
        self.declared_result_range()?;
        let ty = self.symbol()?;
        let fields = self.register_ids()?;

        // write fields in logical source order
        write!(
            self.formatter,
            [space(), token("="), space(), token("aggregate"), space()]
        )?;
        self.write_text(&ty)?;
        write!(self.formatter, [space(), token("(")])?;
        self.write_registers(&fields)?;
        self.write_token(")")
    }

    /// Format one field, element, or variant payload projection.
    fn format_projection(&mut self, opcode: Opcode) -> FormatResult<()> {
        self.declared_result_range()?;
        let (source, _) = self.register_range_id()?;
        let ty = self.symbol()?;
        let index = self.u32()?.to_string();

        // write one logical projection
        write!(self.formatter, [space(), token("="), space()])?;
        self.write_text(self.opcode_name(opcode)?)?;
        self.write_token(" ")?;
        self.write_register(source)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&ty)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&index)
    }

    /// Format one persistent aggregate field or element update.
    fn format_update(&mut self, opcode: Opcode) -> FormatResult<()> {
        self.declared_result_range()?;
        let (source, _) = self.register_range_id()?;
        let ty = self.symbol()?;
        let index = self.u32()?.to_string();
        let (value, _) = self.register_range_id()?;

        // write the original value and replacement component
        write!(self.formatter, [space(), token("="), space()])?;
        self.write_text(self.opcode_name(opcode)?)?;
        self.write_token(" ")?;
        self.write_register(source)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&ty)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&index)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(value)
    }

    /// Format one variant construction.
    fn format_variant_new(&mut self) -> FormatResult<()> {
        self.declared_result_range()?;
        let ty = self.symbol()?;
        let case = self.u32()?.to_string();
        let (payload, payload_word_count) = self.register_range_id()?;

        // write the case and payload when present
        write!(
            self.formatter,
            [space(), token("="), space(), token("variant.new"), space()]
        )?;
        self.write_text(&ty)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&case)?;
        if payload_word_count > 0 {
            write!(self.formatter, [token(","), space()])?;
            self.write_register(payload)?;
        }

        Ok(())
    }

    /// Format one variant discriminant projection.
    fn format_variant_tag(&mut self) -> FormatResult<()> {
        self.declared_result_range()?;
        let (variant, _) = self.register_range_id()?;
        let ty = self.symbol()?;

        // write the logical discriminant projection
        write!(
            self.formatter,
            [space(), token("="), space(), token("variant.tag"), space()]
        )?;
        self.write_register(variant)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&ty)
    }
}
