use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

/// The type of a node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeType {
    Expression,
    Block,
    Definition,
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
            NodeType::Definition => "definition",
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

/// Unique identifier for nodes with dynamic type.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeIdAny {
    pub id: u32,
    pub ty: NodeType,
}

impl NodeIdAny {
    pub fn new(id: u32, ty: NodeType) -> Self {
        Self { id, ty }
    }

    #[inline]
    pub fn get(&self) -> usize {
        self.id as usize
    }
}

impl<T: Node> From<NodeId<T>> for NodeIdAny
where
    T: Node,
{
    fn from(id: NodeId<T>) -> Self {
        Self {
            id: id.id,
            ty: T::TYPE,
        }
    }
}

impl Debug for NodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeIdAny")
            .field("id", &self.id)
            .field("type", &self.ty)
            .finish()
    }
}

/// Unique identifier for nodes in an arena, parameterized by node type.
#[repr(transparent)]
#[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NodeId<T: Node> {
    pub id: u32,
    _ty: PhantomData<fn() -> T>,
}

impl<T: Node> NodeId<T> {
    /// Create a new node id.
    #[inline]
    pub fn new(id: u32) -> Self {
        Self {
            id,
            _ty: PhantomData,
        }
    }

    /// Turn into a NodeIdAny.
    #[inline]
    pub fn into_any(self) -> NodeIdAny {
        NodeIdAny {
            id: self.id,
            ty: T::TYPE,
        }
    }
}

impl<T: Node> Debug for NodeId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeId").field("id", &self.id).finish()
    }
}

// manually mark as Copy since PhantomData over T breaks Copy otherwise (?)
impl<T: Clone + Node> Copy for NodeId<T> {}

impl<T: Node> NodeId<T> {
    #[inline]
    pub fn get(&self) -> usize {
        self.id as usize
    }
}

/// A Node.
pub trait Node: Sized {
    const TYPE: NodeType;
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
