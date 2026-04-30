use serde::{Deserialize, Serialize};

use super::RootHandleId;

/// Progress notification payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressNotification {
    /// Root handle.
    pub handle: RootHandleId,
    /// Progress event payload.
    pub event: ProgressEvent,
}

/// Progress event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Identifier for the ongoing task.
    pub task: String,
    /// Stage message for the task.
    pub message: Option<String>,
    /// Optional progress percent (0 to 100).
    pub percent: Option<u8>,
    /// Whether this event signals completion.
    pub done: bool,
}
