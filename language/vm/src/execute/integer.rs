use std::cmp::Ordering;

use tspp_bytecode::{Instruction, IntegerOperation, RegisterSpan, Scalar};
use tspp_program::{Runtime, Word};

use crate::diagnostic::{Error, Result, Trap};
use crate::machine::Activation;

use super::arithmetic::Arithmetic;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one single-word integer operation.
    #[inline(always)]
    pub(crate) fn execute_integer(
        &mut self,
        instruction: Instruction<'_>,
        operation: IntegerOperation,
        scalar: Scalar,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let overflow_target = if operation.is_overflowing() {
            Some(operands.register()?)
        } else {
            None
        };
        let left = operands.register()?;
        let left = self.read(left.0);
        let right = if operation.input_count() >= 2 {
            let right = operands.register()?;

            Some(self.read(right.0))
        } else {
            None
        };
        let third = if operation.input_count() == 3 {
            let third = operands.register()?;

            Some(self.read(third.0))
        } else {
            None
        };
        let (value, overflow) = Arithmetic::integer_result(operation, scalar, left, right, third)?;

        self.write(target.0, value);
        if let Some(target) = overflow_target {
            self.write(target.0, Word::boolean(overflow));
        }

        Ok(())
    }
}

impl Arithmetic {
    /// Execute one integer operation over unpacked scalar words.
    pub(super) fn integer(
        operation: IntegerOperation,
        scalar: Scalar,
        left: Word,
        right: Option<Word>,
        third: Option<Word>,
    ) -> Result<Word> {
        let (value, _) = Self::integer_result(operation, scalar, left, right, third)?;

        Ok(value)
    }

    /// Execute one integer operation and preserve its overflow result.
    fn integer_result(
        operation: IntegerOperation,
        scalar: Scalar,
        left: Word,
        right: Option<Word>,
        third: Option<Word>,
    ) -> Result<(Word, bool)> {
        let left = left.bits();
        let result = match operation {
            IntegerOperation::Not => (!left, false),
            IntegerOperation::Negate => (0_u64.wrapping_sub(left), false),
            IntegerOperation::LeadingZeroCount => (Self::leading_zeros(left, scalar), false),
            IntegerOperation::TrailingZeroCount => (Self::trailing_zeros(left, scalar), false),
            IntegerOperation::PopulationCount => (
                Self::truncate(left, scalar.bit_width()).count_ones() as u64,
                false,
            ),
            IntegerOperation::ByteSwap => (Self::byte_swap(left, scalar), false),
            IntegerOperation::BitReverse => (Self::bit_reverse(left, scalar), false),
            IntegerOperation::IsolateLowestOne => (left & 0_u64.wrapping_sub(left), false),
            _ => {
                let right = right.ok_or_else(Error::invalid_instruction)?;
                Self::integer_operation(
                    operation,
                    scalar,
                    left,
                    right.bits(),
                    third.map(Word::bits),
                )?
            }
        };
        let result_scalar = operation
            .result_scalar(scalar)
            .ok_or_else(Error::invalid_instruction)?;
        let value = Word::from_bits(result_scalar.encode(result.0));

        Ok((value, result.1))
    }

    /// Execute one scalar integer operation.
    #[inline(always)]
    fn integer_operation(
        operation: IntegerOperation,
        scalar: Scalar,
        left: u64,
        right: u64,
        third: Option<u64>,
    ) -> Result<(u64, bool)> {
        let result = match operation {
            IntegerOperation::Add => (left.wrapping_add(right), false),
            IntegerOperation::Subtract => (left.wrapping_sub(right), false),
            IntegerOperation::Multiply => (left.wrapping_mul(right), false),
            IntegerOperation::Divide if right == 0 => {
                return Err(Error::trap(Trap::DivisionByZero));
            }
            IntegerOperation::Divide if Self::division_overflows(scalar, left, right) => {
                return Err(Error::trap(Trap::IntegerOverflow));
            }
            IntegerOperation::Divide => (Self::divide(scalar, left, right), false),
            IntegerOperation::Remainder if right == 0 => {
                return Err(Error::trap(Trap::DivisionByZero));
            }
            IntegerOperation::Remainder if Self::division_overflows(scalar, left, right) => {
                return Err(Error::trap(Trap::IntegerOverflow));
            }
            IntegerOperation::Remainder => (Self::remainder(scalar, left, right), false),
            IntegerOperation::And => (left & right, false),
            IntegerOperation::Or => (left | right, false),
            IntegerOperation::Xor => (left ^ right, false),
            IntegerOperation::ShiftLeft => (left.wrapping_shl(right as u32), false),
            IntegerOperation::ShiftRight => (Self::shift_right(scalar, left, right as u32), false),
            IntegerOperation::RotateLeft => (Self::rotate_left(left, right as u32, scalar), false),
            IntegerOperation::RotateRight => {
                (Self::rotate_right(left, right as u32, scalar), false)
            }
            IntegerOperation::Equal => ((left == right) as u64, false),
            IntegerOperation::NotEqual => ((left != right) as u64, false),
            IntegerOperation::LessThan => {
                (Self::compare(scalar, left, right).is_lt() as u64, false)
            }
            IntegerOperation::LessEqual => {
                (Self::compare(scalar, left, right).is_le() as u64, false)
            }
            IntegerOperation::GreaterThan => {
                (Self::compare(scalar, left, right).is_gt() as u64, false)
            }
            IntegerOperation::GreaterEqual => {
                (Self::compare(scalar, left, right).is_ge() as u64, false)
            }
            IntegerOperation::AddOverflow
            | IntegerOperation::SubtractOverflow
            | IntegerOperation::MultiplyOverflow => {
                Self::overflowing(operation, scalar, left, right)
            }
            IntegerOperation::AddSaturating | IntegerOperation::SubtractSaturating => {
                (Self::saturating(operation, scalar, left, right), false)
            }
            IntegerOperation::Midpoint => (Self::midpoint(scalar, left, right), false),
            IntegerOperation::Clamp => {
                let maximum = third.ok_or_else(Error::invalid_instruction)?;
                (Self::clamp(scalar, left, right, maximum)?, false)
            }
            IntegerOperation::DivideCeil => (Self::divide_ceil(scalar, left, right)?, false),
            IntegerOperation::RemainderEuclidean => {
                (Self::remainder_euclidean(scalar, left, right)?, false)
            }
            IntegerOperation::IsMultipleOf => {
                (Self::is_multiple_of(scalar, left, right) as u64, false)
            }
            IntegerOperation::AbsDiff => (Self::abs_diff(scalar, left, right), false),
            _ => unreachable!("unary integer operations are handled by the caller"),
        };

        Ok(result)
    }

    /// Compute one exact-width overflowing arithmetic result.
    fn overflowing(
        operation: IntegerOperation,
        scalar: Scalar,
        left: u64,
        right: u64,
    ) -> (u64, bool) {
        let width = scalar.bit_width();
        if scalar.is_signed_integer() {
            let left = Self::signed_integer(scalar, left);
            let right = Self::signed_integer(scalar, right);
            let value = match operation {
                IntegerOperation::AddOverflow => left + right,
                IntegerOperation::SubtractOverflow => left - right,
                IntegerOperation::MultiplyOverflow => left * right,
                _ => unreachable!("overflowing arithmetic uses one overflowing operation"),
            };
            let minimum = -(1_i128 << (width - 1));
            let maximum = (1_i128 << (width - 1)) - 1;
            let overflow = value < minimum || value > maximum;

            (Self::truncate(value as u64, width), overflow)
        } else {
            let left = Self::truncate(left, width) as u128;
            let right = Self::truncate(right, width) as u128;
            let value = match operation {
                IntegerOperation::AddOverflow => left + right,
                IntegerOperation::SubtractOverflow => left.wrapping_sub(right),
                IntegerOperation::MultiplyOverflow => left * right,
                _ => unreachable!("overflowing arithmetic uses one overflowing operation"),
            };
            let maximum = (1_u128 << width) - 1;
            let overflow = match operation {
                IntegerOperation::SubtractOverflow => left < right,
                _ => value > maximum,
            };

            (Self::truncate(value as u64, width), overflow)
        }
    }

    /// Compute one exact-width saturating arithmetic result.
    fn saturating(operation: IntegerOperation, scalar: Scalar, left: u64, right: u64) -> u64 {
        let width = scalar.bit_width();
        if scalar.is_signed_integer() {
            let left = Self::signed_integer(scalar, left);
            let right = Self::signed_integer(scalar, right);
            let value = match operation {
                IntegerOperation::AddSaturating => left + right,
                IntegerOperation::SubtractSaturating => left - right,
                _ => unreachable!("saturating arithmetic uses one saturating operation"),
            };
            let minimum = -(1_i128 << (width - 1));
            let maximum = (1_i128 << (width - 1)) - 1;

            Self::truncate(value.clamp(minimum, maximum) as u64, width)
        } else {
            let left = Self::truncate(left, width) as u128;
            let right = Self::truncate(right, width) as u128;
            let maximum = (1_u128 << width) - 1;
            let value = match operation {
                IntegerOperation::AddSaturating => (left + right).min(maximum),
                IntegerOperation::SubtractSaturating => left.saturating_sub(right),
                _ => unreachable!("saturating arithmetic uses one saturating operation"),
            };

            value as u64
        }
    }

    /// Compare two scalar integers with the representation's signedness.
    fn compare(scalar: Scalar, left: u64, right: u64) -> Ordering {
        if scalar.is_signed_integer() {
            Self::signed_integer(scalar, left).cmp(&Self::signed_integer(scalar, right))
        } else {
            left.cmp(&right)
        }
    }

    /// Divide two scalar integers with the representation's signedness.
    fn divide(scalar: Scalar, left: u64, right: u64) -> u64 {
        if scalar.is_signed_integer() {
            Self::signed_integer(scalar, left).wrapping_div(Self::signed_integer(scalar, right))
                as u64
        } else {
            left / right
        }
    }

    /// Compute one scalar integer remainder with the representation's signedness.
    fn remainder(scalar: Scalar, left: u64, right: u64) -> u64 {
        if scalar.is_signed_integer() {
            Self::signed_integer(scalar, left).wrapping_rem(Self::signed_integer(scalar, right))
                as u64
        } else {
            left % right
        }
    }

    /// Compute one overflow-safe midpoint rounded toward zero.
    fn midpoint(scalar: Scalar, left: u64, right: u64) -> u64 {
        let width = scalar.bit_width();
        if scalar.is_signed_integer() {
            let left = Self::signed_integer(scalar, left);
            let right = Self::signed_integer(scalar, right);

            Self::truncate(((left + right) / 2) as u64, width)
        } else {
            let left = Self::truncate(left, width) as u128;
            let right = Self::truncate(right, width) as u128;

            ((left + right) / 2) as u64
        }
    }

    /// Clamp one integer between validated ordered bounds.
    fn clamp(scalar: Scalar, value: u64, minimum: u64, maximum: u64) -> Result<u64> {
        if Self::compare(scalar, minimum, maximum).is_gt() {
            return Err(Error::trap(Trap::InvalidArithmetic));
        }

        let value = if Self::compare(scalar, value, minimum).is_lt() {
            minimum
        } else if Self::compare(scalar, value, maximum).is_gt() {
            maximum
        } else {
            value
        };

        Ok(value)
    }

    /// Divide one integer and round the quotient toward positive infinity.
    fn divide_ceil(scalar: Scalar, left: u64, right: u64) -> Result<u64> {
        if right == 0 {
            return Err(Error::trap(Trap::DivisionByZero));
        }
        if Self::division_overflows(scalar, left, right) {
            return Err(Error::trap(Trap::IntegerOverflow));
        }

        let quotient = Self::divide(scalar, left, right);
        let remainder = Self::remainder(scalar, left, right);
        let should_increment = remainder != 0
            && Self::compare(scalar, remainder, 0) == Self::compare(scalar, right, 0);
        let quotient = quotient.wrapping_add(u64::from(should_increment));

        Ok(quotient)
    }

    /// Compute one least nonnegative remainder.
    fn remainder_euclidean(scalar: Scalar, left: u64, right: u64) -> Result<u64> {
        if right == 0 {
            return Err(Error::trap(Trap::DivisionByZero));
        }
        if Self::division_overflows(scalar, left, right) {
            return Err(Error::trap(Trap::IntegerOverflow));
        }

        let remainder = Self::remainder(scalar, left, right);
        if !scalar.is_signed_integer() || Self::compare(scalar, remainder, 0).is_ge() {
            return Ok(remainder);
        }
        let divisor = Self::signed_integer(scalar, right);
        let remainder = Self::signed_integer(scalar, remainder);
        let remainder = remainder + divisor.abs();

        Ok(Self::truncate(remainder as u64, scalar.bit_width()))
    }

    /// Return whether one integer is a multiple of another.
    fn is_multiple_of(scalar: Scalar, left: u64, right: u64) -> bool {
        if right == 0 {
            return left == 0;
        }
        if Self::division_overflows(scalar, left, right) {
            return true;
        }

        Self::remainder(scalar, left, right) == 0
    }

    /// Compute one same-width unsigned absolute integer difference.
    fn abs_diff(scalar: Scalar, left: u64, right: u64) -> u64 {
        if scalar.is_signed_integer() {
            let left = Self::signed_integer(scalar, left);
            let right = Self::signed_integer(scalar, right);

            left.abs_diff(right) as u64
        } else {
            let width = scalar.bit_width();
            let left = Self::truncate(left, width);
            let right = Self::truncate(right, width);

            left.abs_diff(right)
        }
    }

    /// Shift one scalar integer right with the representation's signedness.
    fn shift_right(scalar: Scalar, value: u64, count: u32) -> u64 {
        if scalar.is_signed_integer() {
            (Self::signed_integer(scalar, value) >> count) as u64
        } else {
            value >> count
        }
    }

    /// Return whether one signed division exceeds its exact representation.
    fn division_overflows(scalar: Scalar, left: u64, right: u64) -> bool {
        if !scalar.is_signed_integer() {
            return false;
        }

        let width = scalar.bit_width();
        let minimum = -(1_i128 << (width - 1));

        scalar.integer(left) == Some(minimum) && scalar.integer(right) == Some(-1)
    }

    /// Count leading zero bits inside one exact integer width.
    fn leading_zeros(value: u64, scalar: Scalar) -> u64 {
        let width = scalar.bit_width();
        let shift = u64::BITS as u8 - width;

        (value << shift).leading_zeros() as u64
    }

    /// Count trailing zero bits inside one exact integer width.
    fn trailing_zeros(value: u64, scalar: Scalar) -> u64 {
        let width = scalar.bit_width();
        let value = Self::truncate(value, width);

        value.trailing_zeros().min(width as u32) as u64
    }

    /// Reverse bytes inside one exact integer width.
    fn byte_swap(value: u64, scalar: Scalar) -> u64 {
        match scalar.bit_width() {
            8 => value,
            16 => (value as u16).swap_bytes() as u64,
            32 => (value as u32).swap_bytes() as u64,
            64 => value.swap_bytes(),
            _ => unreachable!("integer scalars use power-of-two byte widths"),
        }
    }

    /// Reverse bits inside one exact integer width.
    fn bit_reverse(value: u64, scalar: Scalar) -> u64 {
        let width = scalar.bit_width();
        let shift = u64::BITS as u8 - width;

        Self::truncate(value, width).reverse_bits() >> shift
    }

    /// Rotate bits left inside one exact integer width.
    fn rotate_left(value: u64, count: u32, scalar: Scalar) -> u64 {
        let width = scalar.bit_width();
        let count = count % width as u32;
        let value = Self::truncate(value, width);

        Self::truncate(
            (value << count) | (value >> ((width as u32 - count) % width as u32)),
            width,
        )
    }

    /// Rotate bits right inside one exact integer width.
    fn rotate_right(value: u64, count: u32, scalar: Scalar) -> u64 {
        let width = scalar.bit_width();
        let count = count % width as u32;
        let value = Self::truncate(value, width);

        Self::truncate(
            (value >> count) | (value << ((width as u32 - count) % width as u32)),
            width,
        )
    }

    /// Decode one integer selected by an integer opcode.
    fn signed_integer(scalar: Scalar, bits: u64) -> i128 {
        let Some(value) = scalar.integer(bits) else {
            unreachable!("integer opcodes carry integer scalars");
        };

        value
    }

    /// Truncate one integer to its scalar register representation.
    const fn truncate(value: u64, bit_width: u8) -> u64 {
        if bit_width == u64::BITS as u8 {
            value
        } else {
            value & ((1_u64 << bit_width) - 1)
        }
    }
}

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one integer operation over two register words.
    pub(crate) fn execute_integer128(
        &mut self,
        instruction: Instruction<'_>,
        operation: IntegerOperation,
        is_signed: bool,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);

        // decode destinations before source values
        let value_target = if operation.is_count() || operation.returns_boolean() {
            None
        } else {
            Some(operands.span()?)
        };
        let scalar_target = if operation.is_count() || operation.returns_boolean() {
            Some(operands.register()?)
        } else {
            None
        };
        let overflow_target = if operation.is_overflowing() {
            Some(operands.register()?)
        } else {
            None
        };
        let left = operands.span()?;
        let left = self.read_integer128(left)?;

        // execute unary, count, and binary forms
        let (value, overflow) = match operation {
            IntegerOperation::Not => (!left, false),
            IntegerOperation::Negate => (0_u128.wrapping_sub(left), false),
            IntegerOperation::LeadingZeroCount => (left.leading_zeros() as u128, false),
            IntegerOperation::TrailingZeroCount => (left.trailing_zeros() as u128, false),
            IntegerOperation::PopulationCount => (left.count_ones() as u128, false),
            IntegerOperation::ByteSwap => (left.swap_bytes(), false),
            IntegerOperation::BitReverse => (left.reverse_bits(), false),
            IntegerOperation::IsolateLowestOne => (left & 0_u128.wrapping_sub(left), false),
            _ if operation.uses_count() => {
                let count = operands.register()?;
                let count = self.read(count.0).as_u64() as u32;

                (
                    Self::integer128_shift(operation, left, count, is_signed),
                    false,
                )
            }
            _ => {
                let right = operands.span()?;
                let right = self.read_integer128(right)?;
                let third = if operation.input_count() == 3 {
                    let third = operands.span()?;

                    Some(self.read_integer128(third)?)
                } else {
                    None
                };

                Self::integer128_operation(operation, left, right, third, is_signed)?
            }
        };

        // store the operation's exact result shape
        if let Some(target) = value_target {
            self.write_integer128(target, value)?;
        }
        if let Some(target) = scalar_target {
            let value = if operation.returns_boolean() {
                Word::boolean(value != 0)
            } else {
                Word::uint32(value as u32)
            };

            self.write(target.0, value);
        }
        if let Some(target) = overflow_target {
            self.write(target.0, Word::boolean(overflow));
        }

        Ok(())
    }

    /// Execute one 128-bit integer operation.
    fn integer128_operation(
        operation: IntegerOperation,
        left: u128,
        right: u128,
        third: Option<u128>,
        is_signed: bool,
    ) -> Result<(u128, bool)> {
        let value = match operation {
            IntegerOperation::Add => (left.wrapping_add(right), false),
            IntegerOperation::Subtract => (left.wrapping_sub(right), false),
            IntegerOperation::Multiply => (left.wrapping_mul(right), false),
            IntegerOperation::Divide if right == 0 => {
                return Err(Error::trap(Trap::DivisionByZero));
            }
            IntegerOperation::Divide
                if is_signed && left == i128::MIN as u128 && right == u128::MAX =>
            {
                return Err(Error::trap(Trap::IntegerOverflow));
            }
            IntegerOperation::Divide if is_signed => {
                ((left as i128).wrapping_div(right as i128) as u128, false)
            }
            IntegerOperation::Divide => (left / right, false),
            IntegerOperation::Remainder if right == 0 => {
                return Err(Error::trap(Trap::DivisionByZero));
            }
            IntegerOperation::Remainder
                if is_signed && left == i128::MIN as u128 && right == u128::MAX =>
            {
                return Err(Error::trap(Trap::IntegerOverflow));
            }
            IntegerOperation::Remainder if is_signed => {
                ((left as i128).wrapping_rem(right as i128) as u128, false)
            }
            IntegerOperation::Remainder => (left % right, false),
            IntegerOperation::And => (left & right, false),
            IntegerOperation::Or => (left | right, false),
            IntegerOperation::Xor => (left ^ right, false),
            IntegerOperation::Equal => ((left == right) as u128, false),
            IntegerOperation::NotEqual => ((left != right) as u128, false),
            IntegerOperation::LessThan => (
                Self::integer128_compare(left, right, is_signed).is_lt() as u128,
                false,
            ),
            IntegerOperation::LessEqual => (
                Self::integer128_compare(left, right, is_signed).is_le() as u128,
                false,
            ),
            IntegerOperation::GreaterThan => (
                Self::integer128_compare(left, right, is_signed).is_gt() as u128,
                false,
            ),
            IntegerOperation::GreaterEqual => (
                Self::integer128_compare(left, right, is_signed).is_ge() as u128,
                false,
            ),
            IntegerOperation::AddOverflow if is_signed => {
                let (value, overflow) = (left as i128).overflowing_add(right as i128);

                (value as u128, overflow)
            }
            IntegerOperation::SubtractOverflow if is_signed => {
                let (value, overflow) = (left as i128).overflowing_sub(right as i128);

                (value as u128, overflow)
            }
            IntegerOperation::MultiplyOverflow if is_signed => {
                let (value, overflow) = (left as i128).overflowing_mul(right as i128);

                (value as u128, overflow)
            }
            IntegerOperation::AddOverflow => left.overflowing_add(right),
            IntegerOperation::SubtractOverflow => left.overflowing_sub(right),
            IntegerOperation::MultiplyOverflow => left.overflowing_mul(right),
            IntegerOperation::AddSaturating if is_signed => {
                ((left as i128).saturating_add(right as i128) as u128, false)
            }
            IntegerOperation::SubtractSaturating if is_signed => {
                ((left as i128).saturating_sub(right as i128) as u128, false)
            }
            IntegerOperation::AddSaturating => (left.saturating_add(right), false),
            IntegerOperation::SubtractSaturating => (left.saturating_sub(right), false),
            IntegerOperation::Midpoint if is_signed => {
                ((left as i128).midpoint(right as i128) as u128, false)
            }
            IntegerOperation::Midpoint => (left.midpoint(right), false),
            IntegerOperation::Clamp => {
                let maximum = third.ok_or_else(Error::invalid_instruction)?;
                if Self::integer128_compare(right, maximum, is_signed).is_gt() {
                    return Err(Error::trap(Trap::InvalidArithmetic));
                }
                let value = if Self::integer128_compare(left, right, is_signed).is_lt() {
                    right
                } else if Self::integer128_compare(left, maximum, is_signed).is_gt() {
                    maximum
                } else {
                    left
                };

                (value, false)
            }
            IntegerOperation::DivideCeil => {
                let quotient = Self::integer128_divide(left, right, is_signed)?;
                let remainder = Self::integer128_remainder(left, right, is_signed)?;
                let same_sign = Self::integer128_compare(remainder, 0, is_signed)
                    == Self::integer128_compare(right, 0, is_signed);
                let increment = u128::from(remainder != 0 && same_sign);

                (quotient.wrapping_add(increment), false)
            }
            IntegerOperation::RemainderEuclidean => {
                let remainder = Self::integer128_remainder(left, right, is_signed)?;
                if !is_signed || (remainder as i128) >= 0 {
                    (remainder, false)
                } else {
                    let divisor = right as i128;
                    let magnitude = divisor.unsigned_abs();
                    let remainder = (remainder as i128) as u128;

                    (remainder.wrapping_add(magnitude), false)
                }
            }
            IntegerOperation::IsMultipleOf if right == 0 => ((left == 0) as u128, false),
            IntegerOperation::IsMultipleOf
                if is_signed && left == i128::MIN as u128 && right == u128::MAX =>
            {
                (1, false)
            }
            IntegerOperation::IsMultipleOf if is_signed => {
                (((left as i128) % (right as i128) == 0) as u128, false)
            }
            IntegerOperation::IsMultipleOf => (left.is_multiple_of(right) as u128, false),
            IntegerOperation::AbsDiff if is_signed => {
                ((left as i128).abs_diff(right as i128), false)
            }
            IntegerOperation::AbsDiff => (left.abs_diff(right), false),
            _ => unreachable!("unary and shift operations are handled by the caller"),
        };

        Ok(value)
    }

    /// Divide two 128-bit integers with language traps.
    fn integer128_divide(left: u128, right: u128, is_signed: bool) -> Result<u128> {
        if right == 0 {
            return Err(Error::trap(Trap::DivisionByZero));
        }
        if is_signed && left == i128::MIN as u128 && right == u128::MAX {
            return Err(Error::trap(Trap::IntegerOverflow));
        }

        let quotient = if is_signed {
            ((left as i128) / (right as i128)) as u128
        } else {
            left / right
        };

        Ok(quotient)
    }

    /// Compute one 128-bit remainder with language traps.
    fn integer128_remainder(left: u128, right: u128, is_signed: bool) -> Result<u128> {
        if right == 0 {
            return Err(Error::trap(Trap::DivisionByZero));
        }
        if is_signed && left == i128::MIN as u128 && right == u128::MAX {
            return Err(Error::trap(Trap::IntegerOverflow));
        }

        let remainder = if is_signed {
            ((left as i128) % (right as i128)) as u128
        } else {
            left % right
        };

        Ok(remainder)
    }

    /// Execute one 128-bit shift or rotation.
    fn integer128_shift(
        operation: IntegerOperation,
        value: u128,
        count: u32,
        is_signed: bool,
    ) -> u128 {
        match operation {
            IntegerOperation::ShiftLeft => value.wrapping_shl(count),
            IntegerOperation::ShiftRight if is_signed => {
                (value as i128).wrapping_shr(count) as u128
            }
            IntegerOperation::ShiftRight => value.wrapping_shr(count),
            IntegerOperation::RotateLeft => value.rotate_left(count),
            IntegerOperation::RotateRight => value.rotate_right(count),
            _ => unreachable!("integer counts select shifts or rotations"),
        }
    }

    /// Compare two 128-bit integers with the selected signedness.
    fn integer128_compare(left: u128, right: u128, is_signed: bool) -> Ordering {
        if is_signed {
            (left as i128).cmp(&(right as i128))
        } else {
            left.cmp(&right)
        }
    }

    /// Read one two-word 128-bit integer.
    fn read_integer128(&self, range: RegisterSpan) -> Result<u128> {
        if range.word_count != 2 {
            return Err(self.invalid_instruction());
        }
        let low = self.read(range.start.0).bits() as u128;
        let high = self.read(range.start.0 + 1).bits() as u128;

        Ok(low | high << u64::BITS)
    }

    /// Write one two-word 128-bit integer.
    fn write_integer128(&mut self, range: RegisterSpan, value: u128) -> Result<()> {
        if range.word_count != 2 {
            return Err(self.invalid_instruction());
        }

        self.write(range.start.0, Word::from_bits(value as u64));
        self.write(
            range.start.0 + 1,
            Word::from_bits((value >> u64::BITS) as u64),
        );

        Ok(())
    }
}
