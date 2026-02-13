use std::fmt::{Debug, Formatter};

use destack_source::{NodeSourceMap, NodeSpanType, Span};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    Annotation, AnnotationPosition, Arena, Argument, Blank, Block, Comment, Declaration,
    Declarator, Decorator, DependencyItem, Doc, EnumField, Expression, LocalNodeId, MatchCase,
    Member, Node, NodeType, Parameter, Pattern, PatternField, Property, WhereClause,
};

/// Snapshot of NodeTree allocation lengths for speculative parser restores.
/// NOTE #Cleanup: can we somehow do something better than ast::NodeTreeMark?
#[derive(Debug, Copy, Clone)]
pub struct NodeTreeMark {
    /// The global node id cursor.
    next_global_id: u32,
    /// The expression arena length.
    expressions_len: usize,
    /// The block arena length.
    blocks_len: usize,
    /// The declaration arena length.
    declarations_len: usize,
    /// The property arena length.
    properties_len: usize,
    /// The member arena length.
    members_len: usize,
    /// The enum field arena length.
    enum_fields_len: usize,
    /// The where clause arena length.
    where_clauses_len: usize,
    /// The dependency item arena length.
    dependency_items_len: usize,
    /// The parameter arena length.
    parameters_len: usize,
    /// The argument arena length.
    arguments_len: usize,
    /// The match case arena length.
    match_cases_len: usize,
    /// The pattern arena length.
    patterns_len: usize,
    /// The pattern field arena length.
    pattern_fields_len: usize,
    /// The declarator arena length.
    declarators_len: usize,
    /// The annotation arena length.
    annotations_len: usize,
    /// The blank arena length.
    blanks_len: usize,
    /// The doc arena length.
    docs_len: usize,
    /// The comment arena length.
    comments_len: usize,
    /// The decorator arena length.
    decorators_len: usize,
}

impl NodeTreeMark {
    /// Get the next global node id captured by this mark.
    #[inline]
    pub fn next_global_id(self) -> u32 {
        self.next_global_id
    }
}

/// Mutable AST Node tree for a single source unit. NOT THREAD-SAFE.
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeTree {
    /// The next id to allocate.
    pub(crate) next_global_id: u32,
    /// The local ids of all nodes. Index is the global node id.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// The types of all nodes. Index is the global node id.
    pub(crate) node_type_by_node_id: Vec<NodeType>,
    /// The annotations attached to nodes.
    pub(crate) annotations_by_node_id: FxHashMap<u32, Vec<LocalNodeId<Annotation>>>,
    /// Whether annotation vectors are already globally sorted by start span.
    pub(crate) annotations_are_sorted: bool,
    /// The spans of the NodeTree.
    pub source_map: NodeSourceMap,

    // node arenas
    pub(crate) expressions: Arena<Expression>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) declarations: Arena<Declaration>,
    pub(crate) properties: Arena<Property>,
    pub(crate) members: Arena<Member>,
    pub(crate) enum_fields: Arena<EnumField>,
    pub(crate) where_clauses: Arena<WhereClause>,
    pub(crate) dependency_items: Arena<DependencyItem>,
    pub(crate) parameters: Arena<Parameter>,
    pub(crate) arguments: Arena<Argument>,
    pub(crate) match_cases: Arena<MatchCase>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) pattern_fields: Arena<PatternField>,
    pub(crate) declarators: Arena<Declarator>,
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
            annotations_by_node_id: FxHashMap::with_capacity_and_hasher(
                capacity / 8,
                Default::default(),
            ),
            annotations_are_sorted: true,
            source_map: NodeSourceMap::with_capacity(capacity),
            expressions: Arena::with(capacity),
            blocks: Arena::with(capacity / 8),
            declarations: Arena::with(capacity / 8),
            properties: Arena::with(capacity / 4),
            members: Arena::with(capacity / 8),
            enum_fields: Arena::with(capacity / 16),
            where_clauses: Arena::with(capacity / 16),
            dependency_items: Arena::with(capacity / 16),
            parameters: Arena::with(capacity / 8),
            arguments: Arena::with(capacity / 4),
            match_cases: Arena::with(capacity / 16),
            patterns: Arena::with(capacity / 8),
            pattern_fields: Arena::with(capacity / 8),
            declarators: Arena::with(capacity / 8),
            annotations: Arena::with(capacity / 16),
            blanks: Arena::with(capacity / 16),
            docs: Arena::with(capacity / 16),
            comments: Arena::with(capacity / 16),
            decorators: Arena::with(capacity / 16),
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

    /// Snapshot tree allocation lengths for speculative parser restores.
    #[inline]
    pub fn mark(&self) -> NodeTreeMark {
        NodeTreeMark {
            next_global_id: self.next_global_id,
            expressions_len: self.expressions.len(),
            blocks_len: self.blocks.len(),
            declarations_len: self.declarations.len(),
            properties_len: self.properties.len(),
            members_len: self.members.len(),
            enum_fields_len: self.enum_fields.len(),
            where_clauses_len: self.where_clauses.len(),
            dependency_items_len: self.dependency_items.len(),
            parameters_len: self.parameters.len(),
            arguments_len: self.arguments.len(),
            match_cases_len: self.match_cases.len(),
            patterns_len: self.patterns.len(),
            pattern_fields_len: self.pattern_fields.len(),
            declarators_len: self.declarators.len(),
            annotations_len: self.annotations.len(),
            blanks_len: self.blanks.len(),
            docs_len: self.docs.len(),
            comments_len: self.comments.len(),
            decorators_len: self.decorators.len(),
        }
    }

    /// Restore tree allocation lengths from a speculative mark.
    #[inline]
    pub fn restore_to_mark(&mut self, mark: NodeTreeMark) {
        self.node_type_by_node_id
            .truncate(mark.next_global_id as usize);
        self.local_id_by_node_id
            .truncate(mark.next_global_id as usize);
        self.source_map.prune_from(mark.next_global_id);
        self.next_global_id = mark.next_global_id;

        self.expressions.truncate(mark.expressions_len);
        self.blocks.truncate(mark.blocks_len);
        self.declarations.truncate(mark.declarations_len);
        self.properties.truncate(mark.properties_len);
        self.members.truncate(mark.members_len);
        self.enum_fields.truncate(mark.enum_fields_len);
        self.where_clauses.truncate(mark.where_clauses_len);
        self.dependency_items.truncate(mark.dependency_items_len);
        self.parameters.truncate(mark.parameters_len);
        self.arguments.truncate(mark.arguments_len);
        self.match_cases.truncate(mark.match_cases_len);
        self.patterns.truncate(mark.patterns_len);
        self.pattern_fields.truncate(mark.pattern_fields_len);
        self.declarators.truncate(mark.declarators_len);
        self.annotations.truncate(mark.annotations_len);
        self.blanks.truncate(mark.blanks_len);
        self.docs.truncate(mark.docs_len);
        self.comments.truncate(mark.comments_len);
        self.decorators.truncate(mark.decorators_len);

        // drop annotation links that point outside the restored node range
        self.annotations_by_node_id
            .retain(|target_id, annotation_ids| {
                if *target_id >= mark.next_global_id {
                    return false;
                }
                annotation_ids.retain(|annotation_id| annotation_id.id < mark.next_global_id);
                !annotation_ids.is_empty()
            });
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

    /// Get the main span for a node (identifier span for declarations, etc).
    #[inline]
    pub fn get_main_span<T>(&self, node_id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.source_map.get_main(node_id.id)
    }

    /// Get the main span for a node by its id.
    #[inline]
    pub fn get_main_span_by_id(&self, node_id: u32) -> Option<Span> {
        self.source_map.get_main(node_id)
    }

    /// Set the main span for a node (identifier span for declarations, etc).
    #[inline]
    pub fn set_main_span<T>(&mut self, node_id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_map.set_main(node_id.id, span);
    }

    /// Set a side span for a node.
    #[inline]
    pub fn set_side_span<T>(&mut self, node_id: LocalNodeId<T>, span_type: NodeSpanType, span: Span)
    where
        T: Node,
    {
        self.source_map.set_side(node_id.id, span_type, span);
    }

    /// Get a side span for a node.
    #[inline]
    pub fn get_side_span<T>(&self, node_id: LocalNodeId<T>, span_type: NodeSpanType) -> Option<Span>
    where
        T: Node,
    {
        self.source_map.get_side(node_id.id, span_type)
    }

    /// Get a side span for a node by its id.
    #[inline]
    pub fn get_side_span_by_id(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        self.source_map.get_side(node_id, span_type)
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

    /// Get the spans for annotation side nodes in one scan.
    #[inline]
    pub fn get_side_annotation_spans(&self) -> Vec<Span> {
        let mut spans = Vec::new();
        for (idx, ty) in self.node_type_by_node_id.iter().enumerate() {
            if matches!(*ty, NodeType::Annotation | NodeType::Decorator) {
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
        self.node_type_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_id, &node_type)| {
                if node_type == T::TYPE {
                    Some(LocalNodeId::new(global_id as u32))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Iter nodes of a given type.
    #[inline]
    pub fn iter_nodes<T>(&self) -> impl Iterator<Item = LocalNodeId<T>> + '_
    where
        T: Node,
    {
        self.node_type_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_id, &node_type)| {
                if node_type == T::TYPE {
                    Some(LocalNodeId::new(global_id as u32))
                } else {
                    None
                }
            })
    }

    /// Append a doc to a node by its global id.
    #[inline]
    pub fn append_annotation(&mut self, target_id: u32, annotation: LocalNodeId<Annotation>) {
        debug_assert!(target_id < self.next_global_id);

        // keep vectors sorted by span order without paying a sort pass in the common case
        let annotation_ids = self.annotations_by_node_id.entry(target_id).or_default();
        if let Some(previous_annotation) = annotation_ids.last().copied() {
            let previous_span = self.source_map.get(previous_annotation.id);
            let current_span = self.source_map.get(annotation.id);
            let is_in_non_decreasing_order = previous_span.start < current_span.start
                || previous_span.start == current_span.start
                    && (previous_span.end < current_span.end
                        || previous_span.end == current_span.end
                            && previous_annotation.id <= annotation.id);
            if !is_in_non_decreasing_order {
                self.annotations_are_sorted = false;
            }
        }

        annotation_ids.push(annotation);
    }

    /// Whether there are any annotations attached to a node.
    #[inline]
    pub fn has_annotations(&self, node_id: u32) -> bool {
        self.annotations_by_node_id.contains_key(&node_id)
    }

    /// Has prefix annotations attached to a node.
    #[inline]
    pub fn has_prefix_annotations(&self, node_id: u32) -> bool {
        self.get_annotations(node_id)
            .into_iter()
            .any(|annotation_id| {
                let annotation = self.get(annotation_id);
                annotation.position() == AnnotationPosition::BlockPrefix
                    || annotation.position() == AnnotationPosition::LinePrefix
            })
    }

    /// Has postfix annotations attached to a node.
    #[inline]
    pub fn has_postfix_annotations(&self, node_id: u32) -> bool {
        self.get_annotations(node_id)
            .into_iter()
            .any(|annotation_id| {
                let annotation = self.get(annotation_id);
                annotation.position() == AnnotationPosition::BlockPostfix
                    || annotation.position() == AnnotationPosition::LinePostfix
                    || annotation.position() == AnnotationPosition::LinePostfixBoundary
            })
    }

    /// Has infix annotations attached to a node.
    #[inline]
    pub fn has_infix_annotations(&self, node_id: u32) -> bool {
        self.get_annotations(node_id)
            .into_iter()
            .any(|annotation_id| {
                let annotation = self.get(annotation_id);
                annotation.position() == AnnotationPosition::BlockInfix
            })
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
    pub fn get_all_annotations(&self) -> &FxHashMap<u32, Vec<LocalNodeId<Annotation>>> {
        &self.annotations_by_node_id
    }

    /// Sort all annotations.
    #[inline]
    pub fn sort_annotations(&mut self) {
        if self.annotations_are_sorted {
            return;
        }

        let source_map = &self.source_map;
        for annotation_ids in self.annotations_by_node_id.values_mut() {
            annotation_ids.sort_by(|left, right| {
                let left_span = source_map.get(left.id);
                let right_span = source_map.get(right.id);
                left_span
                    .start
                    .cmp(&right_span.start)
                    .then(left_span.end.cmp(&right_span.end))
                    .then(left.id.cmp(&right.id))
            });
        }

        self.annotations_are_sorted = true;
    }

    /// Build position index for fast enclosing span lookups.
    /// Call this after parsing is complete.
    #[inline]
    pub fn build_position_index(&mut self) {
        self.source_map.build_position_index();
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
    Member => members,
    EnumField => enum_fields,
    WhereClause => where_clauses,
    DependencyItem => dependency_items,
    Parameter => parameters,
    Argument => arguments,
    MatchCase => match_cases,
    Pattern => patterns,
    PatternField => pattern_fields,
    Declarator => declarators,
    Annotation => annotations,
    Blank => blanks,
    Doc => docs,
    Comment => comments,
    Decorator => decorators,
}
