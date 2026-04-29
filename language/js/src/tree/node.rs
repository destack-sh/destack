use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
/// The type of a node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Block,
    CatchClause,
    Statement,
    Expression,
    ArrayElement,
    Declaration,
    Declarator,
    Property,
    Member,
    TypeExpression,
    TupleElement,
    TypeMember,
    EnumField,
    DependencyItem,
    SwitchCase,
    Pattern,
    PatternField,
    AssignPattern,
    AssignPatternField,
    GenericParameter,
    Parameter,
    Argument,
    Annotation,
}

impl NodeType {
    /// Get the name of the node type.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            NodeType::Block => "block",
            NodeType::CatchClause => "catch clause",
            NodeType::Statement => "statement",
            NodeType::Expression => "expression",
            NodeType::ArrayElement => "array element",
            NodeType::Declaration => "declaration",
            NodeType::Declarator => "declarator",
            NodeType::Property => "property",
            NodeType::Member => "member",
            NodeType::TypeExpression => "type expression",
            NodeType::TupleElement => "tuple element",
            NodeType::TypeMember => "type member",
            NodeType::EnumField => "enum field",
            NodeType::DependencyItem => "dependency item",
            NodeType::SwitchCase => "switch case",
            NodeType::Pattern => "pattern",
            NodeType::PatternField => "pattern field",
            NodeType::AssignPattern => "assign pattern",
            NodeType::AssignPatternField => "assign pattern field",
            NodeType::GenericParameter => "generic parameter",
            NodeType::Parameter => "parameter",
            NodeType::Argument => "argument",
            NodeType::Annotation => "annotation",
        }
    }
}

/// Unique identifier for nodes with dynamic type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl<T: Node> TryFrom<LocalNodeIdAny> for LocalNodeId<T> {
    type Error = String;

    fn try_from(id: LocalNodeIdAny) -> Result<Self, Self::Error> {
        if id.ty != T::TYPE {
            return Err(format!(
                "expected {}, got {} for {:?}",
                T::TYPE.name(),
                id.ty.name(),
                id
            ));
        }
        Ok(Self {
            id: id.id,
            _ty: PhantomData,
        })
    }
}

/// Unique identifier for nodes in a local arena, parameterized by node type.
#[repr(transparent)]
#[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct LocalNodeId<T: Node> {
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

/// Manually mark as Copy since PhantomData over T breaks Copy otherwise.
impl<T: Clone + Node> Copy for LocalNodeId<T> {}

impl<T: Node> LocalNodeId<T> {
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
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
    /// Public to everything.
    Public,
    /// Protected to derived constructs.
    Protected,
    /// Private to the closest module scope.
    Private,
}

/// The asynchrony of a function.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Asynchrony {
    /// Synchronous function.
    Sync,
    /// Asynchronous function.
    Async,
}

/// A Mutability is the mutability of a binding (const or mutable).
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mutability {
    /// Cannot be modified (incl. inner even if they are mutable).
    Immutable,
    /// May be modified (incl. inner if they are also mutable).
    Mutable,
}
