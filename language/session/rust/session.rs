#![allow(clippy::new_without_default)]

use dyst_language_diagnostic::Diagnostic;
use dyst_language_source::{Path, PathId, PathPool, StringId, StringPool};

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
