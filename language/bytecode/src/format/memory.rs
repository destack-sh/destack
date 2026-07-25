use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{MemoryOperation, Opcode, RegisterSpan, Scalar};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one packed value load.
    pub(super) fn format_load(&mut self) -> FormatResult<()> {
        self.write_opcode("load")?;
        self.result_span()?;
        let pointer = self.register_id()?;
        let byte_len = self.u32()?.to_string();

        // write the pointer and exact copied byte length
        self.write_comma()?;
        self.write_register(pointer)?;
        self.write_comma()?;
        self.write_text(&byte_len)
    }

    /// Format one packed value store.
    pub(super) fn format_store(&mut self) -> FormatResult<()> {
        let pointer = self.register_id()?;
        let (value, word_count) = self.register_span_id()?;
        let value = RegisterSpan::new(value, word_count);
        let byte_len = self.u32()?.to_string();

        // write the pointer, value, and exact copied byte length
        write!(self.formatter, [token("store"), space()])?;
        self.write_register(pointer)?;
        self.write_comma()?;
        self.write_span(value)?;
        self.write_comma()?;
        self.write_text(&byte_len)
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
            let name = format!("{operation_name}.{scalar_name}");
            self.write_opcode(&name)?;
            self.result()?;
            let pointer = self.register_id()?;
            self.write_comma()?;
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
        let result = self.register_id()?;
        let left = self.register_id()?;
        let right = self.register_id()?;
        let byte_len = self.register_id()?;
        let name = self.opcode_name(Opcode::COMPARE_BYTES)?;

        // write the comparison range
        self.write_opcode(name)?;
        self.write_register(result)?;
        self.write_comma()?;
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
