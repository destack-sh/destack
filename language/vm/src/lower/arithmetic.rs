use destack_mir as mir;
use destack_mir::Type;

use crate::program::{
    BinaryFloat, BinaryFloatKernel, ElementBinaryKernel, ElementUnaryKernel, Instruction, Op,
    TensorBinary, TensorContiguousBinary, TensorContiguousUnary, TensorLayout, TensorUnary,
    UnaryFloat, UnaryFloatKernel, ValueShape, VectorBinary, VectorUnary, value_shape_from_type,
};
use crate::{Error, Result};

use super::frame::{cell_offset, value_offset};
use super::lower::BlockLowerer;
use super::op::{select_binary_op, select_integer_op, select_integer_unary_op, select_unary_op};
use super::pool::Pool;
use super::projection::tensor_element_type;
use super::tensor::tensor_scalar_layout;
use super::vector::{PackedVector, vector_scalar_layout};

const INTEGER_SIGN_BIT: u32 = 1 << 16;

impl<'a> BlockLowerer<'a> {
    /// Require one value reference to be an SSA value.
    fn require_value(
        &self,
        value: mir::ValueReference,
        context: &'static str,
    ) -> Result<mir::Value> {
        value.value().ok_or_else(|| Error::invalid_program(context))
    }

    /// Lower one cell binary instruction.
    fn lower_binary_cell(
        &self,
        op: Op,
        destination: mir::Value,
        left: mir::Value,
        right: mir::Value,
    ) -> Result<Instruction> {
        let destination = cell_offset(self, destination)?;
        let left = cell_offset(self, left)?;
        let right = cell_offset(self, right)?;

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
    ) -> Result<Instruction> {
        // decode vector element layouts
        let (dest_element, element_count, dest_element_type) =
            self.vector_element_projection(destination_type)?;
        let (left_element, left_count, element) = self.vector_element_projection(left_type)?;
        let right_type = self.value_type_for_value(right)?;
        let (right_element, right_count, _) = self.vector_element_projection(right_type)?;

        // require identical vector widths
        if element_count != left_count || element_count != right_count {
            return Err(Error::invalid_instruction());
        }

        // use direct packed operations when one register covers the vector
        let element_layout =
            value_shape_from_type(self.tree, element).ok_or(Error::invalid_instruction())?;
        if let Some(op) = self
            .packed_vector(dest_element, element_count, dest_element_type)?
            .and_then(|shape| vector_packed_binary_op(operator, shape))
        {
            return Ok(Instruction::new(
                op,
                value_offset(self, destination)?,
                value_offset(self, left)?,
                value_offset(self, right)?,
                0,
            ));
        }

        // fall back to a side record for general element counts
        let kernel =
            element_binary_kernel(operator, element_layout).ok_or(Error::invalid_instruction())?;

        Ok(pool.instruction_with_side(
            Op::VectorBinary,
            VectorBinary {
                dest_offset: value_offset(self, destination)?,
                left_offset: value_offset(self, left)?,
                right_offset: value_offset(self, right)?,
                dest_element,
                left_element,
                right_element,
                kernel,
                element_layout: vector_scalar_layout(self.tree, element)?,
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
    ) -> Result<Instruction> {
        // decode tensor element and layouts
        let right_type = self.value_type_for_value(right)?;
        let element = tensor_element_type(self.tree, left_type)
            .ok_or_else(|| Error::invalid_program("tensor binary element"))?;
        let element_layout =
            value_shape_from_type(self.tree, element).ok_or(Error::invalid_instruction())?;
        let kernel =
            element_binary_kernel(operator, element_layout).ok_or(Error::invalid_instruction())?;
        let left_layout = TensorLayout::from_type(self.tree, self.layouts(), left_type)?;
        let right_layout = TensorLayout::from_type(self.tree, self.layouts(), right_type)?;
        let dest_layout = TensorLayout::from_type(self.tree, self.layouts(), destination_type)?;

        // use direct packed operations for register-sized contiguous tensors
        if same_contiguous_tensor_order(&dest_layout, &left_layout, &right_layout)
            && let Some(op) =
                tensor_packed_binary_op(operator, element_layout, dest_layout.element_span_len)
        {
            return Ok(Instruction::new(
                op,
                value_offset(self, destination)?,
                value_offset(self, left)?,
                value_offset(self, right)?,
                0,
            ));
        }

        // use one contiguous descriptor when all views share physical order
        if same_contiguous_tensor_order(&dest_layout, &left_layout, &right_layout) {
            let dest_layout = pool.tensor_layout(dest_layout);

            return Ok(pool.instruction_with_side(
                Op::TensorContiguousBinary,
                TensorContiguousBinary {
                    dest_offset: value_offset(self, destination)?,
                    left_offset: value_offset(self, left)?,
                    right_offset: value_offset(self, right)?,
                    dest_layout,
                    element_layout: tensor_scalar_layout(self.tree, element)?,
                    kernel,
                },
            ));
        }

        // preserve full view metadata for strided tensor operations
        let left_layout = pool.tensor_layout(left_layout);
        let right_layout = pool.tensor_layout(right_layout);
        let dest_layout = pool.tensor_layout(dest_layout);

        Ok(pool.instruction_with_side(
            Op::TensorBinary,
            TensorBinary {
                dest_offset: value_offset(self, destination)?,
                left_offset: value_offset(self, left)?,
                right_offset: value_offset(self, right)?,
                left_layout,
                right_layout,
                dest_layout,
                kernel,
                element_layout: tensor_scalar_layout(self.tree, element)?,
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
    ) -> Result<Instruction> {
        // specialize machine-cell integers by signedness and width
        let layout = self
            .value_shape_map()
            .get(left)
            .or_else(|| value_shape_from_type(self.tree, left_type));
        if let Some(ValueShape::Int { width, signed }) = layout
            && let Some(op) = select_integer_op(operator, signed, width)
        {
            return Ok(Instruction::new(
                op,
                cell_offset(self, destination)?,
                cell_offset(self, left)?,
                cell_offset(self, right)?,
                integer_layout_field(width, signed),
            ));
        }

        // encode 16 bit float formats without a side table
        if let Some(ValueShape::Float { format }) = layout
            && !matches!(format, mir::FloatType::Float32 | mir::FloatType::Float64)
        {
            let kernel =
                BinaryFloatKernel::from_mir(operator).ok_or(Error::invalid_instruction())?;
            let operation = BinaryFloat::new(format, kernel);

            return Ok(Instruction::new(
                Op::BinaryFloat,
                cell_offset(self, destination)?,
                cell_offset(self, left)?,
                cell_offset(self, right)?,
                operation.field(),
            ));
        }

        // select the remaining scalar family
        let op = match select_binary_op(layout, operator) {
            Some(op) => op,
            None => return Err(Error::invalid_instruction()),
        };

        // wide integers use frame byte addresses
        if is_wide_binary_op(op) {
            let Some(ValueShape::Int { width, signed }) = layout else {
                return Err(Error::invalid_instruction());
            };
            let destination = if is_wide_comparison_op(op) {
                cell_offset(self, destination)?
            } else {
                value_offset(self, destination)?
            };

            return Ok(Instruction::new(
                op,
                destination,
                value_offset(self, left)?,
                value_offset(self, right)?,
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
    ) -> Result<Instruction> {
        let destination = cell_offset(self, destination)?;
        let argument = cell_offset(self, argument)?;

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
    ) -> Result<Instruction> {
        // decode vector element layouts
        let (dest_element, element_count, element) =
            self.vector_element_projection(destination_type)?;
        let (argument_element, argument_count, _) =
            self.vector_element_projection(argument_type)?;

        // require identical vector widths
        if element_count != argument_count {
            return Err(Error::invalid_instruction());
        }

        // use direct packed operations when one register covers the vector
        let element_layout =
            value_shape_from_type(self.tree, element).ok_or(Error::invalid_instruction())?;
        if let Some(op) = self
            .packed_vector(dest_element, element_count, element)?
            .and_then(|shape| vector_packed_unary_op(operator, shape))
        {
            return Ok(Instruction::new(
                op,
                value_offset(self, destination)?,
                value_offset(self, argument)?,
                0,
                0,
            ));
        }

        // fall back to a side record for general element counts
        let kernel =
            element_unary_kernel(operator, element_layout).ok_or(Error::invalid_instruction())?;

        Ok(pool.instruction_with_side(
            Op::VectorUnary,
            VectorUnary {
                dest_offset: value_offset(self, destination)?,
                argument_offset: value_offset(self, argument)?,
                dest_element,
                argument_element,
                kernel,
                element_layout: vector_scalar_layout(self.tree, element)?,
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
    ) -> Result<Instruction> {
        // decode tensor element and layouts
        let element = tensor_element_type(self.tree, argument_type)
            .ok_or_else(|| Error::invalid_program("tensor unary element"))?;
        let element_layout =
            value_shape_from_type(self.tree, element).ok_or(Error::invalid_instruction())?;
        let kernel =
            element_unary_kernel(operator, element_layout).ok_or(Error::invalid_instruction())?;
        let argument_layout = TensorLayout::from_type(self.tree, self.layouts(), argument_type)?;
        let dest_layout = TensorLayout::from_type(self.tree, self.layouts(), destination_type)?;

        // use direct packed operations for register-sized contiguous tensors
        if same_contiguous_tensor_unary_order(&dest_layout, &argument_layout)
            && let Some(op) =
                tensor_packed_unary_op(operator, element_layout, dest_layout.element_span_len)
        {
            return Ok(Instruction::new(
                op,
                value_offset(self, destination)?,
                value_offset(self, argument)?,
                0,
                0,
            ));
        }

        // use one contiguous descriptor when both views share physical order
        if same_contiguous_tensor_unary_order(&dest_layout, &argument_layout) {
            let dest_layout = pool.tensor_layout(dest_layout);

            return Ok(pool.instruction_with_side(
                Op::TensorContiguousUnary,
                TensorContiguousUnary {
                    dest_offset: value_offset(self, destination)?,
                    argument_offset: value_offset(self, argument)?,
                    dest_layout,
                    element_layout: tensor_scalar_layout(self.tree, element)?,
                    kernel,
                },
            ));
        }

        // preserve full view metadata for strided tensor operations
        let argument_layout = pool.tensor_layout(argument_layout);
        let dest_layout = pool.tensor_layout(dest_layout);

        Ok(pool.instruction_with_side(
            Op::TensorUnary,
            TensorUnary {
                dest_offset: value_offset(self, destination)?,
                argument_offset: value_offset(self, argument)?,
                argument_layout,
                element_layout: tensor_scalar_layout(self.tree, element)?,
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
    ) -> Result<Instruction> {
        // specialize machine-cell integers by signedness and width
        let layout = self
            .value_shape_map()
            .get(argument)
            .or_else(|| value_shape_from_type(self.tree, argument_type));
        if let Some(ValueShape::Int { width, signed }) = layout
            && let Some(op) = select_integer_unary_op(operator, signed, width)
        {
            return Ok(Instruction::new(
                op,
                cell_offset(self, destination)?,
                cell_offset(self, argument)?,
                0,
                integer_layout_field(width, signed),
            ));
        }

        // encode 16 bit float formats without a side table
        if let Some(ValueShape::Float { format }) = layout
            && !matches!(format, mir::FloatType::Float32 | mir::FloatType::Float64)
        {
            let kernel = UnaryFloatKernel::from_mir(operator).ok_or(Error::invalid_instruction())?;
            let operation = UnaryFloat::new(format, kernel);

            return Ok(Instruction::new(
                Op::UnaryFloat,
                cell_offset(self, destination)?,
                cell_offset(self, argument)?,
                0,
                operation.field(),
            ));
        }

        // select the remaining scalar family
        let op = match select_unary_op(self.value_shape_map(), argument, operator) {
            Some(op) => op,
            None => return Err(Error::invalid_instruction()),
        };

        // wide integers use frame byte addresses
        if is_wide_unary_op(op) {
            let Some(ValueShape::Int { width, signed }) =
                value_shape_from_type(self.tree, argument_type)
            else {
                return Err(Error::invalid_instruction());
            };

            return Ok(Instruction::new(
                op,
                value_offset(self, destination)?,
                value_offset(self, argument)?,
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
        destination: mir::ValueReference,
        operator: mir::BinaryOperator,
        left: mir::ValueReference,
        right: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = self.require_value(destination, "binary destination")?;
        let left = self.require_value(left, "binary left value")?;
        let right = self.require_value(right, "binary right value")?;
        let destination_type = self.value_type_for_value(destination)?;
        let left_type = self.value_type_for_value(left)?;

        // select aggregate arithmetic by static value family
        match self.tree.get(destination_type) {
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
        destination: mir::ValueReference,
        operator: mir::UnaryOperator,
        argument: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = self.require_value(destination, "unary destination")?;
        let argument = self.require_value(argument, "unary argument")?;
        let argument_type = self.value_type_for_value(argument)?;
        let destination_type = self.value_type_for_value(destination)?;

        // select aggregate arithmetic by static value family
        match self.tree.get(destination_type) {
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
    layout: ValueShape,
) -> Option<ElementBinaryKernel> {
    use ElementBinaryKernel as Kernel;
    use mir::BinaryOperator::*;

    Some(match layout {
        ValueShape::Bool => match operator {
            And => Kernel::AndBool,
            Or => Kernel::OrBool,
            Xor => Kernel::XorBool,
            Equal => Kernel::EqBool,
            NotEqual => Kernel::NeBool,
            _ => return None,
        },
        ValueShape::Int { signed, .. } => match (operator, signed) {
            (Add, _) => Kernel::AddInt,
            (Subtract, _) => Kernel::SubInt,
            (Multiply, _) => Kernel::MulInt,
            (SignedDivide, true) => Kernel::DivInt,
            (UnsignedDivide, _) => Kernel::DivUint,
            (SignedRemainder, true) => Kernel::RemInt,
            (UnsignedRemainder, _) => Kernel::RemUint,
            (And, _) => Kernel::AndInt,
            (Or, _) => Kernel::OrInt,
            (Xor, _) => Kernel::XorInt,
            (ShiftLeft, _) => Kernel::ShlInt,
            (ArithmeticShiftRight, true) => Kernel::ShrInt,
            (LogicalShiftRight, _) => Kernel::ShrUint,
            (Equal, _) => Kernel::EqInt,
            (NotEqual, _) => Kernel::NeInt,
            (SignedLessThan, true) => Kernel::LtInt,
            (SignedLessEqual, true) => Kernel::LeInt,
            (SignedGreaterThan, true) => Kernel::GtInt,
            (SignedGreaterEqual, true) => Kernel::GeInt,
            (UnsignedLessThan, _) => Kernel::LtUint,
            (UnsignedLessEqual, _) => Kernel::LeUint,
            (UnsignedGreaterThan, _) => Kernel::GtUint,
            (UnsignedGreaterEqual, _) => Kernel::GeUint,
            _ => return None,
        },
        ValueShape::Float {
            format: mir::FloatType::Float32,
        } => match operator {
            FloatAdd => Kernel::AddF32,
            FloatSubtract => Kernel::SubF32,
            FloatMultiply => Kernel::MulF32,
            FloatDivide => Kernel::DivF32,
            FloatEqual => Kernel::EqF32,
            FloatNotEqual => Kernel::NeF32,
            FloatLessThan => Kernel::LtF32,
            FloatLessEqual => Kernel::LeF32,
            FloatGreaterThan => Kernel::GtF32,
            FloatGreaterEqual => Kernel::GeF32,
            _ => return None,
        },
        ValueShape::Float {
            format: mir::FloatType::Float64,
        } => match operator {
            FloatAdd => Kernel::AddF64,
            FloatSubtract => Kernel::SubF64,
            FloatMultiply => Kernel::MulF64,
            FloatDivide => Kernel::DivF64,
            FloatEqual => Kernel::EqF64,
            FloatNotEqual => Kernel::NeF64,
            FloatLessThan => Kernel::LtF64,
            FloatLessEqual => Kernel::LeF64,
            FloatGreaterThan => Kernel::GtF64,
            FloatGreaterEqual => Kernel::GeF64,
            _ => return None,
        },
        ValueShape::Float { .. } => match operator {
            FloatAdd => Kernel::AddFloat,
            FloatSubtract => Kernel::SubFloat,
            FloatMultiply => Kernel::MulFloat,
            FloatDivide => Kernel::DivFloat,
            FloatEqual => Kernel::EqFloat,
            FloatNotEqual => Kernel::NeFloat,
            FloatLessThan => Kernel::LtFloat,
            FloatLessEqual => Kernel::LeFloat,
            FloatGreaterThan => Kernel::GtFloat,
            FloatGreaterEqual => Kernel::GeFloat,
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

/// Select a direct packed tensor binary opcode.
fn tensor_packed_binary_op(
    operator: mir::BinaryOperator,
    layout: ValueShape,
    element_count: usize,
) -> Option<Op> {
    let shape = packed_tensor_shape(layout, element_count)?;

    vector_packed_binary_op(operator, shape)
}

/// Select a direct packed tensor unary opcode.
fn tensor_packed_unary_op(
    operator: mir::UnaryOperator,
    layout: ValueShape,
    element_count: usize,
) -> Option<Op> {
    let shape = packed_tensor_shape(layout, element_count)?;

    vector_packed_unary_op(operator, shape)
}

/// Return the packed vector shape for one register-sized contiguous tensor.
fn packed_tensor_shape(layout: ValueShape, element_count: usize) -> Option<PackedVector> {
    Some(match (layout, element_count) {
        (
            ValueShape::Int {
                width: 32,
                signed: true,
            },
            4,
        ) => PackedVector::I32x4,
        (
            ValueShape::Int {
                width: 32,
                signed: false,
            },
            4,
        ) => PackedVector::U32x4,
        (
            ValueShape::Int {
                width: 64,
                signed: true,
            },
            2,
        ) => PackedVector::I64x2,
        (
            ValueShape::Int {
                width: 64,
                signed: false,
            },
            2,
        ) => PackedVector::U64x2,
        (
            ValueShape::Float {
                format: mir::FloatType::Float32,
            },
            4,
        ) => PackedVector::F32x4,
        (
            ValueShape::Float {
                format: mir::FloatType::Float64,
            },
            2,
        ) => PackedVector::F64x2,
        _ => return None,
    })
}

/// Select one element unary kernel.
fn element_unary_kernel(
    operator: mir::UnaryOperator,
    layout: ValueShape,
) -> Option<ElementUnaryKernel> {
    use ElementUnaryKernel as Kernel;

    Some(match (operator, layout) {
        (mir::UnaryOperator::Negate, ValueShape::Int { signed: true, .. }) => Kernel::NegInt,
        (mir::UnaryOperator::Not, ValueShape::Int { .. }) => Kernel::NotInt,
        (
            mir::UnaryOperator::FloatNegate,
            ValueShape::Float {
                format: mir::FloatType::Float32,
            },
        ) => Kernel::NegF32,
        (
            mir::UnaryOperator::FloatNegate,
            ValueShape::Float {
                format: mir::FloatType::Float64,
            },
        ) => Kernel::NegF64,
        (mir::UnaryOperator::FloatNegate, ValueShape::Float { .. }) => Kernel::NegFloat,
        (mir::UnaryOperator::Not, ValueShape::Bool) => Kernel::NotBool,
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
