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
    /// Frame map associated with one non-completion exit.
    pub frame_map: u32,
    /// Trap code associated with trap exits.
    pub trap: TrapCode,
}

/// Native entry exit kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitKind {
    /// Execution completed normally.
    Completed = 0,
    /// Execution completed through cancellation cleanup.
    Cancelled = 1,
    /// Execution awaited one asynchronous value.
    Awaited = 2,
    /// Execution yielded one generator value.
    Yielded = 3,
    /// Execution stopped with a language panic.
    Panicked = 4,
    /// Execution stopped for host inspection.
    Stopped = 5,
    /// Execution deoptimized into bytecode state.
    Deoptimized = 6,
    /// Execution trapped.
    Trapped = 7,
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
            frame_map: 0,
            trap: 0,
        }
    }

    /// Set this record to one language panic exit.
    pub fn panic(&mut self) -> ExitCode {
        self.kind = ExitKind::Panicked.code();

        self.kind
    }

    /// Set this record to one debugger stop exit.
    pub fn stop(&mut self, frame_map: u32) -> ExitCode {
        self.kind = ExitKind::Stopped.code();
        self.frame_map = frame_map;

        self.kind
    }

    /// Set this record to one deoptimization exit.
    pub fn deoptimize(&mut self, frame_map: u32) -> ExitCode {
        self.kind = ExitKind::Deoptimized.code();
        self.frame_map = frame_map;

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
            1 => Ok(Self::Cancelled),
            2 => Ok(Self::Awaited),
            3 => Ok(Self::Yielded),
            4 => Ok(Self::Panicked),
            5 => Ok(Self::Stopped),
            6 => Ok(Self::Deoptimized),
            7 => Ok(Self::Trapped),
            code => Err(ExitError { code }),
        }
    }
}
