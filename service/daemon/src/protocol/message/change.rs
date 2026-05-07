use serde::{Deserialize, Serialize};

use destack_source::{Diagnostic, FileId, ModuleId};

use super::FileUpdateImage;

/// Record of one file change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateChangeSummary {
    /// The file id for this change.
    pub file_id: FileId,
    /// File change kind.
    pub kind: UpdateChangeKind,
}

/// File change kind classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateChangeKind {
    /// Destack config change.
    Destack,
    /// Unknown change.
    Unknown,
}

/// Daemon update record for protocol responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonUpdateRecord {
    /// Module id for the update.
    pub module_id: Option<ModuleId>,
    /// File id for the update.
    pub file_id: FileId,
    /// File image when the updated file still exists.
    pub file: Option<FileUpdateImage>,
    /// File change summary.
    pub change: UpdateChangeSummary,
    /// Diagnostics produced by the update.
    pub diagnostics: Vec<Diagnostic>,
}
