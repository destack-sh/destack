use destack_mir as mir;
use destack_mir::Type;

use crate::program::{
    Instruction, Op, TensorBinary, TensorLayout, TensorUnary, ValueLayout, VectorBinary,
    VectorUnary, value_layout_from_type,
};
use crate::{Error, Result};

use super::access::tensor_element_type;
use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::op::{select_binary_op, select_integer_op, select_integer_unary_op, select_unary_op};
use super::pool::Pool;
use super::tensor::tensor_scalar_layout;
use super::vector::vector_scalar_layout;

const INTEGER_SIGN_BIT: u32 = 1 << 16;
impl<'a> BlockLowerer<'a> {
    /// Lower one word binary instruction.
    fn lower_binary_word(
        &self,
        op: Op,
        destination: mir::Value,
        left: mir::Value,
        right: mir::Value,
    ) -> Result<Instruction> {
        let destination = word_offset(self, destination)?;
        let left = word_offset(self, left)?;
        let right = word_offset(self, right)?;

        Ok(Instruction::new(op, destination, left, right, 0))
    }

    /// Lower one word unary instruction.
    fn lower_unary_word(
        &self,
        op: Op,
        destination: mir::Value,
        argument: mir::Value,
    ) -> Result<Instruction> {
        let destination = word_offset(self, destination)?;
        let argument = word_offset(self, argument)?;

        Ok(Instruction::new(op, destination, argument, 0, 0))
    }

    /// Lower one binary instruction.
    pub(super) fn lower_binary(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        operator: mir::BinaryOperator,
        left: mir::ValueReference,
        right: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "binary destination".to_string(),
            })?;
        let left = left.value().ok_or_else(|| Error::MissingRepresentation {
            context: "binary left value".to_string(),
        })?;
        let right = right.value().ok_or_else(|| Error::MissingRepresentation {
            context: "binary right value".to_string(),
        })?;
        let destination_type = self.value_type_for_value(destination)?;
        let left_type = self.value_type_for_value(left)?;

        // select aggregate arithmetic by static value family
        match self.tree.get(destination_type) {
            Type::Vector { .. } => {
                let (dest_element, element_count, _) =
                    self.vector_element_access(destination_type)?;
                let (left_element, left_count, element) = self.vector_element_access(left_type)?;
                let right_type = self.value_type_for_value(right)?;
                let (right_element, right_count, _) = self.vector_element_access(right_type)?;
                if element_count != left_count || element_count != right_count {
                    return Err(Error::InvalidInstruction);
                }
                let element_layout = value_layout_from_type(self.tree, element);
                let op =
                    vector_binary_op(operator, element_layout).ok_or(Error::InvalidInstruction)?;

                return Ok(pool.instruction_with_side_record(
                    op,
                    VectorBinary {
                        dest_offset: value_offset(self, destination)?,
                        left_offset: value_offset(self, left)?,
                        right_offset: value_offset(self, right)?,
                        dest_element,
                        left_element,
                        right_element,
                        element_layout: vector_scalar_layout(self.tree, element)?,
                        element_count,
                    },
                ));
            }
            Type::Tensor { .. } => {
                let right_type = self.value_type_for_value(right)?;
                let element = tensor_element_type(self.tree, left_type).ok_or_else(|| {
                    Error::MissingRepresentation {
                        context: "tensor binary element".to_string(),
                    }
                })?;
                let element_layout = value_layout_from_type(self.tree, element);
                let op =
                    tensor_binary_op(operator, element_layout).ok_or(Error::InvalidInstruction)?;
                let left_layout = pool.tensor_layout(TensorLayout::from_type(
                    self.tree,
                    self.layouts(),
                    left_type,
                )?);
                let right_layout = pool.tensor_layout(TensorLayout::from_type(
                    self.tree,
                    self.layouts(),
                    right_type,
                )?);
                let dest_layout = pool.tensor_layout(TensorLayout::from_type(
                    self.tree,
                    self.layouts(),
                    destination_type,
                )?);

                return Ok(pool.instruction_with_side_record(
                    op,
                    TensorBinary {
                        dest_offset: value_offset(self, destination)?,
                        left_offset: value_offset(self, left)?,
                        right_offset: value_offset(self, right)?,
                        left_layout,
                        right_layout,
                        dest_layout,
                        element_layout: tensor_scalar_layout(self.tree, element)?,
                    },
                ));
            }
            _ => {}
        }

        // specialize machine-word integers by signedness and width
        let layout = self
            .value_layout_map()
            .get(left)
            .or_else(|| Some(value_layout_from_type(self.tree, left_type)));
        if let Some(ValueLayout::Int { width, signed }) = layout
            && let Some(op) = select_integer_op(operator, signed, width)
        {
            return Ok(Instruction::new(
                op,
                word_offset(self, destination)?,
                word_offset(self, left)?,
                word_offset(self, right)?,
                integer_layout_field(width, signed),
            ));
        }

        // select the remaining scalar family
        let op = match select_binary_op(layout, operator) {
            Some(op) => op,
            None => return Err(Error::InvalidInstruction),
        };
        if is_wide_binary_op(op) {
            let Some(ValueLayout::Int { width, signed }) = layout else {
                return Err(Error::InvalidInstruction);
            };

            return Ok(Instruction::new(
                op,
                if is_wide_comparison_op(op) {
                    word_offset(self, destination)?
                } else {
                    value_offset(self, destination)?
                },
                value_offset(self, left)?,
                value_offset(self, right)?,
                integer_layout_field(width, signed),
            ));
        }
        self.lower_binary_word(op, destination, left, right)
    }

    /// Lower one unary instruction.
    pub(super) fn lower_unary(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        operator: mir::UnaryOperator,
        argument: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "unary destination".to_string(),
            })?;
        let argument = argument
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "unary argument".to_string(),
            })?;
        let argument_type = self.value_type_for_value(argument)?;
        let destination_type = self.value_type_for_value(destination)?;

        // select aggregate arithmetic by static value family
        match self.tree.get(destination_type) {
            Type::Vector { .. } => {
                let (dest_element, element_count, element) =
                    self.vector_element_access(destination_type)?;
                let (argument_element, argument_count, _) =
                    self.vector_element_access(argument_type)?;
                if element_count != argument_count {
                    return Err(Error::InvalidInstruction);
                }
                let element_layout = value_layout_from_type(self.tree, element);
                let op =
                    vector_unary_op(operator, element_layout).ok_or(Error::InvalidInstruction)?;

                return Ok(pool.instruction_with_side_record(
                    op,
                    VectorUnary {
                        dest_offset: value_offset(self, destination)?,
                        argument_offset: value_offset(self, argument)?,
                        dest_element,
                        argument_element,
                        element_layout: vector_scalar_layout(self.tree, element)?,
                        element_count,
                    },
                ));
            }
            Type::Tensor { .. } => {
                let element = tensor_element_type(self.tree, argument_type).ok_or_else(|| {
                    Error::MissingRepresentation {
                        context: "tensor unary element".to_string(),
                    }
                })?;
                let element_layout = value_layout_from_type(self.tree, element);
                let op =
                    tensor_unary_op(operator, element_layout).ok_or(Error::InvalidInstruction)?;
                let argument_layout = pool.tensor_layout(TensorLayout::from_type(
                    self.tree,
                    self.layouts(),
                    argument_type,
                )?);
                let dest_layout = pool.tensor_layout(TensorLayout::from_type(
                    self.tree,
                    self.layouts(),
                    destination_type,
                )?);

                return Ok(pool.instruction_with_side_record(
                    op,
                    TensorUnary {
                        dest_offset: value_offset(self, destination)?,
                        argument_offset: value_offset(self, argument)?,
                        argument_layout,
                        element_layout: tensor_scalar_layout(self.tree, element)?,
                        dest_layout,
                    },
                ));
            }
            _ => {}
        }

        // specialize machine-word integers by signedness and width
        let layout = self.value_layout_map().get(argument);
        if let Some(ValueLayout::Int { width, signed }) = layout
            && let Some(op) = select_integer_unary_op(operator, signed, width)
        {
            return Ok(Instruction::new(
                op,
                word_offset(self, destination)?,
                word_offset(self, argument)?,
                0,
                integer_layout_field(width, signed),
            ));
        }

        // select the remaining scalar family
        let op = match select_unary_op(self.value_layout_map(), argument, operator) {
            Some(op) => op,
            None => return Err(Error::InvalidInstruction),
        };
        if is_wide_unary_op(op) {
            let ValueLayout::Int { width, signed } =
                value_layout_from_type(self.tree, argument_type)
            else {
                return Err(Error::InvalidInstruction);
            };

            return Ok(Instruction::new(
                op,
                value_offset(self, destination)?,
                value_offset(self, argument)?,
                0,
                integer_layout_field(width, signed),
            ));
        }

        self.lower_unary_word(op, destination, argument)
    }
}

/// Pack one machine integer layout.
pub(super) fn vector_binary_op(operator: mir::BinaryOperator, layout: ValueLayout) -> Option<Op> {
    use mir::BinaryOperator::*;

    Some(match layout {
        ValueLayout::Bool => match operator {
            And => Op::VectorAndBool,
            Or => Op::VectorOrBool,
            Xor => Op::VectorXorBool,
            Equal => Op::VectorEqBool,
            NotEqual => Op::VectorNeBool,
            _ => return None,
        },
        ValueLayout::Int { signed, .. } => match (operator, signed) {
            (Add, _) => Op::VectorAddInt,
            (Subtract, _) => Op::VectorSubInt,
            (Multiply, _) => Op::VectorMulInt,
            (SignedDivide, true) => Op::VectorDivInt,
            (UnsignedDivide, _) => Op::VectorDivUint,
            (SignedRemainder, true) => Op::VectorRemInt,
            (UnsignedRemainder, _) => Op::VectorRemUint,
            (And, _) => Op::VectorAndInt,
            (Or, _) => Op::VectorOrInt,
            (Xor, _) => Op::VectorXorInt,
            (ShiftLeft, _) => Op::VectorShlInt,
            (ArithmeticShiftRight, true) => Op::VectorShrInt,
            (LogicalShiftRight, _) => Op::VectorShrUint,
            (Equal, _) => Op::VectorEqInt,
            (NotEqual, _) => Op::VectorNeInt,
            (SignedLessThan, true) => Op::VectorLtInt,
            (SignedLessEqual, true) => Op::VectorLeInt,
            (SignedGreaterThan, true) => Op::VectorGtInt,
            (SignedGreaterEqual, true) => Op::VectorGeInt,
            (UnsignedLessThan, _) => Op::VectorLtUint,
            (UnsignedLessEqual, _) => Op::VectorLeUint,
            (UnsignedGreaterThan, _) => Op::VectorGtUint,
            (UnsignedGreaterEqual, _) => Op::VectorGeUint,
            _ => return None,
        },
        ValueLayout::Float { width: 32 } => match operator {
            FloatAdd => Op::VectorAddF32,
            FloatSubtract => Op::VectorSubF32,
            FloatMultiply => Op::VectorMulF32,
            FloatDivide => Op::VectorDivF32,
            FloatEqual => Op::VectorEqF32,
            FloatNotEqual => Op::VectorNeF32,
            FloatLessThan => Op::VectorLtF32,
            FloatLessEqual => Op::VectorLeF32,
            FloatGreaterThan => Op::VectorGtF32,
            FloatGreaterEqual => Op::VectorGeF32,
            _ => return None,
        },
        ValueLayout::Float { width: 64 } => match operator {
            FloatAdd => Op::VectorAddF64,
            FloatSubtract => Op::VectorSubF64,
            FloatMultiply => Op::VectorMulF64,
            FloatDivide => Op::VectorDivF64,
            FloatEqual => Op::VectorEqF64,
            FloatNotEqual => Op::VectorNeF64,
            FloatLessThan => Op::VectorLtF64,
            FloatLessEqual => Op::VectorLeF64,
            FloatGreaterThan => Op::VectorGtF64,
            FloatGreaterEqual => Op::VectorGeF64,
            _ => return None,
        },
        _ => return None,
    })
}

/// Return whether one op uses frame-backed integer bytes.
pub(super) fn tensor_binary_op(operator: mir::BinaryOperator, layout: ValueLayout) -> Option<Op> {
    use mir::BinaryOperator::*;

    Some(match layout {
        ValueLayout::Bool => match operator {
            And => Op::TensorAndBool,
            Or => Op::TensorOrBool,
            Xor => Op::TensorXorBool,
            Equal => Op::TensorEqBool,
            NotEqual => Op::TensorNeBool,
            _ => return None,
        },
        ValueLayout::Int { signed, .. } => match (operator, signed) {
            (Add, _) => Op::TensorAddInt,
            (Subtract, _) => Op::TensorSubInt,
            (Multiply, _) => Op::TensorMulInt,
            (SignedDivide, true) => Op::TensorDivInt,
            (UnsignedDivide, _) => Op::TensorDivUint,
            (SignedRemainder, true) => Op::TensorRemInt,
            (UnsignedRemainder, _) => Op::TensorRemUint,
            (And, _) => Op::TensorAndInt,
            (Or, _) => Op::TensorOrInt,
            (Xor, _) => Op::TensorXorInt,
            (ShiftLeft, _) => Op::TensorShlInt,
            (ArithmeticShiftRight, true) => Op::TensorShrInt,
            (LogicalShiftRight, _) => Op::TensorShrUint,
            (Equal, _) => Op::TensorEqInt,
            (NotEqual, _) => Op::TensorNeInt,
            (SignedLessThan, true) => Op::TensorLtInt,
            (SignedLessEqual, true) => Op::TensorLeInt,
            (SignedGreaterThan, true) => Op::TensorGtInt,
            (SignedGreaterEqual, true) => Op::TensorGeInt,
            (UnsignedLessThan, _) => Op::TensorLtUint,
            (UnsignedLessEqual, _) => Op::TensorLeUint,
            (UnsignedGreaterThan, _) => Op::TensorGtUint,
            (UnsignedGreaterEqual, _) => Op::TensorGeUint,
            _ => return None,
        },
        ValueLayout::Float { width: 32 } => match operator {
            FloatAdd => Op::TensorAddF32,
            FloatSubtract => Op::TensorSubF32,
            FloatMultiply => Op::TensorMulF32,
            FloatDivide => Op::TensorDivF32,
            FloatEqual => Op::TensorEqF32,
            FloatNotEqual => Op::TensorNeF32,
            FloatLessThan => Op::TensorLtF32,
            FloatLessEqual => Op::TensorLeF32,
            FloatGreaterThan => Op::TensorGtF32,
            FloatGreaterEqual => Op::TensorGeF32,
            _ => return None,
        },
        ValueLayout::Float { width: 64 } => match operator {
            FloatAdd => Op::TensorAddF64,
            FloatSubtract => Op::TensorSubF64,
            FloatMultiply => Op::TensorMulF64,
            FloatDivide => Op::TensorDivF64,
            FloatEqual => Op::TensorEqF64,
            FloatNotEqual => Op::TensorNeF64,
            FloatLessThan => Op::TensorLtF64,
            FloatLessEqual => Op::TensorLeF64,
            FloatGreaterThan => Op::TensorGtF64,
            FloatGreaterEqual => Op::TensorGeF64,
            _ => return None,
        },
        _ => return None,
    })
}

/// Return whether one wide op writes one word result.
fn vector_unary_op(operator: mir::UnaryOperator, layout: ValueLayout) -> Option<Op> {
    Some(match (operator, layout) {
        (mir::UnaryOperator::Negate, ValueLayout::Int { signed: true, .. }) => Op::VectorNegInt,
        (mir::UnaryOperator::Not, ValueLayout::Int { .. }) => Op::VectorNotInt,
        (mir::UnaryOperator::FloatNegate, ValueLayout::Float { width: 32 }) => Op::VectorNegF32,
        (mir::UnaryOperator::FloatNegate, ValueLayout::Float { width: 64 }) => Op::VectorNegF64,
        (mir::UnaryOperator::Not, ValueLayout::Bool) => Op::VectorNotBool,
        _ => return None,
    })
}

/// Return whether one op uses frame-backed integer bytes.
fn tensor_unary_op(operator: mir::UnaryOperator, layout: ValueLayout) -> Option<Op> {
    Some(match (operator, layout) {
        (mir::UnaryOperator::Negate, ValueLayout::Int { signed: true, .. }) => Op::TensorNegInt,
        (mir::UnaryOperator::Not, ValueLayout::Int { .. }) => Op::TensorNotInt,
        (mir::UnaryOperator::FloatNegate, ValueLayout::Float { width: 32 }) => Op::TensorNegF32,
        (mir::UnaryOperator::FloatNegate, ValueLayout::Float { width: 64 }) => Op::TensorNegF64,
        (mir::UnaryOperator::Not, ValueLayout::Bool) => Op::TensorNotBool,
        _ => return None,
    })
}

/// Select a vector binary opcode from one element layout.
fn integer_layout_field(width: u16, is_signed: bool) -> u32 {
    let sign = if is_signed { INTEGER_SIGN_BIT } else { 0 };

    u32::from(width) | sign
}

/// Select a tensor binary opcode from one element layout.
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

/// Select a vector unary opcode from one element layout.
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

/// Select a tensor unary opcode from one element layout.
fn is_wide_unary_op(op: Op) -> bool {
    matches!(op, Op::NegWideInt | Op::NotWideInt)
}
