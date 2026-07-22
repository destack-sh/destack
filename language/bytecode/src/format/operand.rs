use destack_fir::format::{FormatError, FormatResult};

use crate::{
    CodeOffset, CounterId, DynamicRelocation, Error, Label, ReferenceType, RegisterId,
    RegisterRange, SamplerId, Scalar, Symbol, ValueType, VectorType,
};

use super::instruction::InstructionFormatter;

impl<'code> InstructionFormatter<'code, '_, '_> {
    /// Read one register id operand.
    pub(super) fn register_id(&mut self) -> FormatResult<RegisterId> {
        self.operands.register().map_err(FormatError::from)
    }

    /// Read one counted register list as physical ids.
    pub(super) fn register_ids(&mut self) -> FormatResult<Vec<RegisterId>> {
        let registers = self.operands.registers().map_err(FormatError::from)?;

        Ok(registers.collect())
    }

    /// Read one packed register range as logical value ids.
    pub(super) fn register_value_ids(&mut self) -> FormatResult<Vec<RegisterId>> {
        let (start, word_count) = self.register_range_id()?;
        let range = RegisterRange::new(start, word_count);
        let registers = self.formatter.context().register_values(range)?;

        Ok(registers)
    }

    /// Read one contiguous register range id and width.
    pub(super) fn register_range_id(&mut self) -> FormatResult<(RegisterId, u16)> {
        let range = self.operands.range().map_err(FormatError::from)?;

        Ok((range.start, range.word_count))
    }

    /// Read one fixed-width vector type.
    pub(super) fn vector_type(&mut self) -> FormatResult<VectorType> {
        self.operands.vector_type().map_err(FormatError::from)
    }

    /// Read one scalar representation operand.
    pub(super) fn scalar(&mut self) -> FormatResult<Scalar> {
        self.operands.scalar().map_err(FormatError::from)
    }

    /// Read one reference representation operand.
    pub(super) fn reference(&mut self) -> FormatResult<ReferenceType> {
        self.operands.reference().map_err(FormatError::from)
    }

    /// Read one complete bytecode value type operand.
    pub(super) fn value_type(&mut self) -> FormatResult<ValueType> {
        self.operands.value_type().map_err(FormatError::from)
    }

    /// Read one counted unsigned 16-bit list.
    pub(super) fn u16_list(&mut self) -> FormatResult<Vec<u16>> {
        let values = self.operands.u16s().map_err(FormatError::from)?;

        Ok(values.collect())
    }

    /// Read one counted unsigned 64-bit list.
    pub(super) fn u64_list(&mut self) -> FormatResult<Vec<u64>> {
        let values = self.operands.u64s().map_err(FormatError::from)?;

        Ok(values.collect())
    }

    /// Read one relocated symbol name.
    pub(super) fn symbol(&mut self) -> FormatResult<String> {
        let target = self.symbol_target()?;

        self.formatter
            .context()
            .relocation_name(target)
            .map(str::to_string)
    }

    /// Read one relocated symbol name and target.
    pub(super) fn symbol_with_target(&mut self) -> FormatResult<(String, Symbol)> {
        let target = self.symbol_target()?;
        let name = self
            .formatter
            .context()
            .relocation_name(target)?
            .to_string();

        Ok((name, target))
    }

    /// Read one relocated symbol target.
    pub(super) fn symbol_target(&mut self) -> FormatResult<Symbol> {
        let offset = self.operand_offset();
        let target = self.formatter.context().relocation(offset)?;
        self.u32()?;

        Ok(target)
    }

    /// Read one dynamic dispatch table relocation.
    pub(super) fn dynamic_relocation(&mut self) -> FormatResult<DynamicRelocation> {
        let offset = self.operand_offset();
        let relocation = self.formatter.context().dynamic_relocation(offset)?;
        self.u32()?;

        Ok(relocation)
    }

    /// Return the next operand's function-local byte offset.
    pub(super) fn operand_offset(&self) -> CodeOffset {
        let header_byte_len = self.instruction.byte_len() - self.instruction.operand_bytes().len();

        CodeOffset(
            self.instruction_offset.0 + header_byte_len as u32 + self.operands.byte_offset() as u32,
        )
    }

    /// Read one relative branch label.
    pub(super) fn branch(&mut self) -> FormatResult<Label> {
        let displacement = self.i32()?;
        let label = self.formatter.context().branch_label(
            self.instruction_offset,
            self.instruction.byte_len(),
            displacement,
        )?;

        Ok(label)
    }

    /// Read one unsigned 16-bit operand.
    pub(super) fn u16(&mut self) -> FormatResult<u16> {
        self.operands.u16().map_err(FormatError::from)
    }

    /// Read one unsigned 32-bit operand.
    pub(super) fn u32(&mut self) -> FormatResult<u32> {
        self.operands.u32().map_err(FormatError::from)
    }

    /// Read one function-local profile counter.
    pub(super) fn counter(&mut self) -> FormatResult<CounterId> {
        self.operands.counter().map_err(FormatError::from)
    }

    /// Read one function-local profile sampler.
    pub(super) fn sampler(&mut self) -> FormatResult<SamplerId> {
        self.operands.sampler().map_err(FormatError::from)
    }

    /// Read one signed 32-bit operand.
    pub(super) fn i32(&mut self) -> FormatResult<i32> {
        self.operands.i32().map_err(FormatError::from)
    }

    /// Read one unsigned 64-bit operand.
    pub(super) fn u64(&mut self) -> FormatResult<u64> {
        self.operands.u64().map_err(FormatError::from)
    }

    /// Read one unsigned 128-bit operand.
    pub(super) fn u128(&mut self) -> FormatResult<u128> {
        self.operands.u128().map_err(FormatError::from)
    }
}

impl From<Error> for FormatError {
    /// Convert malformed bytecode into a formatter syntax error.
    fn from(_error: Error) -> Self {
        Self::SyntaxError {
            message: "instruction contains malformed operands",
        }
    }
}
