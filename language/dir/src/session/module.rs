use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dyst_ast::{self as ast, StringPool};
use dyst_source::{FileId, Uri};

use crate::{DependencyEdge, Expression, LocalNodeId, PackageId, LocalScopeId, LocalSymbolId};

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
#[derive(Debug, Clone)]
pub struct Module {
    /// The id of the Module itself.
    pub id: ModuleId,
    /// The underlying source File.
    pub file: FileId,
    /// The URI of the Module.
    pub uri: Uri,
    /// The package of the Module.
    pub package: Option<PackageId>,

    // source
    /// The AST of the Module (may be empty).
    pub ast: ast::NodeTree,
    /// The top-level AST expressions of the Module.
    pub ast_roots: Vec<ast::LocalNodeId<ast::Expression>>,
    /// The string pool of the Module.
    pub ast_strings: StringPool,

    // binding
    /// The symbol of the Module itself.
    pub symbol: Option<LocalSymbolId>,
    /// The scope of the Module itself.
    pub scope: Option<LocalScopeId>,
    /// The top-level expressions of the Module.
    pub roots: Vec<LocalNodeId<Expression>>,
    // The imports of the Module.
    pub imports: Vec<DependencyEdge>,
}

impl Module {
    /// Create a new Module from a file and expressions.
    pub fn new(
        id: ModuleId,
        file: FileId,
        uri: Uri,
        package: Option<PackageId>,
        ast: ast::NodeTree,
        ast_roots: Vec<ast::LocalNodeId<ast::Expression>>,
        ast_strings: StringPool,
    ) -> Self {
        Self {
            id,
            file,
            uri,
            package,
            ast,
            ast_roots,
            ast_strings,
            // binding
            symbol: None,
            scope: None,
            roots: Vec::new(),
            imports: Vec::new(),
        }
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: ast::LocalNodeId<T>) -> &T
    where
        T: ast::Node,
        ast::NodeTree: ast::NodeTreeImpl<T>,
    {
        self.ast.get(id)
    }

    /// Get nodes for a given type.
    #[inline]
    pub fn get_nodes<T>(&self) -> Vec<ast::LocalNodeId<T>>
    where
        T: ast::Node,
        ast::NodeTree: ast::NodeTreeImpl<T>,
    {
        self.ast.get_nodes::<T>()
    }
}

/// Graph of Modules (including their underlying Files). THREAD-SAFE.
#[derive(Debug)]
pub struct ModuleRegistry {
    /// The modules by id.
    modules_by_id: Mutex<HashMap<ModuleId, Arc<RwLock<Module>>>>,
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
            modules_by_id: Mutex::new(HashMap::new()),
            next_module_id: AtomicU32::new(0),
        }
    }

    /// Get and increment the next module id.
    pub fn next_id(&self) -> ModuleId {
        let next_module_id = self.next_module_id.fetch_add(1, Ordering::Relaxed);
        ModuleId::new(next_module_id)
    }

    /// Insert a module into the graph.
    pub fn insert(&self, module: Module) {
        let mut modules_by_id = self.modules_by_id.lock();
        modules_by_id.insert(module.id, Arc::new(RwLock::new(module)));
    }

    /// Get a module by module id.
    #[inline]
    pub fn get(&self, id: ModuleId) -> Option<Arc<RwLock<Module>>> {
        let modules_by_id = self.modules_by_id.lock();
        modules_by_id.get(&id).cloned()
    }

    /// Iterate over the modules in the registry.
    pub fn iter(&self) -> impl Iterator<Item = Arc<RwLock<Module>>> {
        let modules_by_id = self.modules_by_id.lock();
        let snapshot: Vec<_> = modules_by_id.values().cloned().collect();
        snapshot.into_iter()
    }

    /// Get the number of modules in the registry.
    pub fn len(&self) -> usize {
        let modules_by_id = self.modules_by_id.lock();
        modules_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        let modules_by_id = self.modules_by_id.lock();
        modules_by_id.is_empty()
    }
}
