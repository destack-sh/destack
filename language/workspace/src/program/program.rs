use std::path::PathBuf;
use std::sync::Arc;

use destack_ast as ast;
use destack_base::StringPool;
use destack_source::{
    DiagnosticCollector, File, FileId, FileRegistry, FileSystem, FileType, LanguageType, ModuleId,
    PackageId, Uri,
};
use indexmap::IndexMap;

use crate::{
    ArtifactRegistry, DsConfigOptions, FormatterOptions, LanguageBuiltins, LinterOptions, Module,
    ModuleAst, ModuleRegistry, ModuleType, Package, PackageKind, PackageRegistry, TsConfigOptions,
    TsConfigRegistry,
};

/// Unique identifier for Programs.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProgramId(pub u32);

impl std::fmt::Debug for ProgramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for ProgramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl ProgramId {
    /// Wrap an id as a ProgramId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A Program.
#[derive(Debug)]
pub struct Program {
    // meta
    /// Default formatter options.
    pub formatter: FormatterOptions,
    /// Default linter options.
    pub linter: LinterOptions,
    /// The current working directory.
    pub cwd: PathBuf,
    /// The file system.
    pub fs: Arc<dyn FileSystem>,
    /// The files in the program.
    pub files: Arc<FileRegistry>,

    // content
    /// The modules.
    pub modules: Arc<ModuleRegistry>,
    /// The packages.
    pub packages: Arc<PackageRegistry>,
    /// The tsconfigs (separate registry as tsconfigs can be nested within packages).
    pub tsconfigs: Arc<TsConfigRegistry>,
    // nocheckin: move artifacts and tsconfigs to session? what about strings?
    /// Generated artifacts (from codegen).
    pub artifacts: ArtifactRegistry,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The combined string pool.
    pub strings: StringPool,

    // builtins
    /// Language builtins.
    pub builtins: Option<Arc<LanguageBuiltins>>,
    /// The root module id.
    pub root_module_id: ModuleId,
    /// Fallback file for diagnostics without source anchors.
    pub fallback_file_id: FileId,
}

#[allow(clippy::too_many_arguments)]
impl Program {
    /// Create a new Program with default options.
    pub fn from_fs(cwd: PathBuf, fs: Arc<dyn FileSystem>, files: Arc<FileRegistry>) -> Self {
        Self::from_options(
            FormatterOptions::default(),
            LinterOptions::default(),
            cwd,
            fs,
            files,
        )
    }

    /// Create a new Program.
    pub fn from_options(
        formatter: FormatterOptions,
        linter: LinterOptions,
        cwd: PathBuf,
        fs: Arc<dyn FileSystem>,
        files: Arc<FileRegistry>,
    ) -> Self {
        // set up content registries
        let modules = Arc::new(ModuleRegistry::new());
        let packages = Arc::new(PackageRegistry::new());
        let tsconfigs = Arc::new(TsConfigRegistry::new());
        let artifacts = ArtifactRegistry::new();
        let strings = StringPool::new();
        let diagnostics = DiagnosticCollector::new();

        // create and insert the root package and module (for global caching)
        let (root_module_id, fallback_file_id) =
            Self::make_root(&modules, &packages, files.clone());

        Self {
            formatter,
            linter,
            cwd,
            fs,
            files,

            modules,
            packages,
            tsconfigs,
            artifacts,
            strings,
            diagnostics,
            builtins: None,

            root_module_id,
            fallback_file_id,
        }
    }

    /// Create a new Program with shared registries (for use with Session).
    pub fn new(
        formatter: FormatterOptions,
        linter: LinterOptions,
        cwd: PathBuf,
        fs: Arc<dyn FileSystem>,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        packages: Arc<PackageRegistry>,
        tsconfigs: Arc<TsConfigRegistry>,
        builtins: Option<Arc<LanguageBuiltins>>,
    ) -> Self {
        let artifacts = ArtifactRegistry::new();
        let strings = StringPool::new();
        let diagnostics = DiagnosticCollector::new();

        // create and insert the root package and module
        let (root_module_id, fallback_file_id) =
            Self::make_root(&modules, &packages, files.clone());

        Self {
            formatter,
            linter,
            cwd,
            fs,
            files,

            modules,
            packages,
            tsconfigs,
            artifacts,
            strings,
            diagnostics,
            builtins,

            root_module_id,
            fallback_file_id,
        }
    }

    /// Create and insert the root file, AST, module, and package for caching.
    fn make_root(
        modules: &ModuleRegistry,
        packages: &Arc<PackageRegistry>,
        files: Arc<FileRegistry>,
    ) -> (ModuleId, FileId) {
        // ephemeral package for root
        let root_package_id = PackageId::EPHEMERAL;
        let root_uri = Uri::from_string("<root>");
        let root_package = Package {
            id: root_package_id,
            kind: PackageKind::Ephemeral,
            uri: root_uri.clone(),
            path: None,
            name: Some("<root>".to_string()),
            version: None,
            manifest: None,
            dsconfig: None,
            tsconfig: None,
            targets: IndexMap::new(),
        };
        packages.insert(root_package);

        // root file
        let root_file_id = files.next_id();
        let root_file = File::from_text(
            root_file_id,
            "<destack>".to_string(),
            root_uri.clone(),
            None,
            FileType::Destack,
            String::new(),
        );
        files.insert(root_file);

        // root AST (empty)
        let root_ast = ast::NodeTree::new();

        // root module (uses ephemeral module id)
        let root_module_id = ModuleId::EPHEMERAL;
        let root_module_ast =
            ModuleAst::from_tree(root_module_id, root_ast, Vec::new(), StringPool::new());
        let root_module = Module::from_ast(
            root_module_id,
            root_file_id,
            root_uri,
            None,
            root_package_id,
            None, // no tsconfig for root module
            ModuleType::Script,
            LanguageType::Destack,
            root_module_ast,
        );

        modules.insert(root_module);
        (root_module_id, root_file_id)
    }

    /// Register a module with inline content (pre-loaded, no filesystem read needed).
    pub fn register_inline_module(&self, uri: Uri, content: String, ty: FileType) -> ModuleId {
        // check if module already exists
        if let Some(module_id) = self.modules.get_id_by_uri(&uri) {
            return module_id;
        }

        // create a loaded file (not blank)
        let file_id = self.files.next_id();
        let name = uri
            .to_path()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "<string>".to_string());
        let file = File::from_text(file_id, name, uri.clone(), None, ty, content);
        self.files.insert(file);

        // use ephemeral package for inline content
        let package_id = PackageId::EPHEMERAL;
        let module_id =
            ModuleId::from_relative_path(package_id, std::path::Path::new(uri.as_ref()));
        let module_type = uri
            .to_path()
            .and_then(ModuleType::from_extension)
            .unwrap_or(ModuleType::Script);
        let language_type = LanguageType::from(ty);
        let module = Module::blank(
            module_id,
            file_id,
            uri,
            None,
            package_id,
            None,
            module_type,
            language_type,
        );
        self.modules.insert(module);

        module_id
    }

    /// Access tsconfig options for a module via closure.
    pub fn with_tsconfig_options<T>(
        &self,
        module: &Module,
        f: impl FnOnce(&TsConfigOptions) -> T,
    ) -> Option<T> {
        let tsconfig_id = module.tsconfig_id?;
        let tsconfig = self.tsconfigs.get(tsconfig_id);
        Some(f(&tsconfig.read().options))
    }

    /// Access dsconfig options for a module via closure.
    pub fn with_dsconfig_options<T>(
        &self,
        module: &Module,
        f: impl FnOnce(&DsConfigOptions) -> T,
    ) -> Option<T> {
        let package = self.packages.get(module.package_id);
        let package_guard = package.read();
        let dsconfig = package_guard.dsconfig.as_ref()?;
        Some(f(&dsconfig.options))
    }

    /// Get effective linter options for a module (package dsconfig > program defaults).
    pub fn get_linter_options(&self, module_id: ModuleId) -> LinterOptions {
        let module = self.modules.get(module_id);
        let module = module.read();

        // try package dsconfig first
        if let Some(options) = self.with_dsconfig_options(&module, |ds| ds.linter.clone()) {
            return options;
        }

        // fall back to program defaults
        self.linter.clone()
    }
}
