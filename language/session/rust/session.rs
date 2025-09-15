use std::cell::{Ref, RefCell};

use dyst_language_diagnostic::Diagnostic;
use dyst_language_source::{Path, PathId, PathPool, StringId, StringPool};

/// A session for diagnostic operations.
#[derive(Debug)]
pub struct Session {
    /// The string pool.
    pub strings: RefCell<StringPool>,
    /// The path pool.
    pub paths: RefCell<PathPool>,

    /// The diagnostics emitted in this session.
    pub diagnostics: RefCell<Vec<Diagnostic>>,
}

impl Session {
    /// Create a new session.
    pub fn new() -> Self {
        Self {
            strings: RefCell::new(StringPool::new()),
            paths: RefCell::new(PathPool::new()),
            diagnostics: RefCell::new(Vec::new()),
        }
    }

    /// Handle a Diagnostic.
    pub fn handle_diagnostic(&mut self, diagnostic: Diagnostic) {
        let mut diagnostics = self.diagnostics.borrow_mut();
        if !diagnostics.contains(&diagnostic) {
            diagnostics.push(diagnostic);
        }
    }

    /// Intern a string.
    pub fn intern_string<S: AsRef<str>>(&mut self, string: S) -> StringId {
        self.strings.borrow_mut().intern(string)
    }

    /// Get an interned string.
    pub fn get_string(&self, string_id: StringId) -> Ref<'_, str> {
        Ref::map(self.strings.borrow(), move |p| p.get(string_id))
    }

    /// Intern a path.
    pub fn intern<T: AsRef<[StringId]>>(&mut self, segments: T) -> PathId {
        self.paths.borrow_mut().intern(segments)
    }

    /// Get an interned path.
    pub fn get_path(&self, path_id: PathId) -> Ref<'_, Path> {
        Ref::map(self.paths.borrow(), move |p| p.get(path_id))
    }
}
