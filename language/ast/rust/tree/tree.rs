use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use dyst_source::Span;

use crate::tree::arena::NodeArena;
use crate::{
    Annotation, AnnotationPosition, Argument, ArrayLiteral, Blank, Block, Break, Call, Cast,
    Coalesce, Comment, Continue, Decorator, Defer, Doc, Enum, EnumField, Expression, FieldLiteral,
    For, Function, If, Implement, Index, Let, Loop, Match, MatchCase, Module, Node, NodeId,
    NodeSpanIndex, NodeType, Parameter, Pattern, PatternField, RangeLiteral, Return, ScalarLiteral,
    Struct, StructField, StructLiteral, Tag, Trait, Try, Tuple, TupleField, TupleLiteral, Type,
    Union, UnionField, Use, UseClause, UseItem, While, With, WithClause,
};

/// The Node tree.
#[derive(Clone)]
pub struct NodeTree {
    /// The next id to allocate.
    pub(crate) next_id: u32,
    /// The local ids of all nodes in the AST. Index is the global node id.
    pub(crate) local_id_by_node: Vec<u32>,
    /// The types of all nodes in the AST. Index is the global node id.
    pub(crate) type_by_node: Vec<NodeType>,
    /// The annotations attached to nodes in the AST.
    pub(crate) annotations_per_node: HashMap<u32, Vec<NodeId<Annotation>>>,

    /// The spans of the NodeTree.
    pub spans: NodeSpanIndex,

    // per-node arenas
    // groupings
    pub(crate) expressions: NodeArena<Expression>,
    pub(crate) blocks: NodeArena<Block>,
    // declarations
    pub(crate) modules: NodeArena<Module>,
    pub(crate) structs: NodeArena<Struct>,
    pub(crate) struct_fields: NodeArena<StructField>,
    pub(crate) enums: NodeArena<Enum>,
    pub(crate) enum_fields: NodeArena<EnumField>,
    pub(crate) unions: NodeArena<Union>,
    pub(crate) union_fields: NodeArena<UnionField>,
    pub(crate) traits: NodeArena<Trait>,
    pub(crate) implements: NodeArena<Implement>,
    pub(crate) types: NodeArena<Type>,
    pub(crate) tuples: NodeArena<Tuple>,
    pub(crate) tuple_fields: NodeArena<TupleField>,
    pub(crate) functions: NodeArena<Function>,
    // context
    pub(crate) withs: NodeArena<With>,
    pub(crate) with_clauses: NodeArena<WithClause>,
    pub(crate) uses: NodeArena<Use>,
    pub(crate) use_clauses: NodeArena<UseClause>,
    pub(crate) use_items: NodeArena<UseItem>,
    // control
    pub(crate) ifs: NodeArena<If>,
    pub(crate) whiles: NodeArena<While>,
    pub(crate) fors: NodeArena<For>,
    pub(crate) loops: NodeArena<Loop>,
    pub(crate) breaks: NodeArena<Break>,
    pub(crate) continues: NodeArena<Continue>,
    pub(crate) defers: NodeArena<Defer>,
    pub(crate) returns: NodeArena<Return>,
    pub(crate) trys: NodeArena<Try>,
    // bindings
    pub(crate) lets: NodeArena<Let>,
    pub(crate) parameters: NodeArena<Parameter>,
    pub(crate) arguments: NodeArena<Argument>,
    // literals
    pub(crate) scalar_literals: NodeArena<ScalarLiteral>,
    pub(crate) range_literals: NodeArena<RangeLiteral>,
    pub(crate) array_literals: NodeArena<ArrayLiteral>,
    pub(crate) tuple_literals: NodeArena<TupleLiteral>,
    pub(crate) struct_literals: NodeArena<StructLiteral>,
    pub(crate) field_literals: NodeArena<FieldLiteral>,
    // calls
    pub(crate) indexes: NodeArena<Index>,
    pub(crate) calls: NodeArena<Call>,
    pub(crate) casts: NodeArena<Cast>,
    pub(crate) coalesce: NodeArena<Coalesce>,
    // matching
    pub(crate) matches: NodeArena<Match>,
    pub(crate) patterns: NodeArena<Pattern>,
    pub(crate) pattern_fields: NodeArena<PatternField>,
    pub(crate) match_cases: NodeArena<MatchCase>,
    // annotations
    pub(crate) annotations: NodeArena<Annotation>,
    pub(crate) blanks: NodeArena<Blank>,
    pub(crate) docs: NodeArena<Doc>,
    pub(crate) comments: NodeArena<Comment>,
    pub(crate) tags: NodeArena<Tag>,
    pub(crate) decorators: NodeArena<Decorator>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree").finish()
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
            type_by_node: Vec::with_capacity(capacity),
            annotations_per_node: HashMap::new(),
            spans: NodeSpanIndex::new(),
            // groupings
            expressions: NodeArena::new(),
            blocks: NodeArena::new(),
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
            coalesce: NodeArena::new(),
            // matching
            matches: NodeArena::new(),
            patterns: NodeArena::new(),
            pattern_fields: NodeArena::new(),
            match_cases: NodeArena::new(),
            // annotations
            annotations: NodeArena::new(),
            blanks: NodeArena::new(),
            docs: NodeArena::new(),
            comments: NodeArena::new(),
            tags: NodeArena::new(),
            decorators: NodeArena::new(),
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
        self.type_by_node.push(T::KIND);
        let local_id = <Self as NodeTreeStore<T>>::push(self, node);
        self.local_id_by_node.push(local_id);
        self.spans.append(span);
        NodeId::new(global_id)
    }

    /// Get the type of an untyped node id.
    #[inline]
    pub fn get_type(&self, id: u32) -> NodeType {
        self.type_by_node[id as usize]
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: NodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        let local_id = self.local_id_by_node[id.id as usize];
        <Self as NodeTreeStore<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut<T>(&mut self, id: NodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        let local_id = self.local_id_by_node[id.id as usize];
        <Self as NodeTreeStore<T>>::get_mut(self, local_id)
    }

    /// Get the span for a node.
    #[inline]
    pub fn get_span<T>(&self, node_id: NodeId<T>) -> Span
    where
        T: Node,
    {
        self.spans.get(node_id)
    }

    /// Get the span for a node by its id.
    #[inline]
    pub fn get_span_by_id(&self, node_id: u32) -> Span {
        self.spans.get_by_id(node_id)
    }

    /// Set the span for a node.
    #[inline]
    pub(crate) fn set_span<T>(&mut self, node_id: NodeId<T>, span: Span)
    where
        T: Node,
    {
        self.spans.set(node_id, span);
    }

    /// Get the spans for all nodes of a given type.
    #[inline]
    pub fn get_spans_for(&self, node_type: NodeType) -> Vec<Span> {
        let mut spans = Vec::new();
        for (idx, ty) in self.type_by_node.iter().enumerate() {
            if *ty == node_type {
                spans.push(self.spans.get_by_id(idx as u32));
            }
        }
        spans
    }

    /// Get the nodes for all nodes of a given type.
    #[inline]
    pub fn get_nodes_for<T>(&self) -> Vec<NodeId<T>>
    where
        T: Node,
    {
        let mut nodes = Vec::new();
        for (idx, ty) in self.type_by_node.iter().enumerate() {
            if *ty == T::KIND {
                nodes.push(NodeId::new(idx as u32));
            }
        }
        nodes
    }

    /// Append a doc to a node by its global id.
    #[inline]
    pub fn append_annotation(&mut self, global_id: u32, annotation: NodeId<Annotation>) {
        debug_assert!(global_id < self.next_id);
        self.annotations_per_node
            .entry(global_id)
            .or_default()
            .push(annotation);
    }

    /// Whether there are any annotations attached to a node.
    #[inline]
    pub fn has_annotations_for(&self, node_id: u32) -> bool {
        self.annotations_per_node.contains_key(&node_id)
    }

    /// Get annotations attached to a node.
    #[inline]
    pub fn get_annotations_for(&self, node_id: u32) -> Vec<NodeId<Annotation>> {
        self.annotations_per_node
            .get(&node_id)
            .cloned()
            .unwrap_or_else(Vec::new)
    }

    /// Sort all annotations.
    #[inline]
    pub(crate) fn sort_annotations(&mut self) {
        self.annotations_per_node
            .values_mut()
            .for_each(|annotations| {
                annotations.sort_by_key(|annotation| self.spans.get(*annotation).start)
            });
    }

    /// Get blank annotation attached to a node, cloned as a Vec.
    #[inline]
    pub fn get_blanks_for(&self, node_id: u32) -> Vec<(NodeId<Blank>, AnnotationPosition)> {
        self.get_annotations_for(node_id)
            .into_iter()
            .filter_map(|id| match self.get(id) {
                Annotation::Blank { node, position } => Some((*node, *position)),
                _ => None,
            })
            .collect()
    }

    /// Get comment annotations attached to a node, cloned as a Vec.
    #[inline]
    pub fn get_comments_for(&self, node_id: u32) -> Vec<(NodeId<Comment>, AnnotationPosition)> {
        self.get_annotations_for(node_id)
            .into_iter()
            .filter_map(|id| match self.get(id) {
                Annotation::Comment { node, position } => Some((*node, *position)),
                _ => None,
            })
            .collect()
    }

    /// Get doc annotation attached to a node, cloned as a Vec.
    #[inline]
    pub fn get_docs_for(&self, node_id: u32) -> Vec<(NodeId<Doc>, AnnotationPosition)> {
        self.get_annotations_for(node_id)
            .into_iter()
            .filter_map(|id| match self.get(id) {
                Annotation::Doc { node, position } => Some((*node, *position)),
                _ => None,
            })
            .collect()
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
    Expression => expressions,
    Block => blocks,
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
    Coalesce => coalesce,
    // matching
    Match => matches,
    Pattern => patterns,
    PatternField => pattern_fields,
    MatchCase => match_cases,
    // annotations
    Annotation => annotations,
    Blank => blanks,
    Doc => docs,
    Comment => comments,
    Tag => tags,
    Decorator => decorators,
}
