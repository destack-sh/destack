use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeResult};
use crate::memory::{RawPointer, Value, ValueTag};

use super::Interpreter;

impl Interpreter {
    /// Execute a binary operation.
    pub(super) fn execute_binary(
        &self,
        op: mir::BinaryOperator,
        lhs: Value,
        rhs: Value,
    ) -> RuntimeResult<Value> {
        use mir::BinaryOperator::*;

        // fast path: check tags once and dispatch
        let lhs_tag = lhs.tag();
        let rhs_tag = rhs.tag();

        let result = match (lhs_tag, rhs_tag) {
            // signed integer operations
            (ValueTag::Int, ValueTag::Int) => {
                let a = lhs.raw_data() as i64;
                let b = rhs.raw_data() as i64;
                let width = lhs.width();
                match op {
                    Add => Value::int(a.wrapping_add(b), width),
                    Subtract => Value::int(a.wrapping_sub(b), width),
                    Multiply => Value::int(a.wrapping_mul(b), width),
                    SignedDivide => {
                        if b == 0 {
                            return Err(self.make_error(Error::DivisionByZero));
                        }
                        Value::int(a.wrapping_div(b), width)
                    }
                    SignedRemainder => {
                        if b == 0 {
                            return Err(self.make_error(Error::DivisionByZero));
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
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: format!("compatible types for {op:?}"),
                            actual: format!("{lhs:?}, {rhs:?}"),
                        }));
                    }
                }
            }

            // unsigned integer operations
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = lhs.raw_data();
                let b = rhs.raw_data();
                let width = lhs.width();
                match op {
                    Add => Value::uint(a.wrapping_add(b), width),
                    Subtract => Value::uint(a.wrapping_sub(b), width),
                    Multiply => Value::uint(a.wrapping_mul(b), width),
                    UnsignedDivide => {
                        if b == 0 {
                            return Err(self.make_error(Error::DivisionByZero));
                        }
                        Value::uint(a.wrapping_div(b), width)
                    }
                    UnsignedRemainder => {
                        if b == 0 {
                            return Err(self.make_error(Error::DivisionByZero));
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
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: format!("compatible types for {op:?}"),
                            actual: format!("{lhs:?}, {rhs:?}"),
                        }));
                    }
                }
            }

            // f64 operations
            (ValueTag::Float64, ValueTag::Float64) => {
                let a = f64::from_bits(lhs.raw_data());
                let b = f64::from_bits(rhs.raw_data());
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
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: format!("compatible types for {op:?}"),
                            actual: format!("{lhs:?}, {rhs:?}"),
                        }));
                    }
                }
            }

            // f32 operations
            (ValueTag::Float32, ValueTag::Float32) => {
                let a = f32::from_bits(lhs.raw_data() as u32);
                let b = f32::from_bits(rhs.raw_data() as u32);
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
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: format!("compatible types for {op:?}"),
                            actual: format!("{lhs:?}, {rhs:?}"),
                        }));
                    }
                }
            }

            // boolean operations
            (ValueTag::Bool, ValueTag::Bool) => {
                let a = lhs.raw_data() != 0;
                let b = rhs.raw_data() != 0;
                match op {
                    And => Value::bool(a && b),
                    Or => Value::bool(a || b),
                    Xor => Value::bool(a ^ b),
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: format!("compatible types for {op:?}"),
                            actual: format!("{lhs:?}, {rhs:?}"),
                        }));
                    }
                }
            }

            // incompatible types
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: format!("compatible types for {op:?}"),
                    actual: format!("{lhs:?}, {rhs:?}"),
                }));
            }
        };

        Ok(result)
    }

    /// Execute a unary operation.
    pub(super) fn execute_unary(&self, op: mir::UnaryOperator, arg: Value) -> RuntimeResult<Value> {
        use mir::UnaryOperator::*;

        let result = match (op, arg.tag()) {
            // -arg (signed integer)
            (Negate, ValueTag::Int) => {
                let value = arg.raw_data() as i64;
                Value::int(value.wrapping_neg(), arg.width())
            }

            // -arg (f64)
            (FloatNegate, ValueTag::Float64) => {
                let f = f64::from_bits(arg.raw_data());
                Value::float64(-f)
            }

            // -arg (f32)
            (FloatNegate, ValueTag::Float32) => {
                let f = f32::from_bits(arg.raw_data() as u32);
                Value::float32(-f)
            }

            // !arg (boolean)
            (Not, ValueTag::Bool) => {
                let b = arg.raw_data() != 0;
                Value::bool(!b)
            }

            // ~arg (signed integer bitwise not)
            (Not, ValueTag::Int) => {
                let value = arg.raw_data() as i64;
                Value::int(!value, arg.width())
            }

            // ~arg (unsigned integer bitwise not)
            (Not, ValueTag::UInt) => {
                let value = arg.raw_data();
                Value::uint(!value, arg.width())
            }

            // incompatible type
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: format!("compatible type for {op:?}"),
                    actual: format!("{arg:?}"),
                }));
            }
        };

        Ok(result)
    }

    /// Execute a cast operation.
    pub(super) fn execute_cast(
        &self,
        operator: mir::CastOperator,
        argument: Value,
        to_type: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<Value> {
        let target_type = self.tree.get(to_type);

        let result = match operator {
            // reinterpret bits without conversion
            mir::CastOperator::Bitcast => argument,

            // truncate integer to smaller width
            mir::CastOperator::Truncate => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => return Ok(argument),
                };
                match argument.tag() {
                    ValueTag::Int => {
                        let value = argument.raw_data() as i64;
                        let masked = truncate_signed(value, target_width);
                        Value::int(masked, target_width)
                    }
                    ValueTag::UInt => {
                        let value = argument.raw_data();
                        let masked = truncate_unsigned(value, target_width);
                        Value::uint(masked, target_width)
                    }
                    _ => argument,
                }
            }

            // zero-extend integer to larger width
            mir::CastOperator::ZeroExtend => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => return Ok(argument),
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

            // sign-extend integer to larger width
            mir::CastOperator::SignExtend => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => return Ok(argument),
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

            // float to signed integer
            mir::CastOperator::FloatToSignedInt => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => 64,
                };
                match argument.tag() {
                    ValueTag::Float64 => {
                        let f = f64::from_bits(argument.raw_data());
                        Value::int(f as i64, target_width)
                    }
                    ValueTag::Float32 => {
                        let f = f32::from_bits(argument.raw_data() as u32);
                        Value::int(f as i64, target_width)
                    }
                    _ => argument,
                }
            }

            // float to unsigned integer
            mir::CastOperator::FloatToUnsignedInt => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => 64,
                };
                match argument.tag() {
                    ValueTag::Float64 => {
                        let f = f64::from_bits(argument.raw_data());
                        Value::uint(f as u64, target_width)
                    }
                    ValueTag::Float32 => {
                        let f = f32::from_bits(argument.raw_data() as u32);
                        Value::uint(f as u64, target_width)
                    }
                    _ => argument,
                }
            }

            // signed integer to float
            mir::CastOperator::SignedIntToFloat => {
                let target_width = match target_type {
                    mir::Type::Float { width } => *width,
                    _ => 64,
                };
                match argument.tag() {
                    ValueTag::Int => {
                        let value = argument.raw_data() as i64;
                        if target_width == 32 {
                            Value::float32(value as f32)
                        } else {
                            Value::float64(value as f64)
                        }
                    }
                    _ => argument,
                }
            }

            // unsigned integer to float
            mir::CastOperator::UnsignedIntToFloat => {
                let target_width = match target_type {
                    mir::Type::Float { width } => *width,
                    _ => 64,
                };
                match argument.tag() {
                    ValueTag::UInt => {
                        let value = argument.raw_data();
                        if target_width == 32 {
                            Value::float32(value as f32)
                        } else {
                            Value::float64(value as f64)
                        }
                    }
                    _ => argument,
                }
            }

            // f32 to f64
            mir::CastOperator::FloatExtend => {
                if argument.tag() == ValueTag::Float32 {
                    let f = f32::from_bits(argument.raw_data() as u32);
                    Value::float64(f as f64)
                } else {
                    argument
                }
            }

            // f64 to f32
            mir::CastOperator::FloatTruncate => {
                if argument.tag() == ValueTag::Float64 {
                    let f = f64::from_bits(argument.raw_data());
                    Value::float32(f as f32)
                } else {
                    argument
                }
            }

            // pointer to integer
            mir::CastOperator::PointerToInt => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => 64,
                };
                match argument.tag() {
                    ValueTag::RawPointer | ValueTag::ManagedReference => {
                        Value::uint(argument.raw_data(), target_width)
                    }
                    _ => argument,
                }
            }

            // integer to pointer
            mir::CastOperator::IntToPointer => match argument.tag() {
                ValueTag::UInt | ValueTag::Int => {
                    Value::raw_pointer(RawPointer::new(argument.raw_data()))
                }
                _ => argument,
            },
        };

        Ok(result)
    }
}

/// Truncate a signed integer to a smaller bit width.
fn truncate_signed(value: i64, width: u8) -> i64 {
    if width >= 64 {
        return value;
    }
    let mask = (1u64 << width) - 1;
    let masked = (value as u64) & mask;

    // sign-extend from the new width
    let sign_bit = 1u64 << (width - 1);
    if masked & sign_bit != 0 {
        (masked | !mask) as i64
    } else {
        masked as i64
    }
}

/// Truncate an unsigned integer to a smaller bit width.
fn truncate_unsigned(value: u64, width: u8) -> u64 {
    if width >= 64 {
        return value;
    }
    let mask = (1u64 << width) - 1;
    value & mask
}

/// Sign-extend from one width to another.
fn sign_extend(value: i64, from_width: u8, to_width: u8) -> i64 {
    if from_width >= to_width || from_width >= 64 {
        return value;
    }
    truncate_signed(value, from_width)
}
