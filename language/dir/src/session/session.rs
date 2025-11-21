use dyst_ast::StringPool;
use dyst_source::{DiagnosticCollector, FileRegistry, LanguageOptions};

use crate::{LocalScopeId, ModuleRegistry, PackageRegistry};

/// A session for a language.
#[derive(Debug)]
pub struct Session<'a> {
    /// The language options.
    pub language: LanguageOptions,
    /// The files in the session.
    pub files: &'a FileRegistry,
    /// The root scope.
    pub root_scope_id: Option<LocalScopeId>,
    /// The modules.
    pub modules: ModuleRegistry,
    /// The packages.
    pub packages: PackageRegistry,
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
            root_scope_id: None,
            modules: ModuleRegistry::new(),
            packages: PackageRegistry::new(),
            diagnostics: DiagnosticCollector::new(),
            strings: StringPool::new(),
        }
    }
}
