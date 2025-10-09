use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use dyst_ast as ast;
use dyst_source::SourceId;

/// A unique identifier for an AST node from some Source.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AstNodeId<T: ast::Node> {
    /// The underlying AST node id.
    pub id: ast::NodeId<T>,
    /// The source id of the underlying AST node.
    pub source_id: SourceId,
    /// The type of the underlying AST node.
    _ty: PhantomData<fn() -> T>,
}

impl<T: ast::Node> Debug for AstNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstNodeId")
            .field("id", &self.id)
            .field("source_id", &self.source_id)
            .finish()
    }
}

impl<T: ast::Node> AstNodeId<T> {
    /// Create a new AST node id.
    pub fn new(id: ast::NodeId<T>, source_id: SourceId) -> Self {
        Self {
            id,
            source_id,
            _ty: PhantomData,
        }
    }
}

/// A unique identifier for an AST node from some Source.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AstNodeIdAny {
    /// The underlying AST node id.
    pub id: ast::NodeIdAny,
    /// The source id of the underlying AST node.
    pub source_id: SourceId,
}

impl Debug for AstNodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstNodeIdAny")
            .field("id", &self.id)
            .field("source_id", &self.source_id)
            .finish()
    }
}

impl AstNodeIdAny {
    /// Create a new AST node id.
    pub fn new(id: ast::NodeIdAny, source_id: SourceId) -> Self {
        Self { id, source_id }
    }
}
