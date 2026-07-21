use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Opcode, Scalar, ValueType};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one address operation.
    pub(super) fn format_address(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::GLOBAL_ADDRESS => self.format_global_address(),
            Opcode::FRAME_ADDRESS => self.format_frame_address(),
            Opcode::ADDRESS_OFFSET => self.format_address_offset(),
            Opcode::ADDRESS_ELEMENT => self.format_address_element(),
            Opcode::ADDRESS_DISTANCE => self.format_address_distance(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid address opcode",
            }),
        }
    }

    /// Format one linked global address.
    fn format_global_address(&mut self) -> FormatResult<()> {
        self.result(ValueType::address())?;
        let symbol = self.symbol()?;

        // write the linked global
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("global.address"),
                space()
            ]
        )?;
        self.write_text(&symbol)
    }

    /// Format one frame-slot address.
    fn format_frame_address(&mut self) -> FormatResult<()> {
        self.result(ValueType::address())?;
        let slot = format!("s{}", self.u32()?);

        // write the fixed frame slot
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("frame.address"),
                space()
            ]
        )?;
        self.write_text(&slot)
    }

    /// Format one constant address offset.
    fn format_address_offset(&mut self) -> FormatResult<()> {
        self.result(ValueType::address())?;
        let address = self.register_id()?;
        let offset = self.i32()?.to_string();

        // write the base and byte displacement
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("address.offset"),
                space()
            ]
        )?;
        self.write_register(address)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&offset)
    }

    /// Format one scaled element address.
    fn format_address_element(&mut self) -> FormatResult<()> {
        self.result(ValueType::address())?;
        let address = self.register_id()?;
        let index = self.register_id()?;
        let stride = self.u32()?.to_string();

        // write the scaled address calculation
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("address.element"),
                space()
            ]
        )?;
        self.write_register(address)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(index)?;
        write!(self.formatter, [token(","), space(), token("stride(")])?;
        self.write_text(&stride)?;
        self.write_token(")")
    }

    /// Format one signed distance between two addresses.
    fn format_address_distance(&mut self) -> FormatResult<()> {
        self.result(ValueType::scalar(Scalar::Int64))?;
        let left = self.register_id()?;
        let right = self.register_id()?;

        // write both address operands
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("address.distance"),
                space()
            ]
        )?;
        self.write_register(left)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(right)
    }
}
