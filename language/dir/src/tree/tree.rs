use destack_serde::Reflect;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Debug, Formatter};

use destack_core::StringId;
use destack_source::{
    ByteRange, FileId, ModuleId, MultiSpan, NodeSpanRegion, NodeSpanType, ProvenanceId,
    SourceIndex, Span,
};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::index::NodeIndexEntry;
use super::sparse::SparseNodeMap;
use crate::{
    Arena, Argument, AssignPattern, AssignPatternField, Block, Catch, Declaration, Declarator,
    Decorator, DependencyItem, Documentation, EnumField, Expression, GenericArgument,
    GenericParameter, LocalNodeId, LocalNodeIdAny, MatchArm, Member, Node, NodeParentIndex,
    NodeType, Parameter, Path, Pattern, PatternField, Property, SwitchCase, TreeAttribute,
    TreeCapacity, TreeChild, TreeMark, TreeStore, TupleElement, TypeExpression,
    TypeMappedParameter, TypeMember, WhereClause,
};

/// Mutable DIR tree across a set of related source units.
#[derive(Clone, Serialize, Deserialize, Reflect)]
pub struct Tree {
    /// The module id of the tree.
    pub module_id: ModuleId,

    // node index
    /// The first global node id stored in this tree.
    pub(crate) first_global_id: u32,
    /// The next id to allocate.
    pub(crate) next_global_id: u32,
    /// Dense local id and node type metadata by global node id.
    pub(crate) node_index_by_node_id: Vec<NodeIndexEntry>,
    /// Source ranges and anchors keyed by DIR node id.
    pub source_index: SourceIndex,
    /// The provenance of each node stored in this tree.
    provenance: Vec<ProvenanceId>,

    // node arenas
    pub(crate) expressions: Arena<Expression>,
    pub(crate) type_expressions: Arena<TypeExpression>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) catches: Arena<Catch>,
    pub(crate) declarations: Arena<Declaration>,
    pub(crate) declarators: Arena<Declarator>,
    pub(crate) properties: Arena<Property>,
    pub(crate) type_members: Arena<TypeMember>,
    pub(crate) type_mapped_parameters: Arena<TypeMappedParameter>,
    pub(crate) members: Arena<Member>,
    pub(crate) enum_fields: Arena<EnumField>,
    pub(crate) where_clauses: Arena<WhereClause>,
    pub(crate) dependency_items: Arena<DependencyItem>,
    pub(crate) generic_parameters: Arena<GenericParameter>,
    pub(crate) parameters: Arena<Parameter>,
    pub(crate) generic_arguments: Arena<GenericArgument>,
    pub(crate) tuple_elements: Arena<TupleElement>,
    pub(crate) arguments: Arena<Argument>,
    pub(crate) tree_attributes: Arena<TreeAttribute>,
    pub(crate) tree_children: Arena<TreeChild>,
    pub(crate) match_arms: Arena<MatchArm>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) pattern_fields: Arena<PatternField>,
    pub(crate) assign_patterns: Arena<AssignPattern>,
    pub(crate) assign_pattern_fields: Arena<AssignPatternField>,
    pub(crate) decorators: Arena<Decorator>,
    pub(crate) switch_cases: Arena<SwitchCase>,

    // node side data
    /// Structural membership and parent ids, rebuilt from roots and updated in place.
    parents: NodeParentIndex,
    /// The decorators attached to nodes.
    decorators_by_node_id: BTreeMap<u32, Vec<LocalNodeId<Decorator>>>,
    /// Decorator attachments in insertion order for rollback.
    #[serde(skip)]
    decorator_attachments: Vec<DecoratorAttachment>,
    /// The parsed documentation attached to nodes.
    documentation_by_node_id: SparseNodeMap<Documentation>,
    /// The detached node ids.
    detached_node_ids: BTreeSet<u32>,
}

/// One decorator attachment side-table insertion.
#[derive(Debug, Copy, Clone)]
struct DecoratorAttachment {
    /// The decorated node id.
    target_id: u32,
    /// The decorator node id.
    decorator_id: LocalNodeId<Decorator>,
}

impl Debug for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("len", &self.node_index_by_node_id.len())
            .finish()
    }
}

impl Tree {
    /// Return whether this tree holds any tree literal expression.
    pub fn has_tree_expressions(&self) -> bool {
        self.expressions
            .iter()
            .any(|expression| matches!(expression, Expression::TreeExpression { .. }))
    }

    /// Create a new Tree.
    pub fn new(module_id: ModuleId) -> Self {
        Self::with_capacity(module_id, 0)
    }

    /// Create a new Tree with the given capacity.
    pub fn with_capacity(module_id: ModuleId, capacity: usize) -> Self {
        Self::with_capacities(module_id, TreeCapacity::nodes(capacity))
    }

    /// Create a new Tree with explicit family capacities.
    pub fn with_capacities(module_id: ModuleId, capacity: TreeCapacity) -> Self {
        Self {
            module_id,

            first_global_id: 0,
            next_global_id: 0,
            node_index_by_node_id: Vec::with_capacity(capacity.nodes),
            source_index: SourceIndex::with_capacity(capacity.nodes),
            provenance: Vec::with_capacity(capacity.nodes),

            expressions: Arena::with(capacity.expressions),
            type_expressions: Arena::with(capacity.type_expressions),
            blocks: Arena::new(),
            catches: Arena::new(),
            declarations: Arena::new(),
            declarators: Arena::new(),
            properties: Arena::new(),
            type_members: Arena::new(),
            type_mapped_parameters: Arena::new(),
            members: Arena::new(),
            enum_fields: Arena::new(),
            where_clauses: Arena::new(),
            dependency_items: Arena::new(),
            generic_parameters: Arena::new(),
            parameters: Arena::new(),
            generic_arguments: Arena::new(),
            tuple_elements: Arena::new(),
            arguments: Arena::new(),
            tree_attributes: Arena::new(),
            tree_children: Arena::new(),
            match_arms: Arena::new(),
            patterns: Arena::new(),
            pattern_fields: Arena::new(),
            assign_patterns: Arena::new(),
            assign_pattern_fields: Arena::new(),
            decorators: Arena::new(),
            switch_cases: Arena::new(),

            parents: NodeParentIndex::new(),
            decorators_by_node_id: BTreeMap::new(),
            decorator_attachments: Vec::new(),
            documentation_by_node_id: SparseNodeMap::new(),
            detached_node_ids: BTreeSet::new(),
        }
    }

    /// Reserve additional node and expression capacity.
    pub fn reserve(&mut self, capacity: TreeCapacity) {
        self.node_index_by_node_id.reserve(capacity.nodes);
        self.source_index.reserve(capacity.nodes);
        self.provenance.reserve(capacity.nodes);
        self.expressions.reserve(capacity.expressions);
        self.type_expressions.reserve(capacity.type_expressions);
    }

    /// Create a new tail tree after one immutable base tree.
    pub fn from_base(base: &Tree, capacity: usize) -> Self {
        let mut tree = Self::with_capacity(base.module_id, capacity);
        tree.first_global_id = base.next_global_id();
        tree.next_global_id = base.next_global_id();
        tree.source_index = SourceIndex::with_base(tree.first_global_id, capacity);
        tree.parents = NodeParentIndex::with_base(tree.first_global_id);

        tree
    }

    /// Return the first global node id stored in this tree.
    #[inline]
    pub fn first_global_id(&self) -> u32 {
        self.first_global_id
    }

    /// Return the next global node id this tree will allocate.
    #[inline]
    pub fn next_global_id(&self) -> u32 {
        self.next_global_id
    }

    /// Return the number of nodes stored in this tree.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.node_index_by_node_id.len()
    }

    /// Return whether the tree has no nodes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.node_index_by_node_id.is_empty()
    }

    /// Begin appending parsed nodes from one source file.
    #[inline]
    pub fn begin_source_file(&mut self, file: FileId) {
        self.source_index.begin_source_file(file);
    }

    /// Allocate one parsed node with its file-local source range.
    pub fn insert_parsed<T>(
        &mut self,
        node: T,
        range: ByteRange,
        provenance: ProvenanceId,
    ) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let node_id = self.allocate_node(node, provenance);
        self.source_index.append_parsed(range);

        node_id
    }

    /// Allocate one node and register its global node type.
    fn allocate_node<T>(&mut self, node: T, provenance: ProvenanceId) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;

        let local_id = <Self as TreeStore<T>>::allocate(self, node);
        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.provenance.push(provenance);

        LocalNodeId::new(global_id)
    }

    /// Snapshot tree allocation state for speculative restores.
    #[inline]
    pub fn mark(&self) -> TreeMark {
        TreeMark {
            next_global_id: self.next_global_id,
            decorator_attachments_len: self.decorator_attachments.len(),
        }
    }

    /// Restore tree allocation state from a speculative mark.
    #[inline]
    pub fn restore_to_mark(&mut self, mark: TreeMark) {
        self.restore_arena_tail(mark.next_global_id);

        let retained_node_count = self.node_count_at_global_id(mark.next_global_id);
        self.node_index_by_node_id.truncate(retained_node_count);
        self.provenance.truncate(retained_node_count);
        self.source_index.truncate(mark.next_global_id);
        self.next_global_id = mark.next_global_id;

        self.restore_decorator_attachments(mark);
        self.prune_node_side_tables(mark.next_global_id);
    }

    /// Return the retained node count before one global node id.
    #[inline]
    fn node_count_at_global_id(&self, node_id: u32) -> usize {
        node_id
            .checked_sub(self.first_global_id)
            .unwrap_or_else(|| panic!("DIR node id {node_id} is before this tree")) as usize
    }

    /// Restore typed arenas by replaying the global allocation tail.
    fn restore_arena_tail(&mut self, next_global_id: u32) {
        let retained_node_count = self.node_count_at_global_id(next_global_id);

        for index in (retained_node_count..self.node_index_by_node_id.len()).rev() {
            let entry = self.node_index_by_node_id[index];
            self.truncate_arena(entry.node_type(), entry.local_id() as usize);
        }
    }

    /// Truncate the arena that owns one node type.
    fn truncate_arena(&mut self, node_type: NodeType, len: usize) {
        match node_type {
            NodeType::Expression => self.expressions.truncate(len),
            NodeType::TypeExpression => self.type_expressions.truncate(len),
            NodeType::Block => self.blocks.truncate(len),
            NodeType::Catch => self.catches.truncate(len),
            NodeType::Declaration => self.declarations.truncate(len),
            NodeType::Declarator => self.declarators.truncate(len),
            NodeType::Property => self.properties.truncate(len),
            NodeType::TypeMember => self.type_members.truncate(len),
            NodeType::TypeMappedParameter => self.type_mapped_parameters.truncate(len),
            NodeType::Member => self.members.truncate(len),
            NodeType::EnumField => self.enum_fields.truncate(len),
            NodeType::WhereClause => self.where_clauses.truncate(len),
            NodeType::DependencyItem => self.dependency_items.truncate(len),
            NodeType::GenericParameter => self.generic_parameters.truncate(len),
            NodeType::Parameter => self.parameters.truncate(len),
            NodeType::GenericArgument => self.generic_arguments.truncate(len),
            NodeType::TupleElement => self.tuple_elements.truncate(len),
            NodeType::Argument => self.arguments.truncate(len),
            NodeType::TreeAttribute => self.tree_attributes.truncate(len),
            NodeType::TreeChild => self.tree_children.truncate(len),
            NodeType::MatchArm => self.match_arms.truncate(len),
            NodeType::Pattern => self.patterns.truncate(len),
            NodeType::PatternField => self.pattern_fields.truncate(len),
            NodeType::AssignPattern => self.assign_patterns.truncate(len),
            NodeType::AssignPatternField => self.assign_pattern_fields.truncate(len),
            NodeType::Decorator => self.decorators.truncate(len),
            NodeType::SwitchCase => self.switch_cases.truncate(len),
        }
    }

    /// Restore decorator side-table attachments after one mark.
    fn restore_decorator_attachments(&mut self, mark: TreeMark) {
        while self.decorator_attachments.len() > mark.decorator_attachments_len {
            let Some(attachment) = self.decorator_attachments.pop() else {
                break;
            };

            self.remove_decorator_attachment(attachment);
        }
    }

    /// Remove one decorator side-table attachment.
    fn remove_decorator_attachment(&mut self, attachment: DecoratorAttachment) {
        if let Some(decorator_ids) = self.decorators_by_node_id.get_mut(&attachment.target_id) {
            if decorator_ids.last() == Some(&attachment.decorator_id) {
                decorator_ids.pop();
            } else {
                decorator_ids.retain(|decorator_id| *decorator_id != attachment.decorator_id);
            }

            if decorator_ids.is_empty() {
                self.decorators_by_node_id.remove(&attachment.target_id);
            }
        }
    }

    /// Prune side tables that point at nodes allocated after one mark.
    fn prune_node_side_tables(&mut self, next_global_id: u32) {
        self.parents.truncate(next_global_id);
        self.documentation_by_node_id
            .retain(|node_id, _| node_id < next_global_id);
        self.detached_node_ids
            .retain(|node_id| *node_id < next_global_id);
    }

    /// Return the local metadata index for one global node id.
    #[inline]
    pub(crate) fn node_index(&self, node_id: u32) -> usize {
        let index = node_id
            .checked_sub(self.first_global_id)
            .unwrap_or_else(|| panic!("DIR node id {node_id} is before this tree"));
        let index = index as usize;
        assert!(
            index < self.node_index_by_node_id.len(),
            "DIR node id {node_id} is outside this tree"
        );

        index
    }

    /// Detach a node id from structural traversal.
    pub fn detach(&mut self, node_id: LocalNodeIdAny) {
        self.detached_node_ids.insert(node_id.id);
    }

    /// Check whether a node id is detached.
    pub fn is_detached(&self, node_id: u32) -> bool {
        self.detached_node_ids.contains(&node_id)
    }

    /// Iterate detached node ids.
    #[inline]
    pub fn detached_node_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.detached_node_ids.iter().copied()
    }

    /// Allocate one node with its source span and provenance.
    pub fn insert<T>(&mut self, node: T, span: Span, provenance: ProvenanceId) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let node_id = self.allocate_node(node, provenance);
        self.source_index.append(span);

        node_id
    }

    /// Allocate one node with the complete source ranges of another node.
    pub fn insert_from<T, U>(
        &mut self,
        node: T,
        source: LocalNodeId<U>,
        provenance: ProvenanceId,
    ) -> LocalNodeId<T>
    where
        T: Node,
        U: Node,
        Self: TreeStore<T>,
    {
        let node_id = self.allocate_node(node, provenance);
        self.source_index.append_from(source.id);

        node_id
    }

    /// Get the type of an untyped node id.
    #[inline]
    pub fn get_node_type(&self, id: u32) -> NodeType {
        self.node_index_by_node_id[self.node_index(id)].node_type()
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let local_id = self.node_index_by_node_id[self.node_index(id.id)].local_id();
        <Self as TreeStore<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let local_id = self.node_index_by_node_id[self.node_index(id.id)].local_id();
        <Self as TreeStore<T>>::get_mut(self, local_id)
    }

    /// Iterate over all nodes of a given type together with their NodeId.
    pub fn iter_nodes<'a, T>(&'a self) -> impl Iterator<Item = (LocalNodeId<T>, &'a T)> + 'a
    where
        T: Node + 'a,
        Self: TreeStore<T>,
    {
        self.node_index_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if entry.node_type() == T::TYPE {
                    let node_id = LocalNodeId::new(self.first_global_id + index as u32);
                    let node = <Self as TreeStore<T>>::get(self, entry.local_id());
                    Some((node_id, node))
                } else {
                    None
                }
            })
    }

    /// Iterate over all nodes ids.
    pub fn iter_node_ids(&self) -> impl Iterator<Item = LocalNodeIdAny> + '_ {
        self.node_index_by_node_id
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                LocalNodeIdAny::new(self.first_global_id + index as u32, entry.node_type())
            })
    }

    /// Iterate over all nodes ids of a given type.
    pub fn iter_node_ids_of_type<T>(&self) -> impl Iterator<Item = LocalNodeId<T>> + '_
    where
        T: Node,
        Self: TreeStore<T>,
    {
        self.node_index_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if entry.node_type() == T::TYPE {
                    let node_id = LocalNodeId::new(self.first_global_id + index as u32);
                    Some(node_id)
                } else {
                    None
                }
            })
    }

    /// Get the parent node id for a node id.
    #[inline]
    pub fn get_parent_id(&self, node_id: u32) -> Option<u32> {
        self.parents.get_by_id(node_id)
    }

    /// Return the structural parent index.
    #[inline]
    pub fn parents(&self) -> &NodeParentIndex {
        &self.parents
    }

    /// Build the structural parent index from reachable roots.
    pub fn index_parents<T>(&mut self, roots: &[LocalNodeId<T>])
    where
        T: Node,
    {
        self.parents = NodeParentIndex::from_roots(self, roots);
    }

    /// Get the parent node id for a node id.
    #[inline]
    pub fn get_parent(&self, node_id: u32) -> Option<LocalNodeIdAny> {
        self.get_parent_id(node_id).map(|parent_id| {
            LocalNodeIdAny::new(
                parent_id,
                self.node_index_by_node_id[self.node_index(parent_id)].node_type(),
            )
        })
    }

    /// Return the static name path a reference expression spells, or `None` for a dynamic step.
    pub fn reference_path(&self, id: LocalNodeId<Expression>) -> Option<Path> {
        let mut segments = SmallVec::<[StringId; 1]>::new();
        let mut current = id;

        loop {
            match self.get(current) {
                // the chain root is a bare identifier
                Expression::Identifier { name } => {
                    segments.push(*name);
                    segments.reverse();

                    return Some(Path { segments });
                }

                // extend through one named member segment
                Expression::Member {
                    left,
                    name: Some(name),
                    ..
                } => {
                    segments.push(*name);
                    current = *left;
                }

                // a dynamic step has no static path
                _ => return None,
            }
        }
    }

    /// Return the provenance of one node.
    #[inline]
    pub fn provenance(&self, node_id: u32) -> ProvenanceId {
        self.provenance[self.node_index(node_id)]
    }

    /// Return the enclosing source span for one DIR node.
    #[inline]
    pub fn get_span<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
    {
        self.source_index.get(node_id.id)
    }

    /// Return the file-local source range for one DIR node.
    #[inline]
    pub fn get_range<T>(&self, node_id: LocalNodeId<T>) -> ByteRange
    where
        T: Node,
    {
        self.source_index.get_range(node_id.id)
    }

    /// Return the concrete source extent owned by one DIR node.
    #[inline]
    pub fn get_source_extent<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
    {
        self.get_source_extent_by_id(node_id.id)
    }

    /// Return the concrete source extent owned by one DIR node.
    pub fn get_source_extent_by_id(&self, node_id: u32) -> Span {
        let span = self.get_span_by_id(node_id);
        let parentheses_span = self
            .source_index
            .get_side(node_id, NodeSpanType::Region(NodeSpanRegion::Parentheses));
        let tree_container_span = self
            .source_index
            .get_side(node_id, NodeSpanType::Region(NodeSpanRegion::TreeContainer));
        let span = parentheses_span.map_or(span, |parentheses| span.merge(parentheses));

        tree_container_span.map_or(span, |container| span.merge(container))
    }

    /// Set the enclosing source span for one DIR node.
    #[inline]
    pub fn set_span<T>(&mut self, node_id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_index.set(node_id.id, span);
    }

    /// Set the file-local source range for one DIR node.
    #[inline]
    pub fn set_range<T>(&mut self, node_id: LocalNodeId<T>, range: ByteRange)
    where
        T: Node,
    {
        self.source_index.set_range(node_id.id, range);
    }

    /// Return the enclosing source span for one DIR node.
    #[inline]
    pub fn get_span_by_id(&self, node_id: u32) -> Span {
        self.node_index(node_id);

        self.source_index.get(node_id)
    }

    /// Return the main source span for one DIR node.
    #[inline]
    pub fn get_main_span<T>(&self, node_id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.source_index.get_main(node_id.id)
    }

    /// Return the file-local main source range for one DIR node.
    #[inline]
    pub fn get_main_range<T>(&self, node_id: LocalNodeId<T>) -> Option<ByteRange>
    where
        T: Node,
    {
        self.source_index
            .get_side_range(node_id.id, NodeSpanType::Main)
    }

    /// Return the main source span for one DIR node id.
    #[inline]
    pub fn get_main_span_by_id(&self, node_id: u32) -> Option<Span> {
        self.source_index.get_main(node_id)
    }

    /// Find the innermost typed node span containing one source span.
    #[inline]
    pub fn find_innermost_node_span_owner(
        &self,
        span: Span,
    ) -> Option<destack_source::NodeSpanKey> {
        self.source_index.find_innermost_node_span_owner(span)
    }

    /// Set the main source span for one DIR node.
    #[inline]
    pub fn set_main_span<T>(&mut self, node_id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_index.set_main(node_id.id, span);
    }

    /// Set the file-local main source range for one DIR node.
    #[inline]
    pub fn set_main_range<T>(&mut self, node_id: LocalNodeId<T>, range: ByteRange)
    where
        T: Node,
    {
        self.source_index
            .set_side_range(node_id.id, NodeSpanType::Main, range);
    }

    /// Return the head source span for one DIR node.
    #[inline]
    pub fn get_head_span<T>(&self, node_id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.source_index.get_side(node_id.id, NodeSpanType::Head)
    }

    /// Return the file-local head source range for one DIR node.
    #[inline]
    pub fn get_head_range<T>(&self, node_id: LocalNodeId<T>) -> Option<ByteRange>
    where
        T: Node,
    {
        self.source_index
            .get_side_range(node_id.id, NodeSpanType::Head)
    }

    /// Return the head source span for one DIR node id.
    #[inline]
    pub fn get_head_span_by_id(&self, node_id: u32) -> Option<Span> {
        self.source_index.get_side(node_id, NodeSpanType::Head)
    }

    /// Set the head source span for one DIR node.
    #[inline]
    pub fn set_head_span<T>(&mut self, node_id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_index
            .set_side(node_id.id, NodeSpanType::Head, span);
    }

    /// Set the file-local head source range for one DIR node.
    #[inline]
    pub fn set_head_range<T>(&mut self, node_id: LocalNodeId<T>, range: ByteRange)
    where
        T: Node,
    {
        self.source_index
            .set_side_range(node_id.id, NodeSpanType::Head, range);
    }

    /// Set one side source span for one DIR node.
    #[inline]
    pub fn set_side_span<T>(&mut self, node_id: LocalNodeId<T>, span_type: NodeSpanType, span: Span)
    where
        T: Node,
    {
        self.source_index.set_side(node_id.id, span_type, span);
    }

    /// Set one file-local side source range for one DIR node.
    #[inline]
    pub fn set_side_range<T>(
        &mut self,
        node_id: LocalNodeId<T>,
        span_type: NodeSpanType,
        range: ByteRange,
    ) where
        T: Node,
    {
        self.source_index
            .set_side_range(node_id.id, span_type, range);
    }

    /// Return one side source span for one DIR node.
    #[inline]
    pub fn get_side_span<T>(&self, node_id: LocalNodeId<T>, span_type: NodeSpanType) -> Option<Span>
    where
        T: Node,
    {
        self.source_index.get_side(node_id.id, span_type)
    }

    /// Return one file-local side source range for one DIR node.
    #[inline]
    pub fn get_side_range<T>(
        &self,
        node_id: LocalNodeId<T>,
        span_type: NodeSpanType,
    ) -> Option<ByteRange>
    where
        T: Node,
    {
        self.source_index.get_side_range(node_id.id, span_type)
    }

    /// Return one side source span for one DIR node id.
    #[inline]
    pub fn get_side_span_by_id(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        self.source_index.get_side(node_id, span_type)
    }

    /// Return one file-local side source range for one DIR node id.
    #[inline]
    pub fn get_side_range_by_id(&self, node_id: u32, span_type: NodeSpanType) -> Option<ByteRange> {
        self.source_index.get_side_range(node_id, span_type)
    }

    /// Set one side source span for one DIR node id.
    #[inline]
    pub fn set_side_span_by_id(&mut self, node_id: u32, span_type: NodeSpanType, span: Span) {
        self.source_index.set_side(node_id, span_type, span);
    }

    /// Set one file-local side source range for one DIR node id.
    #[inline]
    pub fn set_side_range_by_id(
        &mut self,
        node_id: u32,
        span_type: NodeSpanType,
        range: ByteRange,
    ) {
        self.source_index.set_side_range(node_id, span_type, range);
    }

    /// Return the spans for all nodes of a given type.
    #[inline]
    pub fn get_spans_for(&self, node_type: NodeType) -> Vec<Span> {
        let mut spans = Vec::new();
        for (index, entry) in self.node_index_by_node_id.iter().enumerate() {
            if entry.node_type() == node_type {
                let node_id = self.first_global_id + index as u32;
                spans.push(self.source_index.get(node_id));
            }
        }

        spans
    }

    /// Return the spans for decorator side nodes.
    #[inline]
    pub fn get_side_decorator_spans(&self) -> Vec<Span> {
        self.get_spans_for(NodeType::Decorator)
    }

    /// Return the combined span of all decorator nodes.
    #[inline]
    pub fn decorator_span(&self) -> MultiSpan {
        MultiSpan::new(self.get_side_decorator_spans())
    }

    /// Return all decorator attachments.
    #[inline]
    pub fn get_all_decorators(&self) -> &BTreeMap<u32, Vec<LocalNodeId<Decorator>>> {
        &self.decorators_by_node_id
    }

    /// Build the source position index for fast enclosing span lookups.
    #[inline]
    pub fn build_position_index(&mut self) {
        self.source_index.build_position_index();
    }

    /// Return true when the node id exists in this tree.
    #[inline]
    pub fn has_node_id(&self, node_id: u32) -> bool {
        node_id >= self.first_global_id
            && ((node_id - self.first_global_id) as usize) < self.node_index_by_node_id.len()
    }

    /// Attach a decorator to a node in source order.
    #[inline]
    pub fn attach_decorator(&mut self, target_id: u32, decorator: LocalNodeId<Decorator>) {
        debug_assert!(target_id < self.next_global_id);

        let decorator_start = self.get_span(decorator).start;
        let position = self
            .decorators_by_node_id
            .get(&target_id)
            .map(|decorators| {
                decorators
                    .partition_point(|candidate| self.get_span(*candidate).start <= decorator_start)
            })
            .unwrap_or(0);

        // track decorators for the target node; index_parents derives the decorator
        // parent from this map, so no parent slot is written here
        self.decorators_by_node_id
            .entry(target_id)
            .or_default()
            .insert(position, decorator);
        self.decorator_attachments.push(DecoratorAttachment {
            target_id,
            decorator_id: decorator,
        });
    }

    /// Whether there are any decorators attached to a node.
    #[inline]
    pub fn has_decorators(&self, node_id: u32) -> bool {
        self.decorators_by_node_id.contains_key(&node_id)
    }

    /// Get decorators attached to a node.
    #[inline]
    pub fn get_decorators(&self, node_id: u32) -> Vec<LocalNodeId<Decorator>> {
        self.decorators_by_node_id
            .get(&node_id)
            .cloned()
            .unwrap_or_else(Vec::new)
    }

    /// Get decorators attached to a node as a borrowed slice.
    #[inline]
    pub fn get_decorators_ref(&self, node_id: u32) -> &[LocalNodeId<Decorator>] {
        self.decorators_by_node_id
            .get(&node_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Set parsed documentation for a node.
    #[inline]
    pub fn set_documentation(&mut self, node_id: u32, documentation: Documentation) {
        self.documentation_by_node_id.insert(node_id, documentation);
    }

    /// Return whether one node has parsed documentation.
    #[inline]
    pub fn has_documentation(&self, node_id: u32) -> bool {
        self.documentation_by_node_id.get_ref(node_id).is_some()
    }

    /// Get parsed documentation attached to a node.
    #[inline]
    pub fn get_documentation(&self, node_id: u32) -> Option<&Documentation> {
        self.documentation_by_node_id.get_ref(node_id)
    }

    /// Iterate over documented node IDs and their documentation.
    #[inline]
    pub fn iter_documentation(&self) -> impl Iterator<Item = (u32, &Documentation)> {
        self.documentation_by_node_id.iter()
    }

    /// Take parsed documentation from one node.
    #[inline]
    pub fn take_documentation(&mut self, node_id: u32) -> Option<Documentation> {
        self.documentation_by_node_id.take(node_id)
    }
}
