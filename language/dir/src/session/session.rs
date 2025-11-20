use dyst_ast::StringPool;
use dyst_source::{DiagnosticCollector, DiagnosticSeverity, FileRegistry, LanguageOptions};
use parking_lot::RwLock;

use crate::{ModuleRegistry, NodeTree, PackageRegistry, ScopeId};

/// A session for a language.
#[derive(Debug)]
pub struct Session<'a> {
    /// The language options.
    pub language: LanguageOptions,
    /// The files in the session.
    pub files: &'a FileRegistry,
    /// The root scope.
    pub root_scope_id: Option<ScopeId>,
    /// The modules.
    pub modules: ModuleRegistry,
    /// The packages.
    pub packages: PackageRegistry,
    /// The combined DIR node tree (excluding derived metadata).
    pub tree: RwLock<NodeTree>,
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
            tree: RwLock::new(NodeTree::new()),
            diagnostics: DiagnosticCollector::new(),
            strings: StringPool::new(),
        }
    }

    /// Whether the session has any diagnostics of the given severity.
    pub fn has_diagnostics_of_severity(&self, severity: DiagnosticSeverity) -> bool {
        self.diagnostics.has_diagnostics_of_severity(severity)
    }

    /// Get the diagnostics status code.
    pub fn get_diagnostics_status_code(&self) -> i32 {
        if self.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
            return 1;
        }
        if self.has_diagnostics_of_severity(DiagnosticSeverity::Warning) {
            return 2;
        }
        0
    }
}
