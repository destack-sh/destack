use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Opcode, Scalar, ValueType};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one pointer operation.
    pub(super) fn format_pointer(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::GLOBAL_ADDRESS => self.format_global_address(),
            Opcode::FRAME_ADDRESS => self.format_frame_address(),
            Opcode::POINTER_OFFSET => self.format_pointer_offset(),
            Opcode::POINTER_INDEX => self.format_pointer_index(),
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
        self.reference()?;

        self.write_result(result, ValueType::pointer())?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("reference.pointer"),
                space()
            ]
        )?;
        self.write_register(reference)
    }

    /// Format one linked global address.
    fn format_global_address(&mut self) -> FormatResult<()> {
        self.result(ValueType::pointer())?;
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
        self.result(ValueType::pointer())?;
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

    /// Format one constant pointer offset.
    fn format_pointer_offset(&mut self) -> FormatResult<()> {
        self.result(ValueType::pointer())?;
        let pointer = self.register_id()?;
        let offset = self.i32()?.to_string();

        // write the base and byte displacement
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("pointer.offset"),
                space()
            ]
        )?;
        self.write_register(pointer)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&offset)
    }

    /// Format one scaled pointer index.
    fn format_pointer_index(&mut self) -> FormatResult<()> {
        self.result(ValueType::pointer())?;
        let pointer = self.register_id()?;
        let index = self.register_id()?;
        let stride = self.u32()?.to_string();

        // write the scaled address calculation
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("pointer.index"),
                space()
            ]
        )?;
        self.write_register(pointer)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(index)?;
        write!(self.formatter, [token(","), space(), token("stride(")])?;
        self.write_text(&stride)?;
        self.write_token(")")
    }

    /// Format one signed distance between two pointers.
    fn format_pointer_distance(&mut self) -> FormatResult<()> {
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
                token("pointer.distance"),
                space()
            ]
        )?;
        self.write_register(left)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(right)
    }
}
