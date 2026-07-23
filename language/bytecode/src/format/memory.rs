use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{MemoryOperation, Opcode, Scalar, ValueType};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one canonical frame-slot value operation.
    pub(super) fn format_frame(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::FRAME_LOAD => self.format_frame_load(),
            Opcode::FRAME_STORE => self.format_frame_store(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid frame opcode",
            }),
        }
    }

    /// Format one canonical frame-slot load.
    fn format_frame_load(&mut self) -> FormatResult<()> {
        self.declared_result_range()?;
        let slot = format!("s{}", self.u32()?);

        // write the canonical frame slot
        write!(
            self.formatter,
            [space(), token("="), space(), token("frame.load"), space()]
        )?;
        self.write_text(&slot)
    }

    /// Format one canonical frame-slot store.
    fn format_frame_store(&mut self) -> FormatResult<()> {
        let slot = format!("s{}", self.u32()?);
        let (value, _) = self.register_range_id()?;

        // write the canonical frame slot and logical value
        write!(self.formatter, [token("frame.store"), space()])?;
        self.write_text(&slot)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(value)
    }

    /// Format one packed value load.
    pub(super) fn format_load(&mut self) -> FormatResult<()> {
        self.declared_result_range()?;
        let pointer = self.register_id()?;
        let ty = self.symbol()?;

        // write the pointer and Program storage type
        write!(
            self.formatter,
            [space(), token("="), space(), token("load"), space()]
        )?;
        self.write_register(pointer)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&ty)
    }

    /// Format one packed value store.
    pub(super) fn format_store(&mut self) -> FormatResult<()> {
        let pointer = self.register_id()?;
        let (value, _) = self.register_range_id()?;
        let ty = self.symbol()?;

        // write the pointer, value, and Program storage type
        write!(self.formatter, [token("store"), space()])?;
        self.write_register(pointer)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(value)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&ty)
    }

    /// Format one scalar load or store.
    pub(super) fn format_memory(
        &mut self,
        operation: MemoryOperation,
        scalar: Scalar,
    ) -> FormatResult<()> {
        let operation_name = operation.name();
        let scalar_name = scalar.name();

        // load one scalar value
        if operation == MemoryOperation::Load {
            self.result(ValueType::scalar(scalar))?;
            let pointer = self.register_id()?;
            write!(self.formatter, [space(), token("="), space()])?;
            self.write_text(operation_name)?;
            self.write_token(".")?;
            self.write_text(scalar_name)?;
            self.write_token(" ")?;
            self.write_register(pointer)
        }
        // store one scalar value
        else {
            let pointer = self.register_id()?;
            let value = self.register_id()?;
            self.write_text(operation_name)?;
            self.write_token(".")?;
            self.write_text(scalar_name)?;
            self.write_token(" ")?;
            self.write_register(pointer)?;
            write!(self.formatter, [token(","), space()])?;
            self.write_register(value)
        }
    }

    /// Format one byte-range operation.
    pub(super) fn format_bytes(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::COPY_BYTES | Opcode::MOVE_BYTES => self.format_byte_transfer(opcode),
            Opcode::FILL_BYTES => self.format_byte_fill(),
            Opcode::COMPARE_BYTES => self.format_byte_compare(),
            _ => Err(FormatError::SyntaxError {
                message: "invalid byte-range opcode",
            }),
        }
    }

    /// Format one byte copy or move.
    fn format_byte_transfer(&mut self, opcode: Opcode) -> FormatResult<()> {
        // decode the target, source, and byte length
        let target = self.register_id()?;
        let source = self.register_id()?;
        let byte_len = self.register_id()?;
        let name = self.opcode_name(opcode)?;

        // preserve source to target order in text
        self.write_text(name)?;
        self.write_token(" ")?;
        self.write_register(source)?;
        write!(self.formatter, [space(), token("->"), space()])?;
        self.write_register(target)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(byte_len)
    }

    /// Format one byte fill.
    fn format_byte_fill(&mut self) -> FormatResult<()> {
        // decode the complete fill range
        let target = self.register_id()?;
        let byte = self.register_id()?;
        let byte_len = self.register_id()?;
        let name = self.opcode_name(Opcode::FILL_BYTES)?;

        // write the fill range
        write!(self.formatter, [token(name), space()])?;
        self.write_register(target)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(byte)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(byte_len)
    }

    /// Format one byte comparison.
    fn format_byte_compare(&mut self) -> FormatResult<()> {
        // decode the signed result and comparison range
        self.result(ValueType::scalar(Scalar::Int32))?;
        let left = self.register_id()?;
        let right = self.register_id()?;
        let byte_len = self.register_id()?;
        let name = self.opcode_name(Opcode::COMPARE_BYTES)?;

        // write the comparison range
        write!(
            self.formatter,
            [space(), token("="), space(), token(name), space()]
        )?;
        self.write_register(left)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(right)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(byte_len)
    }

    /// Format one prefetch hint.
    pub(super) fn format_prefetch(&mut self, opcode: Opcode) -> FormatResult<()> {
        // decode the hinted pointer
        let pointer = self.register_id()?;
        let name = self.opcode_name(opcode)?;

        // write the prefetch hint
        self.write_text(name)?;
        self.write_token(" ")?;
        self.write_register(pointer)
    }
}
