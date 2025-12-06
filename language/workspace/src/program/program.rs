use std::path::PathBuf;
use std::sync::Arc;

use destack_ast as ast;
use destack_dir::{Expression, GlobalNodeIdAny, GlobalScopeId, LocalScopeMark, NodeType};
use destack_source::{
    DiagnosticCollector, File, FileId, FileRegistry, FileSystem, FileType, LanguageOptions,
    ModuleId, PackageId, StringPool, Uri,
};
use indexmap::IndexMap;

use crate::{
    ArtifactRegistry, Module, ModuleAst, ModuleRegistry, Package, PackageKind, PackageRegistry,
    TsConfigRegistry,
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

    // root
    // NOTE #Cleanup: remove Program.root_* stuff?
    /// The root file id.
    pub root_file_id: FileId,
    /// The root module.
    pub root_module_id: ModuleId,
    /// The root scope (in the root module).
    pub root_scope_id: GlobalScopeId,
    /// The root node (in the root module).
    pub root_node_id: GlobalNodeIdAny,
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

        // create and insert the root package, module, and file
        let (root_file_id, root_module_id, root_scope_id, root_node_id) =
            Self::new_root(&modules, &packages, files.clone());

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

            root_file_id,
            root_module_id,
            root_scope_id,
            root_node_id,
        }
    }

    /// Create and insert the root file, AST, module, and package. Return root ids.
    fn new_root(
        modules: &ModuleRegistry,
        packages: &PackageRegistry,
        files: Arc<FileRegistry>,
    ) -> (FileId, ModuleId, GlobalScopeId, GlobalNodeIdAny) {
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
            "<root>".to_string(),
            root_uri.clone(),
            None,
            FileType::Destack,
            r#"/* root program */"#.to_string(),
        );
        files.insert(root_file);
        let root_file = files.get(root_file_id);

        // root AST
        let mut root_ast = ast::NodeTree::new();
        let ast_root_node_id = root_ast.insert(ast::Expression::Error, root_file.span());

        // root module (uses ephemeral module id)
        let root_module_id = ModuleId::EPHEMERAL;
        let root_module_ast = ModuleAst::from_tree(
            root_module_id,
            root_ast,
            vec![ast_root_node_id],
            StringPool::new(),
        );
        let root_module = Module::from_ast(
            root_module_id,
            root_file_id,
            root_uri,
            None,
            root_package_id,
            root_module_ast,
        );

        // root scope / node
        let root_scope_id = root_module.dir.namespace_scope;
        let dir_root_node_id = root_module.dir.tree.write().reserve_from_source(
            NodeType::Expression,
            ast_root_node_id.id,
            (root_scope_id, LocalScopeMark::end()),
            None,
        );
        root_module
            .dir
            .tree
            .write()
            .insert(dir_root_node_id, Expression::Error);

        modules.insert(root_module);

        (
            root_file_id,
            root_module_id,
            root_scope_id.into_global(root_module_id),
            dir_root_node_id.into_global(root_module_id),
        )
    }
}
