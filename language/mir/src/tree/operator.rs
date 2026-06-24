use destack_serde::Reflect;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Binary arithmetic/logic operator.
///
/// These operators have wrapping semantics for integer addition, subtraction, and multiplication.
/// Shift operators mask the shift amount to the bit width.
/// Signed and unsigned division and remainder trap on division by zero.
/// Signed division and remainder also trap on `min_value / -1`.
///
/// For other overflow behaviors, use intrinsics.
/// The `add.overflow` family returns an overflow flag.
/// The `*.unchecked` family has undefined behavior on overflow or division by zero.
/// The `add.sat` and `sub.sat` intrinsics clamp to the numeric bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum BinaryOperator {
    // integer arithmetic
    /// Integer addition.
    Add,
    /// Integer subtraction.
    Subtract,
    /// Integer multiplication.
    Multiply,
    /// Signed integer division.
    SignedDivide,
    /// Unsigned integer division.
    UnsignedDivide,
    /// Signed integer remainder.
    SignedRemainder,
    /// Unsigned integer remainder.
    UnsignedRemainder,

    // floating point arithmetic
    /// Floating point addition.
    FloatAdd,
    /// Floating point subtraction.
    FloatSubtract,
    /// Floating point multiplication.
    FloatMultiply,
    /// Floating point division.
    FloatDivide,

    // bitwise
    /// Bitwise AND.
    And,
    /// Bitwise OR.
    Or,
    /// Bitwise XOR.
    Xor,
    /// Shift left.
    ShiftLeft,
    /// Arithmetic shift right (sign-extending).
    ArithmeticShiftRight,
    /// Logical shift right (zero-extending).
    LogicalShiftRight,

    // integer comparison
    /// Equal.
    Equal,
    /// Not equal.
    NotEqual,
    /// Signed less than.
    SignedLessThan,
    /// Signed less than or equal.
    SignedLessEqual,
    /// Signed greater than.
    SignedGreaterThan,
    /// Signed greater than or equal.
    SignedGreaterEqual,
    /// Unsigned less than.
    UnsignedLessThan,
    /// Unsigned less than or equal.
    UnsignedLessEqual,
    /// Unsigned greater than.
    UnsignedGreaterThan,
    /// Unsigned greater than or equal.
    UnsignedGreaterEqual,

    // floating point comparison (ordered)
    /// Floating point equal.
    FloatEqual,
    /// Floating point not equal.
    FloatNotEqual,
    /// Floating point less than.
    FloatLessThan,
    /// Floating point less than or equal.
    FloatLessEqual,
    /// Floating point greater than.
    FloatGreaterThan,
    /// Floating point greater than or equal.
    FloatGreaterEqual,
}

impl BinaryOperator {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            // integer arithmetic
            BinaryOperator::Add => "int.add",
            BinaryOperator::Subtract => "int.sub",
            BinaryOperator::Multiply => "int.mul",
            BinaryOperator::SignedDivide => "int.div.s",
            BinaryOperator::UnsignedDivide => "int.div.u",
            BinaryOperator::SignedRemainder => "int.rem.s",
            BinaryOperator::UnsignedRemainder => "int.rem.u",
            // float arithmetic
            BinaryOperator::FloatAdd => "float.add",
            BinaryOperator::FloatSubtract => "float.sub",
            BinaryOperator::FloatMultiply => "float.mul",
            BinaryOperator::FloatDivide => "float.div",
            // bitwise
            BinaryOperator::And => "int.and",
            BinaryOperator::Or => "int.or",
            BinaryOperator::Xor => "int.xor",
            BinaryOperator::ShiftLeft => "int.shl",
            BinaryOperator::ArithmeticShiftRight => "int.shr.s",
            BinaryOperator::LogicalShiftRight => "int.shr.u",
            // integer comparison
            BinaryOperator::Equal => "int.eq",
            BinaryOperator::NotEqual => "int.ne",
            BinaryOperator::SignedLessThan => "int.lt.s",
            BinaryOperator::SignedLessEqual => "int.le.s",
            BinaryOperator::SignedGreaterThan => "int.gt.s",
            BinaryOperator::SignedGreaterEqual => "int.ge.s",
            BinaryOperator::UnsignedLessThan => "int.lt.u",
            BinaryOperator::UnsignedLessEqual => "int.le.u",
            BinaryOperator::UnsignedGreaterThan => "int.gt.u",
            BinaryOperator::UnsignedGreaterEqual => "int.ge.u",
            // float comparison
            BinaryOperator::FloatEqual => "float.eq",
            BinaryOperator::FloatNotEqual => "float.ne",
            BinaryOperator::FloatLessThan => "float.lt",
            BinaryOperator::FloatLessEqual => "float.le",
            BinaryOperator::FloatGreaterThan => "float.gt",
            BinaryOperator::FloatGreaterEqual => "float.ge",
        }
    }

    /// Whether this is a comparison operator.
    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::SignedLessThan
                | BinaryOperator::SignedLessEqual
                | BinaryOperator::SignedGreaterThan
                | BinaryOperator::SignedGreaterEqual
                | BinaryOperator::UnsignedLessThan
                | BinaryOperator::UnsignedLessEqual
                | BinaryOperator::UnsignedGreaterThan
                | BinaryOperator::UnsignedGreaterEqual
                | BinaryOperator::FloatEqual
                | BinaryOperator::FloatNotEqual
                | BinaryOperator::FloatLessThan
                | BinaryOperator::FloatLessEqual
                | BinaryOperator::FloatGreaterThan
                | BinaryOperator::FloatGreaterEqual
        )
    }

    /// Whether this is a floating point operator.
    pub fn is_float(&self) -> bool {
        matches!(
            self,
            BinaryOperator::FloatAdd
                | BinaryOperator::FloatSubtract
                | BinaryOperator::FloatMultiply
                | BinaryOperator::FloatDivide
                | BinaryOperator::FloatEqual
                | BinaryOperator::FloatNotEqual
                | BinaryOperator::FloatLessThan
                | BinaryOperator::FloatLessEqual
                | BinaryOperator::FloatGreaterThan
                | BinaryOperator::FloatGreaterEqual
        )
    }

    /// Whether this is an integer arithmetic or bitwise operator.
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::SignedDivide
                | BinaryOperator::UnsignedDivide
                | BinaryOperator::SignedRemainder
                | BinaryOperator::UnsignedRemainder
                | BinaryOperator::And
                | BinaryOperator::Or
                | BinaryOperator::Xor
                | BinaryOperator::ShiftLeft
                | BinaryOperator::ArithmeticShiftRight
                | BinaryOperator::LogicalShiftRight
        )
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for BinaryOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // integer arithmetic
            "int.add" => Ok(BinaryOperator::Add),
            "int.sub" => Ok(BinaryOperator::Subtract),
            "int.mul" => Ok(BinaryOperator::Multiply),
            "int.div.s" => Ok(BinaryOperator::SignedDivide),
            "int.div.u" => Ok(BinaryOperator::UnsignedDivide),
            "int.rem.s" => Ok(BinaryOperator::SignedRemainder),
            "int.rem.u" => Ok(BinaryOperator::UnsignedRemainder),
            // float arithmetic
            "float.add" => Ok(BinaryOperator::FloatAdd),
            "float.sub" => Ok(BinaryOperator::FloatSubtract),
            "float.mul" => Ok(BinaryOperator::FloatMultiply),
            "float.div" => Ok(BinaryOperator::FloatDivide),
            // bitwise
            "int.and" => Ok(BinaryOperator::And),
            "int.or" => Ok(BinaryOperator::Or),
            "int.xor" => Ok(BinaryOperator::Xor),
            "int.shl" | "int.shiftLeft" => Ok(BinaryOperator::ShiftLeft),
            "int.shr.s" | "int.shiftRight.s" => Ok(BinaryOperator::ArithmeticShiftRight),
            "int.shr.u" | "int.shiftRight.u" => Ok(BinaryOperator::LogicalShiftRight),
            // integer comparison
            "int.eq" => Ok(BinaryOperator::Equal),
            "int.ne" => Ok(BinaryOperator::NotEqual),
            "int.lt.s" => Ok(BinaryOperator::SignedLessThan),
            "int.le.s" => Ok(BinaryOperator::SignedLessEqual),
            "int.gt.s" => Ok(BinaryOperator::SignedGreaterThan),
            "int.ge.s" => Ok(BinaryOperator::SignedGreaterEqual),
            "int.lt.u" => Ok(BinaryOperator::UnsignedLessThan),
            "int.le.u" => Ok(BinaryOperator::UnsignedLessEqual),
            "int.gt.u" => Ok(BinaryOperator::UnsignedGreaterThan),
            "int.ge.u" => Ok(BinaryOperator::UnsignedGreaterEqual),
            // float comparison
            "float.eq" => Ok(BinaryOperator::FloatEqual),
            "float.ne" => Ok(BinaryOperator::FloatNotEqual),
            "float.lt" => Ok(BinaryOperator::FloatLessThan),
            "float.le" => Ok(BinaryOperator::FloatLessEqual),
            "float.gt" => Ok(BinaryOperator::FloatGreaterThan),
            "float.ge" => Ok(BinaryOperator::FloatGreaterEqual),
            _ => Err(()),
        }
    }
}

/// Unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum UnaryOperator {
    /// Integer negation.
    Negate,
    /// Floating point negation.
    FloatNegate,
    /// Bitwise NOT.
    Not,
}

impl UnaryOperator {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            UnaryOperator::Negate => "int.negate",
            UnaryOperator::FloatNegate => "float.negate",
            UnaryOperator::Not => "int.not",
        }
    }
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for UnaryOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "int.negate" => Ok(UnaryOperator::Negate),
            "float.negate" => Ok(UnaryOperator::FloatNegate),
            "int.not" => Ok(UnaryOperator::Not),
            _ => Err(()),
        }
    }
}
