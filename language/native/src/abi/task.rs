use serde::{Deserialize, Serialize};

/// Native terminal task outcome.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskOutcome {
    /// The task completed with a typed value.
    Completed = 0,
    /// The task completed through cancellation.
    Cancelled = 1,
}
