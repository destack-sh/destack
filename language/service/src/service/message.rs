use super::FileUpdate;

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

/// Result of applying local language service updates.
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
