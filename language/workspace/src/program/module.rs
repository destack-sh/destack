use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use indexmap::IndexMap;
use parking_lot::RwLock;

use destack_ast::{self as ast};
use destack_dir::{self as dir};
use destack_mir::{self as mir};
use destack_source::{FileId, ModuleId, PackageId, StringId, StringPool, Uri};

use super::ecmascript::ModuleType;

/// A Module is a single source unit.
#[derive(Debug)]
pub struct Module {
    /// The id of the Module itself.
    pub id: ModuleId,
    /// The underlying source File.
    pub file_id: FileId,
    /// The URI of the Module.
    pub uri: Uri,
    /// The path to the Module.
    pub path: Option<PathBuf>,
    /// The package of the Module (every module belongs to a package).
    pub package_id: PackageId,
    /// The source type of the Module (Script vs Module).
    pub module_type: ModuleType,

    /// The AST-level module data.
    pub ast: ModuleAst,
    /// The DIR-level module data.
    pub dir: ModuleDir,
    /// The MIR-level module data.
    pub mir: ModuleMir,
}

#[allow(clippy::too_many_arguments)]
impl Module {
    /// Create a new blank Module (no AST yet, will be populated by Import).
    pub fn blank(
        id: ModuleId,
        file_id: FileId,
        uri: Uri,
        path: Option<PathBuf>,
        package_id: PackageId,
        module_type: ModuleType,
    ) -> Self {
        Self {
            id,
            file_id,
            uri,
            path,
            package_id,
            module_type,
            ast: ModuleAst::new(id),
            dir: ModuleDir::new(id),
            mir: ModuleMir::new(id),
        }
    }

    /// Create a new Module from an AST.
    pub fn from_ast(
        id: ModuleId,
        file_id: FileId,
        uri: Uri,
        path: Option<PathBuf>,
        package_id: PackageId,
        module_type: ModuleType,
        ast: ModuleAst,
    ) -> Self {
        let dir = ModuleDir::new(id);
        let mir = ModuleMir::new(id);
        Self {
            id,
            file_id,
            uri,
            path,
            package_id,
            module_type,
            ast,
            dir,
            mir,
        }
    }

    /// Check if this module has been parsed (has AST content).
    pub fn is_parsed(&self) -> bool {
        !self.ast.roots.is_empty() || !self.ast.tree.is_empty()
    }
}

/// AST-level module data.
#[derive(Debug)]
pub struct ModuleAst {
    /// The id of the Module.
    pub id: ModuleId,
    /// The AST of the Module (may be empty).
    pub tree: ast::NodeTree,
    /// THe AST parent index.
    pub parents: ast::NodeParentIndex,
    /// The top-level AST expressions of the Module.
    pub roots: Vec<ast::LocalNodeId<ast::Expression>>,
    /// The string pool of the Module.
    pub strings: StringPool,
}

impl ModuleAst {
    /// Create a new empty ModuleAst.
    pub fn new(id: ModuleId) -> Self {
        Self {
            id,
            tree: ast::NodeTree::new(),
            parents: ast::NodeParentIndex::new(),
            roots: Vec::new(),
            strings: StringPool::new(),
        }
    }

    /// Create a ModuleAst from a tree.
    pub fn from_tree(
        id: ModuleId,
        tree: ast::NodeTree,
        roots: Vec<ast::LocalNodeId<ast::Expression>>,
        strings: StringPool,
    ) -> Self {
        let parents = ast::NodeParentIndex::from_tree(&tree);
        Self {
            id,
            tree,
            parents,
            roots,
            strings,
        }
    }
}

/// DIR-level module data.
#[derive(Debug)]
pub struct ModuleDir {
    /// The id of the Module.
    pub id: ModuleId,
    /// The symbol of the Module namespace.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The scope of the Module.
    pub namespace_scope: dir::LocalScopeId,
    /// The symbol of the Module default.
    pub default_symbol: dir::LocalSymbolId,
    /// The main DIR node tree of the Module.
    pub tree: RwLock<dir::NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: RwLock<dir::SymbolTable>,
    /// The type side table of the Module (includes types, instances, resolutions).
    pub types: RwLock<dir::TypeTable>,
    /// The top-level expressions of the Module.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Namespace exports: modules whose exports are re-exported via `export * from "..."`.
    pub namespace_exports: RwLock<Vec<ModuleId>>,
    /// Resolved import specifiers to module ids (keyed by (relative_module, specifier)).
    pub imported_modules: RwLock<IndexMap<(Option<ModuleId>, StringId), ModuleId>>,
    /// Exported symbols by key (space, name).
    pub exported_symbols: RwLock<IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::LocalSymbolId>>,
}

impl ModuleDir {
    /// Create a new ModuleDir.
    pub fn new(id: ModuleId) -> Self {
        // set up default namespace and default symbol
        let mut symbols = dir::SymbolTable::new(id);
        let namespace_scope_id = symbols.insert_scope(dir::ScopeKind::Namespace, None, None);
        let (namespace_symbol_id, _) = symbols.insert_symbol(
            dir::SymbolKind::Namespace,
            dir::SymbolType::Void,
            dir::SymbolSpace::Value,
            None,
            (namespace_scope_id, dir::LocalScopeMark::end()),
            Some(dir::DependencyMode::Namespace),
        );
        symbols.get_scope_by_id_mut(namespace_scope_id).owner_id = Some(namespace_symbol_id);
        let (default_symbol_id, _) = symbols.insert_symbol(
            dir::SymbolKind::Namespace,
            dir::SymbolType::Void,
            dir::SymbolSpace::Value,
            None,
            (namespace_scope_id, dir::LocalScopeMark::end()),
            Some(dir::DependencyMode::Default),
        );

        Self {
            id,
            namespace_symbol: namespace_symbol_id,
            namespace_scope: namespace_scope_id,
            default_symbol: default_symbol_id,
            tree: RwLock::new(dir::NodeTree::new(id)),
            symbols: RwLock::new(symbols),
            types: RwLock::new(dir::TypeTable::new(id)),
            roots: Vec::new(),
            namespace_exports: RwLock::new(Vec::new()),
            imported_modules: RwLock::new(IndexMap::new()),
            exported_symbols: RwLock::new(IndexMap::new()),
        }
    }
}

/// MIR-level module data.
#[derive(Debug)]
pub struct ModuleMir {
    /// The id of the Module.
    pub id: ModuleId,
    /// The MIR of the Module (may be empty initially).
    pub tree: RwLock<mir::NodeTree>,
    /// The string pool of the Module's MIR stuff.
    pub strings: StringPool,
}

impl ModuleMir {
    /// Create a new ModuleMir.
    pub fn new(id: ModuleId) -> Self {
        Self {
            id,
            tree: RwLock::new(mir::NodeTree::new()),
            strings: StringPool::new(),
        }
    }
}

/// Registry of Modules. THREAD-SAFE.
#[derive(Debug)]
pub struct ModuleRegistry {
    /// The modules by id.
    modules_by_id: DashMap<ModuleId, Arc<RwLock<Module>>>,
    /// URI-based index for looking up modules by their URI.
    modules_by_uri: DashMap<Uri, ModuleId>,
    /// Path-based index for looking up modules by their path (only for modules with valid paths).
    modules_by_path: DashMap<PathBuf, ModuleId>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    /// Create a new ModuleRegistry.
    pub fn new() -> Self {
        Self {
            modules_by_id: DashMap::new(),
            modules_by_uri: DashMap::new(),
            modules_by_path: DashMap::new(),
        }
    }

    /// Insert a module into the registry.
    pub fn insert(&self, module: Module) {
        let uri = module.uri.clone();
        let path = module.path.clone();
        let id = module.id;
        self.modules_by_id.insert(id, Arc::new(RwLock::new(module)));
        self.modules_by_uri.insert(uri, id);
        if let Some(path) = path {
            self.modules_by_path.insert(path, id);
        }
    }

    /// Check if a module exists by id.
    pub fn contains(&self, id: ModuleId) -> bool {
        self.modules_by_id.contains_key(&id)
    }

    /// Get a module by module id.
    ///
    /// # Panics
    /// Panics if the module is not found.
    #[inline]
    pub fn get(&self, id: ModuleId) -> Arc<RwLock<Module>> {
        self.modules_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("module not found for id: {id:?}"))
            .clone()
    }

    /// Get a module id by its URI.
    pub fn get_id_by_uri(&self, uri: &Uri) -> Option<ModuleId> {
        self.modules_by_uri.get(uri).map(|r| *r.value())
    }

    /// Get a module by its URI.
    pub fn get_by_uri(&self, uri: &Uri) -> Option<Arc<RwLock<Module>>> {
        let id = self.get_id_by_uri(uri)?;
        Some(self.get(id))
    }

    /// Check if a module exists at the given URI.
    pub fn contains_uri(&self, uri: &Uri) -> bool {
        self.modules_by_uri.contains_key(uri)
    }

    /// Get a module id by its path.
    pub fn get_id_by_path(&self, path: &Path) -> Option<ModuleId> {
        self.modules_by_path.get(path).map(|r| *r.value())
    }

    /// Get a module by its path.
    pub fn get_by_path(&self, path: &Path) -> Option<Arc<RwLock<Module>>> {
        let id = self.get_id_by_path(path)?;
        Some(self.get(id))
    }

    /// Check if a module exists at the given path.
    pub fn contains_path(&self, path: &Path) -> bool {
        self.modules_by_path.contains_key(path)
    }

    /// Iterate over the modules in the registry.
    pub fn iter(&self) -> impl Iterator<Item = Arc<RwLock<Module>>> {
        let snapshot: Vec<_> = self
            .modules_by_id
            .iter()
            .map(|r| r.value().clone())
            .collect();
        snapshot.into_iter()
    }

    /// Get the number of modules in the registry.
    pub fn len(&self) -> usize {
        self.modules_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.modules_by_id.is_empty()
    }
}
