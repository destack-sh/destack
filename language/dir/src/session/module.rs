use std::collections::HashMap;

use dyst_ast::{self as ast, SharedStringPool, StringId};
use dyst_source::{FileId, Uri};

use crate::{DependencyItem, Expression, NodeId};

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

/// The kind of a Module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleKind {
    /// Script "module" (doesn't have import/export and is not explicitly declared).
    Script,
    /// Module "module" (has import/export or is explicitly declared).
    Module,
}

/// A Module is a single source unit.
#[derive(Debug, Clone)]
pub struct Module {
    /// The id of the Module itself.
    pub id: ModuleId,
    /// The kind of the Module.
    pub kind: ModuleKind,
    /// The underlying source File.
    pub file_id: FileId,
    /// The URI of the Module.
    pub uri: Uri,

    /// The AST of the Module (may be empty).
    pub ast: ast::MutableNodeTree,
    /// The AST parent index
    pub parents: ast::NodeParentIndex,
    /// The string pool of the Module.
    pub strings: SharedStringPool,

    /// The top-level expressions of the Module.
    pub expressions: Vec<NodeId<Expression>>,
    // The imports of the Module.
    pub imports: Vec<NodeId<DependencyItem>>,
    // The exports of the Module.
    pub exports: HashMap<StringId, NodeId<DependencyItem>>,
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
            kind: ModuleKind::Script,
            file_id,
            uri,
            ast,
            parents,
            strings,
            expressions: Vec::new(),
            imports: Vec::new(),
            exports: HashMap::new(),
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
