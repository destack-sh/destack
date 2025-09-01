use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::rc::Rc;

use destack_language_token::Span;

use crate::ParserMark;

/// The type of a node in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    // Expression
    Expression,
    Statement,
    Block,
    // Literals
    ScalarLiteral,
    ArrayLiteral,
    TupleLiteral,
    StructLiteral,
    FieldLiteral,
    // Declarations
    Struct,
    StructField,
    Union,
    UnionField,
    Trait,
    Function,
    Implement,
    Type,
    Tuple,
    TupleElement,
    FunctionSignature,
    // Using
    Using,
    UsingClause,
    UsingItem,
    // Parameters
    Parameter,
    Argument,
    // Patterns
    Pattern,
    PatternTupleField,
    PatternStructField,
    MatchCase,
}

/// Unique identifier for nodes in an arena, parameterized by node type.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NodeId<T> {
    pub(crate) idx: NonZeroU32,
    pub(crate) _ty: PhantomData<fn() -> T>,
}

impl<T> NodeId<T> {
    /// Convert this NodeId to a zero-based array index.
    #[inline]
    pub fn get(&self) -> usize {
        let val: u32 = self.idx.get() as u32;
        val as usize
    }
}

/// The Node tree.
pub struct NodeTree {
    /// The spans of the nodes in the AST. Index is the node id.
    spans: Vec<Span>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("spans", &self.spans)
            .finish()
    }
}

impl NodeTree {
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Allocate a new node in the tree for the given span.
    pub(crate) fn allocate<T>(&mut self, node: T, span: Span) -> NodeId<T> {
        todo!()
    }

    pub(crate) fn allocate_from_mark<T>(&mut self, node: T, mark: ParserMark) -> NodeId<T> {
        todo!()
    }

    /// Get a node from the tree by its id.
    pub fn get<T>(&self, id: NodeId<T>) -> &T {
        todo!()
    }

    /// Get a mutable node from the tree by its id.
    pub fn get_mut<T>(&mut self, id: NodeId<T>) -> &mut T {
        todo!()
    }
}

/// NodeArena for storing AST nodes.
///
/// Provides stable NodeId handles for nodes and efficient access to both
/// node data and source location information.
pub struct NodeArena<T> {
    /// The Node tree the Arena belongs to.
    tree: Rc<NodeTree>,
    /// The nodes in the arena.
    nodes: Vec<T>,
}

impl<T> Debug for NodeArena<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arena").field("nodes", &self.nodes).finish()
    }
}

impl<T> NodeArena<T> {
    /// Create a new Arena with the given capacity.
    #[inline]
    pub fn with_capacity(tree: Rc<NodeTree>, capacity: usize) -> Self {
        Self {
            tree,
            nodes: Vec::with_capacity(capacity),
        }
    }

    /// Allocate a new node in the arena.
    ///
    /// Returns a stable NodeId that can be used to retrieve the node later.
    #[inline]
    pub(crate) fn allocate(&mut self, node: T) -> NodeId<T> {
        let id = NodeId {
            idx: NonZeroU32::new(self.nodes.len() as u32 + 1).unwrap(),
            _ty: PhantomData {},
        };
        self.nodes.push(node);
        id
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub(crate) fn get(&self, id: NodeId<T>) -> &T {
        &self.nodes[id.get()]
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub(crate) fn get_mut(&mut self, id: NodeId<T>) -> &mut T {
        &mut self.nodes[id.get()]
    }

    /// Reserve capacity for at least n additional nodes.
    #[inline]
    pub(crate) fn reserve(&mut self, n: usize) {
        self.nodes.reserve(n);
    }
}
