use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Native entry exit discriminant.
pub type NativeExitCode = u32;

/// Native entry exit kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum NativeExitKind {
    /// Execution completed normally.
    Completed = 0,
    /// Execution yielded a continuation.
    Yielded = 1,
    /// Execution trapped.
    Trapped = 2,
    /// Execution deoptimized into continuation state.
    Deoptimized = 3,
    /// Execution stopped with a language panic.
    Panicked = 4,
}

impl NativeExitKind {
    /// Return the native ABI exit code.
    pub const fn code(self) -> NativeExitCode {
        self as NativeExitCode
    }
}

/// Native exit code conversion error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeExitError {
    /// The invalid exit code.
    pub code: NativeExitCode,
}

impl fmt::Display for NativeExitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid native exit code {}", self.code)
    }
}

impl Error for NativeExitError {}

impl TryFrom<NativeExitCode> for NativeExitKind {
    type Error = NativeExitError;

    fn try_from(code: NativeExitCode) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(Self::Completed),
            1 => Ok(Self::Yielded),
            2 => Ok(Self::Trapped),
            3 => Ok(Self::Deoptimized),
            4 => Ok(Self::Panicked),
            code => Err(NativeExitError { code }),
        }
    }
}
