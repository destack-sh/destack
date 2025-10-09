//! Nodes in Dyst.
//!
//! The set of allowable ASTs is larger than the set of valid Destack programs.
//! Allowing invalid but syntactically correct ASTs is great for linting and error messages,
//!  and in many cases we can suggest automatic fixes (like `->` to `=>`, or drop ``).

use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use crate::{Keyword, Path};

/// The type of a node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum NodeType {
    // groupings
    Expression,
    Block,
    // definitions
    Definition,
    StructField,
    EnumField,
    UnionField,
    // context
    WithClause,
    WhereClause,
    UseClause,
    UseItem,
    // bindings
    Parameter,
    Argument,
    // matching
    MatchCase,
    Pattern,
    PatternField,
    // annotations
    Annotation,
    Blank,
    Doc,
    Comment,
    Tag,
    Decorator,
}

/// Node types that are annotations.
pub const ANNOTATION_NODE_TYPES: [NodeType; 6] = [
    NodeType::Annotation,
    NodeType::Blank,
    NodeType::Doc,
    NodeType::Comment,
    NodeType::Tag,
    NodeType::Decorator,
];

/// Unique identifier for nodes with dynamic type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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

impl Debug for NodeIdAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeIdAny")
            .field("id", &self.id)
            .field("ty", &self.ty)
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
    pub fn new(id: u32) -> Self {
        Self {
            id,
            _ty: PhantomData,
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
    const KIND: NodeType;
}

/// A Visibility is the visibility of an item.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Visibility {
    /// Public to everything.
    Public,
    /// Private to the closest module scope.
    Private,
}

/// A Runtime is the evaluation context of an expression / function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Runtime {
    /// The dynamic runtime (regular runtime).
    Dynamic,
    /// The static runtime ("comptime").
    Static,
}

/// A Mutability is the mutability of a binding (const or mutable).
#[derive(Debug, Copy, Clone, PartialEq)]
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

/// Scoped Mutability is a mutability that is scoped to a specific pattern.
///
/// Examples:
/// ```
/// var(x, y)
/// const(session.source)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ScopedMutability {
    /// Unscoped mutability (like just `var` or `const`)
    Unscoped {
        /// The mutability of the scoped mutability.
        mutability: Mutability,
    },
    /// Scoped mutability (like `var(x, y)` or `const(session.source)`)
    Scoped {
        /// The mutability of the scoped mutability.
        mutability: Mutability,
        /// The scopes of the scoped mutability.
        scopes: Vec<Path>,
    },
}

impl ScopedMutability {
    /// Whether the mutability is mutable.
    #[inline]
    pub fn is_mutable(&self) -> bool {
        match self {
            ScopedMutability::Unscoped { mutability } => *mutability == Mutability::Mutable,
            ScopedMutability::Scoped { mutability, .. } => *mutability == Mutability::Mutable,
        }
    }

    /// Whether the mutability is immutable.
    #[inline]
    pub fn is_immutable(&self) -> bool {
        match self {
            ScopedMutability::Unscoped { mutability } => *mutability == Mutability::Immutable,
            ScopedMutability::Scoped { mutability, .. } => *mutability == Mutability::Immutable,
        }
    }
}
