use serde::{Deserialize, Serialize};

use super::{
    CommandOutputNotification, DaemonMessageNotification, DiagnosticsNotification,
    PayloadChunkNotification, ProgressNotification, RuntimeOutputNotification,
};

/// Notifications emitted by the daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonNotification {
    /// Publish diagnostics for a workspace.
    Diagnostics(DiagnosticsNotification),
    /// Publish daemon messages.
    Messages(DaemonMessageNotification),
    /// Publish progress updates.
    Progress(ProgressNotification),
    /// Publish chunked payload data.
    PayloadChunk(PayloadChunkNotification),
    /// Publish command output.
    CommandOutput(CommandOutputNotification),
    /// Publish runtime output.
    RuntimeOutput(RuntimeOutputNotification),
}
