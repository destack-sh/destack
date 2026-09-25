use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl FunctionEmitter<'_> {
    /// Emit one scalar splat across a fixed-width vector.
    pub(super) fn emit_vector_splat(
        &mut self,
        destination: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let vector = self.vector_type(destination)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::vector(
            bytecode::VectorOperation::Splat,
        ));
        instruction.register(self.word(value)?);
        instruction.vector_type(vector);

        self.encode(instruction, &[self.register(destination)?])
    }

    /// Emit one vector lane extraction.
    pub(super) fn emit_vector_extract(
        &mut self,
        destination: mir::Value,
        vector: mir::Value,
        index: mir::Value,
    ) -> Result<(), EmitError> {
        let vector_type = self.vector_type(vector)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::vector(
            bytecode::VectorOperation::Extract,
        ));
        instruction.span(self.register(vector)?);
        instruction.register(self.word(index)?);
        instruction.vector_type(vector_type);

        self.encode(instruction, &[self.register(destination)?])
    }

    /// Emit one vector lane insertion.
    pub(super) fn emit_vector_insert(
        &mut self,
        destination: mir::Value,
        vector: mir::Value,
        index: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let vector_type = self.vector_type(vector)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::vector(
            bytecode::VectorOperation::Insert,
        ));
        instruction.span(self.register(vector)?);
        instruction.register(self.word(index)?);
        instruction.register(self.word(value)?);
        instruction.vector_type(vector_type);

        self.encode(instruction, &[self.register(destination)?])
    }

    /// Emit one constant vector lane shuffle.
    pub(super) fn emit_vector_shuffle(
        &mut self,
        destination: mir::Value,
        left: mir::Value,
        right: mir::Value,
        mask: mir::IndexSlice,
    ) -> Result<(), EmitError> {
        // read the vector type and shuffle indices
        let vector = self.vector_type(left)?;
        let inputs = [self.register(left)?, self.register(right)?];
        let mask = self
            .optimized
            .tree
            .get_indices(mask)
            .iter()
            .copied()
            .map(|index| u16::try_from(index).map_err(|_| self.internal("vector lane exceeds u16")))
            .collect::<Result<Vec<_>, _>>()?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::vector(
            bytecode::VectorOperation::Shuffle,
        ));
        instruction
            .span_starts(&inputs)
            .map_err(|error| self.bytecode_error(error))?;
        instruction
            .u16s(&mask)
            .map_err(|error| self.bytecode_error(error))?;
        instruction.vector_type(vector);

        self.encode(instruction, &[self.register(destination)?])
    }

    /// Emit one elementwise vector operation.
    pub(super) fn emit_vector_element(
        &mut self,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    ) -> Result<(), EmitError> {
        let vector = self.vector_type(left)?;
        let vector = if operator == mir::BinaryOperator::UnsignedShiftRight {
            let scalar = vector
                .scalar
                .unsigned()
                .ok_or_else(|| self.internal("unsigned vector shift requires integer lanes"))?;

            bytecode::VectorType::new(scalar, vector.lane_count)
        } else {
            vector
        };
        let operator = Self::vector_operator(operator, vector.scalar)
            .ok_or_else(|| self.internal("invalid vector operation"))?;
        let inputs = [self.register(left)?, self.register(right)?];
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::vector(
            bytecode::VectorOperation::Element,
        ));
        instruction
            .span_starts(&inputs)
            .map_err(|error| self.bytecode_error(error))?;
        instruction.vector_type(vector);
        instruction.u16(operator);

        self.encode(instruction, &[self.register(destination)?])
    }

    /// Emit one vector lane selection.
    pub(super) fn emit_vector_select(
        &mut self,
        destination: mir::Value,
        mask: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
    ) -> Result<(), EmitError> {
        let vector = self.vector_type(then_value)?;
        let inputs = [
            self.register(mask)?,
            self.register(then_value)?,
            self.register(else_value)?,
        ];
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::vector(
            bytecode::VectorOperation::Select,
        ));
        instruction
            .span_starts(&inputs)
            .map_err(|error| self.bytecode_error(error))?;
        instruction.vector_type(vector);

        self.encode(instruction, &[self.register(destination)?])
    }

    /// Emit one vector reduction.
    pub(super) fn emit_vector_reduce(
        &mut self,
        destination: mir::Value,
        operator: mir::VectorReduceOperator,
        vector: mir::Value,
    ) -> Result<(), EmitError> {
        let operation = Self::reduce_operation(operator);
        let vector_type = self.vector_type(vector)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::vector(
            bytecode::VectorOperation::Reduce,
        ));
        instruction.span(self.register(vector)?);
        instruction.vector_type(vector_type);
        instruction.u16(operation as u16);

        self.encode(instruction, &[self.register(destination)?])
    }

    /// Emit one vector lane comparison.
    pub(super) fn emit_vector_compare(
        &mut self,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    ) -> Result<(), EmitError> {
        let vector = self.vector_type(left)?;
        let operator = Self::vector_operator(operator, vector.scalar)
            .ok_or_else(|| self.internal("invalid vector comparison"))?;
        let inputs = [self.register(left)?, self.register(right)?];
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::vector(
            bytecode::VectorOperation::Compare,
        ));
        instruction
            .span_starts(&inputs)
            .map_err(|error| self.bytecode_error(error))?;
        instruction.vector_type(vector);
        instruction.u16(operator);

        self.encode(instruction, &[self.register(destination)?])
    }

    /// Emit one vector lane conversion.
    pub(super) fn emit_vector_convert(
        &mut self,
        destination: mir::Value,
        mode: mir::ConvertMode,
        vector: mir::Value,
    ) -> Result<(), EmitError> {
        let source = self.vector_type(vector)?;
        let target = self.vector_type(destination)?;
        let mode = Self::convert_mode(mode);
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::vector(
            bytecode::VectorOperation::Convert,
        ));
        instruction.span(self.register(vector)?);
        instruction.vector_type(source);
        instruction.vector_type(target);
        instruction.u16(mode as u16);

        self.encode(instruction, &[self.register(destination)?])
    }

    /// Return one MIR value's fixed-width vector representation.
    fn vector_type(&self, value: mir::Value) -> Result<bytecode::VectorType, EmitError> {
        self.register_type(value)?
            .vector_type()
            .ok_or_else(|| self.internal("expected vector value"))
    }

    /// Return one bytecode reduction operation.
    fn reduce_operation(operator: mir::VectorReduceOperator) -> bytecode::ReduceOperation {
        match operator {
            mir::VectorReduceOperator::Add => bytecode::ReduceOperation::Add,
            mir::VectorReduceOperator::Multiply => bytecode::ReduceOperation::Multiply,
            mir::VectorReduceOperator::Min => bytecode::ReduceOperation::Minimum,
            mir::VectorReduceOperator::Max => bytecode::ReduceOperation::Maximum,
            mir::VectorReduceOperator::And => bytecode::ReduceOperation::And,
            mir::VectorReduceOperator::Or => bytecode::ReduceOperation::Or,
            mir::VectorReduceOperator::Xor => bytecode::ReduceOperation::Xor,
        }
    }

    /// Return one bytecode operation for a vector lane representation.
    fn vector_operator(operator: mir::BinaryOperator, scalar: bytecode::Scalar) -> Option<u16> {
        let operation = if scalar.is_float() {
            match operator {
                mir::BinaryOperator::Add => bytecode::FloatOperation::Add as u16,
                mir::BinaryOperator::Subtract => bytecode::FloatOperation::Subtract as u16,
                mir::BinaryOperator::Multiply => bytecode::FloatOperation::Multiply as u16,
                mir::BinaryOperator::Divide => bytecode::FloatOperation::Divide as u16,
                mir::BinaryOperator::Remainder => bytecode::FloatOperation::Remainder as u16,
                mir::BinaryOperator::Equal => bytecode::FloatOperation::Equal as u16,
                mir::BinaryOperator::NotEqual => bytecode::FloatOperation::NotEqual as u16,
                mir::BinaryOperator::LessThan => bytecode::FloatOperation::LessThan as u16,
                mir::BinaryOperator::LessEqual => bytecode::FloatOperation::LessEqual as u16,
                mir::BinaryOperator::GreaterThan => bytecode::FloatOperation::GreaterThan as u16,
                mir::BinaryOperator::GreaterEqual => bytecode::FloatOperation::GreaterEqual as u16,
                _ => return None,
            }
        } else {
            match operator {
                mir::BinaryOperator::Add => bytecode::IntegerOperation::Add as u16,
                mir::BinaryOperator::Subtract => bytecode::IntegerOperation::Subtract as u16,
                mir::BinaryOperator::Multiply => bytecode::IntegerOperation::Multiply as u16,
                mir::BinaryOperator::Divide => bytecode::IntegerOperation::Divide as u16,
                mir::BinaryOperator::Remainder => bytecode::IntegerOperation::Remainder as u16,
                mir::BinaryOperator::And => bytecode::IntegerOperation::And as u16,
                mir::BinaryOperator::Or => bytecode::IntegerOperation::Or as u16,
                mir::BinaryOperator::Xor => bytecode::IntegerOperation::Xor as u16,
                mir::BinaryOperator::ShiftLeft => bytecode::IntegerOperation::ShiftLeft as u16,
                mir::BinaryOperator::ShiftRight | mir::BinaryOperator::UnsignedShiftRight => {
                    bytecode::IntegerOperation::ShiftRight as u16
                }
                mir::BinaryOperator::Equal => bytecode::IntegerOperation::Equal as u16,
                mir::BinaryOperator::NotEqual => bytecode::IntegerOperation::NotEqual as u16,
                mir::BinaryOperator::LessThan => bytecode::IntegerOperation::LessThan as u16,
                mir::BinaryOperator::LessEqual => bytecode::IntegerOperation::LessEqual as u16,
                mir::BinaryOperator::GreaterThan => bytecode::IntegerOperation::GreaterThan as u16,
                mir::BinaryOperator::GreaterEqual => {
                    bytecode::IntegerOperation::GreaterEqual as u16
                }
            }
        };

        Some(operation)
    }
}
