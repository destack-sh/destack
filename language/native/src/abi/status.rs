use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Native entry exit status code.
pub type NativeStatusCode = u32;

/// Native entry exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum NativeStatus {
    /// Execution completed normally.
    Completed = 0,
    /// Execution yielded a continuation.
    Yielded = 1,
    /// Execution trapped.
    Trapped = 2,
    /// Execution deoptimized into VM materialization.
    Deoptimized = 3,
    /// Execution stopped with a language panic.
    Panicked = 4,
}

impl NativeStatus {
    /// Return the native ABI status code.
    pub const fn code(self) -> NativeStatusCode {
        self as NativeStatusCode
    }
}

/// Native status code conversion error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeStatusError {
    /// The invalid status code.
    pub code: NativeStatusCode,
}

impl fmt::Display for NativeStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid native status code {}", self.code)
    }
}

impl Error for NativeStatusError {}

impl TryFrom<NativeStatusCode> for NativeStatus {
    type Error = NativeStatusError;

    fn try_from(code: NativeStatusCode) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(Self::Completed),
            1 => Ok(Self::Yielded),
            2 => Ok(Self::Trapped),
            3 => Ok(Self::Deoptimized),
            4 => Ok(Self::Panicked),
            code => Err(NativeStatusError { code }),
        }
    }
}
