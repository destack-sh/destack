use destack_core::StringId;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Debug, Formatter};

use destack_source::{ModuleId, Span};
use serde::{Deserialize, Serialize};

use crate::{
    Arena, Argument, AssignPattern, AssignPatternField, Block, Declaration, Declarator, Decorator,
    DependencyItem, EnumField, Expression, FunctionRole, GenericArgument, GenericParameter,
    IfCondition, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, MatchCase, Member,
    Node, NodeType, NodeVisitor, NodeVisitorOptions, Parameter, Pattern, PatternField, Property,
    ProvenanceId, ProvenanceMetadata, ProvenanceReason, TupleElement, TypeExpression, TypeMember,
    WhereClause, walk_argument, walk_block, walk_declaration, walk_declarator, walk_decorator,
    walk_dependency_item, walk_enum_field, walk_expression, walk_generic_argument,
    walk_generic_parameter, walk_match_case, walk_member, walk_parameter, walk_pattern,
    walk_pattern_field, walk_property, walk_tuple_element, walk_type_expression, walk_type_member,
    walk_where_clause,
};

/// Normalized semantic documentation attached to one DIR node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Documentation {
    /// The normalized documentation text.
    pub text: StringId,
}

/// Dense metadata for one global DIR node id.
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
            local_id < Self::LOCAL_ID_MASK,
            "DIR node local id exceeds packed index capacity: {local_id}"
        );

        Self {
            packed: local_id | ((node_type as u32) << Self::NODE_TYPE_SHIFT),
        }
    }

    /// Create one placeholder entry for a reserved node slot.
    #[inline]
    pub(crate) fn placeholder(node_type: NodeType) -> Self {
        Self {
            packed: Self::LOCAL_ID_MASK | ((node_type as u32) << Self::NODE_TYPE_SHIFT),
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
            19 => NodeType::AssignPattern,
            20 => NodeType::AssignPatternField,
            21 => NodeType::Decorator,
            _ => unreachable!("invalid DIR node type tag in packed node index"),
        }
    }
}

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
    pub(crate) generic_arguments: Arena<GenericArgument>,
    pub(crate) tuple_elements: Arena<TupleElement>,
    pub(crate) arguments: Arena<Argument>,
    pub(crate) match_cases: Arena<MatchCase>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) pattern_fields: Arena<PatternField>,
    pub(crate) assign_patterns: Arena<AssignPattern>,
    pub(crate) assign_pattern_fields: Arena<AssignPatternField>,
    pub(crate) decorators: Arena<Decorator>,

    // node side data
    /// The parent node id by node id. Index is the global node id.
    /// (Unlike in AST, we can index parents here directly since we have the shape up front.)
    parent_id_by_node_id: Vec<Option<u32>>,
    /// The scopes by node id. Index is the global node id.
    /// (Main data is in BindingTable, but indexed here for efficiency since *every* node needs a scope.)
    scopes_by_node_id: Vec<(LocalScopeId, LocalScopeMark)>,
    /// Provenance metadata for all nodes.
    provenance: ProvenanceMetadata,
    /// The alias node id by AST node id.
    alias_node_id_by_source_id: BTreeMap<u32, u32>,
    /// The alias node id by DIR node id.
    alias_node_id_by_node_id: BTreeMap<u32, u32>,
    /// The decorators attached to nodes.
    decorators_by_node_id: BTreeMap<u32, Vec<LocalNodeId<Decorator>>>,
    /// The normalized documentation attached to nodes.
    documentation_by_node_id: BTreeMap<u32, Documentation>,
    /// The final source span by node id when known.
    source_span_by_node_id: Vec<Option<Span>>,
    /// The node ids explicitly marked inactive.
    inactive_node_ids: BTreeSet<u32>,
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
        Self {
            module_id,

            first_global_id: 0,
            next_global_id: 0,
            node_index_by_node_id: Vec::with_capacity(capacity),

            expressions: Arena::new(),
            type_expressions: Arena::new(),
            blocks: Arena::new(),
            declarations: Arena::new(),
            declarators: Arena::new(),
            properties: Arena::new(),
            type_members: Arena::new(),
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
            decorators: Arena::new(),

            parent_id_by_node_id: Vec::with_capacity(capacity),
            scopes_by_node_id: Vec::with_capacity(capacity),
            provenance: ProvenanceMetadata {
                provenance_by_node_id: Vec::with_capacity(capacity),
                ..ProvenanceMetadata::default()
            },
            alias_node_id_by_source_id: BTreeMap::new(),
            alias_node_id_by_node_id: BTreeMap::new(),
            decorators_by_node_id: BTreeMap::new(),
            documentation_by_node_id: BTreeMap::new(),
            source_span_by_node_id: Vec::with_capacity(capacity),
            inactive_node_ids: BTreeSet::new(),
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

    /// Return the local arena id for one global node id.
    #[inline]
    pub(crate) fn local_id_for_node_id(&self, node_id: u32) -> u32 {
        self.node_index_by_node_id[self.node_index(node_id)].local_id()
    }
    /// Mark a node id as inactive.
    pub fn mark_inactive(&mut self, node_id: LocalNodeIdAny) {
        self.inactive_node_ids.insert(node_id.id);
    }

    /// Check whether a node id is inactive.
    pub fn is_inactive(&self, node_id: u32) -> bool {
        self.inactive_node_ids.contains(&node_id)
    }

    /// Iterate inactive node ids.
    #[inline]
    pub fn inactive_node_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.inactive_node_ids.iter().copied()
    }

    /// Reserve a new node slot in the tree for a node lowered from an AST node.
    pub fn reserve_from_source(
        &mut self,
        node_type: NodeType,
        ast_node_id: u32,
        scope: (LocalScopeId, LocalScopeMark),
        parent_id: Option<LocalNodeIdAny>,
    ) -> LocalNodeIdAny {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;

        self.node_index_by_node_id
            .push(NodeIndexEntry::placeholder(node_type));
        self.scopes_by_node_id.push(scope);
        self.parent_id_by_node_id
            .push(parent_id.map(|parent_id| parent_id.id));
        let provenance_id = self.provenance.create_source(ast_node_id);
        self.provenance.provenance_by_node_id.push(provenance_id);
        self.source_span_by_node_id.push(None);
        self.alias_node_id_by_source_id
            .insert(ast_node_id, global_id);

        LocalNodeIdAny::new(global_id, node_type)
    }

    /// Reserve a new node slot in the tree for a node derived from another DIR node.
    pub fn reserve_from(
        &mut self,
        node_type: NodeType,
        dir_node_id: LocalNodeIdAny,
        scope: (LocalScopeId, LocalScopeMark),
        parent_id: Option<LocalNodeIdAny>,
        reason: Option<ProvenanceReason>,
    ) -> LocalNodeIdAny {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;

        self.node_index_by_node_id
            .push(NodeIndexEntry::placeholder(node_type));
        self.scopes_by_node_id.push(scope);
        self.parent_id_by_node_id
            .push(parent_id.map(|parent_id| parent_id.id));
        let parent_index = self.node_index(dir_node_id.id);
        let parent_provenance = self.provenance.provenance_by_node_id[parent_index];
        let source_id = self.provenance.source_id(parent_provenance);
        let provenance_id = self
            .provenance
            .create_derived(source_id, parent_provenance, reason);
        self.provenance.provenance_by_node_id.push(provenance_id);
        self.source_span_by_node_id
            .push(self.source_span_by_node_id[parent_index]);
        self.alias_node_id_by_node_id
            .insert(dir_node_id.id, global_id);

        LocalNodeIdAny::new(global_id, node_type)
    }

    /// Fill in the node data for a previously reserved slot.
    pub fn insert<T>(&mut self, node_id: LocalNodeIdAny, node: T) -> LocalNodeId<T>
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
        let node_id = self.insert(node_id, node);
        self.adopt_direct_children(LocalNodeIdAny::new(node_id.id, T::TYPE));

        node_id
    }

    /// Add an alias node for a lowered AST id.
    pub fn alias_from_source<T>(&mut self, ast_id: u32, alias: LocalNodeId<T>)
    where
        T: Node,
        Self: TreeStore<T>,
    {
        self.alias_node_id_by_source_id.insert(ast_id, alias.id);
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

    /// Replace a node in-place, preserving the original at a new ID:
    /// - The original node is preserved at a new ID (for diagnostics/codegen)
    /// - The node at `id` is replaced with `replacement`
    /// - An alias is set up from `id` to the preserved original
    ///
    /// Returns the ID of the preserved original node (which is new! - since the original is replaced).
    pub fn replace<T>(&mut self, id: LocalNodeId<T>, replacement: T) -> LocalNodeId<T>
    where
        T: Node + Clone,
        Self: TreeStore<T>,
    {
        let scope = self.get_scope(id);
        let original = self.get(id).clone();

        // preserve original at new ID
        let preserved_id = self.reserve_from(T::TYPE, id.into_any(), scope, None, None);
        let preserved_id: LocalNodeId<T> = self.insert(preserved_id, original);

        // preserved originals exist for alias lookup, not active tree traversal
        self.mark_inactive(preserved_id.into_any());

        // replace in-place
        *self.get_mut(id) = replacement;

        // keep reused child ids attached to the replacement
        self.adopt_direct_children(id.into_any());

        // alias for reverse lookup (id -> preserved)
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

        // the source root is no longer structurally active after its payload moves
        self.mark_inactive(source_id.into_any());

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
        self.parent_id_by_node_id[self.node_index(node_id)]
    }

    /// Get the parent node id for a node id.
    #[inline]
    pub fn get_parent(&self, node_id: u32) -> Option<LocalNodeIdAny> {
        let parent_id = self.parent_id_by_node_id[self.node_index(node_id)];
        parent_id.map(|parent_id| {
            LocalNodeIdAny::new(
                parent_id,
                self.node_index_by_node_id[self.node_index(parent_id)].node_type(),
            )
        })
    }

    /// Walk one node by its erased local id.
    fn walk_node<V: NodeVisitor + ?Sized>(&self, visitor: &mut V, node_id: LocalNodeIdAny) {
        match node_id.ty {
            NodeType::Expression => {
                let typed_id = LocalNodeId::<Expression>::new(node_id.id);
                visitor.visit_expression(self, typed_id, self.get(typed_id));
            }
            NodeType::Block => {
                let typed_id = LocalNodeId::<Block>::new(node_id.id);
                visitor.visit_block(self, typed_id, self.get(typed_id));
            }
            NodeType::Declaration => {
                let typed_id = LocalNodeId::<Declaration>::new(node_id.id);
                visitor.visit_declaration(self, typed_id, self.get(typed_id));
            }
            NodeType::Declarator => {
                let typed_id = LocalNodeId::<Declarator>::new(node_id.id);
                visitor.visit_declarator(self, typed_id, self.get(typed_id));
            }
            NodeType::Property => {
                let typed_id = LocalNodeId::<Property>::new(node_id.id);
                visitor.visit_property(self, typed_id, self.get(typed_id));
            }
            NodeType::TypeMember => {
                let typed_id = LocalNodeId::<TypeMember>::new(node_id.id);
                visitor.visit_type_member(self, typed_id, self.get(typed_id));
            }
            NodeType::Member => {
                let typed_id = LocalNodeId::<Member>::new(node_id.id);
                visitor.visit_member(self, typed_id, self.get(typed_id));
            }
            NodeType::EnumField => {
                let typed_id = LocalNodeId::<EnumField>::new(node_id.id);
                visitor.visit_enum_field(self, typed_id, self.get(typed_id));
            }
            NodeType::WhereClause => {
                let typed_id = LocalNodeId::<WhereClause>::new(node_id.id);
                visitor.visit_where_clause(self, typed_id, self.get(typed_id));
            }
            NodeType::DependencyItem => {
                let typed_id = LocalNodeId::<DependencyItem>::new(node_id.id);
                visitor.visit_dependency_item(self, typed_id, self.get(typed_id));
            }
            NodeType::GenericParameter => {
                let typed_id = LocalNodeId::<GenericParameter>::new(node_id.id);
                visitor.visit_generic_parameter(self, typed_id, self.get(typed_id));
            }
            NodeType::Parameter => {
                let typed_id = LocalNodeId::<Parameter>::new(node_id.id);
                visitor.visit_parameter(self, typed_id, self.get(typed_id));
            }
            NodeType::GenericArgument => {
                let typed_id = LocalNodeId::<GenericArgument>::new(node_id.id);
                visitor.visit_generic_argument(self, typed_id, self.get(typed_id));
            }
            NodeType::TupleElement => {
                let typed_id = LocalNodeId::<TupleElement>::new(node_id.id);
                visitor.visit_tuple_element(self, typed_id, self.get(typed_id));
            }
            NodeType::Argument => {
                let typed_id = LocalNodeId::<Argument>::new(node_id.id);
                visitor.visit_argument(self, typed_id, self.get(typed_id));
            }
            NodeType::TypeExpression => {
                let typed_id = LocalNodeId::<TypeExpression>::new(node_id.id);
                visitor.visit_type_expression(self, typed_id, self.get(typed_id));
            }
            NodeType::MatchCase => {
                let typed_id = LocalNodeId::<MatchCase>::new(node_id.id);
                visitor.visit_match_case(self, typed_id, self.get(typed_id));
            }
            NodeType::Pattern => {
                let typed_id = LocalNodeId::<Pattern>::new(node_id.id);
                visitor.visit_pattern(self, typed_id, self.get(typed_id));
            }
            NodeType::PatternField => {
                let typed_id = LocalNodeId::<PatternField>::new(node_id.id);
                visitor.visit_pattern_field(self, typed_id, self.get(typed_id));
            }
            NodeType::AssignPattern => {
                let typed_id = LocalNodeId::<AssignPattern>::new(node_id.id);
                visitor.visit_assign_pattern(self, typed_id, self.get(typed_id));
            }
            NodeType::AssignPatternField => {
                let typed_id = LocalNodeId::<AssignPatternField>::new(node_id.id);
                visitor.visit_assign_pattern_field(self, typed_id, self.get(typed_id));
            }
            NodeType::Decorator => {
                let typed_id = LocalNodeId::<Decorator>::new(node_id.id);
                visitor.visit_decorator(self, typed_id, self.get(typed_id));
            }
        }
    }

    /// Make one node the owner of its reused direct children.
    fn adopt_direct_children(&mut self, root_id: LocalNodeIdAny) {
        struct DirectChildCollector {
            options: NodeVisitorOptions,
            parent_stack: Vec<LocalNodeIdAny>,
            children: Vec<LocalNodeIdAny>,
        }

        impl NodeVisitor for DirectChildCollector {
            fn options(&self) -> &NodeVisitorOptions {
                &self.options
            }

            fn visit_expression(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<Expression>,
                expression: &Expression,
            ) {
                self.push_node(id.into_any());
                walk_expression(self, tree, id, expression);
                self.parent_stack.pop();
            }

            fn visit_block(&mut self, tree: &Tree, id: LocalNodeId<Block>, block: &Block) {
                self.push_node(id.into_any());
                walk_block(self, tree, id, block);
                self.parent_stack.pop();
            }

            fn visit_declaration(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<Declaration>,
                declaration: &Declaration,
            ) {
                self.push_node(id.into_any());
                walk_declaration(self, tree, id, declaration);
                self.parent_stack.pop();
            }

            fn visit_declarator(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<Declarator>,
                declarator: &Declarator,
            ) {
                self.push_node(id.into_any());
                walk_declarator(self, tree, id, declarator);
                self.parent_stack.pop();
            }

            fn visit_property(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<Property>,
                property: &Property,
            ) {
                self.push_node(id.into_any());
                walk_property(self, tree, id, property);
                self.parent_stack.pop();
            }

            fn visit_type_member(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<TypeMember>,
                type_member: &TypeMember,
            ) {
                self.push_node(id.into_any());
                walk_type_member(self, tree, id, type_member);
                self.parent_stack.pop();
            }

            fn visit_member(&mut self, tree: &Tree, id: LocalNodeId<Member>, member: &Member) {
                self.push_node(id.into_any());
                walk_member(self, tree, id, member);
                self.parent_stack.pop();
            }

            fn visit_enum_field(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<EnumField>,
                enum_field: &EnumField,
            ) {
                self.push_node(id.into_any());
                walk_enum_field(self, tree, id, enum_field);
                self.parent_stack.pop();
            }

            fn visit_where_clause(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<WhereClause>,
                where_clause: &WhereClause,
            ) {
                self.push_node(id.into_any());
                walk_where_clause(self, tree, id, where_clause);
                self.parent_stack.pop();
            }

            fn visit_dependency_item(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<DependencyItem>,
                dependency_item: &DependencyItem,
            ) {
                self.push_node(id.into_any());
                walk_dependency_item(self, tree, id, dependency_item);
                self.parent_stack.pop();
            }

            fn visit_generic_parameter(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<GenericParameter>,
                generic_parameter: &GenericParameter,
            ) {
                self.push_node(id.into_any());
                walk_generic_parameter(self, tree, id, generic_parameter);
                self.parent_stack.pop();
            }

            fn visit_parameter(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<Parameter>,
                parameter: &Parameter,
            ) {
                self.push_node(id.into_any());
                walk_parameter(self, tree, id, parameter);
                self.parent_stack.pop();
            }

            fn visit_argument(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<Argument>,
                argument: &Argument,
            ) {
                self.push_node(id.into_any());
                walk_argument(self, tree, id, argument);
                self.parent_stack.pop();
            }

            fn visit_generic_argument(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<GenericArgument>,
                generic_argument: &GenericArgument,
            ) {
                self.push_node(id.into_any());
                walk_generic_argument(self, tree, id, generic_argument);
                self.parent_stack.pop();
            }

            fn visit_tuple_element(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<TupleElement>,
                tuple_element: &TupleElement,
            ) {
                self.push_node(id.into_any());
                walk_tuple_element(self, tree, id, tuple_element);
                self.parent_stack.pop();
            }

            fn visit_type_expression(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<TypeExpression>,
                type_expression: &TypeExpression,
            ) {
                self.push_node(id.into_any());
                walk_type_expression(self, tree, id, type_expression);
                self.parent_stack.pop();
            }

            fn visit_match_case(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<MatchCase>,
                match_case: &MatchCase,
            ) {
                self.push_node(id.into_any());
                walk_match_case(self, tree, id, match_case);
                self.parent_stack.pop();
            }

            fn visit_pattern(&mut self, tree: &Tree, id: LocalNodeId<Pattern>, pattern: &Pattern) {
                self.push_node(id.into_any());
                walk_pattern(self, tree, id, pattern);
                self.parent_stack.pop();
            }

            fn visit_pattern_field(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<PatternField>,
                pattern_field: &PatternField,
            ) {
                self.push_node(id.into_any());
                walk_pattern_field(self, tree, id, pattern_field);
                self.parent_stack.pop();
            }

            fn visit_decorator(
                &mut self,
                tree: &Tree,
                id: LocalNodeId<Decorator>,
                decorator: &Decorator,
            ) {
                self.push_node(id.into_any());
                walk_decorator(self, tree, id, decorator);
                self.parent_stack.pop();
            }
        }

        impl DirectChildCollector {
            fn push_node(&mut self, node_id: LocalNodeIdAny) {
                if self.parent_stack.len() == 1 {
                    self.children.push(node_id);
                }

                self.parent_stack.push(node_id);
            }
        }

        let children = {
            let mut collector = DirectChildCollector {
                options: NodeVisitorOptions::default(),
                parent_stack: Vec::new(),
                children: Vec::new(),
            };

            self.walk_node(&mut collector, root_id);
            collector.children
        };

        for child_id in children {
            self.set_parent(child_id, Some(root_id));
        }
    }

    /// Set the parent node id for one node.
    #[inline]
    fn set_parent(&mut self, node_id: LocalNodeIdAny, parent_id: Option<LocalNodeIdAny>) {
        let index = self.node_index(node_id.id);
        if let Some(parent_slot) = self.parent_id_by_node_id.get_mut(index) {
            *parent_slot = parent_id.map(|parent_id| parent_id.id);
        }
    }

    /// Assert that walked child ownership matches cached parent links.
    #[cfg(debug_assertions)]
    pub fn debug_assert_valid_parents(&self) {
        fn node_summary(tree: &Tree, node_id: LocalNodeIdAny) -> String {
            match node_id.ty {
                NodeType::Expression => {
                    let node_id = LocalNodeId::<Expression>::new(node_id.id);
                    format!("{:?}", tree.get(node_id))
                }
                NodeType::Block => {
                    let node_id = LocalNodeId::<Block>::new(node_id.id);
                    format!("{:?}", tree.get(node_id))
                }
                _ => format!("{:?}", node_id.ty),
            }
        }

        struct DebugParentValidator {
            options: NodeVisitorOptions,
            parent_stack: Vec<LocalNodeIdAny>,
            root_id: Option<LocalNodeIdAny>,
        }

        impl DebugParentValidator {
            fn push_node(&mut self, tree: &Tree, node_id: LocalNodeIdAny) {
                if let Some(expected_parent) = self.parent_stack.last().copied() {
                    let actual_parent = tree.get_parent(node_id.id);
                    assert_eq!(
                        actual_parent,
                        Some(expected_parent),
                        "DIR parent mismatch: node {node_id:?} {}, expected parent {expected_parent:?} {}, actual parent {actual_parent:?} {}, root {:?}",
                        node_summary(tree, node_id),
                        node_summary(tree, expected_parent),
                        actual_parent
                            .map(|parent_id| node_summary(tree, parent_id))
                            .unwrap_or_else(|| "None".to_string()),
                        self.root_id,
                    );
                } else {
                    self.root_id = Some(node_id);
                }

                self.parent_stack.push(node_id);
            }

            fn pop_node(&mut self) {
                self.parent_stack.pop();

                if self.parent_stack.is_empty() {
                    self.root_id = None;
                }
            }
        }

        macro_rules! validate_visit {
            ($method:ident, $ty:ty, $node_type:expr, $walk:path) => {
                fn $method(&mut self, tree: &Tree, id: LocalNodeId<$ty>, node: &$ty) {
                    self.push_node(tree, id.into_any());
                    $walk(self, tree, id, node);
                    self.pop_node();
                }
            };
        }

        impl NodeVisitor for DebugParentValidator {
            fn options(&self) -> &NodeVisitorOptions {
                &self.options
            }

            validate_visit!(
                visit_expression,
                Expression,
                NodeType::Expression,
                walk_expression
            );
            validate_visit!(visit_block, Block, NodeType::Block, walk_block);
            validate_visit!(
                visit_declaration,
                Declaration,
                NodeType::Declaration,
                walk_declaration
            );
            validate_visit!(
                visit_declarator,
                Declarator,
                NodeType::Declarator,
                walk_declarator
            );
            validate_visit!(visit_property, Property, NodeType::Property, walk_property);
            validate_visit!(visit_member, Member, NodeType::Member, walk_member);
            validate_visit!(
                visit_enum_field,
                EnumField,
                NodeType::EnumField,
                walk_enum_field
            );
            validate_visit!(
                visit_where_clause,
                WhereClause,
                NodeType::WhereClause,
                walk_where_clause
            );
            validate_visit!(
                visit_dependency_item,
                DependencyItem,
                NodeType::DependencyItem,
                walk_dependency_item
            );
            validate_visit!(
                visit_generic_parameter,
                GenericParameter,
                NodeType::GenericParameter,
                walk_generic_parameter
            );
            validate_visit!(
                visit_parameter,
                Parameter,
                NodeType::Parameter,
                walk_parameter
            );
            validate_visit!(
                visit_generic_argument,
                GenericArgument,
                NodeType::GenericArgument,
                walk_generic_argument
            );
            validate_visit!(
                visit_tuple_element,
                TupleElement,
                NodeType::TupleElement,
                walk_tuple_element
            );
            validate_visit!(visit_argument, Argument, NodeType::Argument, walk_argument);
            validate_visit!(
                visit_type_expression,
                TypeExpression,
                NodeType::TypeExpression,
                walk_type_expression
            );
            validate_visit!(
                visit_match_case,
                MatchCase,
                NodeType::MatchCase,
                walk_match_case
            );
            validate_visit!(visit_pattern, Pattern, NodeType::Pattern, walk_pattern);
            validate_visit!(
                visit_pattern_field,
                PatternField,
                NodeType::PatternField,
                walk_pattern_field
            );
            validate_visit!(
                visit_decorator,
                Decorator,
                NodeType::Decorator,
                walk_decorator
            );
        }

        let preserved_alias_targets: BTreeSet<u32> =
            self.alias_node_id_by_node_id.values().copied().collect();

        let mut validator = DebugParentValidator {
            options: NodeVisitorOptions::default(),
            parent_stack: Vec::new(),
            root_id: None,
        };

        for node_id in self.iter_node_ids() {
            if self.is_inactive(node_id.id) || preserved_alias_targets.contains(&node_id.id) {
                continue;
            }

            if self.get_parent_id(node_id.id).is_some() {
                continue;
            }

            if !matches!(node_id.ty, NodeType::Declaration | NodeType::DependencyItem) {
                continue;
            }

            self.walk_node(&mut validator, node_id);
        }
    }

    /// Return whether one expression is in statement position.
    pub fn expression_is_in_statement_position(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let Some(parent_id) = self.get_parent(expression_id.id) else {
            return true;
        };

        match parent_id.ty {
            NodeType::Expression => self
                .expression_is_in_statement_position_inside_parent_expression(
                    LocalNodeId::new(parent_id.id),
                    expression_id,
                ),
            NodeType::Block => self.expression_is_in_statement_position_inside_parent_block(
                LocalNodeId::new(parent_id.id),
                expression_id,
            ),
            NodeType::Declaration => self
                .expression_is_in_statement_position_inside_parent_declaration(
                    LocalNodeId::new(parent_id.id),
                    expression_id,
                ),
            NodeType::Member => self.expression_is_in_statement_position_inside_parent_member(
                LocalNodeId::new(parent_id.id),
                expression_id,
            ),
            NodeType::Property => self.expression_is_in_statement_position_inside_parent_property(
                LocalNodeId::new(parent_id.id),
                expression_id,
            ),
            _ => false,
        }
    }

    /// Return whether one expression inherits statement position through a parent expression.
    fn expression_is_in_statement_position_inside_parent_expression(
        &self,
        parent_expression_id: LocalNodeId<Expression>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let parent_expression = self.get(parent_expression_id);

        let should_inherit_parent_position = match parent_expression {
            // `if let` and `if comptime` branches preserve block values
            Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                let branch_inherits_statement_position =
                    self.if_condition_inherits_statement_position(condition);

                branch_inherits_statement_position
                    && (then_expression.id == expression_id.id
                        || else_expression
                            .as_ref()
                            .is_some_and(|else_expression| else_expression.id == expression_id.id))
            }

            // these bodies inherit statement position from the parent shell
            Expression::Try {
                try_expression,
                catch_expression,
                finally_expression,
                ..
            } => {
                try_expression.id == expression_id.id
                    || catch_expression
                        .as_ref()
                        .is_some_and(|catch_expression| catch_expression.id == expression_id.id)
                    || finally_expression
                        .as_ref()
                        .is_some_and(|finally_expression| finally_expression.id == expression_id.id)
            }
            Expression::Labelled { body, .. } => body.id == expression_id.id,

            // everything else is operand position
            _ => false,
        };

        should_inherit_parent_position
            && self.expression_is_in_statement_position(parent_expression_id)
    }

    /// Return whether one expression is statement-position inside one parent block.
    fn expression_is_in_statement_position_inside_parent_block(
        &self,
        parent_block_id: LocalNodeId<Block>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let parent_block = self.get(parent_block_id);

        parent_block.leading_expressions.contains(&expression_id)
    }

    /// Return whether one expression is statement-position inside one parent declaration.
    fn expression_is_in_statement_position_inside_parent_declaration(
        &self,
        parent_declaration_id: LocalNodeId<Declaration>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let parent_declaration = self.get(parent_declaration_id);

        match parent_declaration {
            // module style declaration bodies host statement sequences
            Declaration::Function(declaration) => {
                declaration.body.as_ref().is_some_and(|body_expression_id| {
                    body_expression_id.id == expression_id.id
                        && self.function_body_is_statement_position(declaration.signature.role)
                })
            }
            Declaration::Global(declaration) => declaration.expressions.contains(&expression_id),
            Declaration::Module(declaration) => declaration.expressions.contains(&expression_id),
            Declaration::Namespace(declaration) => declaration.expressions.contains(&expression_id),

            // everything else treats child expressions as operands
            _ => false,
        }
    }

    /// Return whether one expression is statement-position inside one parent member.
    fn expression_is_in_statement_position_inside_parent_member(
        &self,
        parent_member_id: LocalNodeId<Member>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let parent_member = self.get(parent_member_id);

        match parent_member {
            Member::Method { body, .. } => body
                .as_ref()
                .is_some_and(|body_expression_id| body_expression_id.id == expression_id.id),
            Member::StaticBlock { body, .. } | Member::ComptimeBlock { body, .. } => {
                body.id == expression_id.id
            }
            _ => false,
        }
    }

    /// Return whether one expression is statement-position inside one parent property.
    fn expression_is_in_statement_position_inside_parent_property(
        &self,
        parent_property_id: LocalNodeId<Property>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let parent_property = self.get(parent_property_id);

        match parent_property {
            Property::Method { body, .. } => body
                .as_ref()
                .is_some_and(|body_expression_id| body_expression_id.id == expression_id.id),
            _ => false,
        }
    }

    /// Return whether one `if` branch should inherit statement position.
    fn if_condition_inherits_statement_position(&self, condition: &IfCondition) -> bool {
        match condition {
            IfCondition::Let { .. } => false,
            IfCondition::Expression { condition } => {
                !matches!(self.get(*condition), Expression::Comptime { .. })
            }
        }
    }

    /// Return whether one function body should behave as statement-position.
    fn function_body_is_statement_position(&self, role: Option<FunctionRole>) -> bool {
        if matches!(role, Some(FunctionRole::Constructor | FunctionRole::Setter)) {
            return true;
        }

        true
    }

    /// Get the AST id of a node by its DIR node id.
    #[inline]
    pub fn get_source(&self, node_id: u32) -> u32 {
        let provenance_id = self.provenance.provenance_by_node_id[self.node_index(node_id)];
        self.provenance.source_id(provenance_id)
    }

    /// Set the final source span for one DIR node.
    #[inline]
    pub fn set_span(&mut self, node_id: u32, span: Span) {
        let index = self.node_index(node_id);
        self.source_span_by_node_id[index] = Some(span);
    }

    /// Return the final source span for one DIR node when known.
    #[inline]
    pub fn get_span_by_id(&self, node_id: u32) -> Option<Span> {
        let index = self.node_index(node_id);
        self.source_span_by_node_id[index]
    }

    /// Get the provenance id of one node by its DIR node id.
    #[inline]
    pub fn get_provenance(&self, node_id: u32) -> ProvenanceId {
        self.provenance.provenance_by_node_id[self.node_index(node_id)]
    }

    /// Return true when the node id exists in this tree.
    #[inline]
    pub fn has_node_id(&self, node_id: u32) -> bool {
        node_id >= self.first_global_id
            && ((node_id - self.first_global_id) as usize) < self.node_index_by_node_id.len()
    }

    /// Get the DIR node id by its AST id.
    #[inline]
    pub fn get_node_id_by_source_id(&self, ast_id: u32) -> Option<LocalNodeIdAny> {
        self.alias_node_id_by_source_id
            .get(&ast_id)
            .copied()
            .map(|node_id| {
                LocalNodeIdAny::new(
                    node_id,
                    self.node_index_by_node_id[self.node_index(node_id)].node_type(),
                )
            })
    }

    /// Get the scope for a node id.
    #[inline]
    pub fn get_scope<T: Node>(&self, node_id: LocalNodeId<T>) -> (LocalScopeId, LocalScopeMark) {
        self.scopes_by_node_id[self.node_index(node_id.id)]
    }

    /// Get the scope for an erased node id.
    #[inline]
    pub fn get_scope_any(&self, node_id: LocalNodeIdAny) -> (LocalScopeId, LocalScopeMark) {
        self.scopes_by_node_id[self.node_index(node_id.id)]
    }

    /// Append a decorator to a node by its global id.
    #[inline]
    pub fn append_decorator(
        &mut self,
        target_id: LocalNodeIdAny,
        decorator: LocalNodeId<Decorator>,
    ) {
        // track decorators for the target node
        self.decorators_by_node_id
            .entry(target_id.id)
            .or_default()
            .push(decorator);

        // attach the decorator to its target for parent lookups
        if decorator.id != target_id.id {
            let index = self.node_index(decorator.id);
            if let Some(parent_slot) = self.parent_id_by_node_id.get_mut(index) {
                *parent_slot = Some(target_id.id);
            }
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

/// Map node types to arenas.
pub trait TreeStore<T: Node> {
    /// Allocate a node into the relevant arena.
    fn allocate(tree: &mut Tree, node: T) -> u32;
    /// Get a node from the relevant arena.
    fn get(tree: &Tree, idx: u32) -> &T;
    /// Get a mutable node from the relevant arena.
    fn get_mut(tree: &mut Tree, idx: u32) -> &mut T;
}

macro_rules! impl_tree_store {
    ($ty:ty, $field:ident) => {
        impl TreeStore<$ty> for Tree {
            #[inline]
            fn allocate(tree: &mut Tree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            #[inline]
            fn get(tree: &Tree, idx: u32) -> &$ty {
                tree.$field.get(idx)
            }

            #[inline]
            fn get_mut(tree: &mut Tree, idx: u32) -> &mut $ty {
                tree.$field.get_mut(idx)
            }
        }
    };
}

macro_rules! impl_tree_stores {
    ( $( $ty:ty => $field:ident ),+ $(,)? ) => {
        $( impl_tree_store!($ty, $field); )*
    };
}

// usage
impl_tree_stores! {
    Expression => expressions,
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
    GenericArgument => generic_arguments,
    TupleElement => tuple_elements,
    Argument => arguments,
    TypeExpression => type_expressions,
    MatchCase => match_cases,
    Pattern => patterns,
    PatternField => pattern_fields,
    AssignPattern => assign_patterns,
    AssignPatternField => assign_pattern_fields,
    Decorator => decorators,
}
