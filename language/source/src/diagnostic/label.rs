use serde::{Deserialize, Serialize};
use tspp_core::Blob;
use tspp_serde::Reflect;

use crate::{FileId, Span};

/// One resolved diagnostic location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum DiagnosticTarget {
    /// An exact source span.
    Span(Span),
    /// A whole source file.
    File(FileId),
}

impl DiagnosticTarget {
    /// Return whether this target is selected by one requested range.
    pub fn matches_range(self, range: Span) -> bool {
        match self {
            Self::Span(span) if range.is_empty() => span.owns_cursor(range.start),
            Self::Span(span) => span.intersects(range),
            Self::File(file) => file == range.file,
        }
    }

    /// Return the targeted file.
    pub fn file(&self) -> FileId {
        match self {
            Self::Span(span) => span.file,
            Self::File(file) => *file,
        }
    }

    /// Return the exact span when the target is one.
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Span(span) => Some(*span),
            Self::File(_) => None,
        }
    }
}

/// One concrete source label in a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct DiagnosticLabel {
    /// The exact Blob the target was recorded against.
    pub blob: Blob,
    /// The targeted source location.
    pub target: DiagnosticTarget,
    /// The optional label shown on the target.
    pub message: Option<String>,
}

impl DiagnosticLabel {
    /// Create a source label.
    pub fn new(blob: Blob, target: DiagnosticTarget) -> Self {
        Self {
            blob,
            target,
            message: None,
        }
    }

    /// Create a source label with one message.
    pub fn message(blob: Blob, target: DiagnosticTarget, message: impl Into<String>) -> Self {
        Self {
            blob,
            target,
            message: Some(message.into()),
        }
    }
}

/// Extra context for understanding a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
