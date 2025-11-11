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

impl From<FileId> for ModuleId {
    fn from(file_id: FileId) -> Self {
        Self::new(file_id)
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
    /// The AST parent index
    pub parents: ast::NodeParentIndex,
    /// The top-level expressions of the Module.
    pub expressions: Vec<NodeId<Expression>>,
    /// The string pool of the Module.
    pub strings: StringPool,
}

impl Module {
    /// Create a new Module from a file and expressions.
    pub fn from_file(file: File, ast: ast::NodeTree, strings: StringPool) -> Self {
        let parents = ast::NodeParentIndex::from_tree(&ast);
        Self {
            id: ModuleId::new(file.id),
            kind: ModuleKind::Script,
            file,
            ast,
            parents,
            strings,
            expressions: Vec::new(),
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
