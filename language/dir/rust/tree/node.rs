use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

/// The type of a node in the AST.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum NodeType {
    // // Groupings
    // Expression,
    // Block,
    // // Declarations
    // Module,
    // Struct,
    // StructField,
    // Enum,
    // EnumField,
    // Union,
    // UnionField,
    // Trait,
    // Implement,
    Type,
    // Tuple,
    // TupleField,
    // Function,
    // // Context
    // With,
    // WithClause,
    // Use,
    // UseClause,
    // UseItem,
    // // Control
    // If,
    // While,
    // For,
    // Loop,
    // Break,
    // Continue,
    // Defer,
    // Return,
    // Try,
    // // Bindings
    // Let,
    // Parameter,
    // Argument,
    // // Literals
    // ScalarLiteral,
    // RangeLiteral,
    // TupleLiteral,
    // ArrayLiteral,
    // StructLiteral,
    // FieldLiteral,
    // // Calls
    // Index,
    // Call,
    // Cast,
    // Coalesce,
    // // Matching
    // Match,
    // MatchCase,
    // Pattern,
    // PatternField,
    // // Annotations
    // Annotation,
    // Blank,
    // Doc,
    // Comment,
    // Tag,
    // Decorator,
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

/// A Node in the AST.
pub trait Node: Sized {
    const KIND: NodeType;
}
