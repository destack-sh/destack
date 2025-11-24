use dyst_ast::StringPool;
use dyst_source::{DiagnosticCollector, FileRegistry, FileSystem, LanguageOptions};

use crate::{GlobalScopeId, ModuleRegistry, PackageRegistry};

/// A Program.
#[derive(Debug)]
pub struct Program<'a> {
    /// The language options.
    pub language: LanguageOptions,
    /// The file system.
    pub fs: &'a dyn FileSystem,
    /// The files in the program.
    pub files: &'a FileRegistry,
    /// The root scope.
    pub root_scope_id: Option<GlobalScopeId>,
    /// The modules.
    pub modules: ModuleRegistry,
    /// The packages.
    pub packages: PackageRegistry,
    /// The combined string pool.
    pub strings: StringPool,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
}

impl<'a> Program<'a> {
    /// Create a new Program.
    pub fn new(language: LanguageOptions, fs: &'a dyn FileSystem, files: &'a FileRegistry) -> Self {
        Self {
            language,
            fs,
            files,
            root_scope_id: None,
            modules: ModuleRegistry::new(),
            packages: PackageRegistry::new(),
            strings: StringPool::new(),
            diagnostics: DiagnosticCollector::new(),
        }
    }
}
