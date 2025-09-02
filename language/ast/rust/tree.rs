use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use destack_language_token::Span;

use crate::{
    Argument, ArrayLiteral, Block, Expression, FieldLiteral, Function, FunctionSignature,
    Implement, MatchCase, Node, NodeType, Parameter, ParserMark, Pattern, PatternStructField,
    PatternTupleField, ScalarLiteral, Statement, Struct, StructField, StructLiteral, Trait, Tuple,
    TupleElement, TupleLiteral, Type, Union, UnionField, Using, UsingClause, UsingItem,
};

/// Unique identifier for nodes in an arena, parameterized by node type.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NodeId<T> {
    pub(crate) id: u32,
    pub(crate) _ty: PhantomData<fn() -> T>,
}

impl<T> NodeId<T> {
    #[inline]
    pub fn get(&self) -> usize {
        self.id as usize
    }
}

/// The Node tree.
pub struct NodeTree {
    /// The next id to allocate.
    pub(crate) next_id: u32,
    /// The local ids of all nodes in the AST. Index is the global node id.
    pub(crate) local_ids: Vec<u32>,
    /// The kinds of all nodes in the AST. Index is the global node id.
    pub(crate) kinds: Vec<NodeType>,
    /// The spans of all nodes in the AST. Index is the global node id.
    pub(crate) spans: Vec<Span>,

    // per-node arenas
    // expressions
    expressions: NodeArena<Expression>,
    statements: NodeArena<Statement>,
    blocks: NodeArena<Block>,
    // literals
    scalar_literals: NodeArena<ScalarLiteral>,
    array_literals: NodeArena<ArrayLiteral>,
    tuple_literals: NodeArena<TupleLiteral>,
    struct_literals: NodeArena<StructLiteral>,
    field_literals: NodeArena<FieldLiteral>,
    // declarations
    structs: NodeArena<Struct>,
    struct_fields: NodeArena<StructField>,
    unions: NodeArena<Union>,
    union_fields: NodeArena<UnionField>,
    traits: NodeArena<Trait>,
    functions: NodeArena<Function>,
    implements: NodeArena<Implement>,
    types: NodeArena<Type>,
    tuples: NodeArena<Tuple>,
    tuple_elements: NodeArena<TupleElement>,
    function_signatures: NodeArena<FunctionSignature>,
    // using
    usings: NodeArena<Using>,
    using_clauses: NodeArena<UsingClause>,
    using_items: NodeArena<UsingItem>,
    // parameters
    parameters: NodeArena<Parameter>,
    arguments: NodeArena<Argument>,
    // patterns
    patterns: NodeArena<Pattern>,
    pattern_tuple_fields: NodeArena<PatternTupleField>,
    pattern_struct_fields: NodeArena<PatternStructField>,
    match_cases: NodeArena<MatchCase>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("spans", &self.spans)
            .finish()
    }
}

impl NodeTree {
    /// Create a new NodeTree.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new NodeTree with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            next_id: 0,
            local_ids: Vec::with_capacity(capacity),
            kinds: Vec::with_capacity(capacity),
            spans: Vec::with_capacity(capacity),
            // expressions
            expressions: NodeArena::with_capacity(capacity),
            statements: NodeArena::with_capacity(capacity),
            blocks: NodeArena::with_capacity(capacity),
            // literals
            scalar_literals: NodeArena::with_capacity(capacity),
            array_literals: NodeArena::with_capacity(capacity),
            tuple_literals: NodeArena::with_capacity(capacity),
            struct_literals: NodeArena::with_capacity(capacity),
            field_literals: NodeArena::with_capacity(capacity),
            // declarations
            structs: NodeArena::with_capacity(capacity),
            struct_fields: NodeArena::with_capacity(capacity),
            unions: NodeArena::with_capacity(capacity),
            union_fields: NodeArena::with_capacity(capacity),
            traits: NodeArena::with_capacity(capacity),
            functions: NodeArena::with_capacity(capacity),
            implements: NodeArena::with_capacity(capacity),
            types: NodeArena::with_capacity(capacity),
            tuples: NodeArena::with_capacity(capacity),
            tuple_elements: NodeArena::with_capacity(capacity),
            function_signatures: NodeArena::with_capacity(capacity),
            // using
            usings: NodeArena::with_capacity(capacity),
            using_clauses: NodeArena::with_capacity(capacity),
            using_items: NodeArena::with_capacity(capacity),
            // parameters
            parameters: NodeArena::with_capacity(capacity),
            arguments: NodeArena::with_capacity(capacity),
            // patterns
            patterns: NodeArena::with_capacity(capacity),
            pattern_tuple_fields: NodeArena::with_capacity(capacity),
            pattern_struct_fields: NodeArena::with_capacity(capacity),
            match_cases: NodeArena::with_capacity(capacity),
        }
    }

    /// Allocate a new node in the tree.
    ///
    /// Returns a stable NodeId that can be used to retrieve the node later.
    pub fn allocate<T: Node>(&mut self, node: T, span: Span) -> NodeId<T> {
        let local_id = <Self as NodeTreeStore<T>>::push(self, node);
        let global_id = self.next_id;
        self.next_id = global_id + 1;
        self.kinds.push(T::KIND);
        self.local_ids.push(local_id);
        self.spans.push(span);
        NodeId {
            id: global_id,
            _ty: PhantomData,
        }
    }

    /// Get an immutable reference to the node with the given NodeId.
    pub fn get<T: Node>(&self, id: NodeId<T>) -> &T {
        todo!()
    }

    /// Get a mutable reference to the node with the given NodeId.
    pub fn get_mut<T: Node>(&mut self, id: NodeId<T>) -> &mut T {
        todo!()
    }
}

/// Map node types to arenas.
trait NodeTreeStore<T: Node> {
    fn push(tree: &mut NodeTree, node: T) -> u32;
    fn get<'a>(tree: &'a NodeTree, idx: u32) -> &'a T;
    fn get_mut<'a>(tree: &'a mut NodeTree, idx: u32) -> &'a mut T;
}

impl NodeTreeStore<Expression> for NodeTree {
    fn push(tree: &mut NodeTree, node: Expression) -> u32 {
        tree.expressions.push(node)
    }

    fn get<'a>(tree: &'a NodeTree, idx: u32) -> &'a Expression {
        tree.expressions.get(idx)
    }

    fn get_mut<'a>(tree: &'a mut NodeTree, idx: u32) -> &'a mut Expression {
        tree.expressions.get_mut(idx)
    }
}

/// NodeArena for storing AST nodes.
///
/// Provides stable NodeId handles for nodes and efficient access to both
/// node data and source location information.
pub struct NodeArena<T> {
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
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
        }
    }

    /// Allocate a new node in the tree.
    ///
    /// Returns a stable NodeId that can be used to retrieve the node later.
    #[inline]
    pub fn push(&mut self, node: T) -> u32 {
        let local_id = self.nodes.len() as u32;
        self.nodes.push(node);
        local_id
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get(&self, local_id: u32) -> &T {
        &self.nodes[local_id as usize]
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut(&mut self, local_id: u32) -> &mut T {
        &mut self.nodes[local_id as usize]
    }

    /// Reserve capacity for at least n additional nodes.
    #[inline]
    pub fn reserve(&mut self, n: usize) {
        self.nodes.reserve(n);
    }
}
