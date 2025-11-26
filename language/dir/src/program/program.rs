use std::{path::PathBuf, sync::Arc};

use dyst_ast::StringPool;
use dyst_source::{DiagnosticCollector, FileRegistry, FileSystem, LanguageOptions};

use crate::{DsConfigRegistry, GlobalScopeId, ModuleRegistry, PackageRegistry, TsConfigRegistry};

/// A Program.
#[derive(Debug)]
pub struct Program {
    /// The language options.
    pub language: LanguageOptions,
    /// The current working directory.
    pub root_directory: PathBuf,
    /// The file system.
    pub fs: Arc<dyn FileSystem>,
    /// The files in the program.
    pub files: Arc<FileRegistry>,
    /// The modules.
    pub modules: ModuleRegistry,
    /// The packages.
    pub packages: PackageRegistry,
    /// The tsconfigs.
    pub tsconfigs: TsConfigRegistry,
    /// The dsconfigs.
    pub dsconfigs: DsConfigRegistry,
    /// The combined string pool.
    pub strings: StringPool,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The root scope.
    pub root_scope_id: Option<GlobalScopeId>,
}

impl Program {
    /// Create a new Program.
    pub fn new(
        language: LanguageOptions,
        root_directory: PathBuf,
        fs: Arc<dyn FileSystem>,
        files: Arc<FileRegistry>,
    ) -> Self {
        Self {
            language,
            root_directory,
            fs,
            files,
            modules: ModuleRegistry::new(),
            packages: PackageRegistry::new(),
            tsconfigs: TsConfigRegistry::new(),
            dsconfigs: DsConfigRegistry::new(),
            strings: StringPool::new(),
            diagnostics: DiagnosticCollector::new(),
            root_scope_id: None,
        }
    }
}
