use serde::{Deserialize, Serialize};

use crate::{
    DiagnosticHelp, DiagnosticLabel, DiagnosticNote, DiagnosticSeverity, DiagnosticSuggestion,
    DiagnosticTag, Span,
};

/// A final renderable diagnostic.
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct Diagnostic {
    /// The stable identifier of the diagnostic (like `E001` or `W017`).
    pub code: String,
    /// The DiagnosticSeverity of the diagnostic.
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
    /// Extra semantic tags.
    pub tags: Vec<DiagnosticTag>,
}

impl Diagnostic {
    /// Create one diagnostic.
    pub fn new(
        code: impl Into<String>,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
        primary: DiagnosticLabel,
    ) -> Self {
        Self {
            code: code.into(),
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
        code: impl Into<String>,
        message: impl Into<String>,
        primary: DiagnosticLabel,
    ) -> Self {
        Self::new(code, DiagnosticSeverity::Error, message, primary)
    }

    /// Create one warning diagnostic.
    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        primary: DiagnosticLabel,
    ) -> Self {
        Self::new(code, DiagnosticSeverity::Warning, message, primary)
    }

    /// Return the primary source label.
    pub fn primary_label(&self) -> &DiagnosticLabel {
        &self.primary
    }

    /// Return the primary source span.
    pub fn primary_span(&self) -> Span {
        self.primary.span
    }

    /// Add one source label.
    pub fn label(mut self, label: DiagnosticLabel) -> Self {
        self.labels.push(label);

        self
    }

    /// Add one note.
    pub fn note(mut self, message: impl Into<String>) -> Self {
        self.notes.push(DiagnosticNote::new(message));

        self
    }

    /// Add one help message.
    pub fn help(mut self, message: impl Into<String>) -> Self {
        self.helps.push(DiagnosticHelp::new(message));

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

    /// Return note messages.
    pub fn notes(&self) -> impl Iterator<Item = &DiagnosticNote> {
        self.notes.iter()
    }

    /// Return help messages.
    pub fn helps(&self) -> impl Iterator<Item = &DiagnosticHelp> {
        self.helps.iter()
    }
}
