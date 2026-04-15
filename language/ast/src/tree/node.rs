use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::Keyword;

/// The type of a node.
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NodeType {
    Expression,
    TypeExpression,
    Block,
    Declaration,
    Declarator,
    Property,
    TypeMember,
    Member,
    EnumField,
    WhereClause,
    DependencyItem,
    GenericParameter,
    Parameter,
    GenericArgument,
    TupleElement,
    Argument,
    MatchCase,
    Pattern,
    PatternField,
    Decorator,
}

impl NodeType {
    /// Get the name of the node type.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            NodeType::Expression => "expression",
            NodeType::TypeExpression => "type expression",
            NodeType::Block => "block",
            NodeType::Declaration => "declaration",
            NodeType::Declarator => "declarator",
            NodeType::Property => "property",
            NodeType::TypeMember => "type member",
            NodeType::Member => "member",
            NodeType::EnumField => "enum field",
            NodeType::WhereClause => "where clause",
            NodeType::DependencyItem => "dependency item",
            NodeType::GenericParameter => "generic parameter",
            NodeType::Parameter => "parameter",
            NodeType::GenericArgument => "generic argument",
            NodeType::TupleElement => "tuple element",
            NodeType::Argument => "argument",
            NodeType::MatchCase => "match case",
            NodeType::Pattern => "pattern",
            NodeType::PatternField => "pattern field",
            NodeType::Decorator => "decorator",
        }
    }
}

/// Node types that are decorator side nodes.
pub const DECORATOR_NODE_TYPES: [NodeType; 1] = [NodeType::Decorator];

/// Unique identifier for nodes with dynamic type in a local arena.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

    /// Turn into a typed local node id.
    #[inline]
    pub fn try_into_typed<T: Node>(self) -> Result<LocalNodeId<T>, String> {
        if self.ty != T::TYPE {
            return Err(format!(
                "expected {}, got {} for {}",
                T::TYPE.name(),
                self.ty.name(),
                self.id
            ));
        }
        Ok(LocalNodeId::new(self.id))
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

// manually mark as Copy since PhantomData over T breaks Copy otherwise (?)
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

/// Whether syntax is ambient or concrete.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ambientness {
    /// Ambient syntax declared with `declare`.
    Ambient,
    /// Concrete syntax with a body or emitted value.
    Concrete,
}

impl Ambientness {
    /// Return whether the syntax is ambient.
    #[inline]
    pub fn is_ambient(self) -> bool {
        matches!(self, Self::Ambient)
    }
}

/// The reference type of a binding.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReferenceType {
    /// A value reference (like `^T`).
    Value,
    /// A reference to a mutable binding (like `&T`).
    Reference,
}

/// A Mutability is the mutability of a binding (const or mutable).
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mutability {
    /// Cannot be modified (incl. inner even if they are mutable).
    Immutable,
    /// May be modified (incl. inner if they are also mutable).
    Mutable,
}

impl Mutability {
    /// Get the keyword for this mutability.
    #[inline]
    pub fn to_keyword(&self) -> Keyword {
        match self {
            Mutability::Immutable => Keyword::Const,
            Mutability::Mutable => Keyword::Var,
        }
    }
}
