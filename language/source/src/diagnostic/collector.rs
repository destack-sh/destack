use std::collections::{BTreeMap, HashMap};
use tspp_serde::Reflect;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{Diagnostic, DiagnosticSeverity, DiagnosticTarget, FileId};

/// A collection of diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DiagnosticCollection {
    /// The diagnostics.
    pub diagnostics: Vec<Diagnostic>,
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

    /// Group diagnostics by their primary source file.
    pub fn group_by_file(self) -> HashMap<FileId, Vec<Diagnostic>> {
        let mut files = HashMap::<FileId, Vec<Diagnostic>>::new();

        // move each diagnostic into its primary file
        for diagnostic in self.diagnostics {
            let file_id = diagnostic.primary_label().target.file();
            files.entry(file_id).or_default().push(diagnostic);
        }

        files
    }

    /// Sort diagnostics by canonical file order and primary source location.
    pub fn sort_by_source(&mut self, files: &[FileId]) {
        // index each file's canonical position
        let mut file_order = BTreeMap::new();
        for (index, file) in files.iter().copied().enumerate() {
            file_order.entry(file).or_insert(index);
        }

        // order completed diagnostics by their primary targets
        self.diagnostics.sort_by(|left, right| {
            let left_file = left.primary.target.file();
            let right_file = right.primary.target.file();
            let left_rank = file_order.get(&left_file).copied().unwrap_or(usize::MAX);
            let right_rank = file_order.get(&right_file).copied().unwrap_or(usize::MAX);
            let left_location = diagnostic_location(left.primary.target);
            let right_location = diagnostic_location(right.primary.target);

            left_rank
                .cmp(&right_rank)
                .then_with(|| left_file.cmp(&right_file))
                .then_with(|| left_location.cmp(&right_location))
                .then_with(|| right.severity.cmp(&left.severity))
                .then_with(|| left.id.cmp(&right.id))
                .then_with(|| left.message.cmp(&right.message))
        });
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

/// Return the source-local ordering key for one diagnostic target.
fn diagnostic_location(target: DiagnosticTarget) -> (u8, u32, u32) {
    match target {
        DiagnosticTarget::File(_) => (0, 0, 0),
        DiagnosticTarget::Span(span) => (1, span.start, span.end),
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
