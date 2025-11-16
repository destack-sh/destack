use std::collections::BTreeMap;

use parking_lot::Mutex;

use crate::{Diagnostic, DiagnosticSeverity};

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
            }),
        }
    }

    /// Add a Diagnostic.
    pub fn insert(&self, diagnostic: Diagnostic) {
        self.inner.lock().diagnostics.push(diagnostic);
    }

    /// Merge another diagnostic collector into this one.
    pub fn merge_from(&self, other: &DiagnosticCollector) {
        let mut this_locked = self.inner.lock();
        let other_locked = other.inner.lock();
        this_locked
            .diagnostics
            .extend(other_locked.diagnostics.iter().cloned());
    }

    /// Check if diagnostics of the given DiagnosticSeverity are present.
    pub fn has_diagnostics_of_severity(&self, severity: DiagnosticSeverity) -> bool {
        self.inner
            .lock()
            .diagnostics
            .iter()
            .any(|d| d.severity == severity)
    }

    /// Get the highest severity of the diagnostics.
    pub fn highest_severity(&self) -> Option<DiagnosticSeverity> {
        self.inner
            .lock()
            .diagnostics
            .iter()
            .map(|d| d.severity)
            .max()
    }

    /// Get the number of diagnostics by severity.
    pub fn count_diagnostics_by_severity(&self) -> BTreeMap<DiagnosticSeverity, usize> {
        let mut counts: BTreeMap<DiagnosticSeverity, usize> = BTreeMap::new();
        for diagnostic in self.inner.lock().diagnostics.iter() {
            *counts.entry(diagnostic.severity).or_insert(0) += 1;
        }
        counts
    }

    /// Get a vector clone of diagnostics.
    /// NOTE: This method clones the underlying vector.
    pub fn iter(&self) -> Vec<Diagnostic> {
        self.inner.lock().diagnostics.clone()
    }

    /// Get the number of diagnostics.
    pub fn len(&self) -> usize {
        self.inner.lock().diagnostics.len()
    }

    /// Whether the collector is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().diagnostics.is_empty()
    }
}
