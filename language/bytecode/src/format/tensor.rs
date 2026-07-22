use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    CodeOffset, ConvertMode, FloatOperation, IndexReduceOperation, IntegerOperation,
    ReduceOperation, ReferenceType, RegisterId, Scalar, ScatterOperation, SymbolTag,
    TensorOperation, TieBreak, TypeId, ValueType,
};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one tensor operation.
    pub(super) fn format_tensor(&mut self) -> FormatResult<()> {
        let operation =
            self.instruction
                .opcode()
                .tensor_operation()
                .ok_or(FormatError::SyntaxError {
                    message: "tensor instruction has an invalid opcode",
                })?;
        let result_type = self.decode_tensor_result_type(operation)?;

        match operation {
            TensorOperation::Element | TensorOperation::Compare => {
                self.format_tensor_element(operation, result_type)?
            }
            TensorOperation::Select => self.format_tensor_select(result_type)?,
            TensorOperation::Transpose | TensorOperation::Broadcast => {
                self.format_tensor_reorder(operation, result_type)?
            }
            TensorOperation::Reshape => self.format_tensor_reshape(result_type)?,
            TensorOperation::Slice => self.format_tensor_slice(result_type)?,
            TensorOperation::Pad => self.format_tensor_pad(result_type)?,
            TensorOperation::Concat => self.format_tensor_concat(result_type)?,
            TensorOperation::Splat => self.format_tensor_splat(result_type)?,
            TensorOperation::Convert => self.format_tensor_convert(result_type)?,
            TensorOperation::Bitcast => self.format_tensor_input(operation, result_type)?,
            TensorOperation::Reduce => self.format_tensor_reduce(result_type)?,
            TensorOperation::IndexReduce => self.format_tensor_index_reduce(result_type)?,
            TensorOperation::Contract => self.format_tensor_contract(result_type)?,
            TensorOperation::Gather => self.format_tensor_gather(result_type)?,
            TensorOperation::Scatter => self.format_tensor_scatter(result_type)?,
            TensorOperation::Load | TensorOperation::Extract => {
                self.format_tensor_load(operation, result_type)?
            }
            TensorOperation::Store => self.format_tensor_store()?,
            TensorOperation::Fill => self.format_tensor_fill()?,
            TensorOperation::Copy => self.format_tensor_copy()?,
            TensorOperation::View => self.format_tensor_view(result_type)?,
            TensorOperation::Convolution => self.format_tensor_convolution(result_type)?,
        }

        self.consume_tensor_result(operation, result_type)
    }

    /// Consume the runtime type suffix shared by tensor results.
    fn consume_tensor_result(
        &mut self,
        operation: TensorOperation,
        result_type: Option<ValueType>,
    ) -> FormatResult<()> {
        if matches!(operation, TensorOperation::Load | TensorOperation::Extract) {
            let scalar = self.result_scalar(result_type)?;
            self.require_scalar(scalar)?;
        } else if !matches!(
            operation,
            TensorOperation::Store | TensorOperation::Fill | TensorOperation::Copy
        ) {
            let scalar = self.result_scalar(result_type)?;
            self.require_scalar(scalar)?;
            let reference = self.reference()?;
            let expected = result_type.and_then(ValueType::tensor_reference).ok_or(
                FormatError::SyntaxError {
                    message: "tensor result has no backing reference",
                },
            )?;
            if reference != expected {
                return Err(FormatError::SyntaxError {
                    message: "tensor result reference does not match its type",
                });
            }
            let (_, target) = self.symbol_with_target()?;
            if target.tag != SymbolTag::TYPE {
                return Err(FormatError::SyntaxError {
                    message: "tensor result does not reference a runtime type",
                });
            }
        }

        Ok(())
    }

    /// Format one tensor selection.
    fn format_tensor_select(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let tensors = self.tensor_ids()?;
        let [condition, left, right] = tensors.as_slice() else {
            return Err(FormatError::SyntaxError {
                message: "tensor.select requires three tensor inputs",
            });
        };
        let scalar = self.result_scalar(result_type)?;
        self.require_scalar(scalar)?;
        write!(self.formatter, [token("tensor.select"), space()])?;
        self.write_register(*condition)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(*left)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(*right)
    }

    /// Format one tensor transpose or broadcast.
    fn format_tensor_reorder(
        &mut self,
        operation: TensorOperation,
        result_type: Option<ValueType>,
    ) -> FormatResult<()> {
        self.format_tensor_input(operation, result_type)?;
        let values = self.u16_list()?;
        let name = if operation == TensorOperation::Transpose {
            "permutation"
        } else {
            "axes"
        };

        self.comma()?;
        self.write_named_u16s(name, &values)
    }

    /// Format one tensor reshape.
    fn format_tensor_reshape(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_input(TensorOperation::Reshape, result_type)?;
        let shape = self.register_ids()?;

        self.comma()?;
        self.write_named_registers("shape", &shape)
    }

    /// Format one tensor slice.
    fn format_tensor_slice(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_input(TensorOperation::Slice, result_type)?;

        // write each dynamic slice component
        for name in ["offsets", "sizes", "strides"] {
            let values = self.register_ids()?;
            self.comma()?;
            self.write_named_registers(name, &values)?;
        }

        Ok(())
    }

    /// Format one tensor padding operation.
    fn format_tensor_pad(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let input = self.tensor_id()?;
        let value = self.register_id()?;
        let scalar = self.result_scalar(result_type)?;
        self.require_scalar(scalar)?;
        write!(self.formatter, [token("tensor.pad"), space()])?;
        self.write_register(input)?;
        self.comma()?;
        self.write_token("value(")?;
        self.write_register(value)?;
        self.write_token(")")?;

        // write each padding extent
        for name in ["low", "high", "interior"] {
            let values = self.register_ids()?;
            self.comma()?;
            self.write_named_registers(name, &values)?;
        }

        Ok(())
    }

    /// Format one tensor concatenation.
    fn format_tensor_concat(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let tensors = self.tensor_ids()?;
        let axis = self.u16()?;
        write!(self.formatter, [token("tensor.concat"), space()])?;
        self.write_named_registers("tensors", &tensors)?;
        self.comma()?;
        self.write_token("axis(")?;
        self.write_text(&axis.to_string())?;
        self.write_token(")")
    }

    /// Format one scalar tensor broadcast.
    fn format_tensor_splat(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let value = self.register_id()?;
        let scalar = self.result_scalar(result_type)?;
        self.require_scalar(scalar)?;
        write!(self.formatter, [token("tensor.splat"), space()])?;
        self.write_register(value)
    }

    /// Format one tensor element conversion.
    fn format_tensor_convert(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let value = self.tensor_id()?;
        let source = self.register_tensor_scalar(value)?;
        self.require_scalar(source)?;
        let target = self.result_scalar(result_type)?;
        self.require_scalar(target)?;
        let mode = ConvertMode::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
            message: "tensor conversion has an invalid mode",
        })?;
        write!(self.formatter, [token("tensor.convert"), space()])?;
        self.write_text(mode.name())?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(value)
    }

    /// Format one elementwise or comparison tensor operation.
    fn format_tensor_element(
        &mut self,
        operation: TensorOperation,
        result_type: Option<ValueType>,
    ) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let inputs = self.tensor_ids()?;
        let scalar = self.scalar()?;
        let code = self.u16()?;
        let operator = self.scalar_operator(scalar, code)?;
        if operation == TensorOperation::Compare {
            write!(self.formatter, [token("tensor.compare"), space()])?;
            self.write_text(&operator)?;
            write!(self.formatter, [token(","), space()])?;
        } else {
            self.write_text(&operator)?;
            write!(self.formatter, [space()])?;
        }
        self.write_registers(&inputs)?;

        Ok(())
    }

    /// Format one tensor reduction.
    fn format_tensor_reduce(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let input = self.tensor_id()?;
        let initial = self.register_id()?;
        let scalar = self.result_scalar(result_type)?;
        self.require_scalar(scalar)?;
        let operation =
            ReduceOperation::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
                message: "tensor reduction has an invalid operation",
            })?;
        let axes = self.u16_list()?;
        write!(self.formatter, [token("tensor.reduce"), space()])?;
        self.write_text(operation.name())?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(input)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(initial)?;
        self.comma()?;
        self.write_named_u16s("axes", &axes)?;

        Ok(())
    }

    /// Format one tensor index reduction.
    fn format_tensor_index_reduce(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let input = self.tensor_id()?;
        let scalar = self.register_tensor_scalar(input)?;
        self.require_scalar(scalar)?;
        let operation =
            IndexReduceOperation::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
                message: "tensor index reduction has an invalid operation",
            })?;
        let axis = self.u16()?;
        let tie = TieBreak::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
            message: "tensor index reduction has an invalid tie break",
        })?;
        write!(self.formatter, [token("tensor.indexReduce"), space()])?;
        self.write_text(operation.name())?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(input)?;
        write!(self.formatter, [token(","), space(), token("axis(")])?;
        self.write_text(&axis.to_string())?;
        write!(self.formatter, [token("),"), space(), token("tieBreak(")])?;
        self.write_text(tie.name())?;
        self.write_token(")")?;

        Ok(())
    }

    /// Append one tensor result assignment.
    fn format_tensor_result(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        let result = self.register_id()?;
        let ty = result_type.ok_or(FormatError::SyntaxError {
            message: "tensor instruction has no result type",
        })?;
        self.write_result(result, ty)?;
        write!(self.formatter, [space(), token("="), space()])?;

        Ok(())
    }

    /// Return the tensor result type encoded at the end of this instruction.
    fn decode_tensor_result_type(
        &self,
        operation: TensorOperation,
    ) -> FormatResult<Option<ValueType>> {
        if matches!(
            operation,
            TensorOperation::Store | TensorOperation::Fill | TensorOperation::Copy
        ) {
            return Ok(None);
        }

        let operands = self.instruction.operand_bytes();
        if matches!(operation, TensorOperation::Load | TensorOperation::Extract) {
            let scalar_offset =
                operands
                    .len()
                    .checked_sub(size_of::<u16>())
                    .ok_or(FormatError::SyntaxError {
                        message: "tensor instruction has no result scalar",
                    })?;
            let bytes = operands
                .get(scalar_offset..)
                .ok_or(FormatError::SyntaxError {
                    message: "tensor instruction has no result scalar",
                })?;
            let scalar = u16::from_le_bytes([bytes[0], bytes[1]]);
            let scalar = u8::try_from(scalar).map_err(|_| FormatError::SyntaxError {
                message: "tensor instruction has an invalid result scalar",
            })?;
            let scalar = Scalar::from_code(scalar).ok_or(FormatError::SyntaxError {
                message: "tensor instruction has an invalid result scalar",
            })?;

            return Ok(Some(ValueType::scalar(scalar)));
        }

        let suffix_len = size_of::<u16>() + size_of::<u16>() + size_of::<u32>();
        let suffix_offset =
            operands
                .len()
                .checked_sub(suffix_len)
                .ok_or(FormatError::SyntaxError {
                    message: "tensor instruction has no result type",
                })?;
        let suffix = operands
            .get(suffix_offset..)
            .ok_or(FormatError::SyntaxError {
                message: "tensor instruction has no result type",
            })?;
        let scalar = u16::from_le_bytes([suffix[0], suffix[1]]);
        let scalar = u8::try_from(scalar).map_err(|_| FormatError::SyntaxError {
            message: "tensor instruction has an invalid result scalar",
        })?;
        let scalar = Scalar::from_code(scalar).ok_or(FormatError::SyntaxError {
            message: "tensor instruction has an invalid result scalar",
        })?;
        let reference = ReferenceType::from_bits(u16::from_le_bytes([suffix[2], suffix[3]]))
            .ok_or(FormatError::SyntaxError {
                message: "tensor instruction has an invalid backing reference",
            })?;
        let header_byte_len = self.instruction.byte_len() - operands.len();
        let symbol_byte_offset = header_byte_len + operands.len() - size_of::<u32>();
        let symbol_offset = CodeOffset(self.instruction_offset.0 + symbol_byte_offset as u32);
        let target = self.formatter.context().relocation(symbol_offset)?;
        if target.tag != SymbolTag::TYPE {
            return Err(FormatError::SyntaxError {
                message: "tensor result does not reference a runtime type",
            });
        }
        let ty = if operation == TensorOperation::View {
            let bytes = operands
                .get(..size_of::<u16>() * 2)
                .ok_or(FormatError::SyntaxError {
                    message: "tensor view instruction has no result range",
                })?;
            let word_count = u16::from_le_bytes([bytes[2], bytes[3]]);

            ValueType::tensor_view(scalar, TypeId(target.index), reference, word_count)
        } else {
            ValueType::tensor(scalar, TypeId(target.index), reference.space())
        };

        Ok(Some(ty))
    }

    /// Write one tensor result range.
    fn format_tensor_result_range(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        let (result, word_count) = self.register_range_id()?;
        let ty = result_type.ok_or(FormatError::SyntaxError {
            message: "tensor instruction has no result type",
        })?;
        if word_count != ty.word_count() {
            return Err(FormatError::SyntaxError {
                message: "tensor result width does not match its type",
            });
        }
        self.write_result(result, ty)?;
        write!(self.formatter, [space(), token("="), space()])?;

        Ok(())
    }

    /// Append one tensor result and input.
    fn format_tensor_input(
        &mut self,
        operation: TensorOperation,
        result_type: Option<ValueType>,
    ) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let input = self.tensor_id()?;
        self.write_token("tensor.")?;
        self.write_text(operation.name())?;
        write!(self.formatter, [space()])?;
        self.write_register(input)?;

        Ok(())
    }

    /// Decode one scalar operator.
    fn scalar_operator(&self, scalar: Scalar, code: u16) -> FormatResult<String> {
        let operation = if scalar.is_integer() || scalar == Scalar::Boolean {
            IntegerOperation::from_code(code as u8).map(IntegerOperation::name)
        } else {
            FloatOperation::from_code(code as u8).map(FloatOperation::name)
        };
        let operation = operation.ok_or(FormatError::SyntaxError {
            message: "tensor instruction has an invalid scalar operator",
        })?;
        let prefix = if scalar.is_float() { "float" } else { "int" };

        Ok(format!("{prefix}.{operation}"))
    }

    /// Format one tensor contraction.
    fn format_tensor_contract(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let tensors = self.tensor_ids()?;
        let [left, right] = tensors.as_slice() else {
            return Err(FormatError::SyntaxError {
                message: "tensor.contract requires two tensor inputs",
            });
        };
        let scalar = self.result_scalar(result_type)?;
        self.require_scalar(scalar)?;
        write!(self.formatter, [token("tensor.contract"), space()])?;
        self.write_register(*left)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(*right)?;
        self.comma()?;
        self.write_token("axes(")?;
        for (index, name) in ["leftBatch", "rightBatch", "leftContract", "rightContract"]
            .into_iter()
            .enumerate()
        {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            let values = self.u16_list()?;
            self.write_named_u16s(name, &values)?;
        }
        self.write_token(")")?;

        Ok(())
    }

    /// Format one tensor gather.
    fn format_tensor_gather(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let tensors = self.tensor_ids()?;
        let [input, indices] = tensors.as_slice() else {
            return Err(FormatError::SyntaxError {
                message: "tensor.gather requires two tensor inputs",
            });
        };
        write!(self.formatter, [token("tensor.gather"), space()])?;
        self.write_register(*input)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(*indices)?;
        self.comma()?;
        self.write_token("axes(")?;
        for (index, name) in ["outputOffset", "collapsedInput", "indexToInput"]
            .into_iter()
            .enumerate()
        {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            let values = self.u16_list()?;
            self.write_named_u16s(name, &values)?;
        }
        let dimension = self.u16()?;
        write!(self.formatter, [token(","), space(), token("indexVector(")])?;
        self.write_text(&dimension.to_string())?;
        self.write_token("))")?;
        self.comma()?;
        let sizes = self.u64_list()?;
        self.write_named_u64s("sliceSizes", &sizes)?;

        Ok(())
    }

    /// Format one tensor scatter.
    fn format_tensor_scatter(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let tensors = self.tensor_ids()?;
        let [input, indices, updates] = tensors.as_slice() else {
            return Err(FormatError::SyntaxError {
                message: "tensor.scatter requires three tensor inputs",
            });
        };
        let scalar = self.result_scalar(result_type)?;
        self.require_scalar(scalar)?;
        write!(self.formatter, [token("tensor.scatter"), space()])?;
        self.write_register(*input)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(*indices)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(*updates)?;
        self.comma()?;
        self.write_token("axes(")?;
        for (index, name) in ["updateWindow", "insertedInput", "indexToInput"]
            .into_iter()
            .enumerate()
        {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            let values = self.u16_list()?;
            self.write_named_u16s(name, &values)?;
        }
        let dimension = self.u16()?;
        write!(self.formatter, [token(","), space(), token("indexVector(")])?;
        self.write_text(&dimension.to_string())?;
        self.write_token("))")?;
        self.comma()?;
        let update =
            ScatterOperation::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
                message: "tensor scatter has an invalid update operation",
            })?;
        self.write_token("mode(")?;
        self.write_text(update.name())?;
        self.write_token(")")?;

        Ok(())
    }

    /// Format one tensor scalar load or extract.
    fn format_tensor_load(
        &mut self,
        operation: TensorOperation,
        result_type: Option<ValueType>,
    ) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let input = self.tensor_id()?;
        let indices = self.register_ids()?;
        self.write_token("tensor.")?;
        self.write_text(operation.name())?;
        write!(self.formatter, [space()])?;
        self.write_register(input)?;
        write!(self.formatter, [token(","), space(), token("[")])?;
        self.write_registers(&indices)?;
        self.write_token("]")?;
        Ok(())
    }

    /// Format one tensor scalar store.
    fn format_tensor_store(&mut self) -> FormatResult<()> {
        let view = self.tensor_id()?;
        let indices = self.register_ids()?;
        let value = self.register_id()?;
        let scalar = self.register_tensor_scalar(view)?;
        self.require_scalar(scalar)?;
        write!(self.formatter, [token("tensor.store"), space()])?;
        self.write_register(view)?;
        write!(self.formatter, [token(","), space(), token("[")])?;
        self.write_registers(&indices)?;
        write!(self.formatter, [token("]"), token(","), space()])?;
        self.write_register(value)?;

        Ok(())
    }

    /// Format one tensor fill.
    fn format_tensor_fill(&mut self) -> FormatResult<()> {
        let view = self.tensor_id()?;
        let value = self.register_id()?;
        let scalar = self.register_tensor_scalar(view)?;
        self.require_scalar(scalar)?;
        write!(self.formatter, [token("tensor.fill"), space()])?;
        self.write_register(view)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(value)?;

        Ok(())
    }

    /// Format one tensor copy.
    fn format_tensor_copy(&mut self) -> FormatResult<()> {
        let tensors = self.tensor_ids()?;
        let [source, target] = tensors.as_slice() else {
            return Err(FormatError::SyntaxError {
                message: "tensor.copy requires two tensor inputs",
            });
        };
        let source_type = self.formatter.context().register_type(*source)?;
        let target_type = self.formatter.context().register_type(*target)?;
        if !source_type.is_tensor_view() || source_type != target_type {
            return Err(FormatError::SyntaxError {
                message: "tensor.copy reads invalid tensor views",
            });
        }
        let scalar = self.register_tensor_scalar(*source)?;
        self.require_scalar(scalar)?;
        write!(self.formatter, [token("tensor.copy"), space()])?;
        self.write_register(*source)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(*target)?;

        Ok(())
    }

    /// Format one tensor view.
    fn format_tensor_view(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result_range(result_type)?;
        let input = self.tensor_id()?;
        write!(self.formatter, [token("tensor.view"), space()])?;
        self.write_register(input)?;
        for name in ["offsets", "sizes", "strides"] {
            let values = self.register_ids()?;
            self.comma()?;
            self.write_named_registers(name, &values)?;
        }

        Ok(())
    }

    /// Format one tensor convolution.
    fn format_tensor_convolution(&mut self, result_type: Option<ValueType>) -> FormatResult<()> {
        self.format_tensor_result(result_type)?;
        let tensors = self.tensor_ids()?;
        let [input, kernel] = tensors.as_slice() else {
            return Err(FormatError::SyntaxError {
                message: "tensor.convolution requires two tensor inputs",
            });
        };
        let scalar = self.result_scalar(result_type)?;
        self.require_scalar(scalar)?;
        write!(self.formatter, [token("tensor.convolution"), space()])?;
        self.write_register(*input)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(*kernel)?;
        write!(self.formatter, [token(","), space(), token("axes(")])?;
        for (index, name) in [
            "inputBatch",
            "inputFeature",
            "inputSpatial",
            "kernelInputFeature",
            "kernelOutputFeature",
            "kernelSpatial",
            "outputBatch",
            "outputFeature",
            "outputSpatial",
        ]
        .into_iter()
        .enumerate()
        {
            if index > 0 {
                self.comma()?;
            }
            let values = self.u16_list()?;
            self.write_named_u16s(name, &values)?;
        }
        self.write_token(")")?;
        self.comma()?;
        self.write_token("window(")?;
        for (index, name) in [
            "strides",
            "paddingLow",
            "paddingHigh",
            "baseDilation",
            "windowDilation",
        ]
        .into_iter()
        .enumerate()
        {
            if index > 0 {
                self.comma()?;
            }
            let values = self.u64_list()?;
            self.write_named_u64s(name, &values)?;
        }
        let reversal = self.u16_list()?;
        self.comma()?;
        self.write_token("reversal(")?;
        for (index, value) in reversal.into_iter().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_token(if value == 0 { "false" } else { "true" })?;
        }
        let feature = self.u32()?;
        let batch = self.u32()?;
        self.write_token("))")?;
        self.comma()?;
        self.write_token("groups(feature(")?;
        self.write_text(&feature.to_string())?;
        write!(self.formatter, [token("),"), space(), token("batch(")])?;
        self.write_text(&batch.to_string())?;
        self.write_token("))")?;

        Ok(())
    }

    /// Write one named register list.
    fn write_named_registers(&mut self, name: &str, values: &[RegisterId]) -> FormatResult<()> {
        self.write_text(name)?;
        self.write_token("(")?;
        self.write_registers(values)?;
        self.write_token(")")?;

        Ok(())
    }

    /// Write one named unsigned 16-bit list.
    fn write_named_u16s(&mut self, name: &str, values: &[u16]) -> FormatResult<()> {
        self.write_text(name)?;
        self.write_token("(")?;

        // write values in encoded order
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_text(&value.to_string())?;
        }
        self.write_token(")")?;

        Ok(())
    }

    /// Write one named unsigned 64-bit list.
    fn write_named_u64s(&mut self, name: &str, values: &[u64]) -> FormatResult<()> {
        self.write_text(name)?;
        self.write_token("(")?;

        // write values in encoded order
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_text(&value.to_string())?;
        }
        self.write_token(")")?;

        Ok(())
    }

    /// Decode one tensor operand and return its first register.
    fn tensor_id(&mut self) -> FormatResult<RegisterId> {
        let (register, word_count) = self.register_range_id()?;
        let (_, target) = self.symbol_with_target()?;
        if target.tag != SymbolTag::TYPE {
            return Err(FormatError::SyntaxError {
                message: "tensor operand does not reference a runtime type",
            });
        }
        let value_type = self.formatter.context().register_type(register)?;
        let tensor_type = value_type.tensor_type().ok_or(FormatError::SyntaxError {
            message: "tensor operand reads a non-tensor value",
        })?;
        if word_count != value_type.word_count() || tensor_type != TypeId(target.index) {
            return Err(FormatError::SyntaxError {
                message: "tensor operand does not match its register value",
            });
        }

        Ok(register)
    }

    /// Decode one counted tensor operand list.
    fn tensor_ids(&mut self) -> FormatResult<Vec<RegisterId>> {
        let count = self.u16()?;
        let mut registers = Vec::with_capacity(count as usize);

        // decode tensors in source order
        for _ in 0..count {
            registers.push(self.tensor_id()?);
        }

        Ok(registers)
    }

    /// Return the scalar representation carried by one result type.
    fn result_scalar(&self, result_type: Option<ValueType>) -> FormatResult<Scalar> {
        let result_type = result_type.ok_or(FormatError::SyntaxError {
            message: "tensor instruction has no result type",
        })?;

        result_type
            .tensor_scalar()
            .or_else(|| result_type.scalar_type())
            .ok_or(FormatError::SyntaxError {
                message: "tensor instruction result has no scalar representation",
            })
    }

    /// Return the tensor scalar representation beginning at one register.
    fn register_tensor_scalar(&self, register: RegisterId) -> FormatResult<Scalar> {
        self.formatter
            .context()
            .register_type(register)?
            .tensor_scalar()
            .ok_or(FormatError::SyntaxError {
                message: "tensor instruction reads a non-tensor value",
            })
    }

    /// Consume and require one exact scalar representation.
    fn require_scalar(&mut self, expected: Scalar) -> FormatResult<()> {
        let actual = self.scalar()?;
        if actual != expected {
            return Err(FormatError::SyntaxError {
                message: "tensor scalar operand does not match its value type",
            });
        }

        Ok(())
    }
}
