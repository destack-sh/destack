#![allow(clippy::new_without_default)]

use crate::{Diagnostic, DiagnosticSeverity, Suggestion};

/// A collector for diagnostics and suggestions.
#[derive(Debug, Clone)]
pub struct DiagnosticCollector {
    /// The collected diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// The suggestions for the diagnostics.
    pub suggestions: Vec<Suggestion>,
}

impl DiagnosticCollector {
    /// Create a new diagnostic collector.
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Whether the collector has any diagnostics.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Add a Diagnostic.
    pub fn insert_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Merge another diagnostic collector into this one.
    pub fn merge_from(&mut self, other: &DiagnosticCollector) {
        self.diagnostics.extend(other.diagnostics.iter().cloned());
        self.suggestions.extend(other.suggestions.iter().cloned());
    }

    /// Has diagnostics of the given DiagnosticSeverity.
    pub fn has_diagnostics_of_severity(&self, severity: DiagnosticSeverity) -> bool {
        self.diagnostics.iter().any(|d| d.severity == severity)
    }

    /// Add a Suggestion.
    pub fn insert_suggestion(&mut self, suggestion: Suggestion) {
        self.suggestions.push(suggestion);
    }

    /// Convert the Collector into its diagnostics.
    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// Get an iterator over diagnostics.
    pub fn iter(&self) -> std::slice::Iter<'_, Diagnostic> {
        self.diagnostics.iter()
    }
}
