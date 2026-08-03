use crate::{Opcode, RegisterId};
use destack_fir::format::{FormatError, FormatResult};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one pointer operation.
    pub(super) fn format_pointer(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::POINTER_ADD_IMMEDIATE => self.format_pointer_add_immediate(),
            Opcode::POINTER_ADD => self.format_pointer_add(),
            Opcode::POINTER_ADD_SCALED => self.format_pointer_add_scaled(),
            Opcode::POINTER_DIFF => self.format_pointer_diff(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid pointer opcode",
            }),
        }
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

    /// Format one signed byte offset between two pointers.
    fn format_pointer_diff(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let pointer = self.register_id()?;
        let origin = self.register_id()?;

        // write the pointer and its origin
        self.write_opcode("pointer.diff")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(pointer)?;
        self.write_comma()?;
        self.write_register(origin)
    }
}
