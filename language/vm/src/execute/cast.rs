use tspp_bytecode::{CastOperation, ConvertMode, Instruction, Scalar, ValueType};
use tspp_program::{Runtime, Word};

use crate::diagnostic::{Error, Result, Trap};
use crate::machine::Activation;

use super::arithmetic::Arithmetic;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one scalar or pointer conversion.
    pub(crate) fn execute_cast(
        &mut self,
        instruction: Instruction<'_>,
        operation: CastOperation,
        source: ValueType,
        target: ValueType,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let result = operands.register()?;
        let value = operands.register()?;
        let value = self.read(value.0);

        let value = match operation {
            CastOperation::Truncate => self.truncate_cast(value, target)?,
            CastOperation::Saturate => self.saturate_cast(value, source, target)?,
            CastOperation::SignExtend => self.sign_extend(value, source, target)?,
            CastOperation::ZeroExtend => self.zero_extend(value, source, target)?,
            CastOperation::FloatToInt => self.float_to_int(value, source, target, false)?,
            CastOperation::FloatToIntSaturating => {
                self.float_to_int(value, source, target, true)?
            }
            CastOperation::IntToFloat => self.int_to_float(value, source, target)?,
            CastOperation::FloatConvert => self.float_convert(value, source, target)?,
            CastOperation::Bit => self.bit_cast(value, target)?,
            CastOperation::PointerToInt | CastOperation::IntToPointer => value,
        };

        self.write(result.0, value);

        Ok(())
    }
}

impl Arithmetic {
    /// Convert one numeric scalar under an explicit conversion mode.
    pub(super) fn convert(
        value: Word,
        source: Scalar,
        target: Scalar,
        mode: ConvertMode,
    ) -> Result<Word> {
        if source == target {
            return Ok(Word::from_bits(target.encode(value.bits())));
        }

        // boolean values only convert through identity
        if source == Scalar::Boolean || target == Scalar::Boolean {
            return Err(Error::invalid_instruction());
        }

        // select the numeric conversion domain
        if source.is_integer() && target.is_integer() {
            Self::convert_integer(value, source, target, mode)
        } else if source.is_integer() && target.is_float() {
            Self::convert_integer_to_float(value, source, target, mode)
        } else if source.is_float() && target.is_integer() {
            Self::convert_float_to_integer(value, source, target, mode)
        } else if source.is_float() && target.is_float() {
            Self::convert_float(value, source, target, mode)
        } else {
            Err(Error::invalid_instruction())
        }
    }

    /// Convert one integer between scalar representations.
    fn convert_integer(
        value: Word,
        source: Scalar,
        target: Scalar,
        mode: ConvertMode,
    ) -> Result<Word> {
        let value = source
            .integer(value.bits())
            .ok_or_else(Error::invalid_instruction)?;
        let (minimum, maximum) = target
            .integer_bounds()
            .ok_or_else(Error::invalid_instruction)?;
        let value = if mode == ConvertMode::Saturate {
            value.clamp(minimum, maximum)
        } else if value < minimum || value > maximum {
            return Err(Error::trap(Trap::IntegerOverflow));
        } else {
            value
        };

        Ok(Word::from_bits(target.encode(value as u64)))
    }

    /// Convert one integer into a floating-point representation.
    fn convert_integer_to_float(
        value: Word,
        source: Scalar,
        target: Scalar,
        mode: ConvertMode,
    ) -> Result<Word> {
        let integer = source
            .integer(value.bits())
            .ok_or_else(Error::invalid_instruction)?;
        let value = integer as f64;
        let bits = target
            .float_bits(value)
            .ok_or_else(Error::invalid_instruction)?;

        // exact conversion must preserve the mathematical integer
        if mode == ConvertMode::Exact {
            let converted = target.float(bits).ok_or_else(Error::invalid_instruction)?;
            let is_exact =
                converted.is_finite() && converted.fract() == 0.0 && converted as i128 == integer;
            if !is_exact {
                return Err(Error::trap(Trap::IntegerOverflow));
            }
        }

        Ok(Word::from_bits(target.encode(bits)))
    }

    /// Convert one floating-point value into an integer representation.
    fn convert_float_to_integer(
        value: Word,
        source: Scalar,
        target: Scalar,
        mode: ConvertMode,
    ) -> Result<Word> {
        let value = source
            .float(value.bits())
            .ok_or_else(Error::invalid_instruction)?;
        let (minimum, maximum) = target
            .integer_bounds()
            .ok_or_else(Error::invalid_instruction)?;

        // apply the requested integer rounding rule
        let rounded = match mode {
            ConvertMode::Exact if value.fract() == 0.0 => value,
            ConvertMode::Exact => {
                return Err(Error::trap(Trap::InvalidArithmetic));
            }
            ConvertMode::RoundTiesEven => value.round_ties_even(),
            ConvertMode::RoundTowardZero | ConvertMode::Saturate => value.trunc(),
            ConvertMode::RoundFloor => value.floor(),
            ConvertMode::RoundCeil => value.ceil(),
        };

        // saturating conversion defines non-finite and out-of-range values
        let integer = if mode == ConvertMode::Saturate {
            if rounded.is_nan() {
                0
            } else if rounded <= minimum as f64 {
                minimum
            } else if rounded >= maximum as f64 {
                maximum
            } else {
                rounded as i128
            }
        } else if !rounded.is_finite() || rounded < minimum as f64 || rounded > maximum as f64 {
            return Err(Error::trap(Trap::IntegerOverflow));
        } else {
            rounded as i128
        };

        Ok(Word::from_bits(target.encode(integer as u64)))
    }

    /// Convert one floating-point value into another representation.
    fn convert_float(
        value: Word,
        source: Scalar,
        target: Scalar,
        mode: ConvertMode,
    ) -> Result<Word> {
        let value = source
            .float(value.bits())
            .ok_or_else(Error::invalid_instruction)?;
        let bits = target
            .float_bits(value)
            .ok_or_else(Error::invalid_instruction)?;
        let converted = target.float(bits).ok_or_else(Error::invalid_instruction)?;

        // exact narrowing rejects any representational change
        if mode == ConvertMode::Exact && converted != value {
            return Err(Error::trap(Trap::InvalidArithmetic));
        }

        Ok(Word::from_bits(target.encode(bits)))
    }
}

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Truncate one integer into a narrower representation.
    fn truncate_cast(&self, value: Word, target: ValueType) -> Result<Word> {
        let scalar = target
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;

        Ok(Word::from_bits(scalar.encode(value.bits())))
    }

    /// Saturate one integer into its target representation.
    fn saturate_cast(&self, value: Word, source: ValueType, target: ValueType) -> Result<Word> {
        let source_scalar = source
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let target_scalar = target
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let value = source_scalar
            .integer(value.bits())
            .ok_or_else(|| self.invalid_instruction())?;
        let (minimum, maximum) = target_scalar
            .integer_bounds()
            .ok_or_else(|| self.invalid_instruction())?;
        let value = value.clamp(minimum, maximum);

        Ok(Word::from_bits(target_scalar.encode(value as u64)))
    }

    /// Extend one integer using its source sign bit.
    fn sign_extend(&self, value: Word, source: ValueType, target: ValueType) -> Result<Word> {
        let source = source
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let target = target
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let value = Word::int(value.bits() as i64, source.bit_width());

        Ok(Word::from_bits(target.encode(value.bits())))
    }

    /// Extend one integer with zero high bits.
    fn zero_extend(&self, value: Word, source: ValueType, target: ValueType) -> Result<Word> {
        let source = source
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let target = target
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let value = Word::uint(value.bits(), source.bit_width());

        Ok(Word::from_bits(target.encode(value.bits())))
    }

    /// Convert one floating-point value into an integer.
    fn float_to_int(
        &self,
        value: Word,
        source: ValueType,
        target: ValueType,
        is_saturating: bool,
    ) -> Result<Word> {
        let source = source
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let target = target
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let value = source
            .float(value.bits())
            .ok_or_else(|| self.invalid_instruction())?;
        let (minimum, maximum) = target
            .integer_bounds()
            .ok_or_else(|| self.invalid_instruction())?;
        let lower = minimum as f64;
        let upper = maximum as f64 + 1.0;

        // trap non-saturating conversions outside the exact target domain
        if !is_saturating && (!value.is_finite() || value < lower || value >= upper) {
            return Err(Error::trap(Trap::IntegerOverflow));
        }

        let value = if value.is_nan() {
            0
        } else if value <= lower {
            minimum
        } else if value >= upper {
            maximum
        } else {
            value.trunc() as i128
        };

        Ok(Word::from_bits(target.encode(value as u64)))
    }

    /// Convert one integer into a floating-point representation.
    fn int_to_float(&self, value: Word, source: ValueType, target: ValueType) -> Result<Word> {
        let source = source
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let target = target
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let value = source
            .integer(value.bits())
            .ok_or_else(|| self.invalid_instruction())? as f64;
        let bits = target
            .float_bits(value)
            .ok_or_else(|| self.invalid_instruction())?;

        Ok(Word::from_bits(target.encode(bits)))
    }

    /// Convert one floating-point value into another representation.
    fn float_convert(&self, value: Word, source: ValueType, target: ValueType) -> Result<Word> {
        let source = source
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let target = target
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;
        let value = source
            .float(value.bits())
            .ok_or_else(|| self.invalid_instruction())?;
        let bits = target
            .float_bits(value)
            .ok_or_else(|| self.invalid_instruction())?;

        Ok(Word::from_bits(target.encode(bits)))
    }

    /// Preserve one scalar's low bits under an equal-width interpretation.
    fn bit_cast(&self, value: Word, target: ValueType) -> Result<Word> {
        let target = target
            .scalar_type()
            .ok_or_else(|| self.invalid_instruction())?;

        Ok(Word::from_bits(target.encode(value.bits())))
    }
}
