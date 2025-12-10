use std::path::PathBuf;
use std::sync::Arc;

use destack_ast as ast;
use destack_source::{
    DiagnosticCollector, File, FileId, FileRegistry, FileSystem, FileType, LanguageOptions,
    ModuleId, PackageId, StringPool, Uri,
};
use indexmap::IndexMap;

use crate::{
    ArtifactRegistry, Module, ModuleAst, ModuleRegistry, Package, PackageKind, PackageRegistry,
    ModuleType, TsConfigRegistry,
};

/// A Program.
#[derive(Debug)]
pub struct Program {
    // meta
    /// The language options.
    pub language: LanguageOptions,
    /// The current working directory.
    pub cwd: PathBuf,
    /// The file system.
    pub fs: Arc<dyn FileSystem>,
    /// The files in the program.
    pub files: Arc<FileRegistry>,

    // content
    /// The modules.
    pub modules: ModuleRegistry,
    /// The packages.
    pub packages: PackageRegistry,
    /// The tsconfigs (separate registry as tsconfigs can be nested within packages).
    pub tsconfigs: TsConfigRegistry,
    /// Generated artifacts (from codegen).
    pub artifacts: ArtifactRegistry,
    /// The combined string pool.
    pub strings: StringPool,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,

    // root module for global caching
    /// The root module id (ephemeral module used for caching).
    pub root_module_id: ModuleId,
    /// Fallback file for diagnostics without source anchors.
    pub fallback_file_id: FileId,
}

impl Program {
    /// Create a new Program.
    pub fn new(
        language: LanguageOptions,
        cwd: PathBuf,
        fs: Arc<dyn FileSystem>,
        files: Arc<FileRegistry>,
    ) -> Self {
        // set up content registries
        let modules = ModuleRegistry::new();
        let packages = PackageRegistry::new();
        let tsconfigs = TsConfigRegistry::new();
        let artifacts = ArtifactRegistry::new();
        let strings = StringPool::new();
        let diagnostics = DiagnosticCollector::new();

        // create and insert the root package and module (for global caching)
        let (root_module_id, fallback_file_id) = Self::new_root(&modules, &packages, files.clone());

        Self {
            language,
            cwd,
            fs,
            files,

            modules,
            packages,
            tsconfigs,
            artifacts,
            strings,
            diagnostics,

            root_module_id,
            fallback_file_id,
        }
    }

    /// Create and insert the root file, AST, module, and package for caching.
    fn new_root(
        modules: &ModuleRegistry,
        packages: &PackageRegistry,
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
            package_config: None,
            dsconfig: None,
            main_tsconfig_id: None,
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
        let root_module_ast = ModuleAst::from_tree(
            root_module_id,
            root_ast,
            Vec::new(),
            StringPool::new(),
        );
        let root_module = Module::from_ast(
            root_module_id,
            root_file_id,
            root_uri,
            None,
            root_package_id,
            ModuleType::Script,
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
        let module = Module::blank(module_id, file_id, uri, None, package_id, module_type);
        self.modules.insert(module);

        module_id
    }
}
