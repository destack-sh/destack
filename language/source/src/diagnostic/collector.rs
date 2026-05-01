use std::collections::BTreeMap;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{Diagnostic, DiagnosticSeverity};

/// A collection of diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCollection {
    /// The diagnostics.
    diagnostics: Vec<Diagnostic>,
}

impl Default for DiagnosticCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticCollection {
    /// Create a new diagnostic collection.
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// Create a new diagnostic collection with the given diagnostics.
    pub fn from_diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    /// Add one diagnostic.
    pub fn insert(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Iterate over diagnostics.
    pub fn iter(&self) -> std::slice::Iter<'_, Diagnostic> {
        self.diagnostics.iter()
    }

    /// Clone the diagnostics into a vector.
    pub fn to_vec(&self) -> Vec<Diagnostic> {
        self.diagnostics.clone()
    }

    /// Get the number of diagnostics.
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    /// Whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Merge another diagnostic collection into this one.
    pub fn merge_from(&mut self, other: &DiagnosticCollection) {
        self.diagnostics.extend(other.diagnostics.iter().cloned());
    }

    /// Whether this collection has any diagnostics of the given DiagnosticSeverity.
    pub fn has_diagnostics_of_severity(&self, severity: DiagnosticSeverity) -> bool {
        self.diagnostics.iter().any(|d| d.severity == severity)
    }

    /// Get the highest severity of the diagnostics.
    pub fn highest_severity(&self) -> Option<DiagnosticSeverity> {
        self.diagnostics.iter().map(|d| d.severity).max()
    }

    /// Get the number of diagnostics by severity.
    pub fn count_diagnostics_by_severity(&self) -> BTreeMap<DiagnosticSeverity, usize> {
        let mut counts: BTreeMap<DiagnosticSeverity, usize> = BTreeMap::new();
        for diagnostic in self.diagnostics.iter() {
            *counts.entry(diagnostic.severity).or_insert(0) += 1;
        }
        counts
    }

    /// Get the status code expressing the severity of the diagnostics.
    pub fn get_status_code(&self) -> i32 {
        if self.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
            return 1;
        }
        if self.has_diagnostics_of_severity(DiagnosticSeverity::Warning) {
            return 2;
        }
        0
    }
}

/// Collector for diagnostics and suggestions.
/// Uses an internal Mutex for thread safety.
#[derive(Debug)]
pub struct DiagnosticCollector {
    collection: Mutex<DiagnosticCollection>,
}

impl Clone for DiagnosticCollector {
    fn clone(&self) -> Self {
        let state = self.collection.lock().clone();
        Self {
            collection: Mutex::new(state),
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
            collection: Mutex::new(DiagnosticCollection {
                diagnostics: Vec::new(),
            }),
        }
    }

    /// Add one diagnostic.
    pub fn insert(&self, diagnostic: Diagnostic) {
        self.collection.lock().diagnostics.push(diagnostic);
    }

    /// Merge another diagnostic collector into this one.
    pub fn merge_from(&self, other: &DiagnosticCollector) {
        let mut this_locked = self.collection.lock();
        let other_locked = other.collection.lock();
        this_locked
            .diagnostics
            .extend(other_locked.diagnostics.iter().cloned());
    }

    /// Take the diagnostics from another diagnostic collector.
    pub fn take_from(&self, other: &DiagnosticCollector) {
        let mut this_locked = self.collection.lock();
        let mut other_locked = other.collection.lock();
        this_locked
            .diagnostics
            .append(&mut other_locked.diagnostics);
    }

    /// Drain the diagnostics into a vector.
    pub fn drain(&self) -> Vec<Diagnostic> {
        let mut collection = self.collection.lock();
        collection.diagnostics.drain(..).collect()
    }

    /// Take the collected diagnostics as one owned collection.
    pub fn take_collection(&self) -> DiagnosticCollection {
        let mut collection = self.collection.lock();
        let diagnostics = std::mem::take(&mut collection.diagnostics);

        DiagnosticCollection::from_diagnostics(diagnostics)
    }

    /// Retain diagnostics that match the predicate.
    pub fn retain<F>(&self, mut predicate: F)
    where
        F: FnMut(&Diagnostic) -> bool,
    {
        let mut collection = self.collection.lock();
        collection
            .diagnostics
            .retain(|diagnostic| predicate(diagnostic));
    }

    /// Check if diagnostics of the given DiagnosticSeverity are present.
    pub fn has_diagnostics_of_severity(&self, severity: DiagnosticSeverity) -> bool {
        self.collection.lock().has_diagnostics_of_severity(severity)
    }

    /// Get the highest severity of the diagnostics.
    pub fn highest_severity(&self) -> Option<DiagnosticSeverity> {
        self.collection.lock().highest_severity()
    }

    /// Get the number of diagnostics by severity.
    pub fn count_diagnostics_by_severity(&self) -> BTreeMap<DiagnosticSeverity, usize> {
        self.collection.lock().count_diagnostics_by_severity()
    }

    /// Clone the diagnostics into a vector.
    pub fn to_vec(&self) -> Vec<Diagnostic> {
        self.collection.lock().diagnostics.clone()
    }

    /// Get the number of diagnostics.
    pub fn len(&self) -> usize {
        self.collection.lock().diagnostics.len()
    }

    /// Truncate diagnostics to the specified length.
    pub fn truncate(&self, len: usize) {
        self.collection.lock().diagnostics.truncate(len);
    }

    /// Whether the collector is empty.
    pub fn is_empty(&self) -> bool {
        self.collection.lock().diagnostics.is_empty()
    }

    /// Clone and get the collection of diagnostics.
    pub fn collect(&self) -> DiagnosticCollection {
        let collection = self.collection.lock();
        DiagnosticCollection {
            diagnostics: collection.diagnostics.clone(),
        }
    }
}
