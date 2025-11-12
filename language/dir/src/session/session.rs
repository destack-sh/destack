use dyst_ast::StringPool;
use dyst_source::{DiagnosticCollector, FileRegistry, LanguageOptions};

use crate::{ModuleRegistry, NodeTree};

/// A session for a language.
#[derive(Debug, Clone)]
pub struct Session<'a> {
    /// The language options.
    pub language: LanguageOptions,
    /// The files in the session.
    pub files: &'a FileRegistry,
    /// The modules.
    pub modules: ModuleRegistry,
    /// The combined DIR node tree.
    pub tree: NodeTree,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The combined string pool.
    pub strings: StringPool,
}

impl<'a> Session<'a> {
    /// Create a new Session.
    pub fn new(language: LanguageOptions, files: &'a FileRegistry) -> Self {
        Self {
            language,
            files,
            modules: ModuleRegistry::new(),
            tree: NodeTree::new(),
            diagnostics: DiagnosticCollector::new(),
            strings: StringPool::new(),
        }
    }
}
