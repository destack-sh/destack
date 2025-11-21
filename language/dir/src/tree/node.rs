use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use crate::ModuleId;

/// The type of a node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeType {
    Expression,
    Block,
    Declaration,
    Type,
    TypeField,
    Property,
    EnumField,
    WhereClause,
    WithClause,
    DependencyItem,
    Parameter,
    Argument,
    MatchCase,
    Pattern,
    PatternField,
    Annotation,
}

impl NodeType {
    /// Get the name of the node type.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            NodeType::Expression => "expression",
            NodeType::Block => "block",
            NodeType::Declaration => "declaration",
            NodeType::Type => "type",
            NodeType::TypeField => "type field",
            NodeType::Property => "property",
            NodeType::EnumField => "enum field",
            NodeType::WhereClause => "where clause",
            NodeType::WithClause => "with clause",
            NodeType::DependencyItem => "dependency item",
            NodeType::Parameter => "parameter",
            NodeType::Argument => "argument",
            NodeType::MatchCase => "match case",
            NodeType::Pattern => "pattern",
            NodeType::PatternField => "pattern field",
            NodeType::Annotation => "annotation",
        }
    }
}

/// Unique identifier for nodes with dynamic type in a local arena.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalNodeIdAny {
    pub id: u32,
    pub ty: NodeType,
}

impl LocalNodeIdAny {
    pub fn new(id: u32, ty: NodeType) -> Self {
        Self { id, ty }
    }

    #[inline]
    pub fn get(&self) -> usize {
        self.id as usize
    }
}

impl<T: Node> From<LocalNodeId<T>> for LocalNodeIdAny
where
    T: Node,
{
    fn from(id: LocalNodeId<T>) -> Self {
        Self {
            id: id.id,
            ty: T::TYPE,
        }
    }
}

impl Debug for LocalNodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeIdAny")
            .field("id", &self.id)
            .field("type", &self.ty)
            .finish()
    }
}

impl<T: Node> From<LocalNodeIdAny> for LocalNodeId<T> {
    fn from(id: LocalNodeIdAny) -> Self {
        debug_assert_eq!(id.ty, T::TYPE);
        Self {
            id: id.id,
            _ty: PhantomData,
        }
    }
}

/// Unique identifier for nodes in a local arena, parameterized by node type.
#[repr(transparent)]
#[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct LocalNodeId<T: Node> {
    pub id: u32,
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

    /// Turn into a LocalNodeIdAny.
    #[inline]
    pub fn into_any(self) -> LocalNodeIdAny {
        LocalNodeIdAny {
            id: self.id,
            ty: T::TYPE,
        }
    }
}

impl<T: Node> Debug for LocalNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalNodeId").field("id", &self.id).finish()
    }
}

// manually mark as Copy since PhantomData over T breaks Copy otherwise (?)
impl<T: Clone + Node> Copy for LocalNodeId<T> {}

impl<T: Node> LocalNodeId<T> {
    #[inline]
    pub fn get(&self) -> usize {
        self.id as usize
    }
}

/// Global node id across modules.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalNodeId<T: Node> {
    /// The module id of the global node.
    pub module_id: ModuleId,
    /// The local id of the global node.
    pub local_id: LocalNodeId<T>,
}

impl<T: Node> GlobalNodeId<T> {
    /// Create a new global node id.
    pub fn new(module_id: ModuleId, local_id: LocalNodeId<T>) -> Self {
        Self {
            module_id,
            local_id,
        }
    }
}

impl<T: Node> Debug for GlobalNodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalNodeId")
            .field("module_id", &self.module_id)
            .field("local_id", &self.local_id)
            .finish()
    }
}

impl<T: Node> From<GlobalNodeId<T>> for LocalNodeId<T> {
    fn from(id: GlobalNodeId<T>) -> Self {
        id.local_id
    }
}

/// Global node id across modules.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalNodeIdAny {
    /// The module id of the global node.
    pub module_id: ModuleId,
    /// The local id of the global node.
    pub local_id: LocalNodeIdAny,
}

impl GlobalNodeIdAny {
    /// Create a new global node id.
    pub fn new(module_id: ModuleId, local_id: LocalNodeIdAny) -> Self {
        Self {
            module_id,
            local_id,
        }
    }
}

impl Debug for GlobalNodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalNodeIdAny")
            .field("module_id", &self.module_id)
            .field("local_id", &self.local_id)
            .finish()
    }
}

impl<T: Node> From<GlobalNodeId<T>> for GlobalNodeIdAny {
    fn from(id: GlobalNodeId<T>) -> Self {
        Self {
            module_id: id.module_id,
            local_id: id.local_id.into_any(),
        }
    }
}

impl<T: Node> From<GlobalNodeIdAny> for GlobalNodeId<T> {
    fn from(id: GlobalNodeIdAny) -> Self {
        debug_assert_eq!(id.local_id.ty, T::TYPE);
        Self {
            module_id: id.module_id,
            local_id: id.local_id.into(),
        }
    }
}

impl<T: Node> From<GlobalNodeIdAny> for LocalNodeId<T> {
    fn from(id: GlobalNodeIdAny) -> Self {
        id.local_id.into()
    }
}

impl From<GlobalNodeIdAny> for LocalNodeIdAny {
    fn from(id: GlobalNodeIdAny) -> Self {
        id.local_id
    }
}

/// A Node.
pub trait Node: Sized {
    const TYPE: NodeType;

    /// Whether this node is resolved (ignoring child nodes).
    fn is_resolved(&self) -> bool;
}

/// A Visibility is the visibility of an item.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Visibility {
    /// Public to everything.
    Public,
    /// Protected to derived constructs.
    Protected,
    /// Private to the closest module scope.
    Private,
}

/// A Runtime is the evaluation context of an expression / function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Runtime {
    /// The dynamic runtime (regular runtime).
    Dynamic,
    /// The static runtime (before dynamic evaluation, "comptime").
    Static,
}

/// The asynchrony of a function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Asynchrony {
    /// Synchronous function.
    Sync,
    /// Asynchronous function.
    Async,
}

/// The reference type of a binding.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ReferenceType {
    /// A value reference (like `^T`).
    Value,
    /// A reference to a mutable binding (like `&T`).
    Reference,
}

/// A Mutability is the mutability of a binding (const or mutable).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Mutability {
    /// Cannot be modified (incl. inner even if they are mutable).
    Immutable,
    /// May be modified (incl. inner if they are also mutable).
    Mutable,
}
