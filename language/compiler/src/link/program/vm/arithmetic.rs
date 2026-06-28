use destack_mir as mir;

use crate::LinkResult;
use destack_mir::Type;

use destack_program::vm::{
    BinaryFloat, BinaryFloatKernel, ElementBinaryKernel, ElementUnaryKernel, Instruction, Op,
    TensorBinary, TensorContiguousBinary, TensorContiguousUnary, TensorLayout, TensorUnary,
    UnaryFloat, UnaryFloatKernel, VectorBinary, VectorUnary,
};

use super::lower::BlockLowerer;
use super::op::{select_binary_op, select_integer_op, select_integer_unary_op, select_unary_op};
use super::pool::Pool;
use super::value::Operand;
use super::vector::PackedVector;

const INTEGER_SIGN_BIT: u32 = 1 << 16;

impl<'a> BlockLowerer<'a> {
    /// Lower one cell binary instruction.
    fn lower_binary_cell(
        &self,
        op: Op,
        destination: mir::Value,
        left: mir::Value,
        right: mir::Value,
    ) -> LinkResult<Instruction> {
        let destination = self.cell_offset(destination)?;
        let left = self.cell_offset(left)?;
        let right = self.cell_offset(right)?;

        Ok(Instruction::new(op, destination, left, right, 0))
    }

    /// Lower one vector binary instruction.
    fn lower_vector_binary(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        destination_type: mir::LocalNodeId<mir::Type>,
        operator: mir::BinaryOperator,
        left: mir::Value,
        left_type: mir::LocalNodeId<mir::Type>,
        right: mir::Value,
    ) -> LinkResult<Instruction> {
        // decode vector element layouts
        let (dest_element, element_count, dest_element_type) =
            self.vector_element_projection(destination_type)?;
        let (left_element, left_count, element) = self.vector_element_projection(left_type)?;
        let right_type = self.value_type_for_value(right)?;
        let (right_element, right_count, _) = self.vector_element_projection(right_type)?;

        // require identical vector widths
        if element_count != left_count || element_count != right_count {
            return Err(self.invalid_instruction("vector binary width"));
        }

        // use direct packed operations when one register covers the vector
        let element_layout = self
            .function
            .operand_for_type(element)
            .ok_or_else(|| self.invalid_instruction("vector binary element"))?;
        if let Some(op) = self
            .packed_vector(dest_element, element_count, dest_element_type)?
            .and_then(|shape| vector_packed_binary_op(operator, shape))
        {
            return Ok(Instruction::new(
                op,
                self.value_offset(destination)?,
                self.value_offset(left)?,
                self.value_offset(right)?,
                0,
            ));
        }

        // fall back to a side record for general element counts
        let kernel = element_binary_kernel(operator, element_layout)
            .ok_or_else(|| self.invalid_instruction("vector binary operator"))?;

        Ok(pool.instruction_with_side(
            Op::VectorBinary,
            VectorBinary {
                dest_offset: self.value_offset(destination)?,
                left_offset: self.value_offset(left)?,
                right_offset: self.value_offset(right)?,
                dest_element,
                left_element,
                right_element,
                kernel,
                element_layout: self
                    .function
                    .require_scalar_format(element, "scalar vector element")?,
                element_count,
            },
        ))
    }

    /// Lower one tensor binary instruction.
    fn lower_tensor_binary(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        destination_type: mir::LocalNodeId<mir::Type>,
        operator: mir::BinaryOperator,
        left: mir::Value,
        left_type: mir::LocalNodeId<mir::Type>,
        right: mir::Value,
    ) -> LinkResult<Instruction> {
        // decode tensor element and layouts
        let right_type = self.value_type_for_value(right)?;
        let element = self
            .tensor_element_type(left_type)
            .ok_or_else(|| self.invalid_instruction("tensor binary element"))?;
        let element_layout = self
            .function
            .operand_for_type(element)
            .ok_or_else(|| self.invalid_instruction("tensor binary element layout"))?;
        let kernel = element_binary_kernel(operator, element_layout)
            .ok_or_else(|| self.invalid_instruction("tensor binary operator"))?;
        let left_layout = self.tensor_layout_from_type(left_type)?;
        let right_layout = self.tensor_layout_from_type(right_type)?;
        let dest_layout = self.tensor_layout_from_type(destination_type)?;

        // use one contiguous descriptor when all views share physical order
        if same_contiguous_tensor_order(&dest_layout, &left_layout, &right_layout) {
            let dest_layout = pool.tensor_layout(dest_layout);

            return Ok(pool.instruction_with_side(
                Op::TensorContiguousBinary,
                TensorContiguousBinary {
                    dest_offset: self.value_offset(destination)?,
                    left_offset: self.value_offset(left)?,
                    right_offset: self.value_offset(right)?,
                    dest_layout,
                    element_layout: self
                        .function
                        .require_scalar_format(element, "tensor scalar element")?,
                    kernel,
                },
            ));
        }

        // preserve full view tables for strided tensor operations
        let left_layout = pool.tensor_layout(left_layout);
        let right_layout = pool.tensor_layout(right_layout);
        let dest_layout = pool.tensor_layout(dest_layout);

        Ok(pool.instruction_with_side(
            Op::TensorBinary,
            TensorBinary {
                dest_offset: self.value_offset(destination)?,
                left_offset: self.value_offset(left)?,
                right_offset: self.value_offset(right)?,
                left_layout,
                right_layout,
                dest_layout,
                kernel,
                element_layout: self
                    .function
                    .require_scalar_format(element, "tensor scalar element")?,
            },
        ))
    }

    /// Lower one scalar binary instruction.
    fn lower_scalar_binary(
        &self,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        left_type: mir::LocalNodeId<mir::Type>,
        right: mir::Value,
    ) -> LinkResult<Instruction> {
        // specialize machine-cell integers by signedness and width
        let layout = self
            .operand_map()
            .get(left)
            .or_else(|| self.function.operand_for_type(left_type));
        if let Some(Operand::Int { width, signed }) = layout
            && let Some(op) = select_integer_op(operator, signed, width)
        {
            return Ok(Instruction::new(
                op,
                self.cell_offset(destination)?,
                self.cell_offset(left)?,
                self.cell_offset(right)?,
                integer_layout_field(width, signed),
            ));
        }

        // encode 16 bit float formats without a side table
        if let Some(Operand::Float { format }) = layout
            && !matches!(format, mir::FloatType::Float32 | mir::FloatType::Float64)
        {
            let kernel = BinaryFloatKernel::select(operator)
                .ok_or_else(|| self.invalid_instruction("binary float operator"))?;
            let operation = BinaryFloat::new(format, kernel);

            return Ok(Instruction::new(
                Op::BinaryFloat,
                self.cell_offset(destination)?,
                self.cell_offset(left)?,
                self.cell_offset(right)?,
                operation.field(),
            ));
        }

        // select the remaining scalar family
        let op = match select_binary_op(layout, operator) {
            Some(op) => op,
            None => return Err(self.invalid_instruction("scalar binary operator")),
        };

        // wide integers use frame byte addresses
        if is_wide_binary_op(op) {
            let Some(Operand::Int { width, signed }) = layout else {
                return Err(self.invalid_instruction("wide integer binary operand"));
            };
            let destination = if is_wide_comparison_op(op) {
                self.cell_offset(destination)?
            } else {
                self.value_offset(destination)?
            };

            return Ok(Instruction::new(
                op,
                destination,
                self.value_offset(left)?,
                self.value_offset(right)?,
                integer_layout_field(width, signed),
            ));
        }

        self.lower_binary_cell(op, destination, left, right)
    }

    /// Lower one cell unary instruction.
    fn lower_unary_cell(
        &self,
        op: Op,
        destination: mir::Value,
        argument: mir::Value,
    ) -> LinkResult<Instruction> {
        let destination = self.cell_offset(destination)?;
        let argument = self.cell_offset(argument)?;

        Ok(Instruction::new(op, destination, argument, 0, 0))
    }

    /// Lower one vector unary instruction.
    fn lower_vector_unary(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        destination_type: mir::LocalNodeId<mir::Type>,
        operator: mir::UnaryOperator,
        argument: mir::Value,
        argument_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<Instruction> {
        // decode vector element layouts
        let (dest_element, element_count, element) =
            self.vector_element_projection(destination_type)?;
        let (argument_element, argument_count, _) =
            self.vector_element_projection(argument_type)?;

        // require identical vector widths
        if element_count != argument_count {
            return Err(self.invalid_instruction("vector unary width"));
        }

        // use direct packed operations when one register covers the vector
        let element_layout = self
            .function
            .operand_for_type(element)
            .ok_or_else(|| self.invalid_instruction("vector unary element"))?;
        if let Some(op) = self
            .packed_vector(dest_element, element_count, element)?
            .and_then(|shape| vector_packed_unary_op(operator, shape))
        {
            return Ok(Instruction::new(
                op,
                self.value_offset(destination)?,
                self.value_offset(argument)?,
                0,
                0,
            ));
        }

        // fall back to a side record for general element counts
        let kernel = element_unary_kernel(operator, element_layout)
            .ok_or_else(|| self.invalid_instruction("vector unary operator"))?;

        Ok(pool.instruction_with_side(
            Op::VectorUnary,
            VectorUnary {
                dest_offset: self.value_offset(destination)?,
                argument_offset: self.value_offset(argument)?,
                dest_element,
                argument_element,
                kernel,
                element_layout: self
                    .function
                    .require_scalar_format(element, "scalar vector element")?,
                element_count,
            },
        ))
    }

    /// Lower one tensor unary instruction.
    fn lower_tensor_unary(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        destination_type: mir::LocalNodeId<mir::Type>,
        operator: mir::UnaryOperator,
        argument: mir::Value,
        argument_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<Instruction> {
        // decode tensor element and layouts
        let element = self
            .tensor_element_type(argument_type)
            .ok_or_else(|| self.invalid_instruction("tensor unary element"))?;
        let element_layout = self
            .function
            .operand_for_type(element)
            .ok_or_else(|| self.invalid_instruction("tensor unary element layout"))?;
        let kernel = element_unary_kernel(operator, element_layout)
            .ok_or_else(|| self.invalid_instruction("tensor unary operator"))?;
        let argument_layout = self.tensor_layout_from_type(argument_type)?;
        let dest_layout = self.tensor_layout_from_type(destination_type)?;

        // use one contiguous descriptor when both views share physical order
        if same_contiguous_tensor_unary_order(&dest_layout, &argument_layout) {
            let dest_layout = pool.tensor_layout(dest_layout);

            return Ok(pool.instruction_with_side(
                Op::TensorContiguousUnary,
                TensorContiguousUnary {
                    dest_offset: self.value_offset(destination)?,
                    argument_offset: self.value_offset(argument)?,
                    dest_layout,
                    element_layout: self
                        .function
                        .require_scalar_format(element, "tensor scalar element")?,
                    kernel,
                },
            ));
        }

        // preserve full view tables for strided tensor operations
        let argument_layout = pool.tensor_layout(argument_layout);
        let dest_layout = pool.tensor_layout(dest_layout);

        Ok(pool.instruction_with_side(
            Op::TensorUnary,
            TensorUnary {
                dest_offset: self.value_offset(destination)?,
                argument_offset: self.value_offset(argument)?,
                argument_layout,
                element_layout: self
                    .function
                    .require_scalar_format(element, "tensor scalar element")?,
                dest_layout,
                kernel,
            },
        ))
    }

    /// Lower one scalar unary instruction.
    fn lower_scalar_unary(
        &self,
        destination: mir::Value,
        operator: mir::UnaryOperator,
        argument: mir::Value,
        argument_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<Instruction> {
        // specialize machine-cell integers by signedness and width
        let layout = self
            .operand_map()
            .get(argument)
            .or_else(|| self.function.operand_for_type(argument_type));
        if let Some(Operand::Int { width, signed }) = layout
            && let Some(op) = select_integer_unary_op(operator, signed, width)
        {
            return Ok(Instruction::new(
                op,
                self.cell_offset(destination)?,
                self.cell_offset(argument)?,
                0,
                integer_layout_field(width, signed),
            ));
        }

        // encode 16 bit float formats without a side table
        if let Some(Operand::Float { format }) = layout
            && !matches!(format, mir::FloatType::Float32 | mir::FloatType::Float64)
        {
            let kernel = UnaryFloatKernel::select(operator)
                .ok_or_else(|| self.invalid_instruction("unary float operator"))?;
            let operation = UnaryFloat::new(format, kernel);

            return Ok(Instruction::new(
                Op::UnaryFloat,
                self.cell_offset(destination)?,
                self.cell_offset(argument)?,
                0,
                operation.field(),
            ));
        }

        // select the remaining scalar family
        let op = match select_unary_op(self.operand_map(), argument, operator) {
            Some(op) => op,
            None => return Err(self.invalid_instruction("scalar unary operator")),
        };

        // wide integers use frame byte addresses
        if is_wide_unary_op(op) {
            let Some(Operand::Int { width, signed }) =
                self.function.operand_for_type(argument_type)
            else {
                return Err(self.invalid_instruction("wide integer unary operand"));
            };

            return Ok(Instruction::new(
                op,
                self.value_offset(destination)?,
                self.value_offset(argument)?,
                0,
                integer_layout_field(width, signed),
            ));
        }

        self.lower_unary_cell(op, destination, argument)
    }

    /// Lower one binary instruction.
    pub(super) fn lower_binary(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    ) -> LinkResult<Instruction> {
        let destination_type = self.value_type_for_value(destination)?;
        let left_type = self.value_type_for_value(left)?;

        // select aggregate arithmetic by static value family
        match self.function.tree.get(destination_type) {
            Type::Vector { .. } => {
                return self.lower_vector_binary(
                    pool,
                    destination,
                    destination_type,
                    operator,
                    left,
                    left_type,
                    right,
                );
            }
            Type::Tensor { .. } => {
                return self.lower_tensor_binary(
                    pool,
                    destination,
                    destination_type,
                    operator,
                    left,
                    left_type,
                    right,
                );
            }
            _ => {}
        }

        self.lower_scalar_binary(destination, operator, left, left_type, right)
    }

    /// Lower one unary instruction.
    pub(super) fn lower_unary(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        operator: mir::UnaryOperator,
        argument: mir::Value,
    ) -> LinkResult<Instruction> {
        let argument_type = self.value_type_for_value(argument)?;
        let destination_type = self.value_type_for_value(destination)?;

        // select aggregate arithmetic by static value family
        match self.function.tree.get(destination_type) {
            Type::Vector { .. } => {
                return self.lower_vector_unary(
                    pool,
                    destination,
                    destination_type,
                    operator,
                    argument,
                    argument_type,
                );
            }
            Type::Tensor { .. } => {
                return self.lower_tensor_unary(
                    pool,
                    destination,
                    destination_type,
                    operator,
                    argument,
                    argument_type,
                );
            }
            _ => {}
        }

        self.lower_scalar_unary(destination, operator, argument, argument_type)
    }
}

/// Return whether three tensor layouts share one contiguous element order.
pub(super) fn same_contiguous_tensor_order(
    dest_layout: &TensorLayout,
    left_layout: &TensorLayout,
    right_layout: &TensorLayout,
) -> bool {
    dest_layout.is_contiguous
        && left_layout.is_contiguous
        && right_layout.is_contiguous
        && dest_layout.shape == left_layout.shape
        && dest_layout.shape == right_layout.shape
        && dest_layout.strides == left_layout.strides
        && dest_layout.strides == right_layout.strides
}

/// Return whether two tensor layouts share one contiguous element order.
pub(super) fn same_contiguous_tensor_unary_order(
    dest_layout: &TensorLayout,
    argument_layout: &TensorLayout,
) -> bool {
    dest_layout.is_contiguous
        && argument_layout.is_contiguous
        && dest_layout.shape == argument_layout.shape
        && dest_layout.strides == argument_layout.strides
}

/// Select one element binary kernel.
pub(super) fn element_binary_kernel(
    operator: mir::BinaryOperator,
    layout: Operand,
) -> Option<ElementBinaryKernel> {
    use ElementBinaryKernel::*;
    use mir::BinaryOperator::*;

    Some(match layout {
        Operand::Boolean => match operator {
            And => AndBool,
            Or => OrBool,
            Xor => XorBool,
            Equal => EqBool,
            NotEqual => NeBool,
            _ => return None,
        },
        Operand::Int { signed, .. } => match (operator, signed) {
            (Add, _) => AddInt,
            (Subtract, _) => SubInt,
            (Multiply, _) => MulInt,
            (SignedDivide, true) => DivInt,
            (UnsignedDivide, _) => DivUint,
            (SignedRemainder, true) => RemInt,
            (UnsignedRemainder, _) => RemUint,
            (And, _) => AndInt,
            (Or, _) => OrInt,
            (Xor, _) => XorInt,
            (ShiftLeft, _) => ShlInt,
            (ArithmeticShiftRight, true) => ShrInt,
            (LogicalShiftRight, _) => ShrUint,
            (Equal, _) => EqInt,
            (NotEqual, _) => NeInt,
            (SignedLessThan, true) => LtInt,
            (SignedLessEqual, true) => LeInt,
            (SignedGreaterThan, true) => GtInt,
            (SignedGreaterEqual, true) => GeInt,
            (UnsignedLessThan, _) => LtUint,
            (UnsignedLessEqual, _) => LeUint,
            (UnsignedGreaterThan, _) => GtUint,
            (UnsignedGreaterEqual, _) => GeUint,
            _ => return None,
        },
        Operand::Float {
            format: mir::FloatType::Float32,
        } => match operator {
            FloatAdd => AddF32,
            FloatSubtract => SubF32,
            FloatMultiply => MulF32,
            FloatDivide => DivF32,
            FloatEqual => EqF32,
            FloatNotEqual => NeF32,
            FloatLessThan => LtF32,
            FloatLessEqual => LeF32,
            FloatGreaterThan => GtF32,
            FloatGreaterEqual => GeF32,
            _ => return None,
        },
        Operand::Float {
            format: mir::FloatType::Float64,
        } => match operator {
            FloatAdd => AddF64,
            FloatSubtract => SubF64,
            FloatMultiply => MulF64,
            FloatDivide => DivF64,
            FloatEqual => EqF64,
            FloatNotEqual => NeF64,
            FloatLessThan => LtF64,
            FloatLessEqual => LeF64,
            FloatGreaterThan => GtF64,
            FloatGreaterEqual => GeF64,
            _ => return None,
        },
        Operand::Float { .. } => match operator {
            FloatAdd => AddFloat,
            FloatSubtract => SubFloat,
            FloatMultiply => MulFloat,
            FloatDivide => DivFloat,
            FloatEqual => EqFloat,
            FloatNotEqual => NeFloat,
            FloatLessThan => LtFloat,
            FloatLessEqual => LeFloat,
            FloatGreaterThan => GtFloat,
            FloatGreaterEqual => GeFloat,
            _ => return None,
        },
        _ => return None,
    })
}

/// Select a direct packed vector binary opcode.
fn vector_packed_binary_op(operator: mir::BinaryOperator, shape: PackedVector) -> Option<Op> {
    use mir::BinaryOperator::*;

    Some(match (operator, shape) {
        (Add, PackedVector::I32x4 | PackedVector::U32x4) => Op::PackedAdd32x4,
        (Subtract, PackedVector::I32x4 | PackedVector::U32x4) => Op::PackedSub32x4,
        (Multiply, PackedVector::I32x4 | PackedVector::U32x4) => Op::PackedMul32x4,
        (And, PackedVector::I32x4 | PackedVector::U32x4) => Op::PackedAnd32x4,
        (Or, PackedVector::I32x4 | PackedVector::U32x4) => Op::PackedOr32x4,
        (Xor, PackedVector::I32x4 | PackedVector::U32x4) => Op::PackedXor32x4,
        (ShiftLeft, PackedVector::I32x4 | PackedVector::U32x4) => Op::PackedShl32x4,
        (ArithmeticShiftRight, PackedVector::I32x4) => Op::PackedShrI32x4,
        (LogicalShiftRight, PackedVector::I32x4 | PackedVector::U32x4) => Op::PackedShrU32x4,
        (Add, PackedVector::I64x2 | PackedVector::U64x2) => Op::PackedAdd64x2,
        (Subtract, PackedVector::I64x2 | PackedVector::U64x2) => Op::PackedSub64x2,
        (Multiply, PackedVector::I64x2 | PackedVector::U64x2) => Op::PackedMul64x2,
        (And, PackedVector::I64x2 | PackedVector::U64x2) => Op::PackedAnd64x2,
        (Or, PackedVector::I64x2 | PackedVector::U64x2) => Op::PackedOr64x2,
        (Xor, PackedVector::I64x2 | PackedVector::U64x2) => Op::PackedXor64x2,
        (ShiftLeft, PackedVector::I64x2 | PackedVector::U64x2) => Op::PackedShl64x2,
        (ArithmeticShiftRight, PackedVector::I64x2) => Op::PackedShrI64x2,
        (LogicalShiftRight, PackedVector::I64x2 | PackedVector::U64x2) => Op::PackedShrU64x2,
        (FloatAdd, PackedVector::F32x4) => Op::PackedAddF32x4,
        (FloatSubtract, PackedVector::F32x4) => Op::PackedSubF32x4,
        (FloatMultiply, PackedVector::F32x4) => Op::PackedMulF32x4,
        (FloatDivide, PackedVector::F32x4) => Op::PackedDivF32x4,
        (FloatAdd, PackedVector::F64x2) => Op::PackedAddF64x2,
        (FloatSubtract, PackedVector::F64x2) => Op::PackedSubF64x2,
        (FloatMultiply, PackedVector::F64x2) => Op::PackedMulF64x2,
        (FloatDivide, PackedVector::F64x2) => Op::PackedDivF64x2,
        _ => return None,
    })
}

/// Select a direct packed vector unary opcode.
fn vector_packed_unary_op(operator: mir::UnaryOperator, shape: PackedVector) -> Option<Op> {
    Some(match (operator, shape) {
        (mir::UnaryOperator::Negate, PackedVector::I32x4) => Op::PackedNegI32x4,
        (mir::UnaryOperator::Not, PackedVector::I32x4 | PackedVector::U32x4) => Op::PackedNot32x4,
        (mir::UnaryOperator::Negate, PackedVector::I64x2) => Op::PackedNegI64x2,
        (mir::UnaryOperator::Not, PackedVector::I64x2 | PackedVector::U64x2) => Op::PackedNot64x2,
        (mir::UnaryOperator::FloatNegate, PackedVector::F32x4) => Op::PackedNegF32x4,
        (mir::UnaryOperator::FloatNegate, PackedVector::F64x2) => Op::PackedNegF64x2,
        _ => return None,
    })
}

/// Select one element unary kernel.
fn element_unary_kernel(
    operator: mir::UnaryOperator,
    layout: Operand,
) -> Option<ElementUnaryKernel> {
    use ElementUnaryKernel::*;

    Some(match (operator, layout) {
        (mir::UnaryOperator::Negate, Operand::Int { signed: true, .. }) => NegInt,
        (mir::UnaryOperator::Not, Operand::Int { .. }) => NotInt,
        (
            mir::UnaryOperator::FloatNegate,
            Operand::Float {
                format: mir::FloatType::Float32,
            },
        ) => NegF32,
        (
            mir::UnaryOperator::FloatNegate,
            Operand::Float {
                format: mir::FloatType::Float64,
            },
        ) => NegF64,
        (mir::UnaryOperator::FloatNegate, Operand::Float { .. }) => NegFloat,
        (mir::UnaryOperator::Not, Operand::Boolean) => NotBool,
        _ => return None,
    })
}

/// Encode one integer layout field.
fn integer_layout_field(width: u16, is_signed: bool) -> u32 {
    let sign = if is_signed { INTEGER_SIGN_BIT } else { 0 };

    u32::from(width) | sign
}

/// Return whether one operation needs wide integer side data.
fn is_wide_binary_op(op: Op) -> bool {
    matches!(
        op,
        Op::AddWideInt
            | Op::SubWideInt
            | Op::MulWideInt
            | Op::DivWideInt
            | Op::DivWideUint
            | Op::RemWideInt
            | Op::RemWideUint
            | Op::AndWideInt
            | Op::OrWideInt
            | Op::XorWideInt
            | Op::ShlWideInt
            | Op::ShrWideInt
            | Op::ShrWideUint
            | Op::EqWideInt
            | Op::NeWideInt
            | Op::LtWideInt
            | Op::LtWideUint
            | Op::LeWideInt
            | Op::LeWideUint
            | Op::GtWideInt
            | Op::GtWideUint
            | Op::GeWideInt
            | Op::GeWideUint
    )
}

/// Return whether one operation compares wide integers.
fn is_wide_comparison_op(op: Op) -> bool {
    matches!(
        op,
        Op::EqWideInt
            | Op::NeWideInt
            | Op::LtWideInt
            | Op::LtWideUint
            | Op::LeWideInt
            | Op::LeWideUint
            | Op::GtWideInt
            | Op::GtWideUint
            | Op::GeWideInt
            | Op::GeWideUint
    )
}

/// Return whether one operation reads one wide integer argument.
fn is_wide_unary_op(op: Op) -> bool {
    matches!(op, Op::NegWideInt | Op::NotWideInt)
}
