use destack_mir as mir;

use crate::Word;
use crate::diagnostic::Error;

use super::access;

/// Execute a binary opcode.
#[inline(always)]
pub(crate) fn execute_binary(op: mir::BinaryOperator, lhs: Word, rhs: Word) -> Result<Word, Error> {
    Err(Error::TypeMismatch {
        expected: format!("typed binary opcode for {op:?}"),
        actual: format!("{lhs:?}, {rhs:?}"),
    })
}

/// Execute a signed integer binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_int(
    op: mir::BinaryOperator,
    lhs: Word,
    rhs: Word,
) -> Result<Word, Error> {
    use mir::BinaryOperator::*;

    let a = lhs.as_i64();
    let b = rhs.as_i64();
    let result = match op {
        Add => Word::int(a.wrapping_add(b), Word::BIT_LEN),
        Subtract => Word::int(a.wrapping_sub(b), Word::BIT_LEN),
        Multiply => Word::int(a.wrapping_mul(b), Word::BIT_LEN),
        SignedDivide => {
            if b == 0 {
                return Err(Error::DivisionByZero);
            }
            Word::int(a.wrapping_div(b), Word::BIT_LEN)
        }
        SignedRemainder => {
            if b == 0 {
                return Err(Error::DivisionByZero);
            }
            Word::int(a.wrapping_rem(b), Word::BIT_LEN)
        }
        Equal => Word::bool(a == b),
        NotEqual => Word::bool(a != b),
        SignedLessThan => Word::bool(a < b),
        SignedLessEqual => Word::bool(a <= b),
        SignedGreaterThan => Word::bool(a > b),
        SignedGreaterEqual => Word::bool(a >= b),
        And => Word::int(a & b, Word::BIT_LEN),
        Or => Word::int(a | b, Word::BIT_LEN),
        Xor => Word::int(a ^ b, Word::BIT_LEN),
        ShiftLeft => Word::int(a.wrapping_shl(b as u32), Word::BIT_LEN),
        ArithmeticShiftRight => Word::int(a.wrapping_shr(b as u32), Word::BIT_LEN),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("signed integer opcode for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    Ok(result)
}

/// Execute an unsigned integer binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_uint(
    op: mir::BinaryOperator,
    lhs: Word,
    rhs: Word,
) -> Result<Word, Error> {
    use mir::BinaryOperator::*;

    let a = lhs.as_u64();
    let b = rhs.as_u64();
    let result = match op {
        Add => Word::uint(a.wrapping_add(b), Word::BIT_LEN),
        Subtract => Word::uint(a.wrapping_sub(b), Word::BIT_LEN),
        Multiply => Word::uint(a.wrapping_mul(b), Word::BIT_LEN),
        UnsignedDivide => {
            if b == 0 {
                return Err(Error::DivisionByZero);
            }
            Word::uint(a.wrapping_div(b), Word::BIT_LEN)
        }
        UnsignedRemainder => {
            if b == 0 {
                return Err(Error::DivisionByZero);
            }
            Word::uint(a.wrapping_rem(b), Word::BIT_LEN)
        }
        UnsignedLessThan => Word::bool(a < b),
        UnsignedLessEqual => Word::bool(a <= b),
        UnsignedGreaterThan => Word::bool(a > b),
        UnsignedGreaterEqual => Word::bool(a >= b),
        And => Word::uint(a & b, Word::BIT_LEN),
        Or => Word::uint(a | b, Word::BIT_LEN),
        Xor => Word::uint(a ^ b, Word::BIT_LEN),
        ShiftLeft => Word::uint(a.wrapping_shl(b as u32), Word::BIT_LEN),
        LogicalShiftRight => Word::uint(a.wrapping_shr(b as u32), Word::BIT_LEN),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("unsigned integer opcode for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    Ok(result)
}

/// Execute a float32 binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_float32(
    op: mir::BinaryOperator,
    lhs: Word,
    rhs: Word,
) -> Result<Word, Error> {
    use mir::BinaryOperator::*;

    let a = lhs.as_f32();
    let b = rhs.as_f32();
    let result = match op {
        FloatAdd => Word::float32(a + b),
        FloatSubtract => Word::float32(a - b),
        FloatMultiply => Word::float32(a * b),
        FloatDivide => Word::float32(a / b),
        FloatEqual => Word::bool(a == b),
        FloatNotEqual => Word::bool(a != b),
        FloatLessThan => Word::bool(a < b),
        FloatLessEqual => Word::bool(a <= b),
        FloatGreaterThan => Word::bool(a > b),
        FloatGreaterEqual => Word::bool(a >= b),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("float32 opcode for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    Ok(result)
}

/// Execute a float64 binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_float64(
    op: mir::BinaryOperator,
    lhs: Word,
    rhs: Word,
) -> Result<Word, Error> {
    use mir::BinaryOperator::*;

    let a = lhs.as_f64();
    let b = rhs.as_f64();
    let result = match op {
        FloatAdd => Word::float64(a + b),
        FloatSubtract => Word::float64(a - b),
        FloatMultiply => Word::float64(a * b),
        FloatDivide => Word::float64(a / b),
        FloatEqual => Word::bool(a == b),
        FloatNotEqual => Word::bool(a != b),
        FloatLessThan => Word::bool(a < b),
        FloatLessEqual => Word::bool(a <= b),
        FloatGreaterThan => Word::bool(a > b),
        FloatGreaterEqual => Word::bool(a >= b),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("float64 opcode for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    Ok(result)
}

/// Execute a boolean binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_bool(
    op: mir::BinaryOperator,
    lhs: Word,
    rhs: Word,
) -> Result<Word, Error> {
    use mir::BinaryOperator::*;

    let a = lhs.as_bool();
    let b = rhs.as_bool();
    let result = match op {
        And => Word::bool(a && b),
        Or => Word::bool(a || b),
        Xor => Word::bool(a ^ b),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("boolean opcode for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    Ok(result)
}

/// Execute a unary opcode.
#[inline(always)]
pub(crate) fn execute_unary(op: mir::UnaryOperator, arg: Word) -> Result<Word, Error> {
    Err(Error::TypeMismatch {
        expected: format!("typed unary opcode for {op:?}"),
        actual: format!("{arg:?}"),
    })
}

/// Execute a signed integer unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_int(op: mir::UnaryOperator, arg: Word) -> Result<Word, Error> {
    let value = arg.as_i64();
    let result = match op {
        mir::UnaryOperator::Negate => Word::int(value.wrapping_neg(), Word::BIT_LEN),
        mir::UnaryOperator::Not => Word::int(!value, Word::BIT_LEN),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("signed integer opcode for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    Ok(result)
}

/// Execute an unsigned integer unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_uint(op: mir::UnaryOperator, arg: Word) -> Result<Word, Error> {
    let value = arg.as_u64();
    let result = match op {
        mir::UnaryOperator::Not => Word::uint(!value, Word::BIT_LEN),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("unsigned integer opcode for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    Ok(result)
}

/// Execute a float32 unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_float32(op: mir::UnaryOperator, arg: Word) -> Result<Word, Error> {
    let value = arg.as_f32();
    let result = match op {
        mir::UnaryOperator::FloatNegate => Word::float32(-value),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("float32 opcode for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    Ok(result)
}

/// Execute a float64 unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_float64(op: mir::UnaryOperator, arg: Word) -> Result<Word, Error> {
    let value = arg.as_f64();
    let result = match op {
        mir::UnaryOperator::FloatNegate => Word::float64(-value),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("float64 opcode for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    Ok(result)
}

/// Execute a boolean unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_bool(op: mir::UnaryOperator, arg: Word) -> Result<Word, Error> {
    let value = arg.as_bool();
    let result = match op {
        mir::UnaryOperator::Not => Word::bool(!value),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("boolean opcode for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    Ok(result)
}

/// Execute a cast opcode.
#[inline(always)]
pub(crate) fn execute_cast(
    tree: &mir::Tree,
    operator: mir::CastOperator,
    argument: Word,
    to_type: mir::LocalNodeId<mir::Type>,
) -> Result<Word, Error> {
    let pointer_width_bits = usize::BITS as u16;
    let target_type = tree.get(to_type);

    let result = match operator {
        mir::CastOperator::Bitcast => argument,
        mir::CastOperator::Truncate => {
            let Some((width, is_signed)) =
                target_type.int_info_with_pointer_width(pointer_width_bits)
            else {
                return Ok(argument);
            };
            if is_signed {
                Word::int(truncate_signed(argument.as_i64(), width as u8), width as u8)
            } else {
                Word::uint(
                    truncate_unsigned(argument.as_u64(), width as u8),
                    width as u8,
                )
            }
        }
        mir::CastOperator::ZeroExtend => {
            let Some((width, _)) = target_type.int_info_with_pointer_width(pointer_width_bits)
            else {
                return Ok(argument);
            };
            Word::uint(argument.as_u64(), width as u8)
        }
        mir::CastOperator::SignExtend => {
            let Some((width, _)) = target_type.int_info_with_pointer_width(pointer_width_bits)
            else {
                return Ok(argument);
            };
            Word::int(argument.as_i64(), width as u8)
        }
        mir::CastOperator::FloatToSignedInt => {
            let target_width = integer_target_width(target_type, pointer_width_bits)?;
            let (min_bound, max_bound) =
                integer_bounds(target_width, true).ok_or(Error::InvalidCast)?;
            let converted = float_to_int_checked(argument.as_f64(), min_bound, max_bound)
                .ok_or(Error::BadConversionToInteger)?;
            Word::int(converted as i64, target_width)
        }
        mir::CastOperator::FloatToUnsignedInt => {
            let target_width = integer_target_width(target_type, pointer_width_bits)?;
            let (min_bound, max_bound) =
                integer_bounds(target_width, false).ok_or(Error::InvalidCast)?;
            let converted = float_to_int_checked(argument.as_f64(), min_bound, max_bound)
                .ok_or(Error::BadConversionToInteger)?;
            Word::uint(converted as u64, target_width)
        }
        mir::CastOperator::FloatToSignedIntSaturating => {
            let target_width = integer_target_width(target_type, pointer_width_bits)?;
            let (min_bound, max_bound) =
                integer_bounds(target_width, true).ok_or(Error::InvalidCast)?;
            let converted = float_to_int_saturating(argument.as_f64(), min_bound, max_bound);
            Word::int(converted as i64, target_width)
        }
        mir::CastOperator::FloatToUnsignedIntSaturating => {
            let target_width = integer_target_width(target_type, pointer_width_bits)?;
            let (min_bound, max_bound) =
                integer_bounds(target_width, false).ok_or(Error::InvalidCast)?;
            let converted = float_to_int_saturating(argument.as_f64(), min_bound, max_bound);
            Word::uint(converted as u64, target_width)
        }
        mir::CastOperator::SignedIntToFloat => {
            if target_float_width(target_type)? == 32 {
                Word::float32(argument.as_i64() as f32)
            } else {
                Word::float64(argument.as_i64() as f64)
            }
        }
        mir::CastOperator::UnsignedIntToFloat => {
            if target_float_width(target_type)? == 32 {
                Word::float32(argument.as_u64() as f32)
            } else {
                Word::float64(argument.as_u64() as f64)
            }
        }
        mir::CastOperator::FloatExtend => Word::float64(argument.as_f32() as f64),
        mir::CastOperator::FloatTruncate => Word::float32(argument.as_f64() as f32),
        mir::CastOperator::PointerToInt => {
            let target_width = integer_target_width(target_type, pointer_width_bits)?;
            Word::uint(argument.as_u64(), target_width)
        }
        mir::CastOperator::IntToPointer => cast_integer_to_pointer(argument.as_u64(), target_type)?,
    };

    Ok(result)
}

/// Cast one integer bit pattern into the requested pointer-shaped target type.
fn cast_integer_to_pointer(raw: u64, target_type: &mir::Type) -> Result<Word, Error> {
    access::decode_pointer_bits(raw, target_type)
}

/// Return the integer target width for one type.
fn integer_target_width(target_type: &mir::Type, pointer_width_bits: u16) -> Result<u8, Error> {
    target_type
        .int_info_with_pointer_width(pointer_width_bits)
        .map(|(width, _)| width as u8)
        .ok_or(Error::InvalidCast)
}

/// Return the float target width for one type.
fn target_float_width(target_type: &mir::Type) -> Result<u16, Error> {
    match target_type {
        mir::Type::Float { width } => Ok(*width),
        _ => Err(Error::InvalidCast),
    }
}

/// Truncate a signed integer to a target bit width.
fn truncate_signed(value: i64, width: u8) -> i64 {
    if width >= Word::BIT_LEN {
        return value;
    }

    let mask = (1u64 << width) - 1;
    let masked = (value as u64) & mask;
    let sign_bit = 1u64 << (width - 1);

    if masked & sign_bit != 0 {
        (masked | !mask) as i64
    } else {
        masked as i64
    }
}

/// Truncate an unsigned integer to a target bit width.
fn truncate_unsigned(value: u64, width: u8) -> u64 {
    if width >= Word::BIT_LEN {
        return value;
    }

    let mask = (1u64 << width) - 1;

    value & mask
}

/// Compute integer bounds for a width and signedness.
fn integer_bounds(width: u8, is_signed: bool) -> Option<(i128, i128)> {
    if width == 0 || width > Word::BIT_LEN {
        return None;
    }

    if is_signed {
        let shift = (width - 1) as u32;
        let min = -(1_i128 << shift);
        let max = (1_i128 << shift) - 1;

        Some((min, max))
    } else {
        let shift = width as u32;
        let max = (1_i128 << shift) - 1;

        Some((0, max))
    }
}

/// Convert a float to an integer when the conversion is in range and finite.
fn float_to_int_checked(value: f64, min_bound: i128, max_bound: i128) -> Option<i128> {
    if !value.is_finite() {
        return None;
    }

    let min_float = min_bound as f64;
    let max_float = max_bound as f64;
    let max_rounded = max_float.trunc() as i128;
    let max_is_rounded_up = max_rounded > max_bound;

    let min_ok = value >= min_float;
    let max_ok = if max_is_rounded_up {
        value < max_float
    } else {
        value <= max_float
    };
    if !min_ok || !max_ok {
        return None;
    }

    let truncated = value.trunc() as i128;
    if truncated < min_bound || truncated > max_bound {
        return None;
    }

    Some(truncated)
}

/// Convert a float to an integer using saturating semantics.
fn float_to_int_saturating(value: f64, min_bound: i128, max_bound: i128) -> i128 {
    if value.is_nan() {
        return 0;
    }

    if !value.is_finite() {
        return if value.is_sign_negative() {
            min_bound
        } else {
            max_bound
        };
    }

    let min_float = min_bound as f64;
    let max_float = max_bound as f64;
    let max_rounded = max_float.trunc() as i128;
    let max_is_rounded_up = max_rounded > max_bound;
    if value <= min_float {
        return min_bound;
    }

    if max_is_rounded_up {
        if value >= max_float {
            return max_bound;
        }
    } else if value >= max_float {
        return max_bound;
    }

    let truncated = value.trunc() as i128;
    if truncated < min_bound {
        return min_bound;
    }

    if truncated > max_bound {
        return max_bound;
    }

    truncated
}
