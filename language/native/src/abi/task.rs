use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Native task outcome discriminant.
pub type TaskOutcomeCode = u32;

/// Native terminal task outcome.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskOutcome {
    /// The task completed with a typed value.
    Completed = 0,
    /// The task completed through cancellation.
    Cancelled = 1,
}

/// Native task outcome code conversion error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskOutcomeError {
    /// The invalid task outcome code.
    pub code: TaskOutcomeCode,
}

impl TaskOutcome {
    /// Return the native task outcome code.
    pub const fn code(self) -> TaskOutcomeCode {
        self as TaskOutcomeCode
    }
}

impl fmt::Display for TaskOutcomeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid native task outcome code {}", self.code)
    }
}

impl Error for TaskOutcomeError {}

impl TryFrom<TaskOutcomeCode> for TaskOutcome {
    type Error = TaskOutcomeError;

    fn try_from(code: TaskOutcomeCode) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(Self::Completed),
            1 => Ok(Self::Cancelled),
            code => Err(TaskOutcomeError { code }),
        }
    }
}
