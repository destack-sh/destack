use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

/// Native trap code.
pub type TrapCode = u32;

/// Low-level trap reported by generated native code.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
#[repr(u32)]
pub enum Trap {
    /// Integer division or remainder used a zero divisor.
    DivisionByZero = 1,
    /// Checked integer arithmetic overflowed.
    IntegerOverflow = 2,
    /// Explicit arithmetic validation failed.
    InvalidArithmetic = 3,
    /// Execution reached an unreachable instruction.
    Unreachable = 4,
    /// The explicit abort trap was reached.
    Abort = 5,
    /// A bounds check failed.
    Bounds = 6,
    /// A null check failed.
    Null = 7,
    /// A runtime type check failed.
    Type = 8,
    /// Native stack capacity was exceeded.
    StackOverflow = 9,
}

impl Trap {
    /// Return the native ABI trap code.
    pub const fn code(self) -> TrapCode {
        self as TrapCode
    }
}

/// Native trap code conversion error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrapError {
    /// The invalid trap code.
    pub code: TrapCode,
}

impl fmt::Display for TrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid native trap code {}", self.code)
    }
}

impl Error for TrapError {}

impl TryFrom<TrapCode> for Trap {
    type Error = TrapError;

    fn try_from(code: TrapCode) -> Result<Self, Self::Error> {
        match code {
            1 => Ok(Self::DivisionByZero),
            2 => Ok(Self::IntegerOverflow),
            3 => Ok(Self::InvalidArithmetic),
            4 => Ok(Self::Unreachable),
            5 => Ok(Self::Abort),
            6 => Ok(Self::Bounds),
            7 => Ok(Self::Null),
            8 => Ok(Self::Type),
            9 => Ok(Self::StackOverflow),
            code => Err(TrapError { code }),
        }
    }
}
