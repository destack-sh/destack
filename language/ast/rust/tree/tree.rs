use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use destack_language_token::Span;

use crate::{
    Argument, ArrayLiteral, Assign, Block, Break, Continue, Defer, Doc, DynamicCall, Enum,
    EnumField, Expression, FieldLiteral, For, Function, FunctionSignature, If, Implement, Let,
    Loop, Match, MatchCase, Module, Node, NodeType, Parameter, Pattern, PatternField, Return,
    ScalarLiteral, Statement, StaticCall, Struct, StructField, StructLiteral, Trait, Try, Tuple,
    TupleField, TupleLiteral, Type, Union, UnionField, Using, UsingClause, UsingItem, While,
};

/// Unique identifier for nodes in an arena, parameterized by node type.
#[repr(transparent)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NodeId<T> {
    pub id: u32,
    _ty: PhantomData<fn() -> T>,
}

// manually mark as Copy since PhantomData over T breaks Copy otherwise (?)
impl<T: Clone> Copy for NodeId<T> {}

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
    pub(crate) local_id_by_node: Vec<u32>,
    /// The kinds of all nodes in the AST. Index is the global node id.
    pub(crate) kind_by_node: Vec<NodeType>,
    /// The spans of all nodes in the AST. Index is the global node id.
    pub(crate) spans_per_node: Vec<Span>,
    /// The documentation of all nodes in the AST. Index is the global node id.
    pub(crate) docs_per_node: Vec<Option<NodeId<Doc>>>,

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
    function_signatures: NodeArena<FunctionSignature>,
    // using
    usings: NodeArena<Using>,
    using_clauses: NodeArena<UsingClause>,
    using_items: NodeArena<UsingItem>,
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
    assigns: NodeArena<Assign>,
    parameters: NodeArena<Parameter>,
    arguments: NodeArena<Argument>,
    // literals
    scalar_literals: NodeArena<ScalarLiteral>,
    array_literals: NodeArena<ArrayLiteral>,
    tuple_literals: NodeArena<TupleLiteral>,
    struct_literals: NodeArena<StructLiteral>,
    field_literals: NodeArena<FieldLiteral>,
    // calls
    static_calls: NodeArena<StaticCall>,
    dynamic_calls: NodeArena<DynamicCall>,
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
            // groupings
            blocks: NodeArena::with_capacity(capacity),
            statements: NodeArena::with_capacity(capacity),
            expressions: NodeArena::with_capacity(capacity),
            // declarations
            modules: NodeArena::with_capacity(capacity),
            structs: NodeArena::with_capacity(capacity),
            struct_fields: NodeArena::with_capacity(capacity),
            enums: NodeArena::with_capacity(capacity),
            enum_fields: NodeArena::with_capacity(capacity),
            unions: NodeArena::with_capacity(capacity),
            union_fields: NodeArena::with_capacity(capacity),
            traits: NodeArena::with_capacity(capacity),
            implements: NodeArena::with_capacity(capacity),
            types: NodeArena::with_capacity(capacity),
            tuples: NodeArena::with_capacity(capacity),
            tuple_fields: NodeArena::with_capacity(capacity),
            functions: NodeArena::with_capacity(capacity),
            function_signatures: NodeArena::with_capacity(capacity),
            // using
            usings: NodeArena::with_capacity(capacity),
            using_clauses: NodeArena::with_capacity(capacity),
            using_items: NodeArena::with_capacity(capacity),
            // control
            ifs: NodeArena::with_capacity(capacity),
            whiles: NodeArena::with_capacity(capacity),
            fors: NodeArena::with_capacity(capacity),
            loops: NodeArena::with_capacity(capacity),
            breaks: NodeArena::with_capacity(capacity),
            continues: NodeArena::with_capacity(capacity),
            defers: NodeArena::with_capacity(capacity),
            returns: NodeArena::with_capacity(capacity),
            trys: NodeArena::with_capacity(capacity),
            // bindings
            lets: NodeArena::with_capacity(capacity),
            assigns: NodeArena::with_capacity(capacity),
            parameters: NodeArena::with_capacity(capacity),
            arguments: NodeArena::with_capacity(capacity),
            // literals
            scalar_literals: NodeArena::with_capacity(capacity),
            array_literals: NodeArena::with_capacity(capacity),
            tuple_literals: NodeArena::with_capacity(capacity),
            struct_literals: NodeArena::with_capacity(capacity),
            field_literals: NodeArena::with_capacity(capacity),
            // calls
            static_calls: NodeArena::with_capacity(capacity),
            dynamic_calls: NodeArena::with_capacity(capacity),
            // matching
            matches: NodeArena::with_capacity(capacity),
            patterns: NodeArena::with_capacity(capacity),
            pattern_fields: NodeArena::with_capacity(capacity),
            match_cases: NodeArena::with_capacity(capacity),
            // documentation
            docs: NodeArena::with_capacity(capacity),
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

    /// Set the span for a node.
    pub fn set_span<T>(&mut self, node_id: NodeId<T>, span: Span)
    where
        T: Node,
    {
        self.spans_per_node[node_id.id as usize] = span;
    }

    /// Append documentation to a node.
    /// Merges the documentation with the existing documentation (if it exists).
    pub fn append_documentation<T>(&mut self, node_id: NodeId<T>, documentation: NodeId<Doc>)
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        let local_id = self.local_id_by_node[node_id.id as usize];
        if self.docs_per_node[local_id as usize].is_some() {
            todo!("merge documentation");
        }
        self.docs_per_node[local_id as usize] = Some(documentation);
    }

    /// Get an immutable reference to the node with the given NodeId.
    pub fn get<T>(&self, id: NodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        let local_id = self.local_id_by_node[id.id as usize];
        <Self as NodeTreeStore<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    pub fn get_mut<T>(&mut self, id: NodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        let local_id = self.local_id_by_node[id.id as usize];
        <Self as NodeTreeStore<T>>::get_mut(self, local_id)
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
    FunctionSignature => function_signatures,
    // using
    Using => usings,
    UsingClause => using_clauses,
    UsingItem => using_items,
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
    Assign => assigns,
    Parameter => parameters,
    Argument => arguments,
    // literals
    ScalarLiteral => scalar_literals,
    ArrayLiteral => array_literals,
    TupleLiteral => tuple_literals,
    StructLiteral => struct_literals,
    FieldLiteral => field_literals,
    // calls
    StaticCall => static_calls,
    DynamicCall => dynamic_calls,
    // matching
    Match => matches,
    Pattern => patterns,
    PatternField => pattern_fields,
    MatchCase => match_cases,
    // documentation
    Doc => docs,
}
