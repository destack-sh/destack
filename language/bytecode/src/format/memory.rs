use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::{Address, MemoryOperation, Prefetch, RegisterId, RegisterSpan, Scalar, Transfer};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one packed memory operation.
    pub(super) fn format_memory_range(
        &mut self,
        operation: MemoryOperation,
        address: Address,
        is_volatile: bool,
    ) -> FormatResult<()> {
        let volatility = if is_volatile { ".volatile" } else { "" };
        let name = format!("memory.{}{volatility}", operation.name());
        self.write_opcode(&name)?;

        // load one packed value
        if operation == MemoryOperation::Load {
            self.result_span()?;
            let register = self.register_id()?;
            let byte_len = self.u32()?.to_string();
            self.write_comma()?;
            self.write_address_register(address, register)?;
            self.write_comma()?;
            self.write_text(&byte_len)
        }
        // store one packed value
        else {
            let register = self.register_id()?;
            let (value, word_count) = self.register_span_id()?;
            let byte_len = self.u32()?.to_string();
            self.write_address_register(address, register)?;
            self.write_comma()?;
            self.write_span(RegisterSpan::new(value, word_count))?;
            self.write_comma()?;
            self.write_text(&byte_len)
        }
    }

    /// Format one scalar load or store.
    pub(super) fn format_memory(
        &mut self,
        operation: MemoryOperation,
        address: Address,
        scalar: Scalar,
        is_volatile: bool,
    ) -> FormatResult<()> {
        let volatility = if is_volatile { ".volatile" } else { "" };
        let name = format!("{}{volatility}.{}", operation.name(), scalar.name());
        self.write_opcode(&name)?;

        // load one scalar value
        if operation == MemoryOperation::Load {
            self.result()?;
            let register = self.register_id()?;
            self.write_comma()?;
            self.write_address_register(address, register)?;
        }
        // store one scalar value
        else {
            let register = self.register_id()?;
            let value = self.register_id()?;
            self.write_address_register(address, register)?;
            self.write_comma()?;
            self.write_register(value)?;
        }

        Ok(())
    }

    /// Format one byte copy or move.
    pub(super) fn format_transfer(
        &mut self,
        operation: Transfer,
        target_address: Address,
        source_address: Address,
        is_immediate: bool,
    ) -> FormatResult<()> {
        let target = self.register_id()?;
        let source = self.register_id()?;
        let name = format!("memory.{}", operation.name());

        // write target, source, and byte length
        self.write_opcode(&name)?;
        self.write_address_register(target_address, target)?;
        self.write_comma()?;
        self.write_address_register(source_address, source)?;
        self.write_comma()?;
        self.write_length(is_immediate)
    }

    /// Format one byte fill.
    pub(super) fn format_fill(&mut self, address: Address, is_immediate: bool) -> FormatResult<()> {
        let target = self.register_id()?;
        let byte = self.register_id()?;

        // write the complete fill range
        self.write_opcode("memory.fill")?;
        self.write_address_register(address, target)?;
        self.write_comma()?;
        self.write_register(byte)?;
        self.write_comma()?;
        self.write_length(is_immediate)
    }

    /// Format one byte comparison.
    pub(super) fn format_compare(
        &mut self,
        left_address: Address,
        right_address: Address,
        is_immediate: bool,
    ) -> FormatResult<()> {
        let result = self.register_id()?;
        let left = self.register_id()?;
        let right = self.register_id()?;

        // write the comparison result and byte range
        self.write_opcode("memory.compare")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_address_register(left_address, left)?;
        self.write_comma()?;
        self.write_address_register(right_address, right)?;
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
    pub(super) fn format_prefetch(
        &mut self,
        operation: Prefetch,
        address: Address,
    ) -> FormatResult<()> {
        let register = self.register_id()?;
        let name = format!("prefetch.{}", operation.name());

        self.write_opcode(&name)?;
        self.write_address_register(address, register)
    }

    /// Write one relative reference or process-local pointer register.
    pub(super) fn write_address_register(
        &mut self,
        address: Address,
        register: RegisterId,
    ) -> FormatResult<()> {
        if address == Address::Pointer {
            write!(self.formatter, [token("pointer"), space()])?;
        }

        self.write_register(register)
    }
}
