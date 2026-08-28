use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// The type of a node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum NodeType {
    /// Block.
    Block,
    /// Catch clause.
    CatchClause,
    /// Statement.
    Statement,
    /// Expression.
    Expression,
    /// Array element.
    ArrayElement,
    /// Declaration.
    Declaration,
    /// Declarator.
    Declarator,
    /// Object property.
    Property,
    /// Class member.
    Member,
    /// Import specifier.
    ImportSpecifier,
    /// Export specifier.
    ExportSpecifier,
    /// Re-export specifier.
    ReExportSpecifier,
    /// Import attribute.
    ImportAttribute,
    /// Switch case.
    SwitchCase,
    /// Binding pattern.
    Pattern,
    /// Binding pattern field.
    PatternField,
    /// Assignment pattern.
    AssignPattern,
    /// Assignment pattern field.
    AssignPatternField,
    /// Function parameter.
    Parameter,
    /// Call argument.
    Argument,
}

/// One untyped tree-local node identifier.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct LocalNodeIdAny {
    /// The tree node id.
    pub id: u32,
}

impl LocalNodeIdAny {
    /// Create an untyped node id.
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

impl<T: Node> From<LocalNodeId<T>> for LocalNodeIdAny {
    fn from(id: LocalNodeId<T>) -> Self {
        Self { id: id.id }
    }
}

impl Debug for LocalNodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeIdAny")
            .field("id", &self.id)
            .finish()
    }
}

/// One tree-local node identifier parameterized by node type.
#[repr(transparent)]
#[derive(Serialize, Deserialize, Reflect)]
#[serde(bound = "")]
pub struct LocalNodeId<T: Node> {
    /// The tree node id.
    pub id: u32,
    #[serde(skip)]
    _ty: PhantomData<fn() -> T>,
}

impl<T: Node> LocalNodeId<T> {
    /// Create a new node id.
    #[inline]
    pub fn new(id: u32) -> Self {
        Self {
            id,
            _ty: PhantomData,
        }
    }

    /// Erase the node type.
    #[inline]
    pub fn into_any(self) -> LocalNodeIdAny {
        LocalNodeIdAny { id: self.id }
    }
}

impl<T: Node> Clone for LocalNodeId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Node> Copy for LocalNodeId<T> {}

impl<T: Node> PartialEq for LocalNodeId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Node> Eq for LocalNodeId<T> {}

impl<T: Node> PartialOrd for LocalNodeId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Node> Ord for LocalNodeId<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T: Node> Hash for LocalNodeId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: Node> Debug for LocalNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeId").field("id", &self.id).finish()
    }
}

/// One JavaScript tree node.
pub trait Node: Sized {
    /// The concrete node type.
    const TYPE: NodeType;
}

/// The asynchrony of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Asynchrony {
    /// Synchronous function.
    Sync,
    /// Asynchronous function.
    Async,
}

/// The reassignment behavior of one binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Mutability {
    /// The binding cannot be reassigned.
    Immutable,
    /// The binding may be reassigned.
    Mutable,
}
