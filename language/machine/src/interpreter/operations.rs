//! Binary, unary, and cast operation implementations.

use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeResult};
use crate::memory::{RawPointer, Value};

use super::Interpreter;

impl Interpreter {
    /// Execute a binary operation.
    pub(super) fn execute_binary(
        &self,
        op: mir::BinaryOperator,
        lhs: Value,
        rhs: Value,
    ) -> RuntimeResult<Value> {
        let result = match (op, &lhs, &rhs) {
            // lhs + rhs (signed)
            (
                mir::BinaryOperator::Add,
                Value::Int { value: a, width },
                Value::Int { value: b, .. },
            ) => Value::Int {
                value: a.wrapping_add(*b),
                width: *width,
            },

            // lhs - rhs (signed)
            (
                mir::BinaryOperator::Subtract,
                Value::Int { value: a, width },
                Value::Int { value: b, .. },
            ) => Value::Int {
                value: a.wrapping_sub(*b),
                width: *width,
            },

            // lhs * rhs (signed)
            (
                mir::BinaryOperator::Multiply,
                Value::Int { value: a, width },
                Value::Int { value: b, .. },
            ) => Value::Int {
                value: a.wrapping_mul(*b),
                width: *width,
            },

            // lhs / rhs (signed)
            (
                mir::BinaryOperator::SignedDivide,
                Value::Int { value: a, width },
                Value::Int { value: b, .. },
            ) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Value::Int {
                    value: a.wrapping_div(*b),
                    width: *width,
                }
            }

            // lhs / rhs (unsigned)
            (
                mir::BinaryOperator::UnsignedDivide,
                Value::UInt { value: a, width },
                Value::UInt { value: b, .. },
            ) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Value::UInt {
                    value: a.wrapping_div(*b),
                    width: *width,
                }
            }

            // lhs % rhs (signed)
            (
                mir::BinaryOperator::SignedRemainder,
                Value::Int { value: a, width },
                Value::Int { value: b, .. },
            ) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Value::Int {
                    value: a.wrapping_rem(*b),
                    width: *width,
                }
            }

            // lhs % rhs (unsigned)
            (
                mir::BinaryOperator::UnsignedRemainder,
                Value::UInt { value: a, width },
                Value::UInt { value: b, .. },
            ) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Value::UInt {
                    value: a.wrapping_rem(*b),
                    width: *width,
                }
            }

            // lhs + rhs (unsigned)
            (
                mir::BinaryOperator::Add,
                Value::UInt { value: a, width },
                Value::UInt { value: b, .. },
            ) => Value::UInt {
                value: a.wrapping_add(*b),
                width: *width,
            },

            // lhs - rhs (unsigned)
            (
                mir::BinaryOperator::Subtract,
                Value::UInt { value: a, width },
                Value::UInt { value: b, .. },
            ) => Value::UInt {
                value: a.wrapping_sub(*b),
                width: *width,
            },

            // lhs * rhs (unsigned)
            (
                mir::BinaryOperator::Multiply,
                Value::UInt { value: a, width },
                Value::UInt { value: b, .. },
            ) => Value::UInt {
                value: a.wrapping_mul(*b),
                width: *width,
            },

            // lhs == rhs (signed)
            (
                mir::BinaryOperator::Equal,
                Value::Int { value: a, .. },
                Value::Int { value: b, .. },
            ) => Value::Bool(a == b),

            // lhs != rhs (signed)
            (
                mir::BinaryOperator::NotEqual,
                Value::Int { value: a, .. },
                Value::Int { value: b, .. },
            ) => Value::Bool(a != b),

            // lhs < rhs (signed)
            (
                mir::BinaryOperator::SignedLessThan,
                Value::Int { value: a, .. },
                Value::Int { value: b, .. },
            ) => Value::Bool(a < b),

            // lhs <= rhs (signed)
            (
                mir::BinaryOperator::SignedLessEqual,
                Value::Int { value: a, .. },
                Value::Int { value: b, .. },
            ) => Value::Bool(a <= b),

            // lhs > rhs (signed)
            (
                mir::BinaryOperator::SignedGreaterThan,
                Value::Int { value: a, .. },
                Value::Int { value: b, .. },
            ) => Value::Bool(a > b),

            // lhs >= rhs (signed)
            (
                mir::BinaryOperator::SignedGreaterEqual,
                Value::Int { value: a, .. },
                Value::Int { value: b, .. },
            ) => Value::Bool(a >= b),

            // lhs < rhs (unsigned)
            (
                mir::BinaryOperator::UnsignedLessThan,
                Value::UInt { value: a, .. },
                Value::UInt { value: b, .. },
            ) => Value::Bool(a < b),

            // lhs <= rhs (unsigned)
            (
                mir::BinaryOperator::UnsignedLessEqual,
                Value::UInt { value: a, .. },
                Value::UInt { value: b, .. },
            ) => Value::Bool(a <= b),

            // lhs > rhs (unsigned)
            (
                mir::BinaryOperator::UnsignedGreaterThan,
                Value::UInt { value: a, .. },
                Value::UInt { value: b, .. },
            ) => Value::Bool(a > b),

            // lhs >= rhs (unsigned)
            (
                mir::BinaryOperator::UnsignedGreaterEqual,
                Value::UInt { value: a, .. },
                Value::UInt { value: b, .. },
            ) => Value::Bool(a >= b),

            // lhs && rhs (boolean)
            (mir::BinaryOperator::And, Value::Bool(a), Value::Bool(b)) => Value::Bool(*a && *b),

            // lhs || rhs (boolean)
            (mir::BinaryOperator::Or, Value::Bool(a), Value::Bool(b)) => Value::Bool(*a || *b),

            // lhs ^ rhs (boolean)
            (mir::BinaryOperator::Xor, Value::Bool(a), Value::Bool(b)) => Value::Bool(*a ^ *b),

            // lhs & rhs (signed)
            (
                mir::BinaryOperator::And,
                Value::Int { value: a, width },
                Value::Int { value: b, .. },
            ) => Value::Int {
                value: a & b,
                width: *width,
            },

            // lhs | rhs (signed)
            (
                mir::BinaryOperator::Or,
                Value::Int { value: a, width },
                Value::Int { value: b, .. },
            ) => Value::Int {
                value: a | b,
                width: *width,
            },

            // lhs ^ rhs (signed)
            (
                mir::BinaryOperator::Xor,
                Value::Int { value: a, width },
                Value::Int { value: b, .. },
            ) => Value::Int {
                value: a ^ b,
                width: *width,
            },

            // lhs << rhs (signed)
            (
                mir::BinaryOperator::ShiftLeft,
                Value::Int { value: a, width },
                Value::Int { value: b, .. },
            ) => Value::Int {
                value: a.wrapping_shl(*b as u32),
                width: *width,
            },

            // lhs >> rhs (arithmetic, sign-extending)
            (
                mir::BinaryOperator::ArithmeticShiftRight,
                Value::Int { value: a, width },
                Value::Int { value: b, .. },
            ) => Value::Int {
                value: a.wrapping_shr(*b as u32),
                width: *width,
            },

            // lhs & rhs (unsigned)
            (
                mir::BinaryOperator::And,
                Value::UInt { value: a, width },
                Value::UInt { value: b, .. },
            ) => Value::UInt {
                value: a & b,
                width: *width,
            },

            // lhs | rhs (unsigned)
            (
                mir::BinaryOperator::Or,
                Value::UInt { value: a, width },
                Value::UInt { value: b, .. },
            ) => Value::UInt {
                value: a | b,
                width: *width,
            },

            // lhs ^ rhs (unsigned)
            (
                mir::BinaryOperator::Xor,
                Value::UInt { value: a, width },
                Value::UInt { value: b, .. },
            ) => Value::UInt {
                value: a ^ b,
                width: *width,
            },

            // lhs << rhs (unsigned)
            (
                mir::BinaryOperator::ShiftLeft,
                Value::UInt { value: a, width },
                Value::UInt { value: b, .. },
            ) => Value::UInt {
                value: a.wrapping_shl(*b as u32),
                width: *width,
            },

            // lhs >> rhs (logical, zero-extending)
            (
                mir::BinaryOperator::LogicalShiftRight,
                Value::UInt { value: a, width },
                Value::UInt { value: b, .. },
            ) => Value::UInt {
                value: a.wrapping_shr(*b as u32),
                width: *width,
            },

            // lhs + rhs (f64)
            (mir::BinaryOperator::FloatAdd, Value::Float64(a), Value::Float64(b)) => {
                Value::Float64(a + b)
            }

            // lhs - rhs (f64)
            (mir::BinaryOperator::FloatSubtract, Value::Float64(a), Value::Float64(b)) => {
                Value::Float64(a - b)
            }

            // lhs * rhs (f64)
            (mir::BinaryOperator::FloatMultiply, Value::Float64(a), Value::Float64(b)) => {
                Value::Float64(a * b)
            }

            // lhs / rhs (f64)
            (mir::BinaryOperator::FloatDivide, Value::Float64(a), Value::Float64(b)) => {
                Value::Float64(a / b)
            }

            // lhs + rhs (f32)
            (mir::BinaryOperator::FloatAdd, Value::Float32(a), Value::Float32(b)) => {
                Value::Float32(a + b)
            }

            // lhs - rhs (f32)
            (mir::BinaryOperator::FloatSubtract, Value::Float32(a), Value::Float32(b)) => {
                Value::Float32(a - b)
            }

            // lhs * rhs (f32)
            (mir::BinaryOperator::FloatMultiply, Value::Float32(a), Value::Float32(b)) => {
                Value::Float32(a * b)
            }

            // lhs / rhs (f32)
            (mir::BinaryOperator::FloatDivide, Value::Float32(a), Value::Float32(b)) => {
                Value::Float32(a / b)
            }

            // lhs == rhs (f64)
            (mir::BinaryOperator::FloatEqual, Value::Float64(a), Value::Float64(b)) => {
                Value::Bool(a == b)
            }

            // lhs != rhs (f64)
            (mir::BinaryOperator::FloatNotEqual, Value::Float64(a), Value::Float64(b)) => {
                Value::Bool(a != b)
            }

            // lhs < rhs (f64)
            (mir::BinaryOperator::FloatLessThan, Value::Float64(a), Value::Float64(b)) => {
                Value::Bool(a < b)
            }

            // lhs <= rhs (f64)
            (mir::BinaryOperator::FloatLessEqual, Value::Float64(a), Value::Float64(b)) => {
                Value::Bool(a <= b)
            }

            // lhs > rhs (f64)
            (mir::BinaryOperator::FloatGreaterThan, Value::Float64(a), Value::Float64(b)) => {
                Value::Bool(a > b)
            }

            // lhs >= rhs (f64)
            (mir::BinaryOperator::FloatGreaterEqual, Value::Float64(a), Value::Float64(b)) => {
                Value::Bool(a >= b)
            }

            // lhs == rhs (f32)
            (mir::BinaryOperator::FloatEqual, Value::Float32(a), Value::Float32(b)) => {
                Value::Bool(a == b)
            }

            // lhs != rhs (f32)
            (mir::BinaryOperator::FloatNotEqual, Value::Float32(a), Value::Float32(b)) => {
                Value::Bool(a != b)
            }

            // lhs < rhs (f32)
            (mir::BinaryOperator::FloatLessThan, Value::Float32(a), Value::Float32(b)) => {
                Value::Bool(a < b)
            }

            // lhs <= rhs (f32)
            (mir::BinaryOperator::FloatLessEqual, Value::Float32(a), Value::Float32(b)) => {
                Value::Bool(a <= b)
            }

            // lhs > rhs (f32)
            (mir::BinaryOperator::FloatGreaterThan, Value::Float32(a), Value::Float32(b)) => {
                Value::Bool(a > b)
            }

            // lhs >= rhs (f32)
            (mir::BinaryOperator::FloatGreaterEqual, Value::Float32(a), Value::Float32(b)) => {
                Value::Bool(a >= b)
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
        let result = match (op, &arg) {
            // -arg (signed integer)
            (mir::UnaryOperator::Negate, Value::Int { value, width }) => Value::Int {
                value: value.wrapping_neg(),
                width: *width,
            },

            // -arg (f64)
            (mir::UnaryOperator::FloatNegate, Value::Float64(f)) => Value::Float64(-f),

            // -arg (f32)
            (mir::UnaryOperator::FloatNegate, Value::Float32(f)) => Value::Float32(-f),

            // !arg (boolean)
            (mir::UnaryOperator::Not, Value::Bool(b)) => Value::Bool(!b),

            // ~arg (signed integer bitwise not)
            (mir::UnaryOperator::Not, Value::Int { value, width }) => Value::Int {
                value: !value,
                width: *width,
            },

            // ~arg (unsigned integer bitwise not)
            (mir::UnaryOperator::Not, Value::UInt { value, width }) => Value::UInt {
                value: !value,
                width: *width,
            },

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
        kind: mir::CastKind,
        argument: Value,
        to_type: mir::LocalNodeId<mir::Type>,
    ) -> RuntimeResult<Value> {
        let target_type = self.tree.get(to_type);

        let result = match kind {
            // reinterpret bits without conversion
            mir::CastKind::Bitcast => argument,

            // truncate integer to smaller width
            mir::CastKind::Truncate => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => return Ok(argument),
                };
                match argument {
                    Value::Int { value, .. } => {
                        let masked = truncate_signed(value, target_width);
                        Value::Int {
                            value: masked,
                            width: target_width,
                        }
                    }
                    Value::UInt { value, .. } => {
                        let masked = truncate_unsigned(value, target_width);
                        Value::UInt {
                            value: masked,
                            width: target_width,
                        }
                    }
                    _ => argument,
                }
            }

            // zero-extend integer to larger width
            mir::CastKind::ZeroExtend => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => return Ok(argument),
                };
                match argument {
                    Value::UInt { value, .. } => Value::UInt {
                        value,
                        width: target_width,
                    },
                    Value::Int { value, width } => {
                        let masked = truncate_unsigned(value as u64, width);
                        Value::UInt {
                            value: masked,
                            width: target_width,
                        }
                    }
                    _ => argument,
                }
            }

            // sign-extend integer to larger width
            mir::CastKind::SignExtend => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => return Ok(argument),
                };
                match argument {
                    Value::Int { value, width } => {
                        let extended = sign_extend(value, width, target_width);
                        Value::Int {
                            value: extended,
                            width: target_width,
                        }
                    }
                    Value::UInt { value, width } => {
                        let as_signed = truncate_signed(value as i64, width);
                        let extended = sign_extend(as_signed, width, target_width);
                        Value::Int {
                            value: extended,
                            width: target_width,
                        }
                    }
                    _ => argument,
                }
            }

            // float to signed integer
            mir::CastKind::FloatToSignedInt => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => 64,
                };
                match argument {
                    Value::Float64(f) => Value::Int {
                        value: f as i64,
                        width: target_width,
                    },
                    Value::Float32(f) => Value::Int {
                        value: f as i64,
                        width: target_width,
                    },
                    _ => argument,
                }
            }

            // float to unsigned integer
            mir::CastKind::FloatToUnsignedInt => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => 64,
                };
                match argument {
                    Value::Float64(f) => Value::UInt {
                        value: f as u64,
                        width: target_width,
                    },
                    Value::Float32(f) => Value::UInt {
                        value: f as u64,
                        width: target_width,
                    },
                    _ => argument,
                }
            }

            // signed integer to float
            mir::CastKind::SignedIntToFloat => {
                let target_width = match target_type {
                    mir::Type::Float { width } => *width,
                    _ => 64,
                };
                match argument {
                    Value::Int { value, .. } => {
                        if target_width == 32 {
                            Value::Float32(value as f32)
                        } else {
                            Value::Float64(value as f64)
                        }
                    }
                    _ => argument,
                }
            }

            // unsigned integer to float
            mir::CastKind::UnsignedIntToFloat => {
                let target_width = match target_type {
                    mir::Type::Float { width } => *width,
                    _ => 64,
                };
                match argument {
                    Value::UInt { value, .. } => {
                        if target_width == 32 {
                            Value::Float32(value as f32)
                        } else {
                            Value::Float64(value as f64)
                        }
                    }
                    _ => argument,
                }
            }

            // f32 to f64
            mir::CastKind::FloatExtend => match argument {
                Value::Float32(f) => Value::Float64(f as f64),
                _ => argument,
            },

            // f64 to f32
            mir::CastKind::FloatTruncate => match argument {
                Value::Float64(f) => Value::Float32(f as f32),
                _ => argument,
            },

            // pointer to integer
            mir::CastKind::PointerToInt => {
                let target_width = match target_type {
                    mir::Type::Int { width, .. } => *width as u8,
                    _ => 64,
                };
                match argument {
                    Value::RawPointer(p) => Value::UInt {
                        value: p.id(),
                        width: target_width,
                    },
                    Value::ManagedReference(h) => Value::UInt {
                        value: h.id(),
                        width: target_width,
                    },
                    _ => argument,
                }
            }

            // integer to pointer
            mir::CastKind::IntToPointer => match argument {
                Value::UInt { value, .. } => Value::RawPointer(RawPointer::new(value)),
                Value::Int { value, .. } => Value::RawPointer(RawPointer::new(value as u64)),
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
