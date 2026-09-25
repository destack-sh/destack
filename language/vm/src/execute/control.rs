use tspp_bytecode as bytecode;
use tspp_bytecode::{Comparison, Instruction, Opcode, Scalar, ScalarCheck};
use tspp_program::{Runtime, TypeId, Word};

use crate::diagnostic::{Error, Result, Trap};
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one direct or conditional control transfer.
    #[inline(always)]
    pub(crate) fn execute_control(&self, instruction: Instruction<'_>) -> Result<i32> {
        let mut operands = self.operands(instruction);
        let displacement = if instruction.opcode() == Opcode::JUMP {
            operands.i32()?
        } else {
            let condition = operands.register()?;
            let yes = operands.i32()?;
            let no = operands.i32()?;

            if self.read(condition.0).as_boolean() {
                yes
            } else {
                no
            }
        };

        Ok(displacement)
    }

    /// Execute one scalar check and branch on failure.
    pub(crate) fn execute_check(
        &self,
        instruction: Instruction<'_>,
        check: ScalarCheck,
        scalar: Scalar,
    ) -> Result<Option<i32>> {
        let mut operands = self.operands(instruction);
        let left = operands.register()?;
        let left = self.read(left.0);
        let is_valid = match check {
            ScalarCheck::Nonzero => self.is_nonzero(left, scalar)?,
            ScalarCheck::Shift => {
                let width = operands.u16()?;

                scalar
                    .integer(left.bits())
                    .is_some_and(|value| value >= 0 && value < i128::from(width))
            }
            ScalarCheck::Narrow => {
                let target = operands.u16()?;
                let target =
                    Scalar::from_code(target as u8).ok_or_else(|| self.invalid_instruction())?;
                let value = scalar
                    .integer(left.bits())
                    .ok_or_else(|| self.invalid_instruction())?;
                let (minimum, maximum) = target
                    .integer_bounds()
                    .ok_or_else(|| self.invalid_instruction())?;

                value >= minimum && value <= maximum
            }
            ScalarCheck::AddOverflow
            | ScalarCheck::SubtractOverflow
            | ScalarCheck::MultiplyOverflow => {
                let right = operands.register()?;

                !self.integer_overflows(check, scalar, left, self.read(right.0))?
            }
            ScalarCheck::Bounds => {
                let length = operands.register()?;
                let index = scalar
                    .integer(left.bits())
                    .ok_or_else(|| self.invalid_instruction())?;
                let length = scalar
                    .integer(self.read(length.0).bits())
                    .ok_or_else(|| self.invalid_instruction())?;

                index >= 0 && index < length
            }
            ScalarCheck::Range => {
                let length = operands.register()?;
                let limit = operands.register()?;
                let start = scalar
                    .integer(left.bits())
                    .ok_or_else(|| self.invalid_instruction())?;
                let length = scalar
                    .integer(self.read(length.0).bits())
                    .ok_or_else(|| self.invalid_instruction())?;
                let limit = scalar
                    .integer(self.read(limit.0).bits())
                    .ok_or_else(|| self.invalid_instruction())?;

                start >= 0 && length >= 0 && start <= limit && length <= limit - start
            }
        };
        let failure = operands.i32()?;

        Ok((!is_valid).then_some(failure))
    }

    /// Execute one fused scalar comparison and branch.
    #[inline(always)]
    pub(crate) fn execute_comparison(
        &self,
        instruction: Instruction<'_>,
        comparison: Comparison,
        scalar: Scalar,
    ) -> Result<i32> {
        let mut operands = self.operands(instruction);
        let left = operands.register()?;
        let right = operands.register()?;
        let yes = operands.i32()?;
        let no = operands.i32()?;
        let is_match =
            self.compare_words(comparison, scalar, self.read(left.0), self.read(right.0))?;
        let displacement = if is_match { yes } else { no };

        Ok(displacement)
    }

    /// Execute one inline integer switch.
    pub(crate) fn execute_switch(&self, instruction: Instruction<'_>) -> Result<i32> {
        let mut operands = self.operands(instruction);
        let value = operands.register()?;
        let value = self.read(value.0).bits();
        let case_count = operands.u16()?;
        let mut selected = None;

        // read every case to reach the mandatory fallback operand
        for _ in 0..case_count {
            let case = operands.u64()?;
            let displacement = operands.i32()?;
            if selected.is_none() && value == case {
                selected = Some(displacement);
            }
        }
        let fallback = operands.i32()?;

        let displacement = match selected {
            Some(displacement) => displacement,
            None => fallback,
        };

        Ok(displacement)
    }

    /// Execute one address or runtime type check.
    pub(crate) fn execute_runtime_check(
        &self,
        instruction: Instruction<'_>,
    ) -> Result<Option<i32>> {
        let mut operands = self.operands(instruction);
        let value = operands.register()?;
        let value = self.read(value.0);
        let is_valid = match instruction.opcode() {
            Opcode::CHECK_NULLISH => !value.is_nullish(),
            Opcode::CHECK_EXACT_TYPE => {
                let expected = TypeId(operands.u32()?);

                value.bits() == u64::from(expected.0)
            }
            Opcode::CHECK_SUBTYPE => {
                let expected = TypeId(operands.u32()?);
                let concrete = TypeId(value.bits() as u32);

                self.machine
                    .program
                    .is_subtype(concrete, expected)
                    .map_err(Error::program)?
            }
            _ => unreachable!("runtime check dispatch selects one check opcode"),
        };
        let failure = operands.i32()?;

        Ok((!is_valid).then_some(failure))
    }

    /// Execute one terminal language trap.
    pub(crate) fn execute_trap(&self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let reason = operands.u16()?;
        let reason =
            bytecode::Trap::from_code(reason as u8).ok_or_else(|| self.invalid_instruction())?;
        let trap = match reason {
            bytecode::Trap::Abort => Trap::Abort,
            bytecode::Trap::Bounds => Trap::Bounds,
            bytecode::Trap::Null => Trap::Null,
            bytecode::Trap::Overflow => Trap::IntegerOverflow,
            bytecode::Trap::Arithmetic => Trap::InvalidArithmetic,
            bytecode::Trap::Type => Trap::Type,
        };

        Err(Error::trap(trap))
    }

    /// Return whether one scalar word is nonzero.
    fn is_nonzero(&self, value: Word, scalar: Scalar) -> Result<bool> {
        if scalar.is_integer() {
            Ok(scalar.integer(value.bits()).is_some_and(|value| value != 0))
        } else {
            let value = scalar
                .float(value.bits())
                .ok_or_else(|| self.invalid_instruction())?;

            Ok(value != 0.0)
        }
    }

    /// Return whether one checked integer operation exceeds its representation.
    fn integer_overflows(
        &self,
        check: ScalarCheck,
        scalar: Scalar,
        left: Word,
        right: Word,
    ) -> Result<bool> {
        if scalar.is_signed_integer() {
            let left = scalar
                .integer(left.bits())
                .ok_or_else(|| self.invalid_instruction())?;
            let right = scalar
                .integer(right.bits())
                .ok_or_else(|| self.invalid_instruction())?;
            let value = match check {
                ScalarCheck::AddOverflow => left + right,
                ScalarCheck::SubtractOverflow => left - right,
                ScalarCheck::MultiplyOverflow => left * right,
                _ => unreachable!("overflow checks use one arithmetic check"),
            };
            let (minimum, maximum) = scalar
                .integer_bounds()
                .ok_or_else(|| self.invalid_instruction())?;

            Ok(value < minimum || value > maximum)
        } else {
            let left = scalar
                .integer(left.bits())
                .ok_or_else(|| self.invalid_instruction())? as u128;
            let right = scalar
                .integer(right.bits())
                .ok_or_else(|| self.invalid_instruction())? as u128;
            let maximum = scalar
                .integer_bounds()
                .ok_or_else(|| self.invalid_instruction())?
                .1 as u128;
            let is_overflow = match check {
                ScalarCheck::AddOverflow => left + right > maximum,
                ScalarCheck::SubtractOverflow => left < right,
                ScalarCheck::MultiplyOverflow => left * right > maximum,
                _ => unreachable!("overflow checks use one arithmetic check"),
            };

            Ok(is_overflow)
        }
    }

    /// Compare two words under one scalar representation.
    fn compare_words(
        &self,
        comparison: Comparison,
        scalar: Scalar,
        left: Word,
        right: Word,
    ) -> Result<bool> {
        if scalar.is_integer() {
            let left = scalar
                .integer(left.bits())
                .ok_or_else(|| self.invalid_instruction())?;
            let right = scalar
                .integer(right.bits())
                .ok_or_else(|| self.invalid_instruction())?;

            Ok(Self::compare_values(comparison, left, right))
        } else {
            let left = scalar
                .float(left.bits())
                .ok_or_else(|| self.invalid_instruction())?;
            let right = scalar
                .float(right.bits())
                .ok_or_else(|| self.invalid_instruction())?;

            Ok(Self::compare_values(comparison, left, right))
        }
    }

    /// Compare two partially ordered scalar values.
    fn compare_values<T: PartialOrd + PartialEq>(
        comparison: Comparison,
        left: T,
        right: T,
    ) -> bool {
        match comparison {
            Comparison::Equal => left == right,
            Comparison::NotEqual => left != right,
            Comparison::LessThan => left < right,
            Comparison::LessEqual => left <= right,
            Comparison::GreaterThan => left > right,
            Comparison::GreaterEqual => left >= right,
        }
    }
}
