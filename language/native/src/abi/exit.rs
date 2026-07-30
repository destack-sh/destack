use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::TrapCode;

/// Native entry exit discriminant.
pub type ExitCode = u32;

/// Native non-completion exit details.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exit {
    /// Exit kind written by runtime operations that leave native execution.
    pub kind: ExitCode,
    /// Safepoint associated with stop or deoptimization.
    pub safepoint: u32,
    /// Trap code associated with trap exits.
    pub trap: TrapCode,
}

/// Native entry exit kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitKind {
    /// Execution completed normally.
    Completed = 0,
    /// Execution trapped.
    Trapped = 1,
    /// Execution deoptimized into interpreter state.
    Deoptimized = 2,
    /// Execution stopped with a language panic.
    Panicked = 3,
    /// Execution stopped for host inspection.
    Stopped = 4,
}

/// Native exit code conversion error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitError {
    /// The invalid status code.
    pub code: ExitCode,
}

#[allow(clippy::new_without_default)]
impl Exit {
    /// Create one completed native exit record.
    pub const fn new() -> Self {
        Self {
            kind: ExitKind::Completed.code(),
            safepoint: 0,
            trap: 0,
        }
    }

    /// Set this record to one language panic exit.
    pub fn panic(&mut self) -> ExitCode {
        self.kind = ExitKind::Panicked.code();

        self.kind
    }
}

impl ExitKind {
    /// Return the native ABI exit code.
    pub const fn code(self) -> ExitCode {
        self as ExitCode
    }
}

impl fmt::Display for ExitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid native exit code {}", self.code)
    }
}

impl Error for ExitError {}

impl TryFrom<ExitCode> for ExitKind {
    type Error = ExitError;

    fn try_from(code: ExitCode) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(Self::Completed),
            1 => Ok(Self::Trapped),
            2 => Ok(Self::Deoptimized),
            3 => Ok(Self::Panicked),
            4 => Ok(Self::Stopped),
            code => Err(ExitError { code }),
        }
    }
}
