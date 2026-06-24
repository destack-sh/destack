use crate::FileUpdate;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Message severity for one workspace operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum MessageKind {
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

/// Message payload emitted by one workspace operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Message {
    /// Message severity.
    pub kind: MessageKind,
    /// Stable message code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

impl Message {
    /// Build one informational message payload.
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Info,
            code: code.into(),
            message: message.into(),
        }
    }

    /// Build one warning message payload.
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Warning,
            code: code.into(),
            message: message.into(),
        }
    }

    /// Build one error message payload.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Error,
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Result of applying local workspace updates.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct UpdateBatch {
    /// Update records produced by the operation.
    pub updates: Vec<FileUpdate>,
    /// Message records produced by the operation.
    pub messages: Vec<Message>,
}

impl From<Vec<FileUpdate>> for UpdateBatch {
    fn from(updates: Vec<FileUpdate>) -> Self {
        Self {
            updates,
            messages: Vec::new(),
        }
    }
}
