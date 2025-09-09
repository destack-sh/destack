use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use dyst_language_source::Span;

use crate::{
    Argument, ArrayLiteral, Block, Break, Call, Cast, Continue, Defer, Doc, Enum, EnumField,
    Expression, FieldLiteral, For, Function, If, Implement, Index, Let, Loop, Match, MatchCase,
    Module, Node, NodeType, Parameter, Pattern, PatternField, RangeLiteral, Return, ScalarLiteral,
    Statement, Struct, StructField, StructLiteral, Trait, Try, Tuple, TupleField, TupleLiteral,
    Type, Union, UnionField, Use, UseClause, UseItem, While, With, WithClause,
};

/// Unique identifier for nodes in an arena, parameterized by node type.
#[repr(transparent)]
#[derive(Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NodeId<T: Node> {
    pub id: u32,
    _ty: PhantomData<fn() -> T>,
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

/// The Node tree.
pub struct NodeTree {
    /// The next id to allocate.
    pub(crate) next_id: u32,
    /// The local ids of all nodes in the AST. Index is the global node id.
    pub(crate) local_id_by_node: Vec<u32>,
    /// The kinds of all nodes in the AST. Index is the global node id.
    pub(crate) kind_by_node: Vec<NodeType>,
    /// The spans of all nodes in the AST. Index is the global node id.
    pub(crate) spans_per_node: Vec<Span>,
    /// The documentation of all nodes in the AST. Index is the global node id.
    pub(crate) docs_per_node: Vec<Option<NodeId<Doc>>>,
    /// The tombstones (compacted after parsing).
    pub(crate) tombstones: Vec<u32>,

    // per-node arenas
    // groupings
    blocks: NodeArena<Block>,
    statements: NodeArena<Statement>,
    expressions: NodeArena<Expression>,
    // declarations
    modules: NodeArena<Module>,
    structs: NodeArena<Struct>,
    struct_fields: NodeArena<StructField>,
    enums: NodeArena<Enum>,
    enum_fields: NodeArena<EnumField>,
    unions: NodeArena<Union>,
    union_fields: NodeArena<UnionField>,
    traits: NodeArena<Trait>,
    implements: NodeArena<Implement>,
    types: NodeArena<Type>,
    tuples: NodeArena<Tuple>,
    tuple_fields: NodeArena<TupleField>,
    functions: NodeArena<Function>,
    // context
    withs: NodeArena<With>,
    with_clauses: NodeArena<WithClause>,
    uses: NodeArena<Use>,
    use_clauses: NodeArena<UseClause>,
    use_items: NodeArena<UseItem>,
    // control
    ifs: NodeArena<If>,
    whiles: NodeArena<While>,
    fors: NodeArena<For>,
    loops: NodeArena<Loop>,
    breaks: NodeArena<Break>,
    continues: NodeArena<Continue>,
    defers: NodeArena<Defer>,
    returns: NodeArena<Return>,
    trys: NodeArena<Try>,
    // bindings
    lets: NodeArena<Let>,
    parameters: NodeArena<Parameter>,
    arguments: NodeArena<Argument>,
    // literals
    scalar_literals: NodeArena<ScalarLiteral>,
    range_literals: NodeArena<RangeLiteral>,
    array_literals: NodeArena<ArrayLiteral>,
    tuple_literals: NodeArena<TupleLiteral>,
    struct_literals: NodeArena<StructLiteral>,
    field_literals: NodeArena<FieldLiteral>,
    // calls
    indexes: NodeArena<Index>,
    calls: NodeArena<Call>,
    casts: NodeArena<Cast>,
    // matching
    matches: NodeArena<Match>,
    patterns: NodeArena<Pattern>,
    pattern_fields: NodeArena<PatternField>,
    match_cases: NodeArena<MatchCase>,
    // documentation
    docs: NodeArena<Doc>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("spans", &self.spans_per_node)
            .finish()
    }
}

impl Default for NodeTree {
    fn default() -> Self {
        Self::new()
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
            local_id_by_node: Vec::with_capacity(capacity),
            kind_by_node: Vec::with_capacity(capacity),
            spans_per_node: Vec::with_capacity(capacity),
            docs_per_node: Vec::with_capacity(capacity),
            tombstones: Vec::new(),
            // groupings
            blocks: NodeArena::new(),
            statements: NodeArena::new(),
            expressions: NodeArena::new(),
            // declarations
            modules: NodeArena::new(),
            structs: NodeArena::new(),
            struct_fields: NodeArena::new(),
            enums: NodeArena::new(),
            enum_fields: NodeArena::new(),
            unions: NodeArena::new(),
            union_fields: NodeArena::new(),
            traits: NodeArena::new(),
            implements: NodeArena::new(),
            types: NodeArena::new(),
            tuples: NodeArena::new(),
            tuple_fields: NodeArena::new(),
            functions: NodeArena::new(),
            // context
            withs: NodeArena::new(),
            with_clauses: NodeArena::new(),
            uses: NodeArena::new(),
            use_clauses: NodeArena::new(),
            use_items: NodeArena::new(),
            // control
            ifs: NodeArena::new(),
            whiles: NodeArena::new(),
            fors: NodeArena::new(),
            loops: NodeArena::new(),
            breaks: NodeArena::new(),
            continues: NodeArena::new(),
            defers: NodeArena::new(),
            returns: NodeArena::new(),
            trys: NodeArena::new(),
            // bindings
            lets: NodeArena::new(),
            parameters: NodeArena::new(),
            arguments: NodeArena::new(),
            // literals
            scalar_literals: NodeArena::new(),
            range_literals: NodeArena::new(),
            array_literals: NodeArena::new(),
            tuple_literals: NodeArena::new(),
            struct_literals: NodeArena::new(),
            field_literals: NodeArena::new(),
            // calls
            indexes: NodeArena::new(),
            calls: NodeArena::new(),
            casts: NodeArena::new(),
            // matching
            matches: NodeArena::new(),
            patterns: NodeArena::new(),
            pattern_fields: NodeArena::new(),
            match_cases: NodeArena::new(),
            // documentation
            docs: NodeArena::new(),
        }
    }

    /// Allocate a new node in the tree.
    ///
    /// Returns a stable NodeId that can be used to retrieve the node later.
    pub fn allocate<T>(&mut self, node: T, span: Span) -> NodeId<T>
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        let global_id = self.next_id;
        self.next_id = global_id + 1;
        self.kind_by_node.push(T::KIND);
        let local_id = <Self as NodeTreeStore<T>>::push(self, node);
        self.local_id_by_node.push(local_id);
        self.spans_per_node.push(span);
        self.docs_per_node.push(None);
        NodeId {
            id: global_id,
            _ty: PhantomData,
        }
    }

    /// Free an existing node in the tree.
    /// Will be freed later.
    ///
    /// Returns whether the node was already freed.
    pub fn free<T: Node>(&mut self, node_id: NodeId<T>) {
        let node_id_raw = node_id.get() as u32;
        debug_assert!(
            !self.tombstones.contains(&node_id_raw),
            "node already freed"
        );
        self.tombstones.push(node_id_raw);
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: NodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        debug_assert!(!self.tombstones.contains(&id.id), "node already freed");
        let local_id = self.local_id_by_node[id.id as usize];
        <Self as NodeTreeStore<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub(crate) fn get_mut<T>(&mut self, id: NodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        debug_assert!(!self.tombstones.contains(&id.id), "node already freed");
        let local_id = self.local_id_by_node[id.id as usize];
        <Self as NodeTreeStore<T>>::get_mut(self, local_id)
    }

    /// Get the span for a node.
    #[inline]
    pub fn get_span<T>(&self, node_id: NodeId<T>) -> Span
    where
        T: Node,
    {
        self.spans_per_node[node_id.id as usize]
    }

    /// Set the span for a node.
    #[inline]
    pub(crate) fn set_span<T>(&mut self, node_id: NodeId<T>, span: Span)
    where
        T: Node,
    {
        self.spans_per_node[node_id.id as usize] = span;
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

impl<T> Default for NodeArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> NodeArena<T> {
    /// Create a new empty Arena.
    #[inline]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

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

/// Map node types to arenas.
pub trait NodeTreeStore<T: Node> {
    /// Push a node into the relevant arena.
    fn push(tree: &mut NodeTree, node: T) -> u32;
    /// Get a node from the relevant arena.
    fn get(tree: &NodeTree, idx: u32) -> &T;
    /// Get a mutable node from the relevant arena.
    fn get_mut(tree: &mut NodeTree, idx: u32) -> &mut T;
}

macro_rules! impl_node_tree_store {
    ($ty:ty, $field:ident) => {
        impl NodeTreeStore<$ty> for NodeTree {
            #[inline]
            fn push(tree: &mut NodeTree, node: $ty) -> u32 {
                tree.$field.push(node)
            }

            #[inline]
            fn get(tree: &NodeTree, idx: u32) -> &$ty {
                tree.$field.get(idx)
            }

            #[inline]
            fn get_mut(tree: &mut NodeTree, idx: u32) -> &mut $ty {
                tree.$field.get_mut(idx)
            }
        }
    };
}

macro_rules! impl_node_tree_stores {
    ( $( $ty:ty => $field:ident ),+ $(,)? ) => {
        $( impl_node_tree_store!($ty, $field); )*
    };
}

// usage
impl_node_tree_stores! {
    // groupings
    Block => blocks,
    Statement => statements,
    Expression => expressions,
    // declarations
    Module => modules,
    Struct => structs,
    StructField => struct_fields,
    Enum => enums,
    EnumField => enum_fields,
    Union => unions,
    UnionField => union_fields,
    Trait => traits,
    Implement => implements,
    Type => types,
    Tuple => tuples,
    TupleField => tuple_fields,
    Function => functions,
    // context
    With => withs,
    WithClause => with_clauses,
    Use => uses,
    UseClause => use_clauses,
    UseItem => use_items,
    // control
    If => ifs,
    While => whiles,
    For => fors,
    Loop => loops,
    Break => breaks,
    Continue => continues,
    Defer => defers,
    Return => returns,
    Try => trys,
    // bindings
    Let => lets,
    Parameter => parameters,
    Argument => arguments,
    // literals
    ScalarLiteral => scalar_literals,
    RangeLiteral => range_literals,
    ArrayLiteral => array_literals,
    TupleLiteral => tuple_literals,
    StructLiteral => struct_literals,
    FieldLiteral => field_literals,
    // calls
    Index => indexes,
    Call => calls,
    Cast => casts,
    // matching
    Match => matches,
    Pattern => patterns,
    PatternField => pattern_fields,
    MatchCase => match_cases,
    // documentation
    Doc => docs,
}
