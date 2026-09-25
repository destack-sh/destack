use std::iter;

use serde::{Deserialize, Serialize};
use tspp_core::Blob;
use tspp_serde::Reflect;

use crate::{
    DiagnosticHelp, DiagnosticLabel, DiagnosticNote, DiagnosticSeverity, DiagnosticSuggestion,
    DiagnosticTag,
};

/// A final renderable diagnostic.
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Reflect)]
pub struct Diagnostic {
    /// The canonical diagnostic id.
    pub id: String,
    /// The diagnostic severity.
    pub severity: DiagnosticSeverity,
    /// The message of the diagnostic.
    pub message: String,
    /// The main source label.
    pub primary: DiagnosticLabel,
    /// Additional source labels.
    pub labels: Vec<DiagnosticLabel>,
    /// Extra context for understanding the diagnostic.
    pub notes: Vec<DiagnosticNote>,
    /// Guidance for fixing or avoiding the diagnostic.
    pub helps: Vec<DiagnosticHelp>,
    /// The suggestions for the diagnostic.
    pub suggestions: Vec<DiagnosticSuggestion>,
    /// Extra diagnostic tags.
    pub tags: Vec<DiagnosticTag>,
}

/// One diagnostic occurrence in exact source content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct DiagnosticReference {
    /// The canonical diagnostic id.
    pub id: String,
    /// The primary diagnostic label.
    pub primary: DiagnosticLabel,
}

impl Diagnostic {
    /// Create one diagnostic.
    pub fn new(
        id: impl Into<String>,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
        primary: DiagnosticLabel,
    ) -> Self {
        Self {
            id: id.into(),
            severity,
            message: message.into(),
            primary,
            labels: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            suggestions: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Create one error diagnostic.
    pub fn error(
        id: impl Into<String>,
        message: impl Into<String>,
        primary: DiagnosticLabel,
    ) -> Self {
        Self::new(id, DiagnosticSeverity::Error, message, primary)
    }

    /// Create one warning diagnostic.
    pub fn warning(
        id: impl Into<String>,
        message: impl Into<String>,
        primary: DiagnosticLabel,
    ) -> Self {
        Self::new(id, DiagnosticSeverity::Warning, message, primary)
    }

    /// Return the primary source label.
    pub fn primary_label(&self) -> &DiagnosticLabel {
        &self.primary
    }

    /// Add one source label.
    pub fn label(mut self, label: DiagnosticLabel) -> Self {
        self.labels.push(label);
        self
    }

    /// Add one note.
    pub fn note(mut self, note: impl Into<DiagnosticNote>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add one help message.
    pub fn help(mut self, help: impl Into<DiagnosticHelp>) -> Self {
        self.helps.push(help.into());
        self
    }

    /// Add one suggestion.
    pub fn suggestion(mut self, suggestion: DiagnosticSuggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add one semantic tag.
    pub fn tag(mut self, tag: DiagnosticTag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Return all additional source labels.
    pub fn labels(&self) -> impl Iterator<Item = &DiagnosticLabel> {
        self.labels.iter()
    }

    /// Return every source Blob carried by this diagnostic's labels.
    pub fn blobs(&self) -> impl Iterator<Item = Blob> + '_ {
        iter::once(self.primary.blob)
            .chain(self.labels.iter().map(|label| label.blob))
            .chain(
                self.suggestions
                    .iter()
                    .flat_map(|suggestion| suggestion.labels.iter().map(|label| label.blob)),
            )
    }

    /// Return note messages.
    pub fn notes(&self) -> impl Iterator<Item = &DiagnosticNote> {
        self.notes.iter()
    }

    /// Return help messages.
    pub fn helps(&self) -> impl Iterator<Item = &DiagnosticHelp> {
        self.helps.iter()
    }
}

impl From<&Diagnostic> for DiagnosticReference {
    /// Build a reference to one exact emitted diagnostic.
    fn from(diagnostic: &Diagnostic) -> Self {
        Self {
            id: diagnostic.id.clone(),
            primary: diagnostic.primary.clone(),
        }
    }
}
