use std::path::{Path, PathBuf};

use destack_repository::Revision;
use destack_source::Uri;
use destack_workspace::{Commit, UpdateBatch};
use serde::{Deserialize, Serialize};

use super::{DaemonMessageRecord, DaemonUpdateRecord, DiagnosticBatch, RootHandleId};

/// Request to open a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenRootRequest {
    /// The workspace root owning this daemon root.
    pub workspace: PathBuf,
    /// The root path.
    pub root: PathBuf,
    /// Root open options.
    pub options: RootOpenOptions,
}

/// Options for opening a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootOpenOptions {
    /// Whether to preload root state.
    pub load_index: bool,
}

impl Default for RootOpenOptions {
    /// Return default root open options.
    fn default() -> Self {
        Self { load_index: true }
    }
}

/// Response to opening a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootOpenedResponse {
    /// Assigned root handle id.
    pub handle: RootHandleId,
    /// Diagnostics produced during initialization.
    pub diagnostics: Vec<DiagnosticBatch>,
    /// Messages produced during initialization.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Request to close a root handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloseRootRequest {
    /// Handle to close.
    pub handle: RootHandleId,
}

/// Response to closing a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootClosedResponse {
    /// Closed handle id.
    pub handle: RootHandleId,
}

/// Reason for a root reload request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReloadReason {
    /// Requested on startup.
    Startup,
    /// Requested after a watch overflow.
    Overflow,
    /// Requested by the caller.
    Manual,
    /// Requested after watch roots changed.
    Update,
}

/// Request to reload a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReloadRootRequest {
    /// Root handle.
    pub handle: RootHandleId,
    /// Reason for the reload.
    pub reason: ReloadReason,
}

/// Response to root reloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootReloadResponse {
    /// Root handle.
    pub handle: RootHandleId,
    /// Updates produced during reload.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced during reload.
    pub messages: Vec<DaemonMessageRecord>,
}

impl RootReloadResponse {
    /// Build a reload response from a workspace update batch.
    pub fn new(handle: RootHandleId, batch: &UpdateBatch) -> Self {
        Self {
            handle,
            updates: batch.updates.iter().map(DaemonUpdateRecord::from).collect(),
            messages: batch
                .messages
                .iter()
                .map(DaemonMessageRecord::from)
                .collect(),
        }
    }
}

/// Request to apply a file operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileOperationRequest {
    /// Root handle.
    pub handle: RootHandleId,
    /// Operation payload.
    pub operation: FileOperation,
}

/// Response to a file operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileOperationResponse {
    /// Root handle.
    pub handle: RootHandleId,
    /// Updates produced by the change.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced by the change.
    pub messages: Vec<DaemonMessageRecord>,
}

impl FileOperationResponse {
    /// Build a file operation response from a workspace update batch.
    pub fn new(handle: RootHandleId, batch: &UpdateBatch) -> Self {
        Self {
            handle,
            updates: batch.updates.iter().map(DaemonUpdateRecord::from).collect(),
            messages: batch
                .messages
                .iter()
                .map(DaemonMessageRecord::from)
                .collect(),
        }
    }
}

/// Request to apply a source update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceUpdateRequest {
    /// Root handle.
    pub handle: RootHandleId,
    /// Source update payload.
    pub update: SourceUpdate,
}

/// Response to a source update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceUpdateResponse {
    /// Root handle.
    pub handle: RootHandleId,
    /// Previous revision.
    pub before: Revision,
    /// Updated revision.
    pub after: Revision,
    /// Updates produced by the change.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced by the change.
    pub messages: Vec<DaemonMessageRecord>,
}

impl SourceUpdateResponse {
    /// Build a source update response from a workspace source update.
    pub fn new(handle: RootHandleId, update: &Commit) -> Self {
        Self {
            handle,
            before: update.before,
            after: update.after,
            updates: update
                .updates
                .iter()
                .map(DaemonUpdateRecord::from)
                .collect(),
            messages: update
                .messages
                .iter()
                .map(DaemonMessageRecord::from)
                .collect(),
        }
    }
}

/// Source update payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceUpdate {
    /// Expected base revision.
    pub base: Option<Revision>,
    /// Source edits in this atomic update.
    pub edits: Vec<SourceEdit>,
}

/// Source edit payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceEdit {
    /// Replace with text content.
    SetText {
        /// Path being updated.
        path: PathBuf,
        /// Full text content.
        text: String,
    },
    /// Apply text edits.
    EditText {
        /// Path being updated.
        path: PathBuf,
        /// Text edits.
        edits: Vec<TextEdit>,
    },
    /// Replace with binary content.
    SetBytes {
        /// Path being updated.
        path: PathBuf,
        /// Full binary content.
        bytes: Vec<u8>,
    },
    /// Remove one file.
    Remove {
        /// Path being removed.
        path: PathBuf,
    },
    /// Move one file.
    Move {
        /// Source path.
        from: PathBuf,
        /// Destination path.
        to: PathBuf,
    },
}

/// Source text edit payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    /// Replaced text range.
    pub range: TextRange,
    /// Replacement text.
    pub text: String,
}

/// Source text range payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

/// File operation payload from a protocol client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileOperation {
    /// Open editor text content.
    OpenText {
        /// Path being opened.
        path: PathBuf,
        /// Editor document URI.
        uri: Uri,
        /// Editor document version.
        version: i32,
        /// Current text content.
        content: String,
    },
    /// Open editor binary content.
    OpenBytes {
        /// Path being opened.
        path: PathBuf,
        /// Editor document URI.
        uri: Uri,
        /// Editor document version.
        version: i32,
        /// Current binary content.
        content: Vec<u8>,
    },
    /// Change editor text content.
    ChangeText {
        /// Path being changed.
        path: PathBuf,
        /// Editor document URI.
        uri: Uri,
        /// Editor document version.
        version: i32,
        /// Current text content.
        content: String,
    },
    /// Change editor binary content.
    ChangeBytes {
        /// Path being changed.
        path: PathBuf,
        /// Editor document URI.
        uri: Uri,
        /// Editor document version.
        version: i32,
        /// Current binary content.
        content: Vec<u8>,
    },
    /// Save editor text content.
    SaveText {
        /// Path being saved.
        path: PathBuf,
        /// Current text content.
        content: Option<String>,
    },
    /// Save editor binary content.
    SaveBytes {
        /// Path being saved.
        path: PathBuf,
        /// Current binary content.
        content: Option<Vec<u8>>,
    },
    /// Close editor overlay state and restore filesystem truth.
    Close {
        /// Path being closed.
        path: PathBuf,
    },
    /// Write text content to disk and workspace state.
    WriteText {
        /// Path being written.
        path: PathBuf,
        /// Current text content.
        content: String,
    },
    /// Write binary content to disk and workspace state.
    WriteBytes {
        /// Path being written.
        path: PathBuf,
        /// Current binary content.
        content: Vec<u8>,
    },
    /// Remove a file from disk and workspace state.
    Remove {
        /// Path being removed.
        path: PathBuf,
    },
}

impl FileOperation {
    /// Return the source path for this operation.
    pub fn path(&self) -> &Path {
        match self {
            Self::OpenText { path, .. }
            | Self::OpenBytes { path, .. }
            | Self::ChangeText { path, .. }
            | Self::ChangeBytes { path, .. }
            | Self::SaveText { path, .. }
            | Self::SaveBytes { path, .. }
            | Self::Close { path }
            | Self::WriteText { path, .. }
            | Self::WriteBytes { path, .. }
            | Self::Remove { path } => path,
        }
    }
}
