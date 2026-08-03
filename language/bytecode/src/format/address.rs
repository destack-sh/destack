use crate::{Opcode, RegisterId, RegisterSpan};
use destack_fir::format::{FormatError, FormatResult};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one stable frame or global address.
    pub(super) fn format_address(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::FRAME_ADDRESS => self.format_frame_address(),
            Opcode::GLOBAL_ADDRESS_CONSTANT
            | Opcode::GLOBAL_ADDRESS_LOCAL
            | Opcode::GLOBAL_ADDRESS_SHARED => self.format_global_address(opcode),
            _ => Err(FormatError::SyntaxError {
                message: "invalid address opcode",
            }),
        }
    }

    /// Format one relative reference or native pointer arithmetic operation.
    pub(super) fn format_address_arithmetic(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::REFERENCE_ADD_IMMEDIATE | Opcode::POINTER_ADD_IMMEDIATE => {
                self.format_address_add_immediate(opcode)
            }
            Opcode::REFERENCE_ADD | Opcode::POINTER_ADD => self.format_address_add(opcode),
            Opcode::REFERENCE_ADD_SCALED | Opcode::POINTER_ADD_SCALED => {
                self.format_address_add_scaled(opcode)
            }
            Opcode::REFERENCE_DIFF | Opcode::POINTER_DIFF => self.format_address_diff(opcode),
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
    fn format_global_address(&mut self, opcode: Opcode) -> FormatResult<()> {
        let name = opcode.name().ok_or(FormatError::SyntaxError {
            message: "unnamed global address opcode",
        })?;

        self.write_opcode(name)?;
        self.result()?;
        let symbol = self.global_text()?;

        self.write_comma()?;
        self.write_text(&symbol)
    }

    /// Format one immediate address addition.
    fn format_address_add_immediate(&mut self, opcode: Opcode) -> FormatResult<()> {
        let result = self.register_id()?;
        let address = self.register_id()?;
        let offset = self.i32()?.to_string();

        // write the base and immediate byte displacement
        self.write_address_add(opcode, result, address)?;
        self.write_text(&offset)
    }

    /// Format one register address addition.
    fn format_address_add(&mut self, opcode: Opcode) -> FormatResult<()> {
        let result = self.register_id()?;
        let address = self.register_id()?;
        let offset = self.register_id()?;

        // write the base and register byte offset
        self.write_address_add(opcode, result, address)?;
        self.write_register(offset)
    }

    /// Format one scaled register address addition.
    fn format_address_add_scaled(&mut self, opcode: Opcode) -> FormatResult<()> {
        let result = self.register_id()?;
        let address = self.register_id()?;
        let offset = self.register_id()?;
        let scale = self.u32()?.to_string();

        // write the base, register offset, and static byte scale
        self.write_address_add(opcode, result, address)?;
        self.write_register(offset)?;
        self.write_comma()?;
        self.write_text(&scale)
    }

    /// Write one address addition through its first operand.
    fn write_address_add(
        &mut self,
        opcode: Opcode,
        result: RegisterId,
        address: RegisterId,
    ) -> FormatResult<()> {
        let name = if matches!(
            opcode,
            Opcode::REFERENCE_ADD_IMMEDIATE | Opcode::REFERENCE_ADD | Opcode::REFERENCE_ADD_SCALED
        ) {
            "reference.add"
        } else {
            "pointer.add"
        };
        self.write_opcode(name)?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(address)?;
        self.write_comma()
    }

    /// Format one signed byte offset between two addresses.
    fn format_address_diff(&mut self, opcode: Opcode) -> FormatResult<()> {
        let result = self.register_id()?;
        let address = self.register_id()?;
        let origin = self.register_id()?;
        let name = if opcode == Opcode::REFERENCE_DIFF {
            "reference.diff"
        } else {
            "pointer.diff"
        };

        // write the address and its origin
        self.write_opcode(name)?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(address)?;
        self.write_comma()?;
        self.write_register(origin)
    }
}
