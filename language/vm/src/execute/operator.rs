use destack_mir as mir;

use crate::diagnostic::Error;
use crate::{Value, ValueTag};

use super::access;

/// Execute a binary opcode.
#[inline(always)]
pub(crate) fn execute_binary(
    op: mir::BinaryOperator,
    lhs: Value,
    rhs: Value,
) -> Result<Value, Error> {
    use mir::BinaryOperator::*;

    // capture operand tags
    let lhs_tag = lhs.tag();
    let rhs_tag = rhs.tag();

    // compute result by operand types
    let result = match (lhs_tag, rhs_tag) {
        // signed integer
        (ValueTag::Int, ValueTag::Int) => {
            // decode values
            let a = lhs.raw_data() as i64;
            let b = rhs.raw_data() as i64;
            let width = lhs.width();

            // apply operator
            match op {
                Add => Value::int(a.wrapping_add(b), width),
                Subtract => Value::int(a.wrapping_sub(b), width),
                Multiply => Value::int(a.wrapping_mul(b), width),
                SignedDivide => {
                    if b == 0 {
                        return Err(Error::DivisionByZero);
                    }
                    Value::int(a.wrapping_div(b), width)
                }
                SignedRemainder => {
                    if b == 0 {
                        return Err(Error::DivisionByZero);
                    }
                    Value::int(a.wrapping_rem(b), width)
                }
                Equal => Value::bool(a == b),
                NotEqual => Value::bool(a != b),
                SignedLessThan => Value::bool(a < b),
                SignedLessEqual => Value::bool(a <= b),
                SignedGreaterThan => Value::bool(a > b),
                SignedGreaterEqual => Value::bool(a >= b),
                And => Value::int(a & b, width),
                Or => Value::int(a | b, width),
                Xor => Value::int(a ^ b, width),
                ShiftLeft => Value::int(a.wrapping_shl(b as u32), width),
                ArithmeticShiftRight => Value::int(a.wrapping_shr(b as u32), width),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: format!("compatible types for {op:?}"),
                        actual: format!("{lhs:?}, {rhs:?}"),
                    });
                }
            }
        }

        // unsigned integer
        (ValueTag::UInt, ValueTag::UInt) => {
            // decode values
            let a = lhs.raw_data();
            let b = rhs.raw_data();
            let width = lhs.width();

            // apply operator
            match op {
                Add => Value::uint(a.wrapping_add(b), width),
                Subtract => Value::uint(a.wrapping_sub(b), width),
                Multiply => Value::uint(a.wrapping_mul(b), width),
                UnsignedDivide => {
                    if b == 0 {
                        return Err(Error::DivisionByZero);
                    }
                    Value::uint(a.wrapping_div(b), width)
                }
                UnsignedRemainder => {
                    if b == 0 {
                        return Err(Error::DivisionByZero);
                    }
                    Value::uint(a.wrapping_rem(b), width)
                }
                UnsignedLessThan => Value::bool(a < b),
                UnsignedLessEqual => Value::bool(a <= b),
                UnsignedGreaterThan => Value::bool(a > b),
                UnsignedGreaterEqual => Value::bool(a >= b),
                And => Value::uint(a & b, width),
                Or => Value::uint(a | b, width),
                Xor => Value::uint(a ^ b, width),
                ShiftLeft => Value::uint(a.wrapping_shl(b as u32), width),
                LogicalShiftRight => Value::uint(a.wrapping_shr(b as u32), width),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: format!("compatible types for {op:?}"),
                        actual: format!("{lhs:?}, {rhs:?}"),
                    });
                }
            }
        }

        // f64
        (ValueTag::Float64, ValueTag::Float64) => {
            // decode values
            let a = f64::from_bits(lhs.raw_data());
            let b = f64::from_bits(rhs.raw_data());

            // apply operator
            match op {
                FloatAdd => Value::float64(a + b),
                FloatSubtract => Value::float64(a - b),
                FloatMultiply => Value::float64(a * b),
                FloatDivide => Value::float64(a / b),
                FloatEqual => Value::bool(a == b),
                FloatNotEqual => Value::bool(a != b),
                FloatLessThan => Value::bool(a < b),
                FloatLessEqual => Value::bool(a <= b),
                FloatGreaterThan => Value::bool(a > b),
                FloatGreaterEqual => Value::bool(a >= b),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: format!("compatible types for {op:?}"),
                        actual: format!("{lhs:?}, {rhs:?}"),
                    });
                }
            }
        }

        // f32
        (ValueTag::Float32, ValueTag::Float32) => {
            // decode values
            let a = f32::from_bits(lhs.raw_data() as u32);
            let b = f32::from_bits(rhs.raw_data() as u32);

            // apply operator
            match op {
                FloatAdd => Value::float32(a + b),
                FloatSubtract => Value::float32(a - b),
                FloatMultiply => Value::float32(a * b),
                FloatDivide => Value::float32(a / b),
                FloatEqual => Value::bool(a == b),
                FloatNotEqual => Value::bool(a != b),
                FloatLessThan => Value::bool(a < b),
                FloatLessEqual => Value::bool(a <= b),
                FloatGreaterThan => Value::bool(a > b),
                FloatGreaterEqual => Value::bool(a >= b),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: format!("compatible types for {op:?}"),
                        actual: format!("{lhs:?}, {rhs:?}"),
                    });
                }
            }
        }

        // boolean
        (ValueTag::Bool, ValueTag::Bool) => {
            // decode values
            let a = lhs.raw_data() != 0;
            let b = rhs.raw_data() != 0;

            // apply operator
            match op {
                And => Value::bool(a && b),
                Or => Value::bool(a || b),
                Xor => Value::bool(a ^ b),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: format!("compatible types for {op:?}"),
                        actual: format!("{lhs:?}, {rhs:?}"),
                    });
                }
            }
        }

        // incompatible
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible types for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute a signed integer binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_int(
    op: mir::BinaryOperator,
    lhs: Value,
    rhs: Value,
) -> Result<Value, Error> {
    use mir::BinaryOperator::*;

    // decode values
    let a = lhs.raw_data() as i64;
    let b = rhs.raw_data() as i64;
    let width = lhs.width();

    // apply operator
    let result = match op {
        Add => Value::int(a.wrapping_add(b), width),
        Subtract => Value::int(a.wrapping_sub(b), width),
        Multiply => Value::int(a.wrapping_mul(b), width),
        SignedDivide => {
            if b == 0 {
                return Err(Error::DivisionByZero);
            }
            Value::int(a.wrapping_div(b), width)
        }
        SignedRemainder => {
            if b == 0 {
                return Err(Error::DivisionByZero);
            }
            Value::int(a.wrapping_rem(b), width)
        }
        Equal => Value::bool(a == b),
        NotEqual => Value::bool(a != b),
        SignedLessThan => Value::bool(a < b),
        SignedLessEqual => Value::bool(a <= b),
        SignedGreaterThan => Value::bool(a > b),
        SignedGreaterEqual => Value::bool(a >= b),
        And => Value::int(a & b, width),
        Or => Value::int(a | b, width),
        Xor => Value::int(a ^ b, width),
        ShiftLeft => Value::int(a.wrapping_shl(b as u32), width),
        ArithmeticShiftRight => Value::int(a.wrapping_shr(b as u32), width),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible types for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute an unsigned integer binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_uint(
    op: mir::BinaryOperator,
    lhs: Value,
    rhs: Value,
) -> Result<Value, Error> {
    use mir::BinaryOperator::*;

    // decode values
    let a = lhs.raw_data();
    let b = rhs.raw_data();
    let width = lhs.width();

    // apply operator
    let result = match op {
        Add => Value::uint(a.wrapping_add(b), width),
        Subtract => Value::uint(a.wrapping_sub(b), width),
        Multiply => Value::uint(a.wrapping_mul(b), width),
        UnsignedDivide => {
            if b == 0 {
                return Err(Error::DivisionByZero);
            }
            Value::uint(a.wrapping_div(b), width)
        }
        UnsignedRemainder => {
            if b == 0 {
                return Err(Error::DivisionByZero);
            }
            Value::uint(a.wrapping_rem(b), width)
        }
        UnsignedLessThan => Value::bool(a < b),
        UnsignedLessEqual => Value::bool(a <= b),
        UnsignedGreaterThan => Value::bool(a > b),
        UnsignedGreaterEqual => Value::bool(a >= b),
        And => Value::uint(a & b, width),
        Or => Value::uint(a | b, width),
        Xor => Value::uint(a ^ b, width),
        ShiftLeft => Value::uint(a.wrapping_shl(b as u32), width),
        LogicalShiftRight => Value::uint(a.wrapping_shr(b as u32), width),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible types for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute a float32 binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_float32(
    op: mir::BinaryOperator,
    lhs: Value,
    rhs: Value,
) -> Result<Value, Error> {
    use mir::BinaryOperator::*;

    // decode values
    let a = f32::from_bits(lhs.raw_data() as u32);
    let b = f32::from_bits(rhs.raw_data() as u32);

    // apply operator
    let result = match op {
        FloatAdd => Value::float32(a + b),
        FloatSubtract => Value::float32(a - b),
        FloatMultiply => Value::float32(a * b),
        FloatDivide => Value::float32(a / b),
        FloatEqual => Value::bool(a == b),
        FloatNotEqual => Value::bool(a != b),
        FloatLessThan => Value::bool(a < b),
        FloatLessEqual => Value::bool(a <= b),
        FloatGreaterThan => Value::bool(a > b),
        FloatGreaterEqual => Value::bool(a >= b),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible types for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute a float64 binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_float64(
    op: mir::BinaryOperator,
    lhs: Value,
    rhs: Value,
) -> Result<Value, Error> {
    use mir::BinaryOperator::*;

    // decode values
    let a = f64::from_bits(lhs.raw_data());
    let b = f64::from_bits(rhs.raw_data());

    // apply operator
    let result = match op {
        FloatAdd => Value::float64(a + b),
        FloatSubtract => Value::float64(a - b),
        FloatMultiply => Value::float64(a * b),
        FloatDivide => Value::float64(a / b),
        FloatEqual => Value::bool(a == b),
        FloatNotEqual => Value::bool(a != b),
        FloatLessThan => Value::bool(a < b),
        FloatLessEqual => Value::bool(a <= b),
        FloatGreaterThan => Value::bool(a > b),
        FloatGreaterEqual => Value::bool(a >= b),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible types for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute a boolean binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_bool(
    op: mir::BinaryOperator,
    lhs: Value,
    rhs: Value,
) -> Result<Value, Error> {
    use mir::BinaryOperator::*;

    // decode values
    let a = lhs.raw_data() != 0;
    let b = rhs.raw_data() != 0;

    // apply operator
    let result = match op {
        And => Value::bool(a && b),
        Or => Value::bool(a || b),
        Xor => Value::bool(a ^ b),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible types for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute a unary opcode.
#[inline(always)]
pub(crate) fn execute_unary(op: mir::UnaryOperator, arg: Value) -> Result<Value, Error> {
    use mir::UnaryOperator::*;

    // compute result by operator
    let result = match (op, arg.tag()) {
        (Negate, ValueTag::Int) => {
            let value = arg.raw_data() as i64;
            Value::int(value.wrapping_neg(), arg.width())
        }
        (FloatNegate, ValueTag::Float64) => {
            let f = f64::from_bits(arg.raw_data());
            Value::float64(-f)
        }
        (FloatNegate, ValueTag::Float32) => {
            let f = f32::from_bits(arg.raw_data() as u32);
            Value::float32(-f)
        }
        (Not, ValueTag::Bool) => {
            let b = arg.raw_data() != 0;
            Value::bool(!b)
        }
        (Not, ValueTag::Int) => {
            let value = arg.raw_data() as i64;
            Value::int(!value, arg.width())
        }
        (Not, ValueTag::UInt) => {
            let value = arg.raw_data();
            Value::uint(!value, arg.width())
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible type for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute a signed integer unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_int(op: mir::UnaryOperator, arg: Value) -> Result<Value, Error> {
    use mir::UnaryOperator::*;

    // decode operand
    let value = arg.raw_data() as i64;
    let width = arg.width();

    // apply operator
    let result = match op {
        Negate => Value::int(value.wrapping_neg(), width),
        Not => Value::int(!value, width),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible type for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute an unsigned integer unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_uint(op: mir::UnaryOperator, arg: Value) -> Result<Value, Error> {
    use mir::UnaryOperator::*;

    // decode operand
    let value = arg.raw_data();
    let width = arg.width();

    // apply operator
    let result = match op {
        Not => Value::uint(!value, width),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible type for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute a float32 unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_float32(op: mir::UnaryOperator, arg: Value) -> Result<Value, Error> {
    use mir::UnaryOperator::*;

    // decode operand
    let value = f32::from_bits(arg.raw_data() as u32);

    // apply operator
    let result = match op {
        FloatNegate => Value::float32(-value),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible type for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute a float64 unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_float64(op: mir::UnaryOperator, arg: Value) -> Result<Value, Error> {
    use mir::UnaryOperator::*;

    // decode operand
    let value = f64::from_bits(arg.raw_data());

    // apply operator
    let result = match op {
        FloatNegate => Value::float64(-value),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible type for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute a boolean unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_bool(op: mir::UnaryOperator, arg: Value) -> Result<Value, Error> {
    use mir::UnaryOperator::*;

    // decode operand
    let value = arg.raw_data() != 0;

    // apply operator
    let result = match op {
        Not => Value::bool(!value),
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible type for {op:?}"),
                actual: format!("{arg:?}"),
            });
        }
    };

    // return result
    Ok(result)
}

/// Execute a cast opcode.
#[inline(always)]
pub(crate) fn execute_cast(
    tree: &mir::NodeTree,
    operator: mir::CastOperator,
    argument: Value,
    to_type: mir::LocalNodeId<mir::Type>,
) -> Result<Value, Error> {
    let pointer_width_bits = usize::BITS as u16;

    // load target type
    let target_type = tree.get(to_type);

    // apply cast operator
    let result = match operator {
        mir::CastOperator::Bitcast => argument,

        mir::CastOperator::Truncate => {
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width as u8,
                None => return Ok(argument),
            };
            match argument.tag() {
                ValueTag::Int => {
                    let value = argument.raw_data() as i64;
                    Value::int(truncate_signed(value, target_width), target_width)
                }
                ValueTag::UInt => {
                    let value = argument.raw_data();
                    Value::uint(truncate_unsigned(value, target_width), target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::ZeroExtend => {
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width as u8,
                None => return Ok(argument),
            };
            match argument.tag() {
                ValueTag::UInt => Value::uint(argument.raw_data(), target_width),
                ValueTag::Int => {
                    let value = argument.raw_data() as i64;
                    let width = argument.width();
                    let masked = truncate_unsigned(value as u64, width);
                    Value::uint(masked, target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::SignExtend => {
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width as u8,
                None => return Ok(argument),
            };
            match argument.tag() {
                ValueTag::Int => {
                    let value = argument.raw_data() as i64;
                    let width = argument.width();
                    let extended = sign_extend(value, width, target_width);
                    Value::int(extended, target_width)
                }
                ValueTag::UInt => {
                    let value = argument.raw_data();
                    let width = argument.width();
                    let as_signed = truncate_signed(value as i64, width);
                    let extended = sign_extend(as_signed, width, target_width);
                    Value::int(extended, target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::FloatToSignedInt => {
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width as u8,
                None => 64,
            };
            match argument.tag() {
                ValueTag::Float64 => {
                    let f = f64::from_bits(argument.raw_data());
                    let (min_bound, max_bound) =
                        integer_bounds(target_width, true).ok_or(Error::InvalidCast)?;
                    let converted = float_to_int_checked(f, min_bound, max_bound)
                        .ok_or(Error::BadConversionToInteger)?;
                    Value::int(converted as i64, target_width)
                }
                ValueTag::Float32 => {
                    let f = f32::from_bits(argument.raw_data() as u32);
                    let (min_bound, max_bound) =
                        integer_bounds(target_width, true).ok_or(Error::InvalidCast)?;
                    let converted = float_to_int_checked(f as f64, min_bound, max_bound)
                        .ok_or(Error::BadConversionToInteger)?;
                    Value::int(converted as i64, target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::FloatToUnsignedInt => {
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width as u8,
                None => 64,
            };
            match argument.tag() {
                ValueTag::Float64 => {
                    let f = f64::from_bits(argument.raw_data());
                    let (min_bound, max_bound) =
                        integer_bounds(target_width, false).ok_or(Error::InvalidCast)?;
                    let converted = float_to_int_checked(f, min_bound, max_bound)
                        .ok_or(Error::BadConversionToInteger)?;
                    Value::uint(converted as u64, target_width)
                }
                ValueTag::Float32 => {
                    let f = f32::from_bits(argument.raw_data() as u32);
                    let (min_bound, max_bound) =
                        integer_bounds(target_width, false).ok_or(Error::InvalidCast)?;
                    let converted = float_to_int_checked(f as f64, min_bound, max_bound)
                        .ok_or(Error::BadConversionToInteger)?;
                    Value::uint(converted as u64, target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::FloatToSignedIntSaturating => {
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width as u8,
                None => 64,
            };
            match argument.tag() {
                ValueTag::Float64 => {
                    let f = f64::from_bits(argument.raw_data());
                    let (min_bound, max_bound) =
                        integer_bounds(target_width, true).ok_or(Error::InvalidCast)?;
                    let converted = float_to_int_saturating(f, min_bound, max_bound);
                    Value::int(converted as i64, target_width)
                }
                ValueTag::Float32 => {
                    let f = f32::from_bits(argument.raw_data() as u32);
                    let (min_bound, max_bound) =
                        integer_bounds(target_width, true).ok_or(Error::InvalidCast)?;
                    let converted = float_to_int_saturating(f as f64, min_bound, max_bound);
                    Value::int(converted as i64, target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::FloatToUnsignedIntSaturating => {
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width as u8,
                None => 64,
            };
            match argument.tag() {
                ValueTag::Float64 => {
                    let f = f64::from_bits(argument.raw_data());
                    let (min_bound, max_bound) =
                        integer_bounds(target_width, false).ok_or(Error::InvalidCast)?;
                    let converted = float_to_int_saturating(f, min_bound, max_bound);
                    Value::uint(converted as u64, target_width)
                }
                ValueTag::Float32 => {
                    let f = f32::from_bits(argument.raw_data() as u32);
                    let (min_bound, max_bound) =
                        integer_bounds(target_width, false).ok_or(Error::InvalidCast)?;
                    let converted = float_to_int_saturating(f as f64, min_bound, max_bound);
                    Value::uint(converted as u64, target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::SignedIntToFloat => {
            let target_width = match target_type {
                mir::Type::Float { width } => *width,
                _ => 64,
            };
            match argument.tag() {
                ValueTag::Int => {
                    let value = argument.raw_data() as i64;
                    // narrow to f32 when requested
                    if target_width == 32 {
                        Value::float32(value as f32)
                    }
                    // otherwise use f64
                    else {
                        Value::float64(value as f64)
                    }
                }
                _ => argument,
            }
        }

        mir::CastOperator::UnsignedIntToFloat => {
            let target_width = match target_type {
                mir::Type::Float { width } => *width,
                _ => 64,
            };
            match argument.tag() {
                ValueTag::UInt => {
                    let value = argument.raw_data();
                    // narrow to f32 when requested
                    if target_width == 32 {
                        Value::float32(value as f32)
                    }
                    // otherwise use f64
                    else {
                        Value::float64(value as f64)
                    }
                }
                _ => argument,
            }
        }

        mir::CastOperator::FloatExtend => {
            // extend float32 to float64
            if argument.tag() == ValueTag::Float32 {
                let f = f32::from_bits(argument.raw_data() as u32);
                Value::float64(f as f64)
            }
            // otherwise keep original
            else {
                argument
            }
        }

        mir::CastOperator::FloatTruncate => {
            // truncate float64 to float32
            if argument.tag() == ValueTag::Float64 {
                let f = f64::from_bits(argument.raw_data());
                Value::float32(f as f32)
            }
            // otherwise keep original
            else {
                argument
            }
        }

        mir::CastOperator::PointerToInt => {
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width as u8,
                None => 64,
            };
            match argument.tag() {
                ValueTag::RawPointer
                | ValueTag::HeapReference
                | ValueTag::SharedRawPointer
                | ValueTag::StackPointer
                | ValueTag::FramePointer
                | ValueTag::StaticPointer
                | ValueTag::FunctionPointer => Value::uint(argument.raw_data(), target_width),
                _ => argument,
            }
        }

        mir::CastOperator::IntToPointer => match argument.tag() {
            ValueTag::UInt | ValueTag::Int => {
                let raw = argument.raw_data();
                cast_integer_to_pointer(raw, target_type)?
            }
            _ => argument,
        },
    };

    Ok(result)
}

/// Cast one integer bit pattern into the requested pointer-shaped target type.
fn cast_integer_to_pointer(raw: u64, target_type: &mir::Type) -> Result<Value, Error> {
    access::decode_pointer_bits(raw, target_type)
}

/// Truncate a signed integer to a target bit width.
fn truncate_signed(value: i64, width: u8) -> i64 {
    // handle full width
    if width >= 64 {
        return value;
    }

    // build bit mask
    let mask = (1u64 << width) - 1;
    let masked = (value as u64) & mask;
    let sign_bit = 1u64 << (width - 1);

    // set sign extension when needed
    if masked & sign_bit != 0 {
        (masked | !mask) as i64
    }
    // otherwise keep masked value
    else {
        masked as i64
    }
}

/// Truncate an unsigned integer to a target bit width.
fn truncate_unsigned(value: u64, width: u8) -> u64 {
    // handle full width
    if width >= 64 {
        return value;
    }

    // apply mask
    let mask = (1u64 << width) - 1;
    value & mask
}

/// Sign extend a value from one width to another.
fn sign_extend(value: i64, from_width: u8, to_width: u8) -> i64 {
    // handle no extend case
    if from_width >= to_width || from_width >= 64 {
        return value;
    }

    // extend using truncate logic
    truncate_signed(value, from_width)
}

/// Compute integer bounds for a width and signedness.
fn integer_bounds(width: u8, is_signed: bool) -> Option<(i128, i128)> {
    // reject unsupported widths
    if width == 0 || width > 64 {
        return None;
    }

    // compute signed bounds
    if is_signed {
        let shift = (width - 1) as u32;
        let min = -(1_i128 << shift);
        let max = (1_i128 << shift) - 1;
        Some((min, max))
    } else {
        // compute unsigned bounds
        let shift = width as u32;
        let max = (1_i128 << shift) - 1;
        Some((0, max))
    }
}

/// Convert a float to an integer when the conversion is in range and finite.
fn float_to_int_checked(value: f64, min_bound: i128, max_bound: i128) -> Option<i128> {
    // reject NaN and infinities
    if !value.is_finite() {
        return None;
    }

    // compute float bounds for the target integer range
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

    // reject out of range values
    if !min_ok || !max_ok {
        return None;
    }

    // truncate toward zero and validate
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
