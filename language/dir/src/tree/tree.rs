use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Debug, Formatter};

use destack_source::{ModuleId, NodeSpanRegion, NodeSpanType, SourceIndex, Span};
use serde::{Deserialize, Serialize};

use super::index::NodeIndexEntry;
use super::parent::reparent_direct_children;
use super::sparse::SparseNodeMap;
use crate::{
    Arena, Argument, AssignPattern, AssignPatternField, Block, Catch, Comment, Declaration,
    Declarator, Decorator, DependencyItem, Documentation, EnumField, Expression, GenericArgument,
    GenericParameter, LocalNodeId, LocalNodeIdAny, MatchCase, Member, Node, NodeType, Parameter,
    Pattern, PatternField, Property, TreeCapacity, TreeMark, TreeStore, TupleElement,
    TypeExpression, TypeMappedParameter, TypeMember, WhereClause,
};

/// Mutable DIR tree across a set of related source units.
#[derive(Clone, Serialize, Deserialize)]
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
    /// Source ranges and anchors keyed by parsed node id.
    pub source_index: SourceIndex,

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
    pub(crate) match_cases: Arena<MatchCase>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) pattern_fields: Arena<PatternField>,
    pub(crate) assign_patterns: Arena<AssignPattern>,
    pub(crate) assign_pattern_fields: Arena<AssignPatternField>,
    pub(crate) comments: Vec<Comment>,
    pub(crate) decorators: Arena<Decorator>,

    // node side data
    /// Parent node id overrides by node id.
    parent_id_by_node_id: SparseNodeMap<u32>,
    /// Source node id overrides by DIR node id.
    source_id_by_node_id: SparseNodeMap<u32>,
    /// The alias node id by source node id.
    alias_node_id_by_source_id: BTreeMap<u32, u32>,
    /// The alias node id by DIR node id.
    alias_node_id_by_node_id: BTreeMap<u32, u32>,
    /// The decorators attached to nodes.
    decorators_by_node_id: BTreeMap<u32, Vec<LocalNodeId<Decorator>>>,
    /// Decorator attachments in insertion order for rollback.
    #[serde(skip)]
    decorator_attachments: Vec<DecoratorAttachment>,
    /// The normalized documentation attached to nodes.
    documentation_by_node_id: BTreeMap<u32, Documentation>,
    /// Final source span overrides by node id.
    source_span_by_node_id: SparseNodeMap<Span>,
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
    /// The parent slot value before attachment.
    previous_parent_id: Option<u32>,
}

impl Debug for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("len", &self.node_index_by_node_id.len())
            .finish()
    }
}

impl Tree {
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
            match_cases: Arena::new(),
            patterns: Arena::new(),
            pattern_fields: Arena::new(),
            assign_patterns: Arena::new(),
            assign_pattern_fields: Arena::new(),
            comments: Vec::with_capacity(capacity.comments),
            decorators: Arena::new(),

            parent_id_by_node_id: SparseNodeMap::new(),
            source_id_by_node_id: SparseNodeMap::new(),
            alias_node_id_by_source_id: BTreeMap::new(),
            alias_node_id_by_node_id: BTreeMap::new(),
            decorators_by_node_id: BTreeMap::new(),
            decorator_attachments: Vec::new(),
            documentation_by_node_id: BTreeMap::new(),
            source_span_by_node_id: SparseNodeMap::new(),
            detached_node_ids: BTreeSet::new(),
        }
    }

    /// Create a new tail tree after one immutable base tree.
    pub fn from_base(base: &Tree, capacity: usize) -> Self {
        let mut tree = Self::with_capacity(base.module_id, capacity);
        tree.first_global_id = base.next_global_id();
        tree.next_global_id = base.next_global_id();

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

    /// Return the next id.
    #[inline]
    pub fn next_id(&self) -> u32 {
        self.next_global_id
    }

    /// Return whether the tree has no nodes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.node_index_by_node_id.is_empty()
    }

    /// Allocate a parsed node with source metadata.
    pub fn insert_during_parse<T>(&mut self, node: T, span: Span) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;

        let local_id = <Self as TreeStore<T>>::allocate(self, node);
        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.source_index.append_during_parse(span);

        LocalNodeId::new(global_id)
    }

    /// Snapshot tree allocation state for speculative restores.
    #[inline]
    pub fn mark(&self) -> TreeMark {
        TreeMark {
            next_global_id: self.next_global_id,
            comments_len: self.comments.len(),
            decorator_attachments_len: self.decorator_attachments.len(),
        }
    }

    /// Restore tree allocation state from a speculative mark.
    #[inline]
    pub fn restore_to_mark(&mut self, mark: TreeMark) {
        self.restore_arena_tail(mark.next_global_id);

        let retained_node_count = self.node_count_at_global_id(mark.next_global_id);
        self.node_index_by_node_id.truncate(retained_node_count);
        self.source_index
            .prune_from(retained_node_count, mark.next_global_id);
        self.next_global_id = mark.next_global_id;

        self.comments.truncate(mark.comments_len);
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

            if !entry.is_placeholder() {
                self.truncate_arena(entry.node_type(), entry.local_id() as usize);
            }
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
            NodeType::MatchCase => self.match_cases.truncate(len),
            NodeType::Pattern => self.patterns.truncate(len),
            NodeType::PatternField => self.pattern_fields.truncate(len),
            NodeType::AssignPattern => self.assign_patterns.truncate(len),
            NodeType::AssignPatternField => self.assign_pattern_fields.truncate(len),
            NodeType::Decorator => self.decorators.truncate(len),
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

        if attachment.decorator_id.id < self.next_global_id {
            self.set_parent_id(attachment.decorator_id.id, attachment.previous_parent_id);
        }
    }

    /// Prune side tables that point at nodes allocated after one mark.
    fn prune_node_side_tables(&mut self, next_global_id: u32) {
        self.alias_node_id_by_source_id
            .retain(|_, alias_id| *alias_id < next_global_id);
        self.alias_node_id_by_node_id
            .retain(|node_id, alias_id| *node_id < next_global_id && *alias_id < next_global_id);
        self.parent_id_by_node_id
            .retain(|node_id, parent_id| node_id < next_global_id && parent_id < next_global_id);
        self.source_id_by_node_id
            .retain(|node_id, source_id| node_id < next_global_id && source_id < next_global_id);
        self.source_span_by_node_id
            .retain(|node_id, _| node_id < next_global_id);
        self.documentation_by_node_id
            .retain(|node_id, _| *node_id < next_global_id);
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

    /// Reserve a new node slot in the tree for a node lowered from a source node.
    pub fn reserve_from_source(
        &mut self,
        node_type: NodeType,
        source_node_id: u32,
        parent_id: Option<LocalNodeIdAny>,
    ) -> LocalNodeIdAny {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;

        self.node_index_by_node_id
            .push(NodeIndexEntry::placeholder(node_type));
        self.set_parent_id(global_id, parent_id.map(|parent_id| parent_id.id));
        self.set_source_id(global_id, source_node_id);
        self.alias_node_id_by_source_id
            .insert(source_node_id, global_id);

        LocalNodeIdAny::new(global_id, node_type)
    }

    /// Reserve a new node slot in the tree for a node derived from another DIR node.
    pub fn reserve_from(
        &mut self,
        node_type: NodeType,
        dir_node_id: LocalNodeIdAny,
        parent_id: Option<LocalNodeIdAny>,
    ) -> LocalNodeIdAny {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;

        self.node_index_by_node_id
            .push(NodeIndexEntry::placeholder(node_type));
        self.set_parent_id(global_id, parent_id.map(|parent_id| parent_id.id));
        self.set_source_id(global_id, self.get_source(dir_node_id.id));
        if let Some(span) = self.get_span_by_id(dir_node_id.id) {
            self.set_source_span(global_id, span);
        }
        self.alias_node_id_by_node_id
            .insert(dir_node_id.id, global_id);

        LocalNodeIdAny::new(global_id, node_type)
    }

    /// Allocate one parsed node with source metadata.
    pub fn insert<T>(&mut self, node: T, span: Span) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeStore<T>,
    {
        self.insert_during_parse(node, span)
    }

    /// Fill in the node data for a previously reserved slot.
    pub fn insert_reserved<T>(&mut self, node_id: LocalNodeIdAny, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let local_id = <Self as TreeStore<T>>::allocate(self, node);
        let index = self.node_index(node_id.id);
        self.node_index_by_node_id[index] = NodeIndexEntry::new(local_id, T::TYPE);

        LocalNodeId::new(node_id.id)
    }

    /// Fill in one reserved slot and make the inserted node own its reused direct children.
    pub fn insert_as_owner<T>(&mut self, node_id: LocalNodeIdAny, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeStore<T>,
    {
        let node_id = self.insert_reserved(node_id, node);
        reparent_direct_children(self, LocalNodeIdAny::new(node_id.id, T::TYPE));

        node_id
    }

    /// Add an alias node for a lowered source id.
    pub fn alias_from_source<T>(&mut self, source_node_id: u32, alias: LocalNodeId<T>)
    where
        T: Node,
        Self: TreeStore<T>,
    {
        self.alias_node_id_by_source_id
            .insert(source_node_id, alias.id);
    }

    /// Add an alias node for a derived DIR id.
    pub fn alias_from<T>(&mut self, dir_id: u32, alias: LocalNodeId<T>)
    where
        T: Node,
        Self: TreeStore<T>,
    {
        self.alias_node_id_by_node_id.insert(dir_id, alias.id);
    }

    /// Get the alias node id for a DIR node id, if one exists.
    /// Returns the aliased node id, or None if no alias exists.
    #[inline]
    pub fn get_alias(&self, node_id: u32) -> Option<LocalNodeIdAny> {
        self.alias_node_id_by_node_id
            .get(&node_id)
            .copied()
            .map(|alias_id| {
                LocalNodeIdAny::new(
                    alias_id,
                    self.node_index_by_node_id[self.node_index(alias_id)].node_type(),
                )
            })
    }

    /// Resolve the final alias for a DIR node id, following the alias chain.
    /// Returns the final aliased node id, or the original if no aliases exist.
    #[inline]
    pub fn resolve_alias(&self, node_id: u32) -> LocalNodeIdAny {
        let mut current = node_id;
        while let Some(alias_id) = self.alias_node_id_by_node_id.get(&current) {
            current = *alias_id;
        }
        LocalNodeIdAny::new(
            current,
            self.node_index_by_node_id[self.node_index(current)].node_type(),
        )
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

    /// Replace one node in place and preserve its original payload at a detached id.
    pub fn replace<T>(&mut self, id: LocalNodeId<T>, replacement: T) -> LocalNodeId<T>
    where
        T: Node + Clone,
        Self: TreeStore<T>,
    {
        let original = self.get(id).clone();

        // preserve original at a detached id
        let preserved_id = self.reserve_from(T::TYPE, id.into_any(), None);
        let preserved_id: LocalNodeId<T> = self.insert_reserved(preserved_id, original);

        // hide preserved originals from structural traversal
        self.detach(preserved_id.into_any());

        // replace node payload
        *self.get_mut(id) = replacement;

        // keep reused child ids attached to the replacement
        reparent_direct_children(self, id.into_any());

        // record original payload for reverse lookup
        self.alias_from(id.id, preserved_id);

        preserved_id
    }

    /// Replace one node with the payload of another node and detach the source root.
    pub fn replace_from<T>(
        &mut self,
        id: LocalNodeId<T>,
        source_id: LocalNodeId<T>,
    ) -> LocalNodeId<T>
    where
        T: Node + Clone,
        Self: TreeStore<T>,
    {
        let replacement = self.get(source_id).clone();
        let preserved_id = self.replace(id, replacement);

        // detach the moved root
        self.detach(source_id.into_any());

        preserved_id
    }

    /// Iterate over all nodes of a given type together with their NodeId.
    pub fn iter_nodes_of_type<'a, T>(&'a self) -> impl Iterator<Item = (LocalNodeId<T>, &'a T)> + 'a
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

    /// Iterate all node ids of one type.
    #[inline]
    pub fn iter_nodes<'a, T>(&'a self) -> impl Iterator<Item = LocalNodeId<T>> + 'a
    where
        T: Node + 'a,
        Self: TreeStore<T>,
    {
        self.iter_node_ids_of_type::<T>().into_iter()
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
    pub fn iter_node_ids_of_type<T>(&self) -> Vec<LocalNodeId<T>>
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
            .collect()
    }

    /// Get the parent node id for a node id.
    #[inline]
    pub fn get_parent_id(&self, node_id: u32) -> Option<u32> {
        self.parent_id_by_node_id.get(node_id)
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

    /// Set the parent node id override for one node id.
    #[inline]
    pub(crate) fn set_parent_id(&mut self, node_id: u32, parent_id: Option<u32>) {
        self.node_index(node_id);
        if let Some(parent_id) = parent_id {
            self.parent_id_by_node_id.insert(node_id, parent_id);
        } else {
            self.parent_id_by_node_id.remove(node_id);
        }
    }

    /// Set the source node id override for one node id.
    #[inline]
    fn set_source_id(&mut self, node_id: u32, source_id: u32) {
        self.node_index(node_id);
        if source_id == node_id {
            self.source_id_by_node_id.remove(node_id);
        } else {
            self.source_id_by_node_id.insert(node_id, source_id);
        }
    }

    /// Get the source id of a node by its DIR node id.
    #[inline]
    pub fn get_source(&self, node_id: u32) -> u32 {
        self.node_index(node_id);
        self.source_id_by_node_id.get(node_id).unwrap_or(node_id)
    }

    /// Return the enclosing source span for one parsed node.
    #[inline]
    pub fn get_span<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
    {
        self.source_index.get(node_id.id)
    }

    /// Return the concrete source extent owned by one parsed node.
    #[inline]
    pub fn get_source_extent<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
    {
        let span = self.source_index.get(node_id.id);
        let wrapper_span = self
            .source_index
            .get_side(node_id.id, NodeSpanType::Region(NodeSpanRegion::Wrapper));

        wrapper_span.map_or(span, |wrapper_span| span.merge(wrapper_span))
    }

    /// Set the enclosing source span for one parsed node.
    #[inline]
    pub fn set_span<T>(&mut self, node_id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_index.set(node_id.id, span);
        self.source_span_by_node_id.remove(node_id.id);
    }

    /// Set the final source span for one DIR node.
    #[inline]
    pub fn set_source_span(&mut self, node_id: u32, span: Span) {
        self.node_index(node_id);
        if self.source_index.contains_node(node_id) && self.source_index.get(node_id) == span {
            self.source_span_by_node_id.remove(node_id);
        } else {
            self.source_span_by_node_id.insert(node_id, span);
        }
    }

    /// Return the final source span for one DIR node when known.
    #[inline]
    pub fn get_span_by_id(&self, node_id: u32) -> Option<Span> {
        self.node_index(node_id);
        self.source_span_by_node_id.get(node_id).or_else(|| {
            self.source_index
                .contains_node(node_id)
                .then(|| self.source_index.get(node_id))
        })
    }

    /// Return the main source span for one parsed node.
    #[inline]
    pub fn get_main_span<T>(&self, node_id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.source_index.get_main(node_id.id)
    }

    /// Return the main source span for one parsed node id.
    #[inline]
    pub fn get_main_span_by_id(&self, node_id: u32) -> Option<Span> {
        self.source_index.get_main(node_id)
    }

    /// Set the main source span for one parsed node.
    #[inline]
    pub fn set_main_span<T>(&mut self, node_id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_index.set_main(node_id.id, span);
    }

    /// Return the head source span for one parsed node.
    #[inline]
    pub fn get_head_span<T>(&self, node_id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.source_index.get_side(node_id.id, NodeSpanType::Head)
    }

    /// Return the head source span for one parsed node id.
    #[inline]
    pub fn get_head_span_by_id(&self, node_id: u32) -> Option<Span> {
        self.source_index.get_side(node_id, NodeSpanType::Head)
    }

    /// Set the head source span for one parsed node.
    #[inline]
    pub fn set_head_span<T>(&mut self, node_id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_index
            .set_side(node_id.id, NodeSpanType::Head, span);
    }

    /// Set one side source span for one parsed node.
    #[inline]
    pub fn set_side_span<T>(&mut self, node_id: LocalNodeId<T>, span_type: NodeSpanType, span: Span)
    where
        T: Node,
    {
        self.source_index.set_side(node_id.id, span_type, span);
    }

    /// Return one side source span for one parsed node.
    #[inline]
    pub fn get_side_span<T>(&self, node_id: LocalNodeId<T>, span_type: NodeSpanType) -> Option<Span>
    where
        T: Node,
    {
        self.source_index.get_side(node_id.id, span_type)
    }

    /// Return one side source span for one parsed node id.
    #[inline]
    pub fn get_side_span_by_id(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        self.source_index.get_side(node_id, span_type)
    }

    /// Set one side source span for one parsed node id.
    #[inline]
    pub fn set_side_span_by_id(&mut self, node_id: u32, span_type: NodeSpanType, span: Span) {
        self.source_index.set_side(node_id, span_type, span);
    }

    /// Return the spans for all nodes of a given type.
    #[inline]
    pub fn get_spans_for(&self, node_type: NodeType) -> Vec<Span> {
        let mut spans = Vec::new();
        for (node_id, entry) in self.node_index_by_node_id.iter().enumerate() {
            if entry.node_type() == node_type {
                spans.push(self.source_index.get(node_id as u32));
            }
        }

        spans
    }

    /// Return the spans for decorator side nodes.
    #[inline]
    pub fn get_side_decorator_spans(&self) -> Vec<Span> {
        self.get_spans_for(NodeType::Decorator)
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

    /// Get the DIR node id by its source id.
    #[inline]
    pub fn get_node_id_by_source_id(&self, source_node_id: u32) -> Option<LocalNodeIdAny> {
        self.alias_node_id_by_source_id
            .get(&source_node_id)
            .copied()
            .map(|node_id| {
                LocalNodeIdAny::new(
                    node_id,
                    self.node_index_by_node_id[self.node_index(node_id)].node_type(),
                )
            })
    }

    /// Append a decorator to a node by its global id.
    #[inline]
    pub fn append_decorator(&mut self, target_id: u32, decorator: LocalNodeId<Decorator>) {
        debug_assert!(target_id < self.next_global_id);

        let previous_parent_id = if decorator.id != target_id {
            self.get_parent_id(decorator.id)
        } else {
            None
        };

        // track decorators for the target node
        self.decorators_by_node_id
            .entry(target_id)
            .or_default()
            .push(decorator);
        self.decorator_attachments.push(DecoratorAttachment {
            target_id,
            decorator_id: decorator,
            previous_parent_id,
        });

        // attach the decorator to its target for parent lookups
        if decorator.id != target_id {
            self.set_parent_id(decorator.id, Some(target_id));
        }
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

    /// Set normalized documentation for a node.
    #[inline]
    pub fn set_documentation(&mut self, node_id: u32, documentation: Documentation) {
        self.documentation_by_node_id.insert(node_id, documentation);
    }

    /// Return whether one node has normalized documentation.
    #[inline]
    pub fn has_documentation(&self, node_id: u32) -> bool {
        self.documentation_by_node_id.contains_key(&node_id)
    }

    /// Get normalized documentation attached to a node.
    #[inline]
    pub fn get_documentation(&self, node_id: u32) -> Option<&Documentation> {
        self.documentation_by_node_id.get(&node_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_source::{FileId, ModuleId, PackageId, Span};

    use crate::{Decorator, DecoratorPosition, Expression, Tree, TypeExpression};

    fn test_module_id() -> ModuleId {
        ModuleId::new(PackageId::new(1), 1)
    }

    fn test_span(start: u32) -> Span {
        Span::new(FileId(1), start, start + 1)
    }

    #[test]
    fn test_restore_mark_replays_typed_arena_tail() {
        let mut tree = Tree::new(test_module_id());
        let owner = tree.insert(Expression::Stub, test_span(0));
        let mark = tree.mark();

        let ty = tree.insert(TypeExpression::Missing, test_span(1));
        let decorator_expression = tree.insert(Expression::Stub, test_span(2));
        let decorator = tree.insert(
            Decorator {
                expression: decorator_expression,
                position: DecoratorPosition::LinePrefix,
            },
            test_span(3),
        );
        tree.append_decorator(owner.id, decorator);

        assert_eq!(tree.get_decorators(owner.id), vec![decorator]);

        tree.restore_to_mark(mark);

        assert!(tree.get_decorators(owner.id).is_empty());
        assert!(!tree.has_node_id(ty.id));
        assert!(!tree.has_node_id(decorator_expression.id));
        assert!(!tree.has_node_id(decorator.id));
        assert_eq!(tree.next_global_id(), mark.next_global_id());
    }

    #[test]
    fn test_restore_mark_uses_tail_tree_local_node_count() {
        let mut base = Tree::new(test_module_id());
        base.insert(Expression::Stub, test_span(0));
        base.insert(Expression::Stub, test_span(1));

        let mut tree = Tree::from_base(&base, 4);
        let retained = tree.insert(Expression::Stub, test_span(2));
        let mark = tree.mark();
        let removed = tree.insert(TypeExpression::Missing, test_span(3));

        tree.restore_to_mark(mark);

        assert!(tree.has_node_id(retained.id));
        assert!(!tree.has_node_id(removed.id));
        assert_eq!(tree.next_global_id(), removed.id);
    }
}
