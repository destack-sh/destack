use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use dyst_ast as ast;
use dyst_source::FileId;

/// A unique identifier for an AST node from some File.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AstNodeId<T: ast::Node> {
    /// The underlying AST node id.
    pub id: ast::NodeId<T>,
    /// The source id of the underlying AST node.
    pub file_id: FileId,
    /// The type of the underlying AST node.
    _ty: PhantomData<fn() -> T>,
}

impl<T: ast::Node> Debug for AstNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstNodeId")
            .field("id", &self.id)
            .field("file_id", &self.file_id)
            .finish()
    }
}

impl<T: ast::Node> AstNodeId<T> {
    /// Create a new AST node id.
    pub fn new(id: ast::NodeId<T>, file_id: FileId) -> Self {
        Self {
            id,
            file_id,
            _ty: PhantomData,
        }
    }
}

/// A unique identifier for an AST node from some File.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SourceNodeIdAny {
    /// The underlying AST node id.
    pub id: ast::NodeIdAny,
    /// The source id of the underlying AST node.
    pub file_id: FileId,
}

impl Debug for SourceNodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceNodeIdAny")
            .field("id", &self.id)
            .field("file_id", &self.file_id)
            .finish()
    }
}

impl SourceNodeIdAny {
    /// Create a new source node id.
    pub fn new(id: ast::NodeIdAny, file_id: FileId) -> Self {
        Self { id, file_id }
    }
}
