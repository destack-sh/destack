#![allow(clippy::new_without_default)]

use crate::{Diagnostic, Severity, SourceId};

/// A session for language operations.
#[derive(Debug)]
pub struct Session {
    /// The diagnostics emitted in this session.
    pub diagnostics: Vec<Diagnostic>,
}

impl Session {
    /// Create a new session.
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// Handle a Diagnostic.
    pub fn handle_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Reset all diagnostics.
    pub fn reset_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

    /// Reset diagnostics for a source.
    pub fn reset_diagnostics_for_source(&mut self, source: SourceId) {
        self.diagnostics.retain(|d| d.source != source);
    }

    /// Has diagnostics of the given severity.
    pub fn has_diagnostics_of_severity(&self, severity: Severity) -> bool {
        self.diagnostics.iter().any(|d| d.severity == severity)
    }

    /// Get diagnostics for a source.
    pub fn get_diagnostics_for_source(&self, source: SourceId) -> Vec<Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.source == source)
            .cloned()
            .collect()
    }

    /// Get diagnostics for a predicate.
    pub fn get_diagnostics_for_predicate(
        &self,
        predicate: impl Fn(&Diagnostic) -> bool,
    ) -> Vec<Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| predicate(d))
            .cloned()
            .collect()
    }
}
