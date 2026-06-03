use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::RootHandleId;

/// Daemon message severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonMessageKind {
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

/// Structured daemon message for protocol transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonMessageRecord {
    /// Message severity.
    pub kind: DaemonMessageKind,
    /// Stable message code.
    pub code: String,
    /// Human readable message.
    pub message: String,
    /// Optional path for the message.
    pub path: Option<PathBuf>,
}

/// Notification for daemon messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonMessageNotification {
    /// Root handle.
    pub handle: RootHandleId,
    /// Messages emitted by the daemon.
    pub messages: Vec<DaemonMessageRecord>,
}
