use crate::{
    ConvertMode, ElementOperation, IndexReduceOperation, Operand, ReduceOperation, RegisterId,
    RegisterSpan, ScatterOperation, TensorOperand, TensorOperation,
};
use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one tensor instruction from its exact operand layout.
    pub(super) fn format_tensor(&mut self) -> FormatResult<()> {
        let operation =
            self.instruction
                .opcode()
                .tensor_operation()
                .ok_or(FormatError::SyntaxError {
                    message: "tensor instruction has an invalid opcode",
                })?;
        let layout = self
            .instruction
            .opcode()
            .layout()
            .ok_or(FormatError::SyntaxError {
                message: "tensor instruction has no operand layout",
            })?;
        // write the exact operation and encoded operands
        let name = format!("tensor.{}", operation.name());
        self.write_opcode(&name)?;
        for (index, operand) in layout.operands().iter().enumerate() {
            if index > 0 {
                self.write_comma()?;
            }
            self.format_tensor_operand(operation, *operand)?;
        }

        Ok(())
    }

    /// Format one encoded tensor instruction operand.
    fn format_tensor_operand(
        &mut self,
        operation: TensorOperation,
        operand: Operand,
    ) -> FormatResult<()> {
        match operand {
            Operand::Result => {
                let register = self.register_id()?;
                self.write_register(register)
            }
            Operand::ResultRange => {
                let span = self.read_tensor_span()?;
                self.write_span(span)
            }
            Operand::Register => {
                let register = self.register_id()?;
                self.write_register(register)
            }
            Operand::RegisterList => {
                let registers = self.register_ids()?;
                self.write_register_list(&registers)
            }
            Operand::RegisterSpan => {
                let span = self.read_tensor_span()?;
                self.write_span(span)
            }
            Operand::Tensor => {
                let tensor = self.tensor()?;
                self.write_tensor_value(tensor)
            }
            Operand::TensorList => {
                let tensors = self.tensors()?;
                self.write_tensor_list(&tensors)
            }
            Operand::Layout | Operand::Allocation => {
                let identity = self.relocation_text()?;
                self.write_text(&identity)
            }
            Operand::Operator => {
                let code = self.u16()?;
                let name = self.tensor_operator(operation, code)?;
                self.write_text(&name)
            }
            Operand::Unsigned16 => {
                let value = self.u16()?;
                self.write_text(&value.to_string())
            }
            Operand::Unsigned16List => {
                let values = self.u16_list()?;
                self.write_u16_list(&values)
            }
            Operand::Bits64List => {
                let values = self.u64_list()?;
                self.write_u64_list(&values)
            }
            Operand::ContractionAxes => self.format_contraction_axes(),
            Operand::ConvolutionAxes => self.format_convolution_axes(),
            Operand::Window => self.format_window(),
            Operand::ConvolutionGroups => self.format_convolution_groups(),
            Operand::GatherAxes | Operand::ScatterAxes => self.format_index_axes(),
            _ => Err(FormatError::SyntaxError {
                message: "tensor instruction has an invalid operand layout",
            }),
        }
    }

    /// Decode one tensor operation code with its canonical name.
    fn tensor_operator(&self, operation: TensorOperation, code: u16) -> FormatResult<String> {
        let name = match operation {
            TensorOperation::Element | TensorOperation::Compare => {
                let operation = ElementOperation::from_code(code);
                if let Some(operation) = operation.and_then(ElementOperation::integer_operation) {
                    Some(format!("int.{}", operation.name()))
                } else {
                    operation
                        .and_then(ElementOperation::float_operation)
                        .map(|operation| format!("float.{}", operation.name()))
                }
            }
            TensorOperation::Convert => ConvertMode::from_code(code as u8)
                .map(ConvertMode::name)
                .map(str::to_string),
            TensorOperation::Reduce => ReduceOperation::from_code(code as u8)
                .map(ReduceOperation::name)
                .map(str::to_string),
            TensorOperation::IndexReduce => IndexReduceOperation::from_code(code as u8)
                .map(IndexReduceOperation::name)
                .map(str::to_string),
            TensorOperation::Scatter => ScatterOperation::from_code(code as u8)
                .map(ScatterOperation::name)
                .map(str::to_string),
            _ => None,
        };

        name.ok_or(FormatError::SyntaxError {
            message: "tensor instruction has an invalid operation code",
        })
    }

    /// Write one tensor register span and runtime layout.
    fn write_tensor_value(&mut self, tensor: TensorOperand) -> FormatResult<()> {
        self.write_span(tensor.registers)?;
        write!(self.formatter, [space(), token("@"), space()])?;
        self.write_text(&format!("l{}", tensor.layout.0))?;

        Ok(())
    }

    /// Write one bracketed tensor operand list.
    fn write_tensor_list(&mut self, tensors: &[TensorOperand]) -> FormatResult<()> {
        self.write_token("[")?;
        for (index, tensor) in tensors.iter().copied().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_tensor_value(tensor)?;
        }

        self.write_token("]")
    }

    /// Write one bracketed register list.
    fn write_register_list(&mut self, registers: &[RegisterId]) -> FormatResult<()> {
        self.write_token("[")?;
        self.write_registers(registers)?;
        self.write_token("]")
    }

    /// Write one bracketed unsigned 16-bit list.
    fn write_u16_list(&mut self, values: &[u16]) -> FormatResult<()> {
        self.write_token("[")?;
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_text(&value.to_string())?;
        }

        self.write_token("]")
    }

    /// Write one bracketed unsigned 64-bit list.
    fn write_u64_list(&mut self, values: &[u64]) -> FormatResult<()> {
        self.write_token("[")?;
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_text(&value.to_string())?;
        }

        self.write_token("]")
    }

    /// Format four tensor contraction axis lists.
    fn format_contraction_axes(&mut self) -> FormatResult<()> {
        self.write_token("axes(")?;
        for index in 0..4 {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            let values = self.u16_list()?;
            self.write_u16_list(&values)?;
        }

        self.write_token(")")
    }

    /// Format three tensor convolution dimension mappings.
    fn format_convolution_axes(&mut self) -> FormatResult<()> {
        self.write_token("axes(")?;
        for index in 0..3 {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            let feature = self.u16()?;
            let batch = self.u16()?;
            let spatial = self.u16_list()?;
            self.write_token("(")?;
            self.write_text(&feature.to_string())?;
            write!(self.formatter, [token(","), space()])?;
            self.write_text(&batch.to_string())?;
            write!(self.formatter, [token(","), space()])?;
            self.write_u16_list(&spatial)?;
            self.write_token(")")?;
        }

        self.write_token(")")
    }

    /// Format one tensor convolution window.
    fn format_window(&mut self) -> FormatResult<()> {
        self.write_token("window(")?;
        for index in 0..5 {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            let values = self.u64_list()?;
            self.write_u64_list(&values)?;
        }
        write!(self.formatter, [token(","), space()])?;
        let reversals = self.u16_list()?;
        self.write_u16_list(&reversals)?;
        self.write_token(")")
    }

    /// Format tensor convolution feature and batch group counts.
    fn format_convolution_groups(&mut self) -> FormatResult<()> {
        let feature = self.u32()?;
        let batch = self.u32()?;
        self.write_token("groups(")?;
        self.write_text(&feature.to_string())?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(&batch.to_string())?;
        self.write_token(")")
    }

    /// Format gather or scatter dimension mappings.
    fn format_index_axes(&mut self) -> FormatResult<()> {
        self.write_token("axes(")?;
        for index in 0..3 {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            let values = self.u16_list()?;
            self.write_u16_list(&values)?;
        }
        write!(self.formatter, [token(","), space()])?;
        let index_vector = self.u16()?;
        self.write_text(&index_vector.to_string())?;
        self.write_token(")")
    }

    /// Read one physical tensor register span.
    fn read_tensor_span(&mut self) -> FormatResult<RegisterSpan> {
        let (start, word_count) = self.register_span_id()?;

        Ok(RegisterSpan::new(start, word_count))
    }
}
