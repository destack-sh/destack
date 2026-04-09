use std::sync::Arc;

use destack_session::FileUpdate;
use destack_source::{Diagnostic, File, Uri};

/// Reason for a workspace reload request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadReason {
    /// Requested during startup synchronization.
    Startup,
    /// Requested after a watcher overflow.
    Overflow,
    /// Requested manually.
    Manual,
    /// Requested after watch updates.
    Update,
}

/// Diagnostic snapshot for one document path.
#[derive(Debug, Clone)]
pub struct DocumentDiagnosticSnapshot {
    /// The current file snapshot used for range conversion.
    pub file: Arc<File>,
    /// The diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Diagnostic snapshot for one workspace file.
#[derive(Debug, Clone)]
pub struct WorkspaceDiagnosticSnapshot {
    /// The current file snapshot used for range conversion.
    pub file: Arc<File>,
    /// Publish uri for this snapshot.
    pub publish_uri: Uri,
    /// Publish version for this snapshot when present.
    pub publish_version: Option<i32>,
    /// The diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Message severity for one language service operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageServiceMessageKind {
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

/// Message payload emitted by one language service operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageServiceMessage {
    /// Message severity.
    pub kind: LanguageServiceMessageKind,
    /// Stable message code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

impl LanguageServiceMessage {
    /// Build one warning message payload.
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: LanguageServiceMessageKind::Warning,
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Result of applying local workspace service updates.
#[derive(Debug, Default)]
pub struct LanguageServiceResult {
    /// Update records produced by the operation.
    pub updates: Vec<FileUpdate>,
    /// Message records produced by the operation.
    pub messages: Vec<LanguageServiceMessage>,
}

impl From<Vec<FileUpdate>> for LanguageServiceResult {
    fn from(updates: Vec<FileUpdate>) -> Self {
        Self {
            updates,
            messages: Vec::new(),
        }
    }
}
