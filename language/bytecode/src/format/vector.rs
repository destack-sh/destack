use tspp_fir::format::{FormatError, FormatResult};

use crate::{
    ConvertMode, FloatOperation, IntegerOperation, ReduceOperation, RegisterId, RegisterSpan,
    ValueType, VectorOperation, VectorType,
};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one fixed-width vector instruction.
    pub(super) fn format_vector(&mut self, operation: VectorOperation) -> FormatResult<()> {
        match operation {
            VectorOperation::Splat => self.format_vector_splat(),
            VectorOperation::Insert => self.format_vector_insert(),
            VectorOperation::Extract => self.format_vector_extract(),
            VectorOperation::Shuffle => self.format_vector_shuffle(),
            VectorOperation::Element => self.format_vector_element(),
            VectorOperation::Compare => self.format_vector_compare(),
            VectorOperation::Select => self.format_vector_select(),
            VectorOperation::Reduce => self.format_vector_reduce(),
            VectorOperation::Convert => self.format_vector_convert(),
            VectorOperation::Load => self.format_vector_load(),
            VectorOperation::Store => self.format_vector_store(),
        }
    }

    /// Format one vector splat.
    fn format_vector_splat(&mut self) -> FormatResult<()> {
        let result = self.read_span()?;
        let value = self.register_id()?;
        let vector = self.vector_type()?;
        self.write_vector_opcode("splat")?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_register(value)?;
        self.write_representation(ValueType::vector(vector))
    }

    /// Format one vector lane insertion.
    fn format_vector_insert(&mut self) -> FormatResult<()> {
        let result = self.read_span()?;
        let input = self.read_span()?;
        let index = self.register_id()?;
        let value = self.register_id()?;
        let vector = self.vector_type()?;
        self.write_vector_opcode("insert")?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_span(input)?;
        self.write_comma()?;
        self.write_register(index)?;
        self.write_comma()?;
        self.write_register(value)?;
        self.write_representation(ValueType::vector(vector))
    }

    /// Format one vector lane extraction.
    fn format_vector_extract(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let input = self.read_span()?;
        let index = self.register_id()?;
        let vector = self.vector_type()?;
        self.write_vector_opcode("extract")?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_span(input)?;
        self.write_comma()?;
        self.write_register(index)?;
        self.write_representation(ValueType::vector(vector))
    }

    /// Format one vector lane shuffle.
    fn format_vector_shuffle(&mut self) -> FormatResult<()> {
        let result = self.read_span()?;
        let inputs = self.register_ids()?;
        let lanes = self.u16_list()?;
        let vector = self.vector_type()?;
        self.write_vector_opcode("shuffle")?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_vector_spans(&inputs, vector)?;
        self.write_comma()?;
        self.write_u16s(&lanes)?;
        self.write_representation(ValueType::vector(vector))
    }

    /// Format one elementwise vector operation.
    fn format_vector_element(&mut self) -> FormatResult<()> {
        let result = self.read_span()?;
        let inputs = self.register_ids()?;
        let vector = self.vector_type()?;
        let operator = self.u16()?;
        let operator = self.vector_operator(vector, operator, false)?;
        self.write_vector_opcode(&operator)?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_vector_spans(&inputs, vector)?;
        self.write_representation(ValueType::vector(vector))
    }

    /// Format one vector comparison.
    fn format_vector_compare(&mut self) -> FormatResult<()> {
        let result = self.read_span()?;
        let inputs = self.register_ids()?;
        let vector = self.vector_type()?;
        let operator = self.u16()?;
        let operator = self.vector_operator(vector, operator, true)?;
        let operation = format!("compare.{operator}");
        self.write_vector_opcode(&operation)?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_vector_spans(&inputs, vector)?;
        self.write_representation(ValueType::vector(vector))
    }

    /// Format one vector lane selection.
    fn format_vector_select(&mut self) -> FormatResult<()> {
        let result = self.read_span()?;
        let inputs = self.register_ids()?;
        let [condition, left, right] = inputs.as_slice() else {
            return Err(FormatError::SyntaxError {
                message: "vector select requires three inputs",
            });
        };
        let vector = self.vector_type()?;
        self.write_vector_opcode("select")?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_span(RegisterSpan::new(*condition, vector.mask().word_count()))?;
        self.write_comma()?;
        self.write_span(RegisterSpan::new(*left, vector.word_count()))?;
        self.write_comma()?;
        self.write_span(RegisterSpan::new(*right, vector.word_count()))?;
        self.write_representation(ValueType::vector(vector))
    }

    /// Format one vector reduction.
    fn format_vector_reduce(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let input = self.read_span()?;
        let vector = self.vector_type()?;
        let operation =
            ReduceOperation::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
                message: "vector reduction has an invalid operation",
            })?;
        let operation = format!("reduce.{}", operation.name());
        self.write_vector_opcode(&operation)?;
        self.write_register(result)?;
        self.write_comma()?;
        self.write_span(input)?;
        self.write_representation(ValueType::vector(vector))
    }

    /// Format one vector conversion.
    fn format_vector_convert(&mut self) -> FormatResult<()> {
        let result = self.read_span()?;
        let input = self.read_span()?;
        let source = self.vector_type()?;
        let target = self.vector_type()?;
        let mode = ConvertMode::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
            message: "vector conversion has an invalid mode",
        })?;
        let operation = format!("vector.convert.{}", mode.name());
        self.write_opcode(&operation)?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_span(input)?;
        self.write_conversion(ValueType::vector(source), ValueType::vector(target))
    }

    /// Format one vector load.
    fn format_vector_load(&mut self) -> FormatResult<()> {
        let result = self.read_span()?;
        let pointer = self.register_id()?;
        let vector = self.vector_type()?;
        self.write_vector_opcode("load")?;
        self.write_span(result)?;
        self.write_comma()?;
        self.write_register(pointer)?;
        self.write_representation(ValueType::vector(vector))
    }

    /// Format one vector store.
    fn format_vector_store(&mut self) -> FormatResult<()> {
        let pointer = self.register_id()?;
        let value = self.read_span()?;
        let vector = self.vector_type()?;
        self.write_opcode("vector.store")?;
        self.write_register(pointer)?;
        self.write_comma()?;
        self.write_span(value)?;
        self.write_representation(ValueType::vector(vector))
    }

    /// Write one vector opcode.
    fn write_vector_opcode(&mut self, operation: &str) -> FormatResult<()> {
        let operation = format!("vector.{operation}");

        self.write_opcode(&operation)
    }

    /// Write physical spans for vector start registers.
    fn write_vector_spans(
        &mut self,
        registers: &[RegisterId],
        vector: VectorType,
    ) -> FormatResult<()> {
        for (index, register) in registers.iter().copied().enumerate() {
            if index > 0 {
                self.write_comma()?;
            }
            self.write_span(RegisterSpan::new(register, vector.word_count()))?;
        }

        Ok(())
    }

    /// Decode one vector scalar operation.
    fn vector_operator(
        &self,
        vector: VectorType,
        code: u16,
        is_comparison: bool,
    ) -> FormatResult<String> {
        let name = if vector.scalar.is_float() {
            FloatOperation::from_code(code as u8)
                .filter(|operation| {
                    if is_comparison {
                        operation.returns_boolean() && operation.input_count() == 2
                    } else {
                        !operation.returns_boolean()
                    }
                })
                .map(FloatOperation::name)
        } else {
            IntegerOperation::from_code(code as u8)
                .filter(|operation| {
                    if is_comparison {
                        operation.returns_boolean() && operation.input_count() == 2
                    } else {
                        !operation.returns_boolean() && !operation.is_overflowing()
                    }
                })
                .map(IntegerOperation::name)
        };

        name.map(str::to_string).ok_or(FormatError::SyntaxError {
            message: "vector operation has an invalid scalar operation",
        })
    }

    /// Read one physical register span.
    fn read_span(&mut self) -> FormatResult<RegisterSpan> {
        let (start, word_count) = self.register_span_id()?;

        Ok(RegisterSpan::new(start, word_count))
    }

    /// Write one unsigned lane list.
    fn write_u16s(&mut self, values: &[u16]) -> FormatResult<()> {
        self.write_token("[")?;
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                self.write_comma()?;
            }
            self.write_text(&value.to_string())?;
        }

        self.write_token("]")
    }
}
