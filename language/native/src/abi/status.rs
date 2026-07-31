use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Native runtime operation status code.
pub type RuntimeStatusCode = u32;

/// Native runtime operation status.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeStatus {
    /// Native execution may continue.
    Continue = 0,
    /// Native execution must return the kind stored in the activation exit record.
    Exit = 1,
}

/// Native runtime status code conversion error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeStatusError {
    /// The invalid status code.
    pub code: RuntimeStatusCode,
}

impl RuntimeStatus {
    /// Return the native runtime status code.
    pub const fn code(self) -> RuntimeStatusCode {
        self as RuntimeStatusCode
    }
}

impl fmt::Display for RuntimeStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid native runtime status code {}",
            self.code
        )
    }
}

impl Error for RuntimeStatusError {}

impl TryFrom<RuntimeStatusCode> for RuntimeStatus {
    type Error = RuntimeStatusError;

    fn try_from(code: RuntimeStatusCode) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(Self::Continue),
            1 => Ok(Self::Exit),
            code => Err(RuntimeStatusError { code }),
        }
    }
}
