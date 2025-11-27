use std::path::PathBuf;
use std::sync::Arc;

use dyst_ast as ast;
use dyst_source::{
    DiagnosticCollector, File, FileId, FileRegistry, FileSystem, FileType, LanguageOptions,
    StringPool, Uri,
};

use crate::{
    DsConfigRegistry, Expression, GlobalNodeIdAny, GlobalScopeId, LocalScopeMark, Module, ModuleId,
    ModuleRegistry, NodeType, PackageRegistry, TsConfigRegistry,
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
    /// The tsconfigs.
    pub tsconfigs: TsConfigRegistry,
    /// The dsconfigs.
    pub dsconfigs: DsConfigRegistry,
    /// The combined string pool.
    pub strings: StringPool,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,

    // root
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
        let dsconfigs = DsConfigRegistry::new();
        let strings = StringPool::new();
        let diagnostics = DiagnosticCollector::new();

        // create and insert the root module and file
        let (root_file_id, root_module_id, root_scope_id, root_node_id) =
            Self::new_root(&modules, &files);

        Self {
            language,
            cwd,
            fs,
            files,

            modules,
            packages,
            tsconfigs,
            dsconfigs,
            strings,
            diagnostics,

            root_file_id,
            root_module_id,
            root_scope_id,
            root_node_id,
        }
    }

    /// Create and insert the root file, AST, and module. Return root ids.
    fn new_root(
        modules: &ModuleRegistry,
        files: &Arc<FileRegistry>,
    ) -> (FileId, ModuleId, GlobalScopeId, GlobalNodeIdAny) {
        // root file
        let root_file_id = files.next_id();
        let root_file = File::from_text(
            root_file_id,
            "<root>".to_string(),
            Uri::from_string("<root>"),
            None,
            FileType::Dyst,
            r#"/* root program */"#.to_string(),
        );
        files.insert(root_file);
        let root_file = files.get(root_file_id);

        // root AST
        let mut root_ast = ast::NodeTree::new();
        let ast_root_node_id = root_ast.insert(ast::Expression::Error, root_file.span());

        // root module
        let root_module_id = modules.next_id();
        let root_module = Module::new(
            root_module_id,
            root_file_id,
            Uri::from_string("<root>"),
            None,
            None,
            root_ast,
            vec![ast_root_node_id],
            StringPool::new(),
        );

        // root scope / node
        let root_scope_id = root_module.namespace_scope;
        let dir_root_node_id = root_module.tree.write().reserve_from_source(
            NodeType::Expression,
            ast_root_node_id,
            (root_scope_id, LocalScopeMark::end()),
            None,
        );
        root_module
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
