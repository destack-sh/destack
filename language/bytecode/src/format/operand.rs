use destack_fir::format::{FormatError, FormatResult};

use crate::{
    CodeOffset, CounterId, Error, Label, ReferenceType, RegisterId, RegisterRange, SamplerId,
    Scalar, Symbol, ValueType, VectorType,
};

use super::instruction::InstructionFormatter;

impl<'code> InstructionFormatter<'code, '_, '_> {
    /// Read one register id operand.
    pub(super) fn register_id(&mut self) -> FormatResult<RegisterId> {
        self.operands.register().map_err(FormatError::from)
    }

    /// Read one counted register list as physical ids.
    pub(super) fn register_ids(&mut self) -> FormatResult<Vec<RegisterId>> {
        self.operands.registers().map_err(FormatError::from)
    }

    /// Read one packed register range as logical value ids.
    pub(super) fn register_value_ids(&mut self) -> FormatResult<Vec<RegisterId>> {
        let (start, word_count) = self.register_range_id()?;
        let range = RegisterRange::new(start, word_count);
        let registers = self.formatter.context().register_values(range)?;

        Ok(registers)
    }

    /// Read one packed register range and require the expected logical value types.
    pub(super) fn typed_register_ids(
        &mut self,
        types: &[ValueType],
    ) -> FormatResult<Vec<RegisterId>> {
        let (start, word_count) = self.register_range_id()?;
        let expected_word_count = types.iter().map(|ty| ty.word_count()).sum::<u16>();

        // require the encoded range to cover every logical value
        if word_count != expected_word_count {
            return Err(FormatError::SyntaxError {
                message: "instruction register range does not match its value types",
            });
        }
        let range = RegisterRange::new(start, word_count);
        let registers = self.formatter.context().register_values(range)?;

        // require the exact logical value count
        if registers.len() != types.len() {
            return Err(FormatError::SyntaxError {
                message: "instruction register values do not match their expected types",
            });
        }

        // require every logical value to retain its expected type
        for (register, expected) in registers.iter().zip(types) {
            let actual = self.formatter.context().register_type(*register)?;
            if actual != *expected {
                return Err(FormatError::SyntaxError {
                    message: "instruction register values do not match their expected types",
                });
            }
        }

        Ok(registers)
    }

    /// Read one contiguous register range id and width.
    pub(super) fn register_range_id(&mut self) -> FormatResult<(RegisterId, u16)> {
        let range = self.operands.range().map_err(FormatError::from)?;

        Ok((range.start, range.word_count))
    }

    /// Read one fixed-width vector type.
    pub(super) fn vector_type(&mut self) -> FormatResult<VectorType> {
        let bytes = self.operands.take::<4>().map_err(FormatError::from)?;
        let scalar = Scalar::from_code(bytes[0]).ok_or(FormatError::SyntaxError {
            message: "vector operand has an invalid scalar",
        })?;
        let lane_count = u16::from_le_bytes([bytes[2], bytes[3]]);

        Ok(VectorType::new(scalar, lane_count))
    }

    /// Read one scalar representation operand.
    pub(super) fn scalar(&mut self) -> FormatResult<Scalar> {
        Scalar::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
            message: "instruction has an invalid scalar operand",
        })
    }

    /// Read one reference representation operand.
    pub(super) fn reference(&mut self) -> FormatResult<ReferenceType> {
        self.operands.reference().map_err(FormatError::from)
    }

    /// Read one complete bytecode value type operand.
    pub(super) fn value_type(&mut self) -> FormatResult<ValueType> {
        let bytes = self
            .operands
            .take::<{ ValueType::BYTE_LEN }>()
            .map_err(FormatError::from)?;
        let ty = ValueType::from_bytes(bytes).ok_or(FormatError::SyntaxError {
            message: "instruction contains an invalid value type",
        })?;

        Ok(ty)
    }

    /// Read one counted unsigned 16-bit list.
    pub(super) fn u16_list(&mut self) -> FormatResult<Vec<u16>> {
        self.operands.u16s().map_err(FormatError::from)
    }

    /// Read one counted unsigned 64-bit list.
    pub(super) fn u64_list(&mut self) -> FormatResult<Vec<u64>> {
        self.operands.u64s().map_err(FormatError::from)
    }

    /// Read one relocated symbol name.
    pub(super) fn symbol(&mut self) -> FormatResult<String> {
        self.symbol_with_target().map(|(name, _)| name)
    }

    /// Read one relocated symbol name and target.
    pub(super) fn symbol_with_target(&mut self) -> FormatResult<(String, Symbol)> {
        let offset = self.symbol_offset();
        let target = self.formatter.context().relocation(offset)?;
        let name = self
            .formatter
            .context()
            .relocation_name(target)?
            .to_string();
        self.u32()?;

        Ok((name, target))
    }

    /// Return one symbol operand's function-local byte offset.
    pub(super) fn symbol_offset(&self) -> CodeOffset {
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
