use std::fmt::{Debug, Formatter};

use destack_source::{NodeSourceMap, NodeSpanType, Span};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    Arena, Argument, Block, Comment, Declaration, Declarator, Decorator, DecoratorPosition,
    DependencyItem, EnumField, Expression, GenericArgument, GenericParameter, LocalNodeId,
    MatchCase, Member, Node, NodeType, Parameter, Pattern, PatternField, Property, TupleElement,
    TypeExpression, TypeMember, WhereClause,
};

/// Dense metadata for one global node id.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct NodeIndexEntry {
    /// The packed local id and node type.
    packed: u32,
}

impl NodeIndexEntry {
    const NODE_TYPE_SHIFT: u32 = 24;
    const LOCAL_ID_MASK: u32 = (1 << Self::NODE_TYPE_SHIFT) - 1;

    /// Pack one local id and node type into a dense entry.
    #[inline]
    pub(crate) fn new(local_id: u32, node_type: NodeType) -> Self {
        debug_assert!(
            local_id <= Self::LOCAL_ID_MASK,
            "node local id exceeds packed index capacity: {local_id}"
        );
        Self {
            packed: local_id | ((node_type as u32) << Self::NODE_TYPE_SHIFT),
        }
    }

    /// Return the local arena id for this entry.
    #[inline]
    pub(crate) fn local_id(self) -> u32 {
        self.packed & Self::LOCAL_ID_MASK
    }

    /// Return the concrete node type for this entry.
    #[inline]
    pub(crate) fn node_type(self) -> NodeType {
        match (self.packed >> Self::NODE_TYPE_SHIFT) as u8 {
            0 => NodeType::Expression,
            1 => NodeType::TypeExpression,
            2 => NodeType::Block,
            3 => NodeType::Declaration,
            4 => NodeType::Declarator,
            5 => NodeType::Property,
            6 => NodeType::TypeMember,
            7 => NodeType::Member,
            8 => NodeType::EnumField,
            9 => NodeType::WhereClause,
            10 => NodeType::DependencyItem,
            11 => NodeType::GenericParameter,
            12 => NodeType::Parameter,
            13 => NodeType::GenericArgument,
            14 => NodeType::TupleElement,
            15 => NodeType::Argument,
            16 => NodeType::MatchCase,
            17 => NodeType::Pattern,
            18 => NodeType::PatternField,
            19 => NodeType::Decorator,
            _ => unreachable!("invalid node type tag in packed node index"),
        }
    }
}

/// Snapshot of NodeTree allocation lengths for speculative parser restores.
/// NOTE #Cleanup: can we somehow do something better than ast::NodeTreeMark?
#[derive(Debug, Copy, Clone)]
pub struct NodeTreeMark {
    /// The global node id cursor.
    next_global_id: u32,
    /// The expression arena length.
    expressions_len: usize,
    /// The type expression arena length.
    type_expressions_len: usize,
    /// The block arena length.
    blocks_len: usize,
    /// The declaration arena length.
    declarations_len: usize,
    /// The declarator arena length.
    declarators_len: usize,
    /// The property arena length.
    properties_len: usize,
    /// The type field arena length.
    type_members_len: usize,
    /// The member arena length.
    members_len: usize,
    /// The enum field arena length.
    enum_fields_len: usize,
    /// The where clause arena length.
    where_clauses_len: usize,
    /// The dependency item arena length.
    dependency_items_len: usize,
    /// The generic parameter arena length.
    generic_parameters_len: usize,
    /// The parameter arena length.
    parameters_len: usize,
    /// The argument arena length.
    arguments_len: usize,
    /// The generic argument arena length.
    generic_arguments_len: usize,
    /// The tuple element arena length.
    tuple_elements_len: usize,
    /// The match case arena length.
    match_cases_len: usize,
    /// The pattern arena length.
    patterns_len: usize,
    /// The pattern field arena length.
    pattern_fields_len: usize,
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
    /// Dense local id and node type metadata by global node id.
    pub(crate) node_index_by_node_id: Vec<NodeIndexEntry>,
    /// The decorators attached to nodes.
    pub(crate) decorators_by_node_id: FxHashMap<u32, Vec<LocalNodeId<Decorator>>>,
    /// Whether decorator vectors are already globally sorted by start span.
    pub(crate) decorators_are_sorted: bool,
    /// The spans of the NodeTree.
    pub source_map: NodeSourceMap,

    // node arenas
    pub(crate) expressions: Arena<Expression>,
    pub(crate) type_expressions: Arena<TypeExpression>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) declarations: Arena<Declaration>,
    pub(crate) declarators: Arena<Declarator>,
    pub(crate) properties: Arena<Property>,
    pub(crate) type_members: Arena<TypeMember>,
    pub(crate) members: Arena<Member>,
    pub(crate) enum_fields: Arena<EnumField>,
    pub(crate) where_clauses: Arena<WhereClause>,
    pub(crate) dependency_items: Arena<DependencyItem>,
    pub(crate) generic_parameters: Arena<GenericParameter>,
    pub(crate) parameters: Arena<Parameter>,
    pub(crate) arguments: Arena<Argument>,
    pub(crate) generic_arguments: Arena<GenericArgument>,
    pub(crate) tuple_elements: Arena<TupleElement>,
    pub(crate) match_cases: Arena<MatchCase>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) pattern_fields: Arena<PatternField>,
    pub(crate) comments: Vec<Comment>,
    pub(crate) decorators: Arena<Decorator>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("next_global_id", &self.next_global_id)
            .field("node_count", &self.node_index_by_node_id.len())
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
            node_index_by_node_id: Vec::with_capacity(capacity),
            decorators_by_node_id: FxHashMap::with_capacity_and_hasher(
                capacity / 8,
                Default::default(),
            ),
            decorators_are_sorted: true,
            source_map: NodeSourceMap::with_capacity(capacity),
            expressions: Arena::with(capacity),
            type_expressions: Arena::with(capacity / 4),
            blocks: Arena::with(capacity / 8),
            declarations: Arena::with(capacity / 8),
            declarators: Arena::with(capacity / 8),
            properties: Arena::with(capacity / 4),
            type_members: Arena::with(capacity / 4),
            members: Arena::with(capacity / 8),
            enum_fields: Arena::with(capacity / 16),
            where_clauses: Arena::with(capacity / 16),
            dependency_items: Arena::with(capacity / 16),
            generic_parameters: Arena::with(capacity / 16),
            parameters: Arena::with(capacity / 8),
            arguments: Arena::with(capacity / 4),
            generic_arguments: Arena::with(capacity / 4),
            tuple_elements: Arena::with(capacity / 4),
            match_cases: Arena::with(capacity / 16),
            patterns: Arena::with(capacity / 8),
            pattern_fields: Arena::with(capacity / 8),
            comments: Vec::with_capacity(capacity / 16),
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
        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.source_map.append(span);
        LocalNodeId::new(global_id)
    }

    /// Allocate a new node while building a fresh tree during parsing.
    pub fn insert_during_parse<T>(&mut self, node: T, span: Span) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.source_map.append_during_parse(span);
        LocalNodeId::new(global_id)
    }

    /// Prune nodes from the tree. Resets the next id to the given index.
    #[inline]
    pub fn reset_to(&mut self, from_idx: u32) {
        self.node_index_by_node_id.truncate(from_idx as usize);
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
            type_expressions_len: self.type_expressions.len(),
            blocks_len: self.blocks.len(),
            declarations_len: self.declarations.len(),
            declarators_len: self.declarators.len(),
            properties_len: self.properties.len(),
            type_members_len: self.type_members.len(),
            members_len: self.members.len(),
            enum_fields_len: self.enum_fields.len(),
            where_clauses_len: self.where_clauses.len(),
            dependency_items_len: self.dependency_items.len(),
            generic_parameters_len: self.generic_parameters.len(),
            parameters_len: self.parameters.len(),
            arguments_len: self.arguments.len(),
            generic_arguments_len: self.generic_arguments.len(),
            tuple_elements_len: self.tuple_elements.len(),
            match_cases_len: self.match_cases.len(),
            patterns_len: self.patterns.len(),
            pattern_fields_len: self.pattern_fields.len(),
            comments_len: self.comments.len(),
            decorators_len: self.decorators.len(),
        }
    }

    /// Restore tree allocation lengths from a speculative mark.
    #[inline]
    pub fn restore_to_mark(&mut self, mark: NodeTreeMark) {
        self.node_index_by_node_id
            .truncate(mark.next_global_id as usize);
        self.source_map.prune_from(mark.next_global_id);
        self.next_global_id = mark.next_global_id;

        self.expressions.truncate(mark.expressions_len);
        self.type_expressions.truncate(mark.type_expressions_len);
        self.blocks.truncate(mark.blocks_len);
        self.declarations.truncate(mark.declarations_len);
        self.declarators.truncate(mark.declarators_len);
        self.properties.truncate(mark.properties_len);
        self.type_members.truncate(mark.type_members_len);
        self.members.truncate(mark.members_len);
        self.enum_fields.truncate(mark.enum_fields_len);
        self.where_clauses.truncate(mark.where_clauses_len);
        self.dependency_items.truncate(mark.dependency_items_len);
        self.generic_parameters
            .truncate(mark.generic_parameters_len);
        self.parameters.truncate(mark.parameters_len);
        self.arguments.truncate(mark.arguments_len);
        self.generic_arguments.truncate(mark.generic_arguments_len);
        self.tuple_elements.truncate(mark.tuple_elements_len);
        self.match_cases.truncate(mark.match_cases_len);
        self.patterns.truncate(mark.patterns_len);
        self.pattern_fields.truncate(mark.pattern_fields_len);
        self.comments.truncate(mark.comments_len);
        self.decorators.truncate(mark.decorators_len);

        // drop decorator links that point outside the restored node range
        self.decorators_by_node_id
            .retain(|target_id, decorator_ids| {
                if *target_id >= mark.next_global_id {
                    return false;
                }
                decorator_ids.retain(|decorator_id| decorator_id.id < mark.next_global_id);
                !decorator_ids.is_empty()
            });
    }

    /// Get the type of an untyped node id.
    #[inline]
    pub fn get_node_type(&self, id: u32) -> NodeType {
        self.node_index_by_node_id[id as usize].node_type()
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.node_index_by_node_id[id.id as usize].local_id();
        <Self as NodeTreeImpl<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.node_index_by_node_id[id.id as usize].local_id();
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

    /// Get the head span for a node.
    #[inline]
    pub fn get_head_span<T>(&self, node_id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.source_map.get_side(node_id.id, NodeSpanType::Head)
    }

    /// Get the head span for a node by its id.
    #[inline]
    pub fn get_head_span_by_id(&self, node_id: u32) -> Option<Span> {
        self.source_map.get_side(node_id, NodeSpanType::Head)
    }

    /// Set the main span for a node (identifier span for declarations, etc).
    #[inline]
    pub fn set_main_span<T>(&mut self, node_id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_map.set_main(node_id.id, span);
    }

    /// Set the head span for a node.
    #[inline]
    pub fn set_head_span<T>(&mut self, node_id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_map
            .set_side(node_id.id, NodeSpanType::Head, span);
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
        for (idx, entry) in self.node_index_by_node_id.iter().enumerate() {
            if entry.node_type() == node_type {
                spans.push(self.source_map.get(idx as u32));
            }
        }
        spans
    }

    /// Get the spans for decorator side nodes in one scan.
    #[inline]
    pub fn get_side_decorator_spans(&self) -> Vec<Span> {
        let mut spans = Vec::new();
        for (idx, entry) in self.node_index_by_node_id.iter().enumerate() {
            if matches!(entry.node_type(), NodeType::Decorator) {
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
        for (idx, entry) in self.node_index_by_node_id.iter().enumerate() {
            if entry.node_type() == T::TYPE {
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
        self.node_index_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_id, entry)| {
                if entry.node_type() == T::TYPE {
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
        self.node_index_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_id, entry)| {
                if entry.node_type() == T::TYPE {
                    Some(LocalNodeId::new(global_id as u32))
                } else {
                    None
                }
            })
    }

    /// Append a decorator to a node by its global id.
    #[inline]
    pub fn append_decorator(&mut self, target_id: u32, decorator: LocalNodeId<Decorator>) {
        debug_assert!(target_id < self.next_global_id);

        // keep vectors sorted by span order without paying a sort pass in the common case
        let decorator_ids = self.decorators_by_node_id.entry(target_id).or_default();
        if let Some(previous_decorator) = decorator_ids.last().copied() {
            let previous_span = self.source_map.get(previous_decorator.id);
            let current_span = self.source_map.get(decorator.id);
            let is_in_non_decreasing_order = previous_span.start < current_span.start
                || previous_span.start == current_span.start
                    && (previous_span.end < current_span.end
                        || previous_span.end == current_span.end
                            && previous_decorator.id <= decorator.id);
            if !is_in_non_decreasing_order {
                self.decorators_are_sorted = false;
            }
        }

        decorator_ids.push(decorator);
    }

    /// Whether there are any decorators attached to a node.
    #[inline]
    pub fn has_decorators(&self, node_id: u32) -> bool {
        self.decorators_by_node_id.contains_key(&node_id)
    }

    /// Has prefix decorators attached to a node.
    #[inline]
    pub fn has_prefix_decorators(&self, node_id: u32) -> bool {
        self.get_decorators_ref(node_id)
            .iter()
            .any(|&decorator_id| {
                let decorator = self.get(decorator_id);
                decorator.position == DecoratorPosition::BlockPrefix
                    || decorator.position == DecoratorPosition::LinePrefix
            })
    }

    /// Has postfix decorators attached to a node.
    #[inline]
    pub fn has_postfix_decorators(&self, node_id: u32) -> bool {
        self.get_decorators_ref(node_id)
            .iter()
            .any(|&decorator_id| {
                let decorator = self.get(decorator_id);
                decorator.position == DecoratorPosition::BlockPostfix
                    || decorator.position == DecoratorPosition::LinePostfix
                    || decorator.position == DecoratorPosition::LinePostfixBoundary
            })
    }

    /// Has infix decorators attached to a node.
    #[inline]
    pub fn has_infix_decorators(&self, node_id: u32) -> bool {
        self.get_decorators_ref(node_id)
            .iter()
            .any(|&decorator_id| {
                let decorator = self.get(decorator_id);
                decorator.position == DecoratorPosition::BlockInfix
            })
    }

    /// Get decorators attached to a node by reference.
    #[inline]
    pub fn get_decorators_ref(&self, node_id: u32) -> &[LocalNodeId<Decorator>] {
        self.decorators_by_node_id
            .get(&node_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Get decorators attached to a node.
    #[inline]
    pub fn get_decorators(&self, node_id: u32) -> Vec<LocalNodeId<Decorator>> {
        self.get_decorators_ref(node_id).to_vec()
    }

    /// Get all decorators.
    #[inline]
    pub fn get_all_decorators(&self) -> &FxHashMap<u32, Vec<LocalNodeId<Decorator>>> {
        &self.decorators_by_node_id
    }

    /// Return all raw comments in source order.
    #[inline]
    pub fn comments(&self) -> &[Comment] {
        &self.comments
    }

    /// Return all raw comments in source order, mutably.
    #[inline]
    pub fn comments_mut(&mut self) -> &mut Vec<Comment> {
        &mut self.comments
    }

    /// Append one raw comment.
    #[inline]
    pub fn push_comment(&mut self, comment: Comment) {
        self.comments.push(comment);
    }

    /// Sort all decorators.
    #[inline]
    pub fn sort_decorators(&mut self) {
        if self.decorators_are_sorted {
            return;
        }

        let source_map = &self.source_map;
        for decorator_ids in self.decorators_by_node_id.values_mut() {
            decorator_ids.sort_by(|left, right| {
                let left_span = source_map.get(left.id);
                let right_span = source_map.get(right.id);
                left_span
                    .start
                    .cmp(&right_span.start)
                    .then(left_span.end.cmp(&right_span.end))
                    .then(left.id.cmp(&right.id))
            });
        }

        self.decorators_are_sorted = true;
    }

    /// Remap decorator target node ids in one linear pass.
    #[inline]
    pub fn remap_decorator_targets(
        &mut self,
        mut remap: impl FnMut(u32, LocalNodeId<Decorator>, &Decorator, Span) -> u32,
    ) {
        // collect existing edges so target remaps can rewrite map keys safely
        let mut entries = Vec::new();
        for (target_id, decorator_ids) in &self.decorators_by_node_id {
            for &decorator_id in decorator_ids {
                let decorator = self.get(decorator_id);
                let decorator_span = self.source_map.get(decorator_id.id);
                let remapped_target_id = remap(*target_id, decorator_id, decorator, decorator_span);
                entries.push((remapped_target_id, decorator_id));
            }
        }

        // rebuild attachment map from remapped edges
        self.decorators_by_node_id.clear();
        self.decorators_are_sorted = true;
        for (target_id, decorator_id) in entries {
            self.append_decorator(target_id, decorator_id);
        }
    }

    /// Move matching decorators from one node id to another.
    #[inline]
    pub fn move_decorators_if(
        &mut self,
        from_target_id: u32,
        to_target_id: u32,
        mut should_move: impl FnMut(LocalNodeId<Decorator>, &Decorator, Span) -> bool,
    ) -> usize {
        if from_target_id == to_target_id {
            return 0;
        }

        let Some(mut decorator_ids) = self.decorators_by_node_id.remove(&from_target_id) else {
            return 0;
        };

        let mut moved_decorator_ids = Vec::new();
        decorator_ids.retain(|decorator_id| {
            let decorator = self.get(*decorator_id);
            let decorator_span = self.source_map.get(decorator_id.id);
            if should_move(*decorator_id, decorator, decorator_span) {
                moved_decorator_ids.push(*decorator_id);
                return false;
            }
            true
        });

        if !decorator_ids.is_empty() {
            self.decorators_by_node_id
                .insert(from_target_id, decorator_ids);
        }

        let moved_count = moved_decorator_ids.len();
        for decorator_id in moved_decorator_ids {
            self.append_decorator(to_target_id, decorator_id);
        }
        moved_count
    }

    /// Build position index for fast enclosing span lookups.
    /// Call this after parsing is complete.
    #[inline]
    pub fn build_position_index(&mut self) {
        self.source_map.build_position_index();
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
    TypeExpression => type_expressions,
    Block => blocks,
    Declaration => declarations,
    Declarator => declarators,
    Property => properties,
    TypeMember => type_members,
    Member => members,
    EnumField => enum_fields,
    WhereClause => where_clauses,
    DependencyItem => dependency_items,
    GenericParameter => generic_parameters,
    Parameter => parameters,
    Argument => arguments,
    GenericArgument => generic_arguments,
    TupleElement => tuple_elements,
    MatchCase => match_cases,
    Pattern => patterns,
    PatternField => pattern_fields,
    Decorator => decorators,
}
