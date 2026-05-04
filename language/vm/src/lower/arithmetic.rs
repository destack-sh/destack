use destack_mir as mir;
use destack_mir::Type;

use crate::program::{Instruction, Op, ValueLayout, value_layout_from_type};
use crate::{Error, Result};

use super::frame::word_offset;
use super::lower::BlockLowerer;
use super::op::{select_binary_op, select_integer_op, select_integer_unary_op, select_unary_op};
use super::pool::Pool;

const INTEGER_SIGN_BIT: u32 = 1 << 16;

/// Return one binary operator operand.
pub(super) fn binary_operator_operand(operator: mir::BinaryOperator) -> u32 {
    match operator {
        mir::BinaryOperator::Add => 0,
        mir::BinaryOperator::Subtract => 1,
        mir::BinaryOperator::Multiply => 2,
        mir::BinaryOperator::SignedDivide => 3,
        mir::BinaryOperator::UnsignedDivide => 4,
        mir::BinaryOperator::SignedRemainder => 5,
        mir::BinaryOperator::UnsignedRemainder => 6,
        mir::BinaryOperator::FloatAdd => 7,
        mir::BinaryOperator::FloatSubtract => 8,
        mir::BinaryOperator::FloatMultiply => 9,
        mir::BinaryOperator::FloatDivide => 10,
        mir::BinaryOperator::And => 11,
        mir::BinaryOperator::Or => 12,
        mir::BinaryOperator::Xor => 13,
        mir::BinaryOperator::ShiftLeft => 14,
        mir::BinaryOperator::ArithmeticShiftRight => 15,
        mir::BinaryOperator::LogicalShiftRight => 16,
        mir::BinaryOperator::Equal => 17,
        mir::BinaryOperator::NotEqual => 18,
        mir::BinaryOperator::SignedLessThan => 19,
        mir::BinaryOperator::SignedLessEqual => 20,
        mir::BinaryOperator::SignedGreaterThan => 21,
        mir::BinaryOperator::SignedGreaterEqual => 22,
        mir::BinaryOperator::UnsignedLessThan => 23,
        mir::BinaryOperator::UnsignedLessEqual => 24,
        mir::BinaryOperator::UnsignedGreaterThan => 25,
        mir::BinaryOperator::UnsignedGreaterEqual => 26,
        mir::BinaryOperator::FloatEqual => 27,
        mir::BinaryOperator::FloatNotEqual => 28,
        mir::BinaryOperator::FloatLessThan => 29,
        mir::BinaryOperator::FloatLessEqual => 30,
        mir::BinaryOperator::FloatGreaterThan => 31,
        mir::BinaryOperator::FloatGreaterEqual => 32,
    }
}

/// Return one unary operator operand.
fn unary_operator_operand(operator: mir::UnaryOperator) -> u32 {
    match operator {
        mir::UnaryOperator::Negate => 0,
        mir::UnaryOperator::FloatNegate => 1,
        mir::UnaryOperator::Not => 2,
    }
}

/// Pack one machine integer layout.
fn integer_layout_operand(width: u16, is_signed: bool) -> u32 {
    let sign = if is_signed { INTEGER_SIGN_BIT } else { 0 };

    u32::from(width) | sign
}

/// Return whether one op uses frame-backed integer bytes.
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

/// Return whether one op uses frame-backed integer bytes.
fn is_wide_unary_op(op: Op) -> bool {
    matches!(op, Op::NegWideInt | Op::NotWideInt)
}

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
        _pool: &mut Pool<'_>,
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
            context: "binary left operand".to_string(),
        })?;
        let right = right.value().ok_or_else(|| Error::MissingRepresentation {
            context: "binary right operand".to_string(),
        })?;
        let left_type = self.value_type_for_value(left)?;

        // keep vector and tensor operations in their elementwise executor
        if matches!(
            self.tree.get(left_type),
            Type::Vector { .. } | Type::Tensor { .. }
        ) {
            return Ok(Instruction::new(
                Op::BinaryElementwise,
                destination.id(),
                left.id(),
                right.id(),
                binary_operator_operand(operator),
            ));
        }

        // specialize machine-word integers by signedness and width
        let layout = self.value_layout_map().get(left);
        if let Some(ValueLayout::Int { width, signed }) = layout
            && let Some(op) = select_integer_op(operator, signed, width)
        {
            return Ok(Instruction::new(
                op,
                word_offset(self, destination)?,
                word_offset(self, left)?,
                word_offset(self, right)?,
                integer_layout_operand(width, signed),
            ));
        }

        // select the remaining scalar family
        let layout = layout.or_else(|| Some(value_layout_from_type(self.tree, left_type)));
        let op = select_binary_op(layout, operator);
        if is_wide_binary_op(op) {
            return Ok(Instruction::new(
                op,
                destination.id(),
                left.id(),
                right.id(),
                0,
            ));
        }
        if op == Op::BinaryElementwise {
            return Err(Error::InvalidInstruction);
        }

        self.lower_binary_word(op, destination, left, right)
    }

    /// Lower one unary instruction.
    pub(super) fn lower_unary(
        &self,
        _pool: &mut Pool<'_>,
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

        // keep vector and tensor operations in their elementwise executor
        if matches!(
            self.tree.get(argument_type),
            Type::Vector { .. } | Type::Tensor { .. }
        ) {
            return Ok(Instruction::new(
                Op::UnaryElementwise,
                destination.id(),
                argument.id(),
                unary_operator_operand(operator),
                0,
            ));
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
                integer_layout_operand(width, signed),
            ));
        }

        // select the remaining scalar family
        let op = select_unary_op(self.value_layout_map(), argument, operator);
        if is_wide_unary_op(op) {
            return Ok(Instruction::new(op, destination.id(), argument.id(), 0, 0));
        }
        if op == Op::UnaryElementwise {
            return Err(Error::InvalidInstruction);
        }

        self.lower_unary_word(op, destination, argument)
    }
}
