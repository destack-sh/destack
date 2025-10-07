use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use dyst_source::Span;

use crate::tree::arena::NodeArena;
use crate::{
    Annotation, AnnotationPosition, Argument, Blank, Block, Comment, Decorator, Definition, Doc,
    EnumField, Expression, MatchCase, Node, NodeId, NodeSpanIndex, NodeType, Parameter, Pattern,
    PatternField, StructField, Tag, UnionField, UseClause, UseItem, WhereClause, WithClause,
};

/// The Node tree.
#[derive(Clone)]
pub struct NodeTree {
    /// The next id to allocate.
    pub(crate) next_id: u32,
    /// The local ids of all nodes. Index is the global node id.
    pub(crate) local_id_by_node: Vec<u32>,
    /// The types of all nodes. Index is the global node id.
    pub(crate) type_by_node: Vec<NodeType>,
    /// The annotations attached to nodes.
    pub(crate) annotations_per_node: HashMap<u32, Vec<NodeId<Annotation>>>,

    /// The spans of the NodeTree.
    pub spans: NodeSpanIndex,

    // per-node arenas
    // groupings
    pub(crate) expressions: NodeArena<Expression>,
    pub(crate) blocks: NodeArena<Block>,
    // definitions
    pub(crate) definitions: NodeArena<Definition>,
    pub(crate) struct_fields: NodeArena<StructField>,
    pub(crate) enum_fields: NodeArena<EnumField>,
    pub(crate) union_fields: NodeArena<UnionField>,
    // context
    pub(crate) with_clauses: NodeArena<WithClause>,
    pub(crate) where_clauses: NodeArena<WhereClause>,
    pub(crate) use_clauses: NodeArena<UseClause>,
    pub(crate) use_items: NodeArena<UseItem>,
    // bindings
    pub(crate) parameters: NodeArena<Parameter>,
    pub(crate) arguments: NodeArena<Argument>,
    // matching
    pub(crate) match_cases: NodeArena<MatchCase>,
    pub(crate) patterns: NodeArena<Pattern>,
    pub(crate) pattern_fields: NodeArena<PatternField>,
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
            // definitions
            definitions: NodeArena::new(),
            struct_fields: NodeArena::new(),
            enum_fields: NodeArena::new(),
            union_fields: NodeArena::new(),
            // context
            with_clauses: NodeArena::new(),
            where_clauses: NodeArena::new(),
            use_clauses: NodeArena::new(),
            use_items: NodeArena::new(),
            // bindings
            parameters: NodeArena::new(),
            arguments: NodeArena::new(),
            // matching
            match_cases: NodeArena::new(),
            patterns: NodeArena::new(),
            pattern_fields: NodeArena::new(),
            // annotations
            annotations: NodeArena::new(),
            blanks: NodeArena::new(),
            docs: NodeArena::new(),
            comments: NodeArena::new(),
            tags: NodeArena::new(),
            decorators: NodeArena::new(),
        }
    }

    /// Get the next id.
    #[inline]
    pub fn next_id(&self) -> u32 {
        self.next_id
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

    /// Prune nodes from the tree. Resets the next id to the given index.
    #[inline]
    pub fn reset_to(&mut self, from_idx: u32) {
        // collect local ids by node type
        let mut local_ids_by_node: HashMap<NodeType, Vec<u32>> = HashMap::new();
        for idx in from_idx..self.next_id {
            let node_type = self.type_by_node[idx as usize];
            let local_id = self.local_id_by_node[idx as usize];
            local_ids_by_node
                .entry(node_type)
                .or_default()
                .push(local_id);
        }
        // deallocate nodes
        for (node_type, local_ids) in local_ids_by_node {
            self.deallocate(node_type, local_ids);
        }
        self.type_by_node.truncate(from_idx as usize);
        self.local_id_by_node.truncate(from_idx as usize);
        // reset spans & next_id
        self.spans.prune_from(from_idx);
        self.next_id = from_idx;
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
    pub fn set_span<T>(&mut self, node_id: NodeId<T>, span: Span)
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
    pub fn get_nodes<T>(&self) -> Vec<NodeId<T>>
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

    /// Remove a given local node.
    #[inline]
    fn deallocate(&mut self, node_type: NodeType, local_ids: Vec<u32>) {
        match node_type {
            // groupings
            NodeType::Expression => self.expressions.deallocate(local_ids),
            NodeType::Block => self.blocks.deallocate(local_ids),
            // definitions
            NodeType::Definition => self.definitions.deallocate(local_ids),
            NodeType::StructField => self.struct_fields.deallocate(local_ids),
            NodeType::EnumField => self.enum_fields.deallocate(local_ids),
            NodeType::UnionField => self.union_fields.deallocate(local_ids),
            // context
            NodeType::WithClause => self.with_clauses.deallocate(local_ids),
            NodeType::WhereClause => self.where_clauses.deallocate(local_ids),
            NodeType::UseClause => self.use_clauses.deallocate(local_ids),
            NodeType::UseItem => self.use_items.deallocate(local_ids),
            // bindings
            NodeType::Parameter => self.parameters.deallocate(local_ids),
            NodeType::Argument => self.arguments.deallocate(local_ids),
            // matching
            NodeType::MatchCase => self.match_cases.deallocate(local_ids),
            NodeType::Pattern => self.patterns.deallocate(local_ids),
            NodeType::PatternField => self.pattern_fields.deallocate(local_ids),
            // annotations
            NodeType::Annotation => self.annotations.deallocate(local_ids),
            NodeType::Blank => self.blanks.deallocate(local_ids),
            NodeType::Doc => self.docs.deallocate(local_ids),
            NodeType::Comment => self.comments.deallocate(local_ids),
            NodeType::Tag => self.tags.deallocate(local_ids),
            NodeType::Decorator => self.decorators.deallocate(local_ids),
        }
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
    pub fn sort_annotations(&mut self) {
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
    // definitions
    Definition => definitions,
    StructField => struct_fields,
    EnumField => enum_fields,
    UnionField => union_fields,
    // context
    WithClause => with_clauses,
    WhereClause => where_clauses,
    UseClause => use_clauses,
    UseItem => use_items,
    // bindings
    Parameter => parameters,
    Argument => arguments,
    // matching
    MatchCase => match_cases,
    Pattern => patterns,
    PatternField => pattern_fields,
    // annotations
    Annotation => annotations,
    Blank => blanks,
    Doc => docs,
    Comment => comments,
    Tag => tags,
    Decorator => decorators,
}
