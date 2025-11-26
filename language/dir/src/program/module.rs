use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use parking_lot::RwLock;

use dyst_ast::{self as ast, StringPool};
use dyst_source::{FileId, Uri};

use crate::{
    AnalysisTable, DependencyMode, Expression, LocalNodeId, LocalScopeId, LocalScopeMark,
    LocalSymbolId, NodeTree, PackageId, ScopeKind, SymbolKind, SymbolSpace, SymbolTable, TypeTable,
};

/// Unique identifier for Modules.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub u32);

impl ModuleId {
    /// Wrap an id as a ModuleId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Module type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleType {
    /// ECMAScript module.
    EcmaScript,
    /// CommonJS module.
    CommonJs,
}

/// A Module is a single source unit.
#[derive(Debug)]
pub struct Module {
    /// The id of the Module itself.
    pub id: ModuleId,
    /// The underlying source File.
    pub file_id: FileId,
    /// The URI of the Module.
    pub uri: Uri,
    /// THe path to the Module.
    pub path: Option<PathBuf>,
    /// The package of the Module.
    pub package_id: Option<PackageId>,

    // ast
    /// The AST of the Module (may be empty).
    pub ast: ast::NodeTree,
    /// The top-level AST expressions of the Module.
    pub ast_roots: Vec<ast::LocalNodeId<ast::Expression>>,
    /// The string pool of the Module.
    pub ast_strings: StringPool,

    // dir
    /// The symbol of the Module namespace.
    pub namespace_symbol: LocalSymbolId,
    /// The scope of the Module.
    pub namespace_scope: LocalScopeId,
    /// The symbol of the Module default.
    pub default_symbol: LocalSymbolId,
    /// The main DIR node tree of the Module.
    pub tree: RwLock<NodeTree>,
    /// The symbol side table of the Module.
    pub symbols: RwLock<SymbolTable>,
    /// The type side table of the Module.
    pub types: RwLock<TypeTable>,
    /// The analysis side table of the Module.
    pub analysis: RwLock<AnalysisTable>,
    /// The top-level expressions of the Module.
    pub roots: Vec<LocalNodeId<Expression>>,
}

#[allow(clippy::too_many_arguments)]
impl Module {
    /// Create a new Module from an AST.
    pub fn new(
        id: ModuleId,
        file: FileId,
        uri: Uri,
        path: Option<PathBuf>,
        package: Option<PackageId>,
        ast: ast::NodeTree,
        ast_roots: Vec<ast::LocalNodeId<ast::Expression>>,
        ast_strings: StringPool,
    ) -> Self {
        let mut symbols = SymbolTable::new(id);
        let namespace_scope_id = symbols.insert_scope(ScopeKind::Namespace, None, None);
        let (namespace_symbol_id, _) = symbols.insert_symbol(
            SymbolKind::Namespace,
            SymbolSpace::Value,
            None,
            (namespace_scope_id, LocalScopeMark::end()),
            Some(DependencyMode::Namespace),
        );
        symbols.get_scope_by_id_mut(namespace_scope_id).owner_id = Some(namespace_symbol_id);
        let (default_symbol_id, _) = symbols.insert_symbol(
            SymbolKind::Namespace,
            SymbolSpace::Value,
            None,
            (namespace_scope_id, LocalScopeMark::end()),
            Some(DependencyMode::Default),
        );

        Self {
            id,
            file_id: file,
            uri,
            path,
            package_id: package,
            // astgit ad
            ast,
            ast_roots,
            ast_strings,
            // dir
            namespace_symbol: namespace_symbol_id,
            namespace_scope: namespace_scope_id,
            default_symbol: default_symbol_id,
            tree: RwLock::new(NodeTree::new(id)),
            symbols: RwLock::new(symbols),
            types: RwLock::new(TypeTable::new(id)),
            analysis: RwLock::new(AnalysisTable::new(id)),
            roots: Vec::new(),
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
    /// The next module id.
    next_module_id: AtomicU32,
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
            next_module_id: AtomicU32::new(0),
        }
    }

    /// Get and increment the next module id.
    pub fn next_id(&self) -> ModuleId {
        let next_module_id = self.next_module_id.fetch_add(1, Ordering::Relaxed);
        ModuleId::new(next_module_id)
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

    /// Get a module by module id.
    ///
    /// # Panics
    /// Panics if the module is not found.
    #[inline]
    pub fn get(&self, id: ModuleId) -> Arc<RwLock<Module>> {
        self.modules_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("module not found: {id:?}"))
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
