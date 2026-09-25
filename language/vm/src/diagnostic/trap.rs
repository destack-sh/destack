use std::{error, fmt};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// One language trap reached during bytecode execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Trap {
    /// Integer division or remainder used a zero divisor.
    DivisionByZero,
    /// Checked integer arithmetic overflowed.
    IntegerOverflow,
    /// Explicit arithmetic validation failed.
    InvalidArithmetic,
    /// Execution reached an unreachable instruction.
    Unreachable,
    /// The explicit abort trap was reached.
    Abort,
    /// A bounds check failed.
    Bounds,
    /// A null check failed.
    Null,
    /// A runtime type check failed.
    Type,
}

impl fmt::Display for Trap {
    /// Format one language trap.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::DivisionByZero => "division by zero",
            Self::IntegerOverflow => "integer overflow",
            Self::InvalidArithmetic => "invalid arithmetic",
            Self::Unreachable => "unreachable code",
            Self::Abort => "abort",
            Self::Bounds => "bounds check failed",
            Self::Null => "null check failed",
            Self::Type => "type check failed",
        };

        formatter.write_str(message)
    }
}

impl error::Error for Trap {}
