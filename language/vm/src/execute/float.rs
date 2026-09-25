use tspp_bytecode::{FloatOperation, Instruction, Scalar};
use tspp_program::{Runtime, Word};

use crate::diagnostic::{Error, Result, Trap};
use crate::machine::Activation;

use super::arithmetic::Arithmetic;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one scalar floating-point operation.
    #[inline(always)]
    pub(crate) fn execute_float(
        &mut self,
        instruction: Instruction<'_>,
        operation: FloatOperation,
        scalar: Scalar,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
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

        let value = Arithmetic::float(operation, scalar, left, right, third)?;

        self.write(target.0, value);

        Ok(())
    }
}

impl Arithmetic {
    /// Execute one floating-point operation over unpacked scalar words.
    pub(super) fn float(
        operation: FloatOperation,
        scalar: Scalar,
        left: Word,
        right: Option<Word>,
        third: Option<Word>,
    ) -> Result<Word> {
        if scalar == Scalar::Float64 {
            Self::float64(operation, left, right, third)
        } else {
            Self::float32(operation, scalar, left, right, third)
        }
    }

    /// Execute one binary64 operation.
    fn float64(
        operation: FloatOperation,
        left: Word,
        right: Option<Word>,
        third: Option<Word>,
    ) -> Result<Word> {
        let left = left.as_f64();
        let right = right.map(Word::as_f64);
        let third = third.map(Word::as_f64);

        // predicates produce canonical boolean words
        if operation.returns_boolean() {
            let value = match operation {
                FloatOperation::IsFinite => left.is_finite(),
                FloatOperation::IsInfinite => left.is_infinite(),
                _ => Self::compare_float(
                    operation,
                    left,
                    right.ok_or_else(Error::invalid_instruction)?,
                ),
            };

            return Ok(Word::boolean(value));
        }

        let value = match operation {
            FloatOperation::Add => left + right.ok_or_else(Error::invalid_instruction)?,
            FloatOperation::Subtract => left - right.ok_or_else(Error::invalid_instruction)?,
            FloatOperation::Multiply => left * right.ok_or_else(Error::invalid_instruction)?,
            FloatOperation::Divide => left / right.ok_or_else(Error::invalid_instruction)?,
            FloatOperation::Remainder => left % right.ok_or_else(Error::invalid_instruction)?,
            FloatOperation::Negate => -left,
            FloatOperation::SquareRoot => left.sqrt(),
            FloatOperation::CubeRoot => left.cbrt(),
            FloatOperation::Absolute => left.abs(),
            FloatOperation::FusedMultiplyAdd => left.mul_add(
                right.ok_or_else(Error::invalid_instruction)?,
                third.ok_or_else(Error::invalid_instruction)?,
            ),
            FloatOperation::CopySign => {
                left.copysign(right.ok_or_else(Error::invalid_instruction)?)
            }
            FloatOperation::Minimum => {
                Self::minimum64(left, right.ok_or_else(Error::invalid_instruction)?)
            }
            FloatOperation::Maximum => {
                Self::maximum64(left, right.ok_or_else(Error::invalid_instruction)?)
            }
            FloatOperation::Sin => left.sin(),
            FloatOperation::Cos => left.cos(),
            FloatOperation::Tan => left.tan(),
            FloatOperation::Asin => left.asin(),
            FloatOperation::Acos => left.acos(),
            FloatOperation::Atan => left.atan(),
            FloatOperation::Atan2 => left.atan2(right.ok_or_else(Error::invalid_instruction)?),
            FloatOperation::Exp => left.exp(),
            FloatOperation::Expm1 => left.exp_m1(),
            FloatOperation::Exp2 => left.exp2(),
            FloatOperation::Log => left.ln(),
            FloatOperation::Log1p => left.ln_1p(),
            FloatOperation::Log2 => left.log2(),
            FloatOperation::Log10 => left.log10(),
            FloatOperation::Pow => left.powf(right.ok_or_else(Error::invalid_instruction)?),
            FloatOperation::Floor => left.floor(),
            FloatOperation::Ceil => left.ceil(),
            FloatOperation::Truncate => left.trunc(),
            FloatOperation::RoundTiesEven => left.round_ties_even(),
            FloatOperation::Round => Self::round_ties_positive_infinity64(left),
            FloatOperation::RoundTiesAway => left.round(),
            FloatOperation::Midpoint => {
                left.midpoint(right.ok_or_else(Error::invalid_instruction)?)
            }
            FloatOperation::Clamp => {
                let minimum = right.ok_or_else(Error::invalid_instruction)?;
                let maximum = third.ok_or_else(Error::invalid_instruction)?;
                if minimum > maximum || minimum.is_nan() || maximum.is_nan() {
                    return Err(Error::trap(Trap::InvalidArithmetic));
                }

                left.clamp(minimum, maximum)
            }
            _ => unreachable!("floating-point predicates return before arithmetic"),
        };

        Ok(Word::float64(value))
    }

    /// Execute one binary32 operation.
    fn float32(
        operation: FloatOperation,
        scalar: Scalar,
        left: Word,
        right: Option<Word>,
        third: Option<Word>,
    ) -> Result<Word> {
        let left = scalar
            .float(left.bits())
            .ok_or_else(Error::invalid_instruction)? as f32;
        let right = match right {
            Some(value) => Some(
                scalar
                    .float(value.bits())
                    .ok_or_else(Error::invalid_instruction)? as f32,
            ),
            None => None,
        };
        let third = match third {
            Some(value) => Some(
                scalar
                    .float(value.bits())
                    .ok_or_else(Error::invalid_instruction)? as f32,
            ),
            None => None,
        };

        // predicates produce canonical boolean words
        if operation.returns_boolean() {
            let value = match operation {
                FloatOperation::IsFinite => left.is_finite(),
                FloatOperation::IsInfinite => left.is_infinite(),
                _ => Self::compare_float(
                    operation,
                    left,
                    right.ok_or_else(Error::invalid_instruction)?,
                ),
            };

            return Ok(Word::boolean(value));
        }

        let value = match operation {
            FloatOperation::Add => left + right.ok_or_else(Error::invalid_instruction)?,
            FloatOperation::Subtract => left - right.ok_or_else(Error::invalid_instruction)?,
            FloatOperation::Multiply => left * right.ok_or_else(Error::invalid_instruction)?,
            FloatOperation::Divide => left / right.ok_or_else(Error::invalid_instruction)?,
            FloatOperation::Remainder => left % right.ok_or_else(Error::invalid_instruction)?,
            FloatOperation::Negate => -left,
            FloatOperation::SquareRoot => left.sqrt(),
            FloatOperation::CubeRoot => left.cbrt(),
            FloatOperation::Absolute => left.abs(),
            FloatOperation::FusedMultiplyAdd => left.mul_add(
                right.ok_or_else(Error::invalid_instruction)?,
                third.ok_or_else(Error::invalid_instruction)?,
            ),
            FloatOperation::CopySign => {
                left.copysign(right.ok_or_else(Error::invalid_instruction)?)
            }
            FloatOperation::Minimum => {
                Self::minimum32(left, right.ok_or_else(Error::invalid_instruction)?)
            }
            FloatOperation::Maximum => {
                Self::maximum32(left, right.ok_or_else(Error::invalid_instruction)?)
            }
            FloatOperation::Sin => left.sin(),
            FloatOperation::Cos => left.cos(),
            FloatOperation::Tan => left.tan(),
            FloatOperation::Asin => left.asin(),
            FloatOperation::Acos => left.acos(),
            FloatOperation::Atan => left.atan(),
            FloatOperation::Atan2 => left.atan2(right.ok_or_else(Error::invalid_instruction)?),
            FloatOperation::Exp => left.exp(),
            FloatOperation::Expm1 => left.exp_m1(),
            FloatOperation::Exp2 => left.exp2(),
            FloatOperation::Log => left.ln(),
            FloatOperation::Log1p => left.ln_1p(),
            FloatOperation::Log2 => left.log2(),
            FloatOperation::Log10 => left.log10(),
            FloatOperation::Pow => left.powf(right.ok_or_else(Error::invalid_instruction)?),
            FloatOperation::Floor => left.floor(),
            FloatOperation::Ceil => left.ceil(),
            FloatOperation::Truncate => left.trunc(),
            FloatOperation::RoundTiesEven => left.round_ties_even(),
            FloatOperation::Round => Self::round_ties_positive_infinity32(left),
            FloatOperation::RoundTiesAway => left.round(),
            FloatOperation::Midpoint => {
                left.midpoint(right.ok_or_else(Error::invalid_instruction)?)
            }
            FloatOperation::Clamp => {
                let minimum = right.ok_or_else(Error::invalid_instruction)?;
                let maximum = third.ok_or_else(Error::invalid_instruction)?;
                if minimum > maximum || minimum.is_nan() || maximum.is_nan() {
                    return Err(Error::trap(Trap::InvalidArithmetic));
                }

                left.clamp(minimum, maximum)
            }
            _ => unreachable!("floating-point predicates return before arithmetic"),
        };
        let bits = scalar
            .float_bits(value as f64)
            .ok_or_else(Error::invalid_instruction)?;

        Ok(Word::from_bits(scalar.encode(bits)))
    }

    /// Compare two floating-point values with ordered language comparisons.
    fn compare_float<T: PartialOrd + PartialEq>(
        operation: FloatOperation,
        left: T,
        right: T,
    ) -> bool {
        match operation {
            FloatOperation::Equal => left == right,
            FloatOperation::NotEqual => left != right,
            FloatOperation::LessThan => left < right,
            FloatOperation::LessEqual => left <= right,
            FloatOperation::GreaterThan => left > right,
            FloatOperation::GreaterEqual => left >= right,
            _ => unreachable!("floating-point comparison uses one comparison operation"),
        }
    }

    /// Round one binary32 value to the nearest integer with ties toward positive infinity.
    fn round_ties_positive_infinity32(value: f32) -> f32 {
        let lower = value.floor();
        let distance = value - lower;
        let rounded = if distance < 0.5 { lower } else { lower + 1.0 };

        rounded.copysign(value)
    }

    /// Round one binary64 value to the nearest integer with ties toward positive infinity.
    fn round_ties_positive_infinity64(value: f64) -> f64 {
        let lower = value.floor();
        let distance = value - lower;
        let rounded = if distance < 0.5 { lower } else { lower + 1.0 };

        rounded.copysign(value)
    }

    /// Select the IEEE minimum binary32 value.
    fn minimum32(left: f32, right: f32) -> f32 {
        if left.is_nan() {
            left
        } else if right.is_nan() {
            right
        } else if left == right {
            if left.is_sign_negative() { left } else { right }
        } else {
            left.min(right)
        }
    }

    /// Select the IEEE maximum binary32 value.
    fn maximum32(left: f32, right: f32) -> f32 {
        if left.is_nan() {
            left
        } else if right.is_nan() {
            right
        } else if left == right {
            if left.is_sign_positive() { left } else { right }
        } else {
            left.max(right)
        }
    }

    /// Select the IEEE minimum binary64 value.
    fn minimum64(left: f64, right: f64) -> f64 {
        if left.is_nan() {
            left
        } else if right.is_nan() {
            right
        } else if left == right {
            if left.is_sign_negative() { left } else { right }
        } else {
            left.min(right)
        }
    }

    /// Select the IEEE maximum binary64 value.
    fn maximum64(left: f64, right: f64) -> f64 {
        if left.is_nan() {
            left
        } else if right.is_nan() {
            right
        } else if left == right {
            if left.is_sign_positive() { left } else { right }
        } else {
            left.max(right)
        }
    }
}
