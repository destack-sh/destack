use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{DiagnosticLabel, PatchSet};

/// Whether a suggestion can be applied automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Applicability {
    /// The suggestion is machine-applicable.
    Automatic,
    /// The suggestion is machine-applicable but may change behavior.
    Unsafe,
    /// The suggestion is maybe incorrect.
    Dangerous,
}

/// One suggested source change for a diagnostic.
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Reflect)]
pub struct DiagnosticSuggestion {
    /// Exact source patches for machine application.
    pub patches: PatchSet,
    /// Source labels to show with the suggestion.
    pub labels: Vec<DiagnosticLabel>,
    /// The message of the suggestion.
    pub message: String,
    /// The applicability of the suggestion.
    pub applicability: Applicability,
}

impl DiagnosticSuggestion {
    /// Create one diagnostic suggestion.
    pub fn new(
        message: impl Into<String>,
        patches: PatchSet,
        applicability: Applicability,
    ) -> Self {
        Self {
            patches,
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
