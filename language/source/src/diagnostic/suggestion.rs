use serde::{Deserialize, Serialize};

use crate::{BatchEdit, DiagnosticLabel};

/// Whether a suggestion can be applied automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Applicability {
    /// The suggestion is machine-applicable.
    Automatic,
    /// The suggestion is machine-applicable but may change behavior.
    Unsafe,
    /// The suggestion is maybe incorrect.
    Dangerous,
}

/// One suggested source change for a diagnostic.
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct DiagnosticSuggestion {
    /// Exact source edits for machine application.
    pub edits: BatchEdit,
    /// Source labels to show with the suggestion.
    pub labels: Vec<DiagnosticLabel>,
    /// The message of the suggestion.
    pub message: String,
    /// The applicability of the suggestion.
    pub applicability: Applicability,
}

impl DiagnosticSuggestion {
    /// Create one diagnostic suggestion.
    pub fn new(message: impl Into<String>, edits: BatchEdit, applicability: Applicability) -> Self {
        Self {
            edits,
            labels: Vec::new(),
            message: message.into(),
            applicability,
        }
    }

    /// Add one source label.
    pub fn label(mut self, label: DiagnosticLabel) -> Self {
        self.labels.push(label);

        self
    }
}
