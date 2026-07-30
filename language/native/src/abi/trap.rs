use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Native trap code.
pub type TrapCode = u32;

/// Low-level trap reported by generated native code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Trap {
    /// Integer arithmetic overflowed.
    IntegerOverflow = 1,
    /// A checked cast failed.
    InvalidCast = 2,
    /// A bounds check failed.
    Bounds = 3,
    /// A null reference was used.
    NullReference = 4,
    /// An unreachable block executed.
    Unreachable = 5,
    /// Native stack capacity was exceeded.
    StackOverflow = 6,
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
            1 => Ok(Self::IntegerOverflow),
            2 => Ok(Self::InvalidCast),
            3 => Ok(Self::Bounds),
            4 => Ok(Self::NullReference),
            5 => Ok(Self::Unreachable),
            6 => Ok(Self::StackOverflow),
            code => Err(TrapError { code }),
        }
    }
}
