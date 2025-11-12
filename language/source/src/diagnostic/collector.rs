use parking_lot::Mutex;

use crate::{Diagnostic, DiagnosticSeverity, Suggestion};

/// Collector for diagnostics and suggestions.
/// Uses an internal Mutex for thread safety.
#[derive(Debug)]
pub struct DiagnosticCollector {
    inner: Mutex<DiagnosticCollection>,
}

#[derive(Debug, Clone)]
struct DiagnosticCollection {
    /// The diagnostics.
    diagnostics: Vec<Diagnostic>,
    /// The suggestions.
    suggestions: Vec<Suggestion>,
}

impl Clone for DiagnosticCollector {
    fn clone(&self) -> Self {
        let state = self.inner.lock().clone();
        Self {
            inner: Mutex::new(state),
        }
    }
}

impl Default for DiagnosticCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticCollector {
    /// Create a new diagnostic collector.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(DiagnosticCollection {
                diagnostics: Vec::new(),
                suggestions: Vec::new(),
            }),
        }
    }

    /// Check whether the collector has any diagnostics.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().diagnostics.is_empty()
    }

    /// Add a Diagnostic.
    pub fn insert_diagnostic(&self, diagnostic: Diagnostic) {
        self.inner.lock().diagnostics.push(diagnostic);
    }

    /// Merge another diagnostic collector into this one.
    pub fn merge_from(&self, other: &DiagnosticCollector) {
        let mut this_locked = self.inner.lock();
        let other_locked = other.inner.lock();
        this_locked
            .diagnostics
            .extend(other_locked.diagnostics.iter().cloned());
        this_locked
            .suggestions
            .extend(other_locked.suggestions.iter().cloned());
    }

    /// Check if diagnostics of the given DiagnosticSeverity are present.
    pub fn has_diagnostics_of_severity(&self, severity: DiagnosticSeverity) -> bool {
        self.inner
            .lock()
            .diagnostics
            .iter()
            .any(|d| d.severity == severity)
    }

    /// Add a Suggestion.
    pub fn insert_suggestion(&self, suggestion: Suggestion) {
        self.inner.lock().suggestions.push(suggestion);
    }

    /// Convert the Collector into its diagnostics (move out).
    pub fn into_vec(self) -> Vec<Diagnostic> {
        // TODO @Robustness: This consumes the collector and exposes diagnostics only,
        // so suggestions will be dropped.
        self.inner.into_inner().diagnostics
    }

    /// Get a vector clone of diagnostics.
    /// NOTE: This method clones the underlying vector.
    pub fn iter(&self) -> Vec<Diagnostic> {
        self.inner.lock().diagnostics.clone()
    }
}
