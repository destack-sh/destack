#![allow(clippy::new_without_default)]

use dyst_diagnostic::{Diagnostic, Severity};
use dyst_source::{Path, PathId, PathPool, SourceId, StringId, StringPool};

/// A session for diagnostic operations.
#[derive(Debug)]
pub struct Session {
    /// The string pool.
    pub strings: StringPool,
    /// The path pool.
    pub paths: PathPool,

    /// The diagnostics emitted in this session.
    pub diagnostics: Vec<Diagnostic>,
}

impl Session {
    /// Create a new session.
    pub fn new() -> Self {
        Self {
            strings: StringPool::new(),
            paths: PathPool::new(),
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

    /// Intern a string.
    pub fn intern_string<S: AsRef<str>>(&mut self, string: S) -> StringId {
        self.strings.intern(string)
    }

    /// Get an interned string.
    pub fn get_string(&self, string_id: StringId) -> &str {
        self.strings.get(string_id)
    }

    /// Intern a path.
    pub fn intern<T: AsRef<[StringId]>>(&mut self, segments: T) -> PathId {
        self.paths.intern(segments)
    }

    /// Get an interned path.
    pub fn get_path(&self, path_id: PathId) -> &Path {
        self.paths.get(path_id)
    }
}
