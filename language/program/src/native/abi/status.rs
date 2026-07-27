use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Native runtime operation status code.
pub type NativeRuntimeStatusCode = u32;

/// Native runtime operation status.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeRuntimeStatus {
    /// Native execution may continue.
    Continue = 0,
    /// The operation failed normally and native code should take its failure edge.
    Failed = 1,
    /// Native execution must return the exit kind stored in the context.
    Exit = 2,
}

/// Native runtime status code conversion error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeRuntimeStatusError {
    /// The invalid status code.
    pub code: NativeRuntimeStatusCode,
}

impl NativeRuntimeStatus {
    /// Return the native runtime status code.
    pub const fn code(self) -> NativeRuntimeStatusCode {
        self as NativeRuntimeStatusCode
    }
}

impl fmt::Display for NativeRuntimeStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid native runtime status code {}",
            self.code
        )
    }
}

impl Error for NativeRuntimeStatusError {}

impl TryFrom<NativeRuntimeStatusCode> for NativeRuntimeStatus {
    type Error = NativeRuntimeStatusError;

    fn try_from(code: NativeRuntimeStatusCode) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(Self::Continue),
            1 => Ok(Self::Failed),
            2 => Ok(Self::Exit),
            code => Err(NativeRuntimeStatusError { code }),
        }
    }
}
