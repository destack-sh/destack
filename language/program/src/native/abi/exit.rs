use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::NativeTrapCode;

/// Native entry exit discriminant.
pub type NativeExitCode = u32;

/// Native non-completion exit details.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeExit {
    /// Exit kind written by runtime operations that leave native execution.
    pub kind: NativeExitCode,
    /// Safepoint associated with stop or deoptimization.
    pub safepoint: u32,
    /// Trap code associated with trap exits.
    pub trap: NativeTrapCode,
}

/// Native entry exit kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeExitKind {
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
pub struct NativeExitError {
    /// The invalid status code.
    pub code: NativeExitCode,
}

#[allow(clippy::new_without_default)]
impl NativeExit {
    /// Create one completed native exit record.
    pub const fn new() -> Self {
        Self {
            kind: NativeExitKind::Completed.code(),
            safepoint: 0,
            trap: 0,
        }
    }

    /// Set this record to one language panic exit.
    pub fn panic(&mut self) -> NativeExitCode {
        self.kind = NativeExitKind::Panicked.code();

        self.kind
    }
}

impl NativeExitKind {
    /// Return the native ABI exit code.
    pub const fn code(self) -> NativeExitCode {
        self as NativeExitCode
    }
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
            1 => Ok(Self::Trapped),
            2 => Ok(Self::Deoptimized),
            3 => Ok(Self::Panicked),
            4 => Ok(Self::Stopped),
            code => Err(NativeExitError { code }),
        }
    }
}
