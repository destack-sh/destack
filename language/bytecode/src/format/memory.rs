use destack_fir::format::{FormatError, FormatResult};

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
        self.write_opcode("store")?;
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
        self.write_opcode(operation.name())?;

        // load one scalar value
        if operation == MemoryOperation::Load {
            self.result()?;
            let pointer = self.register_id()?;
            self.write_comma()?;
            self.write_register(pointer)?;
        }
        // store one scalar value
        else {
            let pointer = self.register_id()?;
            let value = self.register_id()?;
            self.write_register(pointer)?;
            self.write_comma()?;
            self.write_register(value)?;
        }

        self.write_scalar_representation(scalar)
    }

    /// Format one byte range operation.
    pub(super) fn format_range(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            Opcode::MEMORY_COPY | Opcode::MEMORY_MOVE => self.format_transfer(opcode, false),
            Opcode::MEMORY_COPY_IMMEDIATE | Opcode::MEMORY_MOVE_IMMEDIATE => {
                self.format_transfer(opcode, true)
            }
            Opcode::MEMORY_FILL => self.format_fill(false),
            Opcode::MEMORY_FILL_IMMEDIATE => self.format_fill(true),
            Opcode::MEMORY_COMPARE => self.format_compare(false),
            Opcode::MEMORY_COMPARE_IMMEDIATE => self.format_compare(true),
            _ => Err(FormatError::SyntaxError {
                message: "invalid byte range opcode",
            }),
        }
    }

    /// Format one byte copy or move.
    fn format_transfer(&mut self, opcode: Opcode, is_immediate: bool) -> FormatResult<()> {
        let target = self.register_id()?;
        let source = self.register_id()?;
        let name = self.opcode_name(opcode)?;

        // write target, source, and byte length
        self.write_opcode(name)?;
        self.write_register(target)?;
        self.write_comma()?;
        self.write_register(source)?;
        self.write_comma()?;
        self.write_length(is_immediate)
    }

    /// Format one byte fill.
    fn format_fill(&mut self, is_immediate: bool) -> FormatResult<()> {
        let target = self.register_id()?;
        let byte = self.register_id()?;

        // write the complete fill range
        self.write_opcode("memory.fill")?;
        self.write_register(target)?;
        self.write_comma()?;
        self.write_register(byte)?;
        self.write_comma()?;
        self.write_length(is_immediate)
    }

    /// Format one byte comparison.
    fn format_compare(&mut self, is_immediate: bool) -> FormatResult<()> {
        let result = self.register_id()?;
        let left = self.register_id()?;
        let right = self.register_id()?;

        // write the comparison result and byte range
        self.write_opcode("memory.compare")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_register(left)?;
        self.write_comma()?;
        self.write_register(right)?;
        self.write_comma()?;
        self.write_length(is_immediate)
    }

    /// Format one register or immediate byte length.
    fn write_length(&mut self, is_immediate: bool) -> FormatResult<()> {
        if is_immediate {
            let length = self.u32()?.to_string();

            self.write_text(&length)
        } else {
            let length = self.register_id()?;

            self.write_register(length)
        }
    }

    /// Format one prefetch hint.
    pub(super) fn format_prefetch(&mut self, opcode: Opcode) -> FormatResult<()> {
        let pointer = self.register_id()?;
        let name = self.opcode_name(opcode)?;

        self.write_opcode(name)?;
        self.write_register(pointer)
    }
}
