use serde::{Deserialize, Serialize};

use crate::{FileContentId, LabeledSpan, Span};

/// One concrete source label in a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagnosticLabel {
    /// The exact file content containing the span.
    pub content: FileContentId,
    /// The concrete source span.
    pub span: Span,
    /// The optional label shown on the span.
    pub message: Option<String>,
}

impl DiagnosticLabel {
    /// Create a source label.
    pub fn new(content: FileContentId, span: Span) -> Self {
        Self {
            content,
            span,
            message: None,
        }
    }

    /// Create a source label with one message.
    pub fn message(content: FileContentId, span: Span, message: impl Into<String>) -> Self {
        Self {
            content,
            span,
            message: Some(message.into()),
        }
    }

    /// Create a labeled span for rendering.
    pub fn to_labeled_span(&self, fallback: &str) -> LabeledSpan {
        let label = self.message.clone().unwrap_or_else(|| fallback.to_string());

        LabeledSpan::new(self.span, label)
    }
}

/// Extra context for understanding a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagnosticNote {
    /// The note message.
    pub message: String,
}

impl DiagnosticNote {
    /// Create one diagnostic note.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<String> for DiagnosticNote {
    /// Build one diagnostic note from a message.
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for DiagnosticNote {
    /// Build one diagnostic note from a message.
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

/// Guidance for fixing or avoiding a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagnosticHelp {
    /// The help message.
    pub message: String,
}

impl DiagnosticHelp {
    /// Create one diagnostic help.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<String> for DiagnosticHelp {
    /// Build one diagnostic help from a message.
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for DiagnosticHelp {
    /// Build one diagnostic help from a message.
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}
