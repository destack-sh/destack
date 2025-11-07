use dyst_ast::{self as ast, StringPool};
use dyst_source::{File, FileId};

use crate::{Expression, NodeId};

/// Unique identifier for Modules.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub FileId);

impl ModuleId {
    /// Wrap an id as a ModuleId.
    pub fn new(file_id: FileId) -> Self {
        Self(file_id)
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
    pub file: File,

    /// The AST of the Module (may be empty).
    pub ast: ast::NodeTree,
    /// The string pool of the Module.
    pub strings: StringPool,
    /// The top-level expressions of the Module.
    pub expressions: Vec<NodeId<Expression>>,
}

impl Module {
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
