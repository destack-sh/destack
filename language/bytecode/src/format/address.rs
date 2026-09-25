use crate::{Opcode, RegisterId, RegisterSpan};
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

    /// Format one address arithmetic operation.
    pub(super) fn format_address_arithmetic(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::ADDRESS_ADD_IMMEDIATE => self.format_address_add_immediate(),
            Opcode::ADDRESS_ADD => self.format_address_add(),
            Opcode::ADDRESS_ADD_SCALED => self.format_address_add_scaled(),
            Opcode::ADDRESS_DIFF => self.format_address_diff(),
            Opcode::ADDRESS_POINTER => self.format_address_rebase("address.pointer"),
            Opcode::ADDRESS_REFERENCE => self.format_address_rebase("address.reference"),
            _ => Err(FormatError::SyntaxError {
                message: "invalid address arithmetic opcode",
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

    /// Format one immediate address addition.
    fn format_address_add_immediate(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let address = self.register_id()?;
        let offset = self.i32()?.to_string();

        // write the base and immediate byte displacement
        self.write_address_add(result, address)?;
        self.write_text(&offset)
    }

    /// Format one register address addition.
    fn format_address_add(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let address = self.register_id()?;
        let offset = self.register_id()?;

        // write the base and register byte offset
        self.write_address_add(result, address)?;
        self.write_register(offset)
    }

    /// Format one scaled register address addition.
    fn format_address_add_scaled(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let address = self.register_id()?;
        let offset = self.register_id()?;
        let scale = self.u32()?.to_string();

        // write the base, register offset, and static byte scale
        self.write_address_add(result, address)?;
        self.write_register(offset)?;
        self.write_comma()?;
        self.write_text(&scale)
    }

    /// Write one address addition through its first operand.
    fn write_address_add(&mut self, result: RegisterId, address: RegisterId) -> FormatResult<()> {
        self.write_opcode("address.add")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(address)?;
        self.write_comma()
    }

    /// Format one rebase between a world reference and a native pointer.
    fn format_address_rebase(&mut self, name: &str) -> FormatResult<()> {
        let result = self.register_id()?;
        let address = self.register_id()?;

        // write the rebased address
        self.write_opcode(name)?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(address)
    }

    /// Format one signed byte offset between two addresses.
    fn format_address_diff(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let address = self.register_id()?;
        let origin = self.register_id()?;
        // write the address and its origin
        self.write_opcode("address.diff")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(address)?;
        self.write_comma()?;
        self.write_register(origin)
    }
}
