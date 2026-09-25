use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Binary arithmetic/logic operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum BinaryOperator {
    // arithmetic
    /// Addition.
    Add,
    /// Subtraction.
    Subtract,
    /// Multiplication.
    Multiply,
    /// Division.
    Divide,
    /// Remainder.
    Remainder,

    // bitwise
    /// Logical or bitwise AND.
    And,
    /// Logical or bitwise OR.
    Or,
    /// Bitwise XOR.
    Xor,
    /// Shift left.
    ShiftLeft,
    /// Type-directed shift right.
    ShiftRight,
    /// Zero-extending shift right.
    UnsignedShiftRight,

    // comparison
    /// Equal.
    Equal,
    /// Not equal.
    NotEqual,
    /// Less than.
    LessThan,
    /// Less than or equal.
    LessEqual,
    /// Greater than.
    GreaterThan,
    /// Greater than or equal.
    GreaterEqual,
}

impl BinaryOperator {
    /// Return the canonical operation name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Remainder => "rem",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::ShiftLeft => "shl",
            Self::ShiftRight => "shr",
            Self::UnsignedShiftRight => "ushr",
            Self::Equal => "eq",
            Self::NotEqual => "ne",
            Self::LessThan => "lt",
            Self::LessEqual => "le",
            Self::GreaterThan => "gt",
            Self::GreaterEqual => "ge",
        }
    }

    /// Whether this is a comparison operator.
    pub const fn is_comparison(self) -> bool {
        matches!(
            self,
            BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::LessThan
                | BinaryOperator::LessEqual
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterEqual
        )
    }

    /// Return whether this operator is commutative.
    pub const fn is_commutative(self) -> bool {
        matches!(
            self,
            BinaryOperator::Add
                | BinaryOperator::Multiply
                | BinaryOperator::And
                | BinaryOperator::Or
                | BinaryOperator::Xor
                | BinaryOperator::Equal
                | BinaryOperator::NotEqual
        )
    }

    /// Return the comparison operator for swapped operands.
    pub fn swap_operands(self) -> Option<Self> {
        match self {
            BinaryOperator::Equal | BinaryOperator::NotEqual => Some(self),
            BinaryOperator::LessThan => Some(BinaryOperator::GreaterThan),
            BinaryOperator::LessEqual => Some(BinaryOperator::GreaterEqual),
            BinaryOperator::GreaterThan => Some(BinaryOperator::LessThan),
            BinaryOperator::GreaterEqual => Some(BinaryOperator::LessEqual),
            _ => None,
        }
    }
}

impl FromStr for BinaryOperator {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "add" => Ok(Self::Add),
            "sub" => Ok(Self::Subtract),
            "mul" => Ok(Self::Multiply),
            "div" => Ok(Self::Divide),
            "rem" => Ok(Self::Remainder),
            "and" => Ok(Self::And),
            "or" => Ok(Self::Or),
            "xor" => Ok(Self::Xor),
            "shl" => Ok(Self::ShiftLeft),
            "shr" => Ok(Self::ShiftRight),
            "ushr" => Ok(Self::UnsignedShiftRight),
            "eq" => Ok(Self::Equal),
            "ne" => Ok(Self::NotEqual),
            "lt" => Ok(Self::LessThan),
            "le" => Ok(Self::LessEqual),
            "gt" => Ok(Self::GreaterThan),
            "ge" => Ok(Self::GreaterEqual),
            _ => Err(()),
        }
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum UnaryOperator {
    /// Numeric negation.
    Negate,
    /// Logical or bitwise inversion.
    Not,
}

impl UnaryOperator {
    /// Return the canonical operation name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Negate => "negate",
            Self::Not => "not",
        }
    }
}

impl FromStr for UnaryOperator {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "negate" => Ok(Self::Negate),
            "not" => Ok(Self::Not),
            _ => Err(()),
        }
    }
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
