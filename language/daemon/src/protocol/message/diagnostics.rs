use serde::{Deserialize, Serialize};

use destack_source::{Diagnostic, FileId};

use super::RootHandleId;

/// Diagnostic batch for notifications.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticBatch {
    /// The file id for these diagnostics.
    pub file_id: FileId,
    /// Diagnostics for the file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Notification for diagnostics updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticsNotification {
    /// Root handle.
    pub handle: RootHandleId,
    /// Diagnostics grouped by file.
    pub diagnostics: Vec<DiagnosticBatch>,
}
