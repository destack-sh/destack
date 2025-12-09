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
        use mir::BinaryOperator::*;

        let result = match (op, &lhs, &rhs) {
            // integer arithmetic
            (Add, Value::Int { value: a, width }, Value::Int { value: b, .. }) => Value::Int {
                value: a.wrapping_add(*b),
                width: *width,
            },
            (Subtract, Value::Int { value: a, width }, Value::Int { value: b, .. }) => Value::Int {
                value: a.wrapping_sub(*b),
                width: *width,
            },
            (Multiply, Value::Int { value: a, width }, Value::Int { value: b, .. }) => Value::Int {
                value: a.wrapping_mul(*b),
                width: *width,
            },
            (SignedDivide, Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Value::Int {
                    value: a.wrapping_div(*b),
                    width: *width,
                }
            }
            (UnsignedDivide, Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Value::UInt {
                    value: a.wrapping_div(*b),
                    width: *width,
                }
            }
            (SignedRemainder, Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Value::Int {
                    value: a.wrapping_rem(*b),
                    width: *width,
                }
            }
            (UnsignedRemainder, Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Value::UInt {
                    value: a.wrapping_rem(*b),
                    width: *width,
                }
            }

            // unsigned arithmetic
            (Add, Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => Value::UInt {
                value: a.wrapping_add(*b),
                width: *width,
            },
            (Subtract, Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                Value::UInt {
                    value: a.wrapping_sub(*b),
                    width: *width,
                }
            }
            (Multiply, Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                Value::UInt {
                    value: a.wrapping_mul(*b),
                    width: *width,
                }
            }

            // integer comparison
            (Equal, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Value::Bool(a == b)
            }
            (NotEqual, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Value::Bool(a != b)
            }
            (SignedLessThan, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Value::Bool(a < b)
            }
            (SignedLessEqual, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Value::Bool(a <= b)
            }
            (SignedGreaterThan, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Value::Bool(a > b)
            }
            (SignedGreaterEqual, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Value::Bool(a >= b)
            }
            (UnsignedLessThan, Value::UInt { value: a, .. }, Value::UInt { value: b, .. }) => {
                Value::Bool(a < b)
            }
            (UnsignedLessEqual, Value::UInt { value: a, .. }, Value::UInt { value: b, .. }) => {
                Value::Bool(a <= b)
            }
            (UnsignedGreaterThan, Value::UInt { value: a, .. }, Value::UInt { value: b, .. }) => {
                Value::Bool(a > b)
            }
            (UnsignedGreaterEqual, Value::UInt { value: a, .. }, Value::UInt { value: b, .. }) => {
                Value::Bool(a >= b)
            }

            // boolean operations
            (And, Value::Bool(a), Value::Bool(b)) => Value::Bool(*a && *b),
            (Or, Value::Bool(a), Value::Bool(b)) => Value::Bool(*a || *b),
            (Xor, Value::Bool(a), Value::Bool(b)) => Value::Bool(*a ^ *b),

            // bitwise operations on integers
            (And, Value::Int { value: a, width }, Value::Int { value: b, .. }) => Value::Int {
                value: a & b,
                width: *width,
            },
            (Or, Value::Int { value: a, width }, Value::Int { value: b, .. }) => Value::Int {
                value: a | b,
                width: *width,
            },
            (Xor, Value::Int { value: a, width }, Value::Int { value: b, .. }) => Value::Int {
                value: a ^ b,
                width: *width,
            },
            (ShiftLeft, Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                Value::Int {
                    value: a.wrapping_shl(*b as u32),
                    width: *width,
                }
            }
            (ArithmeticShiftRight, Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                Value::Int {
                    value: a.wrapping_shr(*b as u32),
                    width: *width,
                }
            }
            (LogicalShiftRight, Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                Value::UInt {
                    value: a.wrapping_shr(*b as u32),
                    width: *width,
                }
            }

            // float arithmetic
            (FloatAdd, Value::Float64(a), Value::Float64(b)) => Value::Float64(a + b),
            (FloatSubtract, Value::Float64(a), Value::Float64(b)) => Value::Float64(a - b),
            (FloatMultiply, Value::Float64(a), Value::Float64(b)) => Value::Float64(a * b),
            (FloatDivide, Value::Float64(a), Value::Float64(b)) => Value::Float64(a / b),

            (FloatAdd, Value::Float32(a), Value::Float32(b)) => Value::Float32(a + b),
            (FloatSubtract, Value::Float32(a), Value::Float32(b)) => Value::Float32(a - b),
            (FloatMultiply, Value::Float32(a), Value::Float32(b)) => Value::Float32(a * b),
            (FloatDivide, Value::Float32(a), Value::Float32(b)) => Value::Float32(a / b),

            // float comparison
            (FloatEqual, Value::Float64(a), Value::Float64(b)) => Value::Bool(a == b),
            (FloatNotEqual, Value::Float64(a), Value::Float64(b)) => Value::Bool(a != b),
            (FloatLessThan, Value::Float64(a), Value::Float64(b)) => Value::Bool(a < b),
            (FloatLessEqual, Value::Float64(a), Value::Float64(b)) => Value::Bool(a <= b),
            (FloatGreaterThan, Value::Float64(a), Value::Float64(b)) => Value::Bool(a > b),
            (FloatGreaterEqual, Value::Float64(a), Value::Float64(b)) => Value::Bool(a >= b),

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

        let result = match (op, &arg) {
            (Negate, Value::Int { value, width }) => Value::Int {
                value: value.wrapping_neg(),
                width: *width,
            },
            (FloatNegate, Value::Float64(f)) => Value::Float64(-f),
            (FloatNegate, Value::Float32(f)) => Value::Float32(-f),
            (Not, Value::Bool(b)) => Value::Bool(!b),
            (Not, Value::Int { value, width }) => Value::Int {
                value: !value,
                width: *width,
            },
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
    pub(super) fn execute_cast(&self, kind: mir::CastKind, arg: Value) -> RuntimeResult<Value> {
        use mir::CastKind::*;

        let result = match kind {
            Bitcast => arg,
            Truncate => arg,
            ZeroExtend | SignExtend => arg,
            FloatToSignedInt => {
                if let Value::Float64(f) = arg {
                    Value::Int {
                        value: f as i64,
                        width: 64,
                    }
                } else if let Value::Float32(f) = arg {
                    Value::Int {
                        value: f as i64,
                        width: 64,
                    }
                } else {
                    arg
                }
            }
            FloatToUnsignedInt => {
                if let Value::Float64(f) = arg {
                    Value::UInt {
                        value: f as u64,
                        width: 64,
                    }
                } else if let Value::Float32(f) = arg {
                    Value::UInt {
                        value: f as u64,
                        width: 64,
                    }
                } else {
                    arg
                }
            }
            SignedIntToFloat => {
                if let Value::Int { value, .. } = arg {
                    Value::Float64(value as f64)
                } else {
                    arg
                }
            }
            UnsignedIntToFloat => {
                if let Value::UInt { value, .. } = arg {
                    Value::Float64(value as f64)
                } else {
                    arg
                }
            }
            FloatExtend => {
                if let Value::Float32(f) = arg {
                    Value::Float64(f as f64)
                } else {
                    arg
                }
            }
            FloatTruncate => {
                if let Value::Float64(f) = arg {
                    Value::Float32(f as f32)
                } else {
                    arg
                }
            }
            PointerToInt => {
                if let Value::RawPointer(p) = arg {
                    Value::UInt {
                        value: p.id(),
                        width: 64,
                    }
                } else {
                    arg
                }
            }
            IntToPointer => {
                if let Value::UInt { value, .. } = arg {
                    Value::RawPointer(RawPointer::new(value))
                } else if let Value::Int { value, .. } = arg {
                    Value::RawPointer(RawPointer::new(value as u64))
                } else {
                    arg
                }
            }
        };

        Ok(result)
    }
}
