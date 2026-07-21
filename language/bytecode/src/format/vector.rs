use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    ConvertMode, FloatOperation, IntegerOperation, ReduceOperation, RegisterId, Scalar, ValueType,
    VectorOperation, VectorType,
};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one fixed width vector operation.
    pub(super) fn format_vector(&mut self) -> FormatResult<()> {
        let operation =
            self.instruction
                .opcode()
                .vector_operation()
                .ok_or(FormatError::SyntaxError {
                    message: "vector instruction has an invalid opcode",
                })?;

        match operation {
            VectorOperation::Splat => self.format_vector_splat(),
            VectorOperation::Insert => self.format_vector_insert(),
            VectorOperation::Extract => self.format_vector_extract(),
            VectorOperation::Shuffle => self.format_vector_shuffle(),
            VectorOperation::Element => self.format_vector_element(false),
            VectorOperation::Compare => self.format_vector_element(true),
            VectorOperation::Select => self.format_vector_select(),
            VectorOperation::Reduce => self.format_vector_reduce(),
            VectorOperation::Convert => self.format_vector_convert(),
            VectorOperation::Load => self.format_vector_load(),
            VectorOperation::Store => self.format_vector_store(),
        }
    }

    /// Format one scalar broadcast.
    fn format_vector_splat(&mut self) -> FormatResult<()> {
        let result = self.register_range_id()?;
        let value = self.register_id()?;
        let vector = self.vector_type()?;
        self.require_vector_value(value, ValueType::scalar(vector.scalar))?;

        self.vector_result(result, vector)?;

        // write the scalar broadcast
        write!(
            self.formatter,
            [space(), token("="), space(), token("vector.splat"), space()]
        )?;
        self.write_register(value)?;

        Ok(())
    }

    /// Format one lane replacement.
    fn format_vector_insert(&mut self) -> FormatResult<()> {
        let result = self.register_range_id()?;
        let (input, input_word_count) = self.register_range_id()?;
        let index = self.register_id()?;
        let value = self.register_id()?;
        let vector = self.vector_type()?;
        let input_type = self.formatter.context().register_type(input)?;
        if input_type != ValueType::vector(vector) || input_word_count != input_type.word_count() {
            return Err(FormatError::SyntaxError {
                message: "vector insertion reads an invalid input range",
            });
        }
        self.require_vector_value(index, ValueType::scalar(Scalar::Uint32))?;
        self.require_vector_value(value, ValueType::scalar(vector.scalar))?;

        // write the lane replacement
        self.vector_result(result, vector)?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("vector.insert"),
                space()
            ]
        )?;
        self.write_register(input)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(index)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(value)?;

        Ok(())
    }

    /// Format one lane extraction.
    fn format_vector_extract(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let (input, input_word_count) = self.register_range_id()?;
        let index = self.register_id()?;
        let vector = self.vector_type()?;
        let input_type = self.formatter.context().register_type(input)?;
        if input_type != ValueType::vector(vector) || input_word_count != input_type.word_count() {
            return Err(FormatError::SyntaxError {
                message: "vector extraction reads an invalid input range",
            });
        }
        self.require_vector_value(index, ValueType::scalar(Scalar::Uint32))?;

        // write the lane projection
        self.write_result(result, ValueType::scalar(vector.scalar))?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("vector.extract"),
                space()
            ]
        )?;
        self.write_register(input)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(index)?;

        Ok(())
    }

    /// Format one static lane shuffle.
    fn format_vector_shuffle(&mut self) -> FormatResult<()> {
        let result = self.register_range_id()?;
        let inputs = self.register_ids()?;
        let lanes = self.u16_list()?;
        let vector = self.vector_type()?;
        let vector_type = ValueType::vector(vector);
        for input in &inputs {
            self.require_vector_value(*input, vector_type)?;
        }
        let lane_limit = u32::from(vector.lane_count) * 2;
        let are_lanes_valid = lanes.len() == vector.lane_count as usize
            && lanes.iter().all(|lane| u32::from(*lane) < lane_limit);
        if !are_lanes_valid {
            return Err(FormatError::SyntaxError {
                message: "vector shuffle has invalid lanes",
            });
        }

        // write the selected lane ordering
        self.vector_result(result, vector)?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("vector.shuffle"),
                space()
            ]
        )?;
        self.write_registers(&inputs)?;
        write!(self.formatter, [token(","), space(), token("[")])?;

        // write the lane selection in encoded order
        for (index, lane) in lanes.into_iter().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_text(&lane.to_string())?;
        }
        self.write_token("]")?;

        Ok(())
    }

    /// Format one elementwise vector operation.
    fn format_vector_element(&mut self, is_comparison: bool) -> FormatResult<()> {
        let result = self.register_range_id()?;
        let inputs = self.register_ids()?;
        let vector = self.vector_type()?;
        let operator = self.u16()?;
        let operation = self.vector_operator(vector, operator)?;
        let result_type = if is_comparison { vector.mask() } else { vector };
        for input in &inputs {
            self.require_vector_value(*input, ValueType::vector(vector))?;
        }

        self.vector_result(result, result_type)?;

        // write comparisons as an explicit scalar operator operand
        if is_comparison {
            write!(
                self.formatter,
                [
                    space(),
                    token("="),
                    space(),
                    token("vector.compare"),
                    space()
                ]
            )?;
            self.write_text(&operation)?;
            write!(self.formatter, [token(","), space()])?;
        } else {
            write!(self.formatter, [space(), token("="), space()])?;
            self.write_text(&operation)?;
            write!(self.formatter, [space()])?;
        }
        self.write_registers(&inputs)?;

        Ok(())
    }

    /// Format one lane selection.
    fn format_vector_select(&mut self) -> FormatResult<()> {
        let result = self.register_range_id()?;
        let inputs = self.register_ids()?;
        let vector = self.vector_type()?;
        let [condition, left, right] = inputs.as_slice() else {
            return Err(FormatError::SyntaxError {
                message: "vector selection has an invalid input count",
            });
        };
        self.require_vector_value(*condition, ValueType::vector(vector.mask()))?;
        self.require_vector_value(*left, ValueType::vector(vector))?;
        self.require_vector_value(*right, ValueType::vector(vector))?;

        // write the lane selection
        self.vector_result(result, vector)?;
        write!(
            self.formatter,
            [space(), token("="), space(), token("select"), space()]
        )?;
        self.write_registers(&inputs)?;

        Ok(())
    }

    /// Format one horizontal reduction.
    fn format_vector_reduce(&mut self) -> FormatResult<()> {
        let result = self.register_id()?;
        let (input, input_word_count) = self.register_range_id()?;
        let vector = self.vector_type()?;
        let input_type = self.formatter.context().register_type(input)?;
        if input_type != ValueType::vector(vector) || input_word_count != input_type.word_count() {
            return Err(FormatError::SyntaxError {
                message: "vector reduction reads an invalid input range",
            });
        }
        let operation =
            ReduceOperation::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
                message: "vector reduction has an invalid operation",
            })?;

        // write the horizontal reduction
        self.write_result(result, ValueType::scalar(vector.scalar))?;
        let family = if vector.scalar.is_float() {
            "float"
        } else {
            "int"
        };
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("vector.reduce"),
                space()
            ]
        )?;
        self.write_text(family)?;
        self.write_token(".")?;
        self.write_text(operation.name())?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(input)?;

        Ok(())
    }

    /// Format one lane representation conversion.
    fn format_vector_convert(&mut self) -> FormatResult<()> {
        let result = self.register_range_id()?;
        let (input, input_word_count) = self.register_range_id()?;
        let source = self.vector_type()?;
        let target = self.vector_type()?;
        let mode = ConvertMode::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
            message: "vector conversion has an invalid mode",
        })?;

        // require the encoded source range to match the register file
        let input_type = self.formatter.context().register_type(input)?;
        if input_type != ValueType::vector(source) || input_word_count != input_type.word_count() {
            return Err(FormatError::SyntaxError {
                message: "vector conversion reads an invalid source range",
            });
        }

        // write the representation conversion
        self.vector_result(result, target)?;
        write!(
            self.formatter,
            [
                space(),
                token("="),
                space(),
                token("vector.convert"),
                space()
            ]
        )?;
        self.write_text(mode.name())?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(input)?;

        Ok(())
    }

    /// Format one vector load.
    fn format_vector_load(&mut self) -> FormatResult<()> {
        let result = self.register_range_id()?;
        let address = self.register_id()?;
        let vector = self.vector_type()?;
        self.require_vector_value(address, ValueType::address())?;

        // write the typed vector load
        self.vector_result(result, vector)?;
        write!(
            self.formatter,
            [space(), token("="), space(), token("load"), space()]
        )?;
        self.write_register(address)?;

        Ok(())
    }

    /// Format one vector store.
    fn format_vector_store(&mut self) -> FormatResult<()> {
        let address = self.register_id()?;
        let (value, word_count) = self.register_range_id()?;
        let vector = self.vector_type()?;
        self.require_vector_value(address, ValueType::address())?;

        // require the encoded input range to match the register file
        let value_type = self.formatter.context().register_type(value)?;
        if value_type != ValueType::vector(vector) || word_count != value_type.word_count() {
            return Err(FormatError::SyntaxError {
                message: "vector store reads an invalid value range",
            });
        }

        // write the typed vector store
        write!(self.formatter, [token("store"), space()])?;
        self.write_register(address)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(value)?;

        Ok(())
    }

    /// Append and assign one vector result range.
    fn vector_result(&mut self, result: (RegisterId, u16), vector: VectorType) -> FormatResult<()> {
        let ty = ValueType::vector(vector);

        // require the physical result range to fit the logical vector
        if result.1 != ty.word_count() {
            return Err(FormatError::SyntaxError {
                message: "vector result width does not match its type",
            });
        }

        self.write_result(result.0, ty)
    }

    /// Decode one typed vector operator name.
    fn vector_operator(&self, vector: VectorType, code: u16) -> FormatResult<String> {
        let family = if vector.scalar.is_float() {
            "float"
        } else {
            "int"
        };
        let operation = if vector.scalar.is_float() {
            FloatOperation::from_code(code as u8).map(FloatOperation::name)
        } else {
            IntegerOperation::from_code(code as u8).map(IntegerOperation::name)
        };
        let operation = operation.ok_or(FormatError::SyntaxError {
            message: "vector instruction has an invalid operation",
        })?;

        Ok(format!("{family}.{operation}"))
    }

    /// Require one register to contain an exact vector operand type.
    fn require_vector_value(&self, register: RegisterId, expected: ValueType) -> FormatResult<()> {
        if self.formatter.context().register_type(register)? != expected {
            return Err(FormatError::SyntaxError {
                message: "vector operand does not match its encoded type",
            });
        }

        Ok(())
    }
}
