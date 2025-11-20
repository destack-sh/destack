use parking_lot::Mutex;
use std::collections::HashMap;

use dyst_ast::{self as ast, StringPool};
use dyst_source::{FileId, Uri};

use crate::{DependencyEdge, Expression, NodeId, PackageId, ScopeId};

/// Unique identifier for Modules.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    /// The scope of the Module itself.
    pub scope: Option<ScopeId>,
    /// The AST of the Module (may be empty).
    pub ast: ast::NodeTree,
    /// The AST parent index
    pub parents: ast::NodeParentIndex,
    /// The string pool of the Module.
    pub strings: StringPool,

    /// The top-level expressions of the Module.
    pub expressions: Vec<NodeId<Expression>>,
    // The imports of the Module.
    pub imports: Vec<DependencyEdge>,
    // The exports of the Module.
    pub exports: Vec<DependencyEdge>,
}

impl Module {
    /// Create a new Module from a file and expressions.
    pub fn from_file(
        id: ModuleId,
        file: FileId,
        uri: Uri,
        package: Option<PackageId>,
        ast: ast::NodeTree,
        strings: StringPool,
    ) -> Self {
        let parents = ast::NodeParentIndex::from_tree(&ast);
        Self {
            id,
            file,
            uri,
            package,
            scope: None,
            ast,
            parents,
            strings,
            expressions: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: ast::NodeId<T>) -> &T
    where
        T: ast::Node,
        ast::NodeTree: ast::NodeTreeImpl<T>,
    {
        self.ast.get(id)
    }

    /// Get nodes for a given type.
    #[inline]
    pub fn get_nodes<T>(&self) -> Vec<ast::NodeId<T>>
    where
        T: ast::Node,
        ast::NodeTree: ast::NodeTreeImpl<T>,
    {
        self.ast.get_nodes::<T>()
    }
}

/// Inner state for ModuleRegistry. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
struct ModuleRegistryState {
    /// The modules by id.
    modules_by_id: HashMap<ModuleId, Module>,
    /// The modules by file id.
    modules_by_file_id: HashMap<FileId, ModuleId>,
    /// The next module id.
    next_module_id: u32,
}

/// Graph of Modules (including their underlying Files). THREAD-SAFE.
#[derive(Debug)]
pub struct ModuleRegistry {
    state: Mutex<ModuleRegistryState>,
}

impl Clone for ModuleRegistry {
    fn clone(&self) -> Self {
        let state = self.state.lock().clone();
        Self {
            state: Mutex::new(state),
        }
    }
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
            state: Mutex::new(ModuleRegistryState {
                modules_by_id: HashMap::new(),
                modules_by_file_id: HashMap::new(),
                next_module_id: 0,
            }),
        }
    }

    /// Get and increment the next module id.
    pub fn next_id(&self) -> ModuleId {
        let mut state = self.state.lock();
        let id = ModuleId::new(state.next_module_id);
        state.next_module_id += 1;
        id
    }

    /// Insert a module into the graph.
    pub fn insert(&self, module: Module) {
        let mut state = self.state.lock();
        state.modules_by_file_id.insert(module.file, module.id);
        state.modules_by_id.insert(module.id, module);
    }

    /// Iterate over the modules in the graph.
    pub fn iter(&self) -> impl Iterator<Item = Module> {
        let state = self.state.lock();
        state
            .modules_by_id
            .values()
            // nocheckin: no ModuleRegistry clone in iter
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Get a module by module id.
    #[inline]
    pub fn get(&self, id: ModuleId) -> Option<Module> {
        let state = self.state.lock();
        state.modules_by_id.get(&id).cloned()
    }

    /// Get a module by file id.
    #[inline]
    pub fn get_by_file_id(&self, file_id: FileId) -> Option<Module> {
        let state = self.state.lock();
        state
            .modules_by_file_id
            .get(&file_id)
            .and_then(|id| state.modules_by_id.get(id))
            .cloned()
    }

    /// Get the number of modules in the registry.
    pub fn len(&self) -> usize {
        let state = self.state.lock();
        state.modules_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        let state = self.state.lock();
        state.modules_by_id.is_empty()
    }
}
