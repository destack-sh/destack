use serde::{Deserialize, Serialize};

use crate::{LabeledSpan, Span};

/// One concrete source label in a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagnosticLabel {
    /// The concrete source span.
    pub span: Span,
    /// The optional label shown on the span.
    pub message: Option<String>,
}

impl DiagnosticLabel {
    /// Create a source label.
    pub fn new(span: Span) -> Self {
        Self {
            span,
            message: None,
        }
    }

    /// Create a source label with one message.
    pub fn message(span: Span, message: impl Into<String>) -> Self {
        Self {
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
