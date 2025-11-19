use dyst_ast::{self as ast, SharedStringPool};
use dyst_source::{FileId, Uri};

use crate::{DependencyEdge, Expression, NodeId, ScopeId};

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

/// A Module is a single source unit.
#[derive(Debug, Clone)]
pub struct Module {
    /// The id of the Module itself.
    pub id: ModuleId,
    /// The underlying source File.
    pub file_id: FileId,
    /// The URI of the Module.
    pub uri: Uri,

    /// The scope of the Module.
    pub scope: Option<ScopeId>,
    /// The AST of the Module (may be empty).
    pub ast: ast::MutableNodeTree,
    /// The AST parent index
    pub parents: ast::NodeParentIndex,
    /// The string pool of the Module.
    pub strings: SharedStringPool,

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
        file_id: FileId,
        uri: Uri,
        ast: ast::MutableNodeTree,
        strings: SharedStringPool,
    ) -> Self {
        let parents = ast::NodeParentIndex::from_tree(&ast);
        Self {
            id,
            file_id,
            uri,
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
        ast::MutableNodeTree: ast::MutableNodeTreeImpl<T>,
    {
        self.ast.get(id)
    }

    /// Get nodes for a given type.
    #[inline]
    pub fn get_nodes<T>(&self) -> Vec<ast::NodeId<T>>
    where
        T: ast::Node,
        ast::MutableNodeTree: ast::MutableNodeTreeImpl<T>,
    {
        self.ast.get_nodes::<T>()
    }
}
