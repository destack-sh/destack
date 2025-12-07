use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use destack_source::{FileSourceMap, Span};

use crate::{
    Annotation, AnnotationPosition, Arena, Argument, Blank, Block, Comment, Declaration, Decorator,
    DependencyItem, Doc, EnumField, Expression, LocalNodeId, MatchCase, Node, NodeType, Parameter,
    Pattern, PatternField, Property, WhereClause,
};

/// Mutable AST Node tree for a single source unit. NOT THREAD-SAFE.
#[derive(Clone)]
pub struct NodeTree {
    /// The next id to allocate.
    pub(crate) next_global_id: u32,
    /// The local ids of all nodes. Index is the global node id.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// The types of all nodes. Index is the global node id.
    pub(crate) node_type_by_node_id: Vec<NodeType>,
    /// The annotations attached to nodes.
    pub(crate) annotations_by_node_id: HashMap<u32, Vec<LocalNodeId<Annotation>>>,
    /// The spans of the NodeTree.
    pub source_map: FileSourceMap,

    // node arenas
    pub(crate) expressions: Arena<Expression>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) declarations: Arena<Declaration>,
    pub(crate) properties: Arena<Property>,
    pub(crate) enum_fields: Arena<EnumField>,
    pub(crate) where_clauses: Arena<WhereClause>,
    pub(crate) dependency_items: Arena<DependencyItem>,
    pub(crate) parameters: Arena<Parameter>,
    pub(crate) arguments: Arena<Argument>,
    pub(crate) match_cases: Arena<MatchCase>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) pattern_fields: Arena<PatternField>,
    pub(crate) annotations: Arena<Annotation>,
    pub(crate) blanks: Arena<Blank>,
    pub(crate) docs: Arena<Doc>,
    pub(crate) comments: Arena<Comment>,
    pub(crate) decorators: Arena<Decorator>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("next_global_id", &self.next_global_id)
            .field("node_count", &self.local_id_by_node_id.len())
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
            next_global_id: 0,
            local_id_by_node_id: Vec::with_capacity(capacity),
            node_type_by_node_id: Vec::with_capacity(capacity),
            annotations_by_node_id: HashMap::new(),
            source_map: FileSourceMap::new(),
            expressions: Arena::new(),
            blocks: Arena::new(),
            declarations: Arena::new(),
            properties: Arena::new(),
            enum_fields: Arena::new(),
            where_clauses: Arena::new(),
            dependency_items: Arena::new(),
            parameters: Arena::new(),
            arguments: Arena::new(),
            match_cases: Arena::new(),
            patterns: Arena::new(),
            pattern_fields: Arena::new(),
            annotations: Arena::new(),
            blanks: Arena::new(),
            docs: Arena::new(),
            comments: Arena::new(),
            decorators: Arena::new(),
        }
    }

    /// Get the next id.
    #[inline]
    pub fn next_id(&self) -> u32 {
        self.next_global_id
    }

    /// Check if the tree is empty (no nodes).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.next_global_id == 0
    }

    /// Allocate a new node in the tree.
    ///
    /// Returns a stable NodeId that can be used to retrieve the node later.
    pub fn insert<T>(&mut self, node: T, span: Span) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        self.node_type_by_node_id.push(T::TYPE);
        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id.push(local_id);
        self.source_map.append(span);
        LocalNodeId::new(global_id)
    }

    /// Prune nodes from the tree. Resets the next id to the given index.
    #[inline]
    pub fn reset_to(&mut self, from_idx: u32) {
        self.node_type_by_node_id.truncate(from_idx as usize);
        self.local_id_by_node_id.truncate(from_idx as usize);
        // reset spans & next_id
        self.source_map.prune_from(from_idx);
        self.next_global_id = from_idx;
    }

    /// Get the type of an untyped node id.
    #[inline]
    pub fn get_node_type(&self, id: u32) -> NodeType {
        self.node_type_by_node_id[id as usize]
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get_mut(self, local_id)
    }

    /// Get the span for a node.
    #[inline]
    pub fn get_span<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
    {
        self.source_map.get(node_id.id)
    }

    /// Get the span for a node by its id.
    #[inline]
    pub fn get_span_by_id(&self, node_id: u32) -> Span {
        self.source_map.get(node_id)
    }

    /// Set the span for a node.
    #[inline]
    pub fn set_span<T>(&mut self, node_id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_map.set(node_id.id, span);
    }

    /// Get the spans for all nodes of a given type.
    #[inline]
    pub fn get_spans_for(&self, node_type: NodeType) -> Vec<Span> {
        let mut spans = Vec::new();
        for (idx, ty) in self.node_type_by_node_id.iter().enumerate() {
            if *ty == node_type {
                spans.push(self.source_map.get(idx as u32));
            }
        }
        spans
    }

    /// Get the nodes for all nodes of a given type.
    #[inline]
    pub fn get_nodes<T>(&self) -> Vec<LocalNodeId<T>>
    where
        T: Node,
    {
        let mut nodes = Vec::new();
        for (idx, ty) in self.node_type_by_node_id.iter().enumerate() {
            if *ty == T::TYPE {
                nodes.push(LocalNodeId::new(idx as u32));
            }
        }
        nodes
    }

    /// Get the nodes for a given type.
    #[inline]
    pub fn get_nodes_for<T>(&self) -> Vec<LocalNodeId<T>>
    where
        T: Node,
    {
        self.local_id_by_node_id
            .iter()
            .filter_map(|id| {
                if self.node_type_by_node_id[*id as usize] == T::TYPE {
                    Some(LocalNodeId::new(*id))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Iter nodes of a given type.
    #[inline]
    pub fn iter_nodes<T>(&self) -> impl Iterator<Item = LocalNodeId<T>>
    where
        T: Node,
    {
        self.local_id_by_node_id.iter().filter_map(|id| {
            if self.node_type_by_node_id[*id as usize] == T::TYPE {
                Some(LocalNodeId::new(*id))
            } else {
                None
            }
        })
    }

    /// Append a doc to a node by its global id.
    #[inline]
    pub fn append_annotation(&mut self, target_id: u32, annotation: LocalNodeId<Annotation>) {
        debug_assert!(target_id < self.next_global_id);
        self.annotations_by_node_id
            .entry(target_id)
            .or_default()
            .push(annotation);
    }

    /// Whether there are any annotations attached to a node.
    #[inline]
    pub fn has_annotations(&self, node_id: u32) -> bool {
        self.annotations_by_node_id.contains_key(&node_id)
    }

    /// Get annotations attached to a node.
    #[inline]
    pub fn get_annotations(&self, node_id: u32) -> Vec<LocalNodeId<Annotation>> {
        self.annotations_by_node_id
            .get(&node_id)
            .cloned()
            .unwrap_or_else(Vec::new)
    }

    /// Get all annotations.
    #[inline]
    pub fn get_all_annotations(&self) -> &HashMap<u32, Vec<LocalNodeId<Annotation>>> {
        &self.annotations_by_node_id
    }

    /// Sort all annotations.
    #[inline]
    pub fn sort_annotations(&mut self) {
        self.annotations_by_node_id
            .values_mut()
            .for_each(|annotations| {
                annotations.sort_by_key(|annotation| self.source_map.get(annotation.id).start)
            });
    }

    /// Get blank annotation attached to a node, cloned as a Vec.
    #[inline]
    pub fn get_blanks_for(&self, node_id: u32) -> Vec<(LocalNodeId<Blank>, AnnotationPosition)> {
        self.get_annotations(node_id)
            .into_iter()
            .filter_map(|id| match self.get(id) {
                Annotation::Blank { node, position } => Some((*node, *position)),
                _ => None,
            })
            .collect()
    }

    /// Get comment annotations attached to a node, cloned as a Vec.
    #[inline]
    pub fn get_comments_for(
        &self,
        node_id: u32,
    ) -> Vec<(LocalNodeId<Comment>, AnnotationPosition)> {
        self.get_annotations(node_id)
            .into_iter()
            .filter_map(|id| match self.get(id) {
                Annotation::Comment { node, position } => Some((*node, *position)),
                _ => None,
            })
            .collect()
    }

    /// Get doc annotation attached to a node, cloned as a Vec.
    #[inline]
    pub fn get_docs_for(&self, node_id: u32) -> Vec<(LocalNodeId<Doc>, AnnotationPosition)> {
        self.get_annotations(node_id)
            .into_iter()
            .filter_map(|id| match self.get(id) {
                Annotation::Doc { node, position } => Some((*node, *position)),
                _ => None,
            })
            .collect()
    }
}

/// Map node types to arenas.
pub trait NodeTreeImpl<T: Node> {
    /// Allocate a node into the relevant arena.
    fn allocate(tree: &mut NodeTree, node: T) -> u32;
    /// Get a node from the relevant arena.
    fn get(tree: &NodeTree, idx: u32) -> &T;
    /// Get a mutable node from the relevant arena.
    fn get_mut(tree: &mut NodeTree, idx: u32) -> &mut T;
}

macro_rules! impl_node_tree_store {
    ($ty:ty, $field:ident) => {
        impl NodeTreeImpl<$ty> for NodeTree {
            #[inline]
            fn allocate(tree: &mut NodeTree, node: $ty) -> u32 {
                tree.$field.allocate(node)
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

impl_node_tree_stores! {
    Expression => expressions,
    Block => blocks,
    Declaration => declarations,
    Property => properties,
    EnumField => enum_fields,
    WhereClause => where_clauses,
    DependencyItem => dependency_items,
    Parameter => parameters,
    Argument => arguments,
    MatchCase => match_cases,
    Pattern => patterns,
    PatternField => pattern_fields,
    Annotation => annotations,
    Blank => blanks,
    Doc => docs,
    Comment => comments,
    Decorator => decorators,
}
