#![allow(clippy::new_without_default)]

use crate::{Diagnostic, Severity, SourceId, Suggestion};

/// A collector for diagnostics and suggestions.
#[derive(Debug)]
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

    /// Handle a Diagnostic.
    pub fn handle_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Has diagnostics of the given severity.
    pub fn has_diagnostics_of_severity(&self, severity: Severity) -> bool {
        self.diagnostics.iter().any(|d| d.severity == severity)
    }
}
