use dyst_ast::StringPool;
use dyst_source::{DiagnosticCollector, FileRegistry, LanguageOptions};

use crate::{GlobalScopeId, ModuleRegistry, PackageRegistry};

/// A Program.
#[derive(Debug)]
pub struct Program<'a> {
    /// The language options.
    pub language: LanguageOptions,
    /// The files in the program.
    pub files: &'a FileRegistry,
    /// The root scope.
    pub root_scope_id: Option<GlobalScopeId>,
    /// The modules.
    pub modules: ModuleRegistry,
    /// The packages.
    pub packages: PackageRegistry,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The combined string pool.
    pub strings: StringPool,
}

impl<'a> Program<'a> {
    /// Create a new Program.
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
