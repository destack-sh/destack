use destack_bytecode::{FloatOperation, Instruction, Scalar};
use destack_program::{Runtime, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

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
        let addend = if operation.input_count() == 3 {
            let addend = operands.register()?;

            Some(self.read(addend.0))
        } else {
            None
        };

        let value = self.float_value(operation, scalar, left, right, addend)?;

        self.write(target.0, value);

        Ok(())
    }

    /// Execute one floating-point operation over unpacked scalar words.
    pub(super) fn float_value(
        &self,
        operation: FloatOperation,
        scalar: Scalar,
        left: Word,
        right: Option<Word>,
        addend: Option<Word>,
    ) -> Result<Word> {
        if scalar == Scalar::Float64 {
            self.float64(operation, left, right, addend)
        } else {
            self.float32(operation, scalar, left, right, addend)
        }
    }

    /// Execute one binary64 operation.
    fn float64(
        &self,
        operation: FloatOperation,
        left: Word,
        right: Option<Word>,
        addend: Option<Word>,
    ) -> Result<Word> {
        let left = left.as_f64();
        let right = right.map(Word::as_f64);
        let addend = addend.map(Word::as_f64);

        // comparisons produce canonical boolean words
        if operation.is_comparison() {
            let right = right.ok_or_else(|| self.invalid_instruction())?;
            let value = Self::compare_float(operation, left, right);

            return Ok(Word::boolean(value));
        }

        let value = match operation {
            FloatOperation::Add => left + right.ok_or_else(|| self.invalid_instruction())?,
            FloatOperation::Subtract => left - right.ok_or_else(|| self.invalid_instruction())?,
            FloatOperation::Multiply => left * right.ok_or_else(|| self.invalid_instruction())?,
            FloatOperation::Divide => left / right.ok_or_else(|| self.invalid_instruction())?,
            FloatOperation::Remainder => left % right.ok_or_else(|| self.invalid_instruction())?,
            FloatOperation::Negate => -left,
            FloatOperation::SquareRoot => left.sqrt(),
            FloatOperation::Absolute => left.abs(),
            FloatOperation::FusedMultiplyAdd => left.mul_add(
                right.ok_or_else(|| self.invalid_instruction())?,
                addend.ok_or_else(|| self.invalid_instruction())?,
            ),
            FloatOperation::CopySign => {
                left.copysign(right.ok_or_else(|| self.invalid_instruction())?)
            }
            FloatOperation::Minimum => {
                Self::minimum64(left, right.ok_or_else(|| self.invalid_instruction())?)
            }
            FloatOperation::Maximum => {
                Self::maximum64(left, right.ok_or_else(|| self.invalid_instruction())?)
            }
            FloatOperation::Sin => left.sin(),
            FloatOperation::Cos => left.cos(),
            FloatOperation::Tan => left.tan(),
            FloatOperation::Asin => left.asin(),
            FloatOperation::Acos => left.acos(),
            FloatOperation::Atan => left.atan(),
            FloatOperation::Atan2 => left.atan2(right.ok_or_else(|| self.invalid_instruction())?),
            FloatOperation::Exp => left.exp(),
            FloatOperation::Exp2 => left.exp2(),
            FloatOperation::Log => left.ln(),
            FloatOperation::Log2 => left.log2(),
            FloatOperation::Log10 => left.log10(),
            FloatOperation::Pow => left.powf(right.ok_or_else(|| self.invalid_instruction())?),
            FloatOperation::Floor => left.floor(),
            FloatOperation::Ceil => left.ceil(),
            FloatOperation::Truncate => left.trunc(),
            FloatOperation::RoundTiesEven => left.round_ties_even(),
            _ => unreachable!("floating-point comparisons return before arithmetic"),
        };

        Ok(Word::float64(value))
    }

    /// Execute one binary32, binary16, or bfloat16 operation.
    fn float32(
        &self,
        operation: FloatOperation,
        scalar: Scalar,
        left: Word,
        right: Option<Word>,
        addend: Option<Word>,
    ) -> Result<Word> {
        let left = scalar
            .float(left.bits())
            .ok_or_else(|| self.invalid_instruction())? as f32;
        let right = match right {
            Some(value) => Some(
                scalar
                    .float(value.bits())
                    .ok_or_else(|| self.invalid_instruction())? as f32,
            ),
            None => None,
        };
        let addend = match addend {
            Some(value) => Some(
                scalar
                    .float(value.bits())
                    .ok_or_else(|| self.invalid_instruction())? as f32,
            ),
            None => None,
        };

        // comparisons produce canonical boolean words
        if operation.is_comparison() {
            let right = right.ok_or_else(|| self.invalid_instruction())?;
            let value = Self::compare_float(operation, left, right);

            return Ok(Word::boolean(value));
        }

        let value = match operation {
            FloatOperation::Add => left + right.ok_or_else(|| self.invalid_instruction())?,
            FloatOperation::Subtract => left - right.ok_or_else(|| self.invalid_instruction())?,
            FloatOperation::Multiply => left * right.ok_or_else(|| self.invalid_instruction())?,
            FloatOperation::Divide => left / right.ok_or_else(|| self.invalid_instruction())?,
            FloatOperation::Remainder => left % right.ok_or_else(|| self.invalid_instruction())?,
            FloatOperation::Negate => -left,
            FloatOperation::SquareRoot => left.sqrt(),
            FloatOperation::Absolute => left.abs(),
            FloatOperation::FusedMultiplyAdd => left.mul_add(
                right.ok_or_else(|| self.invalid_instruction())?,
                addend.ok_or_else(|| self.invalid_instruction())?,
            ),
            FloatOperation::CopySign => {
                left.copysign(right.ok_or_else(|| self.invalid_instruction())?)
            }
            FloatOperation::Minimum => {
                Self::minimum32(left, right.ok_or_else(|| self.invalid_instruction())?)
            }
            FloatOperation::Maximum => {
                Self::maximum32(left, right.ok_or_else(|| self.invalid_instruction())?)
            }
            FloatOperation::Sin => left.sin(),
            FloatOperation::Cos => left.cos(),
            FloatOperation::Tan => left.tan(),
            FloatOperation::Asin => left.asin(),
            FloatOperation::Acos => left.acos(),
            FloatOperation::Atan => left.atan(),
            FloatOperation::Atan2 => left.atan2(right.ok_or_else(|| self.invalid_instruction())?),
            FloatOperation::Exp => left.exp(),
            FloatOperation::Exp2 => left.exp2(),
            FloatOperation::Log => left.ln(),
            FloatOperation::Log2 => left.log2(),
            FloatOperation::Log10 => left.log10(),
            FloatOperation::Pow => left.powf(right.ok_or_else(|| self.invalid_instruction())?),
            FloatOperation::Floor => left.floor(),
            FloatOperation::Ceil => left.ceil(),
            FloatOperation::Truncate => left.trunc(),
            FloatOperation::RoundTiesEven => left.round_ties_even(),
            _ => unreachable!("floating-point comparisons return before arithmetic"),
        };
        let bits = scalar
            .float_bits(value as f64)
            .ok_or_else(|| self.invalid_instruction())?;

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
