use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

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
    /// Dependency item.
    DependencyItem,
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
    /// Annotation.
    Annotation,
}

impl NodeType {
    /// Get the name of the node type.
    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::CatchClause => "catch clause",
            Self::Statement => "statement",
            Self::Expression => "expression",
            Self::ArrayElement => "array element",
            Self::Declaration => "declaration",
            Self::Declarator => "declarator",
            Self::Property => "property",
            Self::Member => "member",
            Self::DependencyItem => "dependency item",
            Self::SwitchCase => "switch case",
            Self::Pattern => "pattern",
            Self::PatternField => "pattern field",
            Self::AssignPattern => "assign pattern",
            Self::AssignPatternField => "assignment pattern field",
            Self::Parameter => "parameter",
            Self::Argument => "argument",
            Self::Annotation => "annotation",
        }
    }
}

/// Unique identifier for nodes with dynamic type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct LocalNodeIdAny {
    /// The tree node id.
    pub id: u32,
    /// The node type.
    pub ty: NodeType,
}

impl LocalNodeIdAny {
    /// Create an untyped node id.
    pub fn new(id: u32, ty: NodeType) -> Self {
        Self { id, ty }
    }
}

impl<T: Node> From<LocalNodeId<T>> for LocalNodeIdAny {
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
#[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
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

/// A Node.
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
