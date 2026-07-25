use crate::{Opcode, RegisterId, RegisterSpan};
use destack_fir::format::{FormatError, FormatResult};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one pointer operation.
    pub(super) fn format_pointer(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::ADDRESS => self.format_address(),
            Opcode::GLOBAL_ADDRESS => self.format_global_address(),
            Opcode::POINTER_ADD_IMMEDIATE => self.format_pointer_add_immediate(),
            Opcode::POINTER_ADD => self.format_pointer_add(),
            Opcode::POINTER_ADD_SCALED => self.format_pointer_add_scaled(),
            Opcode::POINTER_DISTANCE => self.format_pointer_distance(),
            Opcode::REFERENCE_POINTER => self.format_reference_pointer(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid pointer opcode",
            }),
        }
    }

    /// Format one stable heap reference projection.
    fn format_reference_pointer(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let reference = self.register_id()?;
        let reference_type = self.reference()?;

        self.write_reference_opcode("reference.pointer", reference_type)?;
        self.write_result(result)?;
        self.write_comma()?;
        self.write_register(reference)
    }

    /// Format one linked global address.
    fn format_global_address(&mut self) -> FormatResult<()> {
        self.write_opcode("global.address")?;
        self.result()?;
        let symbol = self.relocation_text()?;

        // write the linked global
        self.write_comma()?;
        self.write_text(&symbol)
    }

    /// Format the stable address of one register value.
    fn format_address(&mut self) -> FormatResult<()> {
        self.write_opcode("address")?;
        self.result()?;
        let (register, word_count) = self.register_span_id()?;
        let value = RegisterSpan::new(register, word_count);

        // write the addressable register value
        self.write_comma()?;
        self.write_span(value)
    }

    /// Format one immediate pointer addition.
    fn format_pointer_add_immediate(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let pointer = self.register_id()?;
        let offset = self.i32()?.to_string();

        // write the base and immediate byte displacement
        self.write_pointer_add(result, pointer)?;
        self.write_text(&offset)
    }

    /// Format one register pointer addition.
    fn format_pointer_add(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let pointer = self.register_id()?;
        let offset = self.register_id()?;

        // write the base and register byte offset
        self.write_pointer_add(result, pointer)?;
        self.write_register(offset)
    }

    /// Format one scaled register pointer addition.
    fn format_pointer_add_scaled(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let pointer = self.register_id()?;
        let offset = self.register_id()?;
        let scale = self.u32()?.to_string();

        // write the base, register offset, and static byte scale
        self.write_pointer_add(result, pointer)?;
        self.write_register(offset)?;
        self.write_comma()?;
        self.write_text(&scale)
    }

    /// Write one pointer addition through its first operand.
    fn write_pointer_add(&mut self, result: RegisterId, pointer: RegisterId) -> FormatResult<()> {
        self.write_opcode("pointer.add")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(pointer)?;
        self.write_comma()
    }

    /// Format one signed distance between two pointers.
    fn format_pointer_distance(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let left = self.register_id()?;
        let right = self.register_id()?;

        // write both pointer operands
        self.write_opcode("pointer.distance")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(left)?;
        self.write_comma()?;
        self.write_register(right)
    }
}
