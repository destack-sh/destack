use destack_core::StringId;
use destack_source::AdaptImage;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Arena, Argument, Block, Declaration, Declarator, Decorator, DependencyItem, EnumField,
    Expression, FunctionMode, GenericArgument, GenericParameter, IfCondition, LocalNodeId,
    LocalNodeIdAny, LocalScopeId, LocalScopeMark, MatchCase, Member, Node, NodeType, NodeVisitor,
<<<<<<< HEAD
    NodeVisitorOptions, Parameter, Pattern, PatternField, Property, Provenance, ProvenanceId,
    ProvenanceReason, TypeExpression, TypeProperty, WhereClause,
||||||| parent of 1ffb34b5c5 (feat(language/dir): mirror tuple elements and align shared AST shapes)
    NodeVisitorOptions, Parameter, Pattern, PatternField, Property, TypeExpression, TypeProperty,
    WhereClause,
=======
    NodeVisitorOptions, Parameter, Pattern, PatternField, Property, TupleElement, TypeExpression,
    TypeProperty, WhereClause,
>>>>>>> 1ffb34b5c5 (feat(language/dir): mirror tuple elements and align shared AST shapes)
};

/// Normalized semantic documentation attached to one DIR node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub struct Documentation {
    /// The normalized documentation text.
    pub text: StringId,
}

/// Mutable DIR Node tree across a set of related source units. NOT THREAD-SAFE.
#[derive(Clone, Serialize, Deserialize, AdaptImage)]
pub struct NodeTree {
    /// The module id of the node tree.
    pub module_id: ModuleId,

    // node index
    /// The next id to allocate.
    pub(crate) next_global_id: u32,
    /// The local ids of all nodes. Index is the global node id.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// The types of all nodes. Index is the global node id.
    pub(crate) node_type_by_node_id: Vec<NodeType>,

    // node arenas
    pub(crate) expressions: Arena<Expression>,
    pub(crate) type_expressions: Arena<TypeExpression>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) declarations: Arena<Declaration>,
    pub(crate) declarators: Arena<Declarator>,
    pub(crate) properties: Arena<Property>,
    pub(crate) type_properties: Arena<TypeProperty>,
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
    pub(crate) decorators: Arena<Decorator>,

    // node side data
    /// The parent node id by node id. Index is the global node id.
    /// (Unlike in AST, we can index parents here directly since we have the shape up front.)
    parent_id_by_node_id: Vec<Option<u32>>,
    /// The scopes by node id. Index is the global node id.
    /// (Main data is in SymbolTable, but indexed here for efficiency since *every* node needs a scope.)
    scopes_by_node_id: Vec<(LocalScopeId, LocalScopeMark)>,
    /// Provenance metadata for all nodes.
    provenance: Provenance,
    /// The alias node id by AST node id.
    alias_node_id_by_source_id: HashMap<u32, u32>,
    /// The alias node id by DIR node id.
    alias_node_id_by_node_id: HashMap<u32, u32>,
    /// The decorators attached to nodes.
    decorators_by_node_id: HashMap<u32, Vec<LocalNodeId<Decorator>>>,
    /// The normalized documentation attached to nodes.
    documentation_by_node_id: HashMap<u32, Documentation>,
    /// The node ids explicitly marked inactive.
    inactive_node_ids: HashSet<u32>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("len", &self.local_id_by_node_id.len())
            .finish()
    }
}

impl NodeTree {
    /// Create a new NodeTree.
    pub fn new(module_id: ModuleId) -> Self {
        Self::with_capacity(module_id, 0)
    }

    /// Create a new NodeTree with the given capacity.
    pub fn with_capacity(module_id: ModuleId, capacity: usize) -> Self {
        Self {
            module_id,

            next_global_id: 0,
            local_id_by_node_id: Vec::with_capacity(capacity),
            node_type_by_node_id: Vec::with_capacity(capacity),

            expressions: Arena::new(),
            type_expressions: Arena::new(),
            blocks: Arena::new(),
            declarations: Arena::new(),
            declarators: Arena::new(),
            properties: Arena::new(),
            type_properties: Arena::new(),
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
            decorators: Arena::new(),

            parent_id_by_node_id: Vec::with_capacity(capacity),
            scopes_by_node_id: Vec::with_capacity(capacity),
            provenance: Provenance {
                provenance_by_node_id: Vec::with_capacity(capacity),
                ..Provenance::default()
            },
            alias_node_id_by_source_id: HashMap::new(),
            alias_node_id_by_node_id: HashMap::new(),
            decorators_by_node_id: HashMap::new(),
            documentation_by_node_id: HashMap::new(),
            inactive_node_ids: HashSet::new(),
        }
    }
    /// Mark a node id as inactive.
    pub fn mark_inactive(&mut self, node_id: LocalNodeIdAny) {
        self.inactive_node_ids.insert(node_id.id);
    }

    /// Check whether a node id is inactive.
    pub fn is_inactive(&self, node_id: u32) -> bool {
        self.inactive_node_ids.contains(&node_id)
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

        self.node_type_by_node_id.push(node_type);
        self.local_id_by_node_id.push(u32::MAX); // placeholder, filled by insert
        self.scopes_by_node_id.push(scope);
        self.parent_id_by_node_id
            .push(parent_id.map(|parent_id| parent_id.id));
        let provenance_id = self.provenance.create_source(ast_node_id);
        self.provenance.provenance_by_node_id.push(provenance_id);
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

        self.node_type_by_node_id.push(node_type);
        self.local_id_by_node_id.push(u32::MAX); // placeholder, filled by insert
        self.scopes_by_node_id.push(scope);
        self.parent_id_by_node_id
            .push(parent_id.map(|parent_id| parent_id.id));
        let parent_provenance = self.provenance.provenance_by_node_id[dir_node_id.id as usize];
        let source_id = self.provenance.source_id(parent_provenance);
        let provenance_id = self
            .provenance
            .create_derived(source_id, parent_provenance, reason);
        self.provenance.provenance_by_node_id.push(provenance_id);
        self.alias_node_id_by_node_id
            .insert(dir_node_id.id, global_id);

        LocalNodeIdAny::new(global_id, node_type)
    }

    /// Fill in the node data for a previously reserved slot.
    pub fn insert<T>(&mut self, node_id: LocalNodeIdAny, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id[node_id.id as usize] = local_id;
        LocalNodeId::new(node_id.id)
    }

    /// Fill in one reserved slot and make the inserted node own its reused direct children.
    pub fn insert_as_owner<T>(&mut self, node_id: LocalNodeIdAny, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let node_id = self.insert(node_id, node);
        self.adopt_direct_children(LocalNodeIdAny::new(node_id.id, T::TYPE));

        node_id
    }

    /// Add an alias node for a lowered AST id.
    pub fn alias_from_source<T>(&mut self, ast_id: u32, alias: LocalNodeId<T>)
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.alias_node_id_by_source_id.insert(ast_id, alias.id);
    }

    /// Add an alias node for a derived DIR id.
    pub fn alias_from<T>(&mut self, dir_id: u32, alias: LocalNodeId<T>)
    where
        T: Node,
        Self: NodeTreeImpl<T>,
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
                LocalNodeIdAny::new(alias_id, self.node_type_by_node_id[alias_id as usize])
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
        LocalNodeIdAny::new(current, self.node_type_by_node_id[current as usize])
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

    /// Replace a node in-place, preserving the original at a new ID:
    /// - The original node is preserved at a new ID (for diagnostics/codegen)
    /// - The node at `id` is replaced with `replacement`
    /// - An alias is set up from `id` to the preserved original
    ///
    /// Returns the ID of the preserved original node (which is new! - since the original is replaced).
    pub fn replace<T>(&mut self, id: LocalNodeId<T>, replacement: T) -> LocalNodeId<T>
    where
        T: Node + Clone,
        Self: NodeTreeImpl<T>,
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
        Self: NodeTreeImpl<T>,
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
        Self: NodeTreeImpl<T>,
    {
        self.local_id_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_index, &local_index)| {
                if self.node_type_by_node_id[global_index] == T::TYPE {
                    let node_id = LocalNodeId::new(global_index as u32);
                    let node = <Self as NodeTreeImpl<T>>::get(self, local_index);
                    Some((node_id, node))
                } else {
                    None
                }
            })
    }

    /// Iterate over all nodes ids.
    pub fn iter_node_ids(&self) -> impl Iterator<Item = LocalNodeIdAny> + '_ {
        self.node_type_by_node_id
            .iter()
            .enumerate()
            .map(|(global_index, &type_id)| LocalNodeIdAny::new(global_index as u32, type_id))
    }

    /// Iterate over all nodes ids of a given type.
    pub fn iter_node_ids_of_type<T>(&self) -> Vec<LocalNodeId<T>>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.local_id_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_index, _)| {
                if self.node_type_by_node_id[global_index] == T::TYPE {
                    let node_id = LocalNodeId::new(global_index as u32);
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
        self.parent_id_by_node_id[node_id as usize]
    }

    /// Get the parent node id for a node id.
    #[inline]
    pub fn get_parent(&self, node_id: u32) -> Option<LocalNodeIdAny> {
        let parent_id = self.parent_id_by_node_id[node_id as usize];
        parent_id.map(|parent_id| {
            LocalNodeIdAny::new(parent_id, self.node_type_by_node_id[parent_id as usize])
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
            NodeType::TypeProperty => {
                let typed_id = LocalNodeId::<TypeProperty>::new(node_id.id);
                visitor.visit_type_property(self, typed_id, self.get(typed_id));
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
                tree: &NodeTree,
                id: LocalNodeId<Expression>,
                expression: &Expression,
            ) {
                self.push_node(id.into_any());
                crate::walk_expression(self, tree, id, expression);
                self.parent_stack.pop();
            }

            fn visit_block(&mut self, tree: &NodeTree, id: LocalNodeId<Block>, block: &Block) {
                self.push_node(id.into_any());
                crate::walk_block(self, tree, id, block);
                self.parent_stack.pop();
            }

            fn visit_declaration(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<Declaration>,
                declaration: &Declaration,
            ) {
                self.push_node(id.into_any());
                crate::walk_declaration(self, tree, id, declaration);
                self.parent_stack.pop();
            }

            fn visit_declarator(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<Declarator>,
                declarator: &Declarator,
            ) {
                self.push_node(id.into_any());
                crate::walk_declarator(self, tree, id, declarator);
                self.parent_stack.pop();
            }

            fn visit_property(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<Property>,
                property: &Property,
            ) {
                self.push_node(id.into_any());
                crate::walk_property(self, tree, id, property);
                self.parent_stack.pop();
            }

            fn visit_type_property(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<TypeProperty>,
                type_property: &TypeProperty,
            ) {
                self.push_node(id.into_any());
                crate::walk_type_property(self, tree, id, type_property);
                self.parent_stack.pop();
            }

            fn visit_member(&mut self, tree: &NodeTree, id: LocalNodeId<Member>, member: &Member) {
                self.push_node(id.into_any());
                crate::walk_member(self, tree, id, member);
                self.parent_stack.pop();
            }

            fn visit_enum_field(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<EnumField>,
                enum_field: &EnumField,
            ) {
                self.push_node(id.into_any());
                crate::walk_enum_field(self, tree, id, enum_field);
                self.parent_stack.pop();
            }

            fn visit_where_clause(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<WhereClause>,
                where_clause: &WhereClause,
            ) {
                self.push_node(id.into_any());
                crate::walk_where_clause(self, tree, id, where_clause);
                self.parent_stack.pop();
            }

            fn visit_dependency_item(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<DependencyItem>,
                dependency_item: &DependencyItem,
            ) {
                self.push_node(id.into_any());
                crate::walk_dependency_item(self, tree, id, dependency_item);
                self.parent_stack.pop();
            }

            fn visit_generic_parameter(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<GenericParameter>,
                generic_parameter: &GenericParameter,
            ) {
                self.push_node(id.into_any());
                crate::walk_generic_parameter(self, tree, id, generic_parameter);
                self.parent_stack.pop();
            }

            fn visit_parameter(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<Parameter>,
                parameter: &Parameter,
            ) {
                self.push_node(id.into_any());
                crate::walk_parameter(self, tree, id, parameter);
                self.parent_stack.pop();
            }

            fn visit_argument(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<Argument>,
                argument: &Argument,
            ) {
                self.push_node(id.into_any());
                crate::walk_argument(self, tree, id, argument);
                self.parent_stack.pop();
            }

            fn visit_generic_argument(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<GenericArgument>,
                generic_argument: &GenericArgument,
            ) {
                self.push_node(id.into_any());
                crate::walk_generic_argument(self, tree, id, generic_argument);
                self.parent_stack.pop();
            }

            fn visit_tuple_element(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<TupleElement>,
                tuple_element: &TupleElement,
            ) {
                self.push_node(id.into_any());
                crate::walk_tuple_element(self, tree, id, tuple_element);
                self.parent_stack.pop();
            }

            fn visit_type_expression(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<TypeExpression>,
                type_expression: &TypeExpression,
            ) {
                self.push_node(id.into_any());
                crate::walk_type_expression(self, tree, id, type_expression);
                self.parent_stack.pop();
            }

            fn visit_match_case(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<MatchCase>,
                match_case: &MatchCase,
            ) {
                self.push_node(id.into_any());
                crate::walk_match_case(self, tree, id, match_case);
                self.parent_stack.pop();
            }

            fn visit_pattern(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<Pattern>,
                pattern: &Pattern,
            ) {
                self.push_node(id.into_any());
                crate::walk_pattern(self, tree, id, pattern);
                self.parent_stack.pop();
            }

            fn visit_pattern_field(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<PatternField>,
                pattern_field: &PatternField,
            ) {
                self.push_node(id.into_any());
                crate::walk_pattern_field(self, tree, id, pattern_field);
                self.parent_stack.pop();
            }

            fn visit_decorator(
                &mut self,
                tree: &NodeTree,
                id: LocalNodeId<Decorator>,
                decorator: &Decorator,
            ) {
                self.push_node(id.into_any());
                crate::walk_decorator(self, tree, id, decorator);
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
        if let Some(parent_slot) = self.parent_id_by_node_id.get_mut(node_id.id as usize) {
            *parent_slot = parent_id.map(|parent_id| parent_id.id);
        }
    }

    /// Assert that walked child ownership matches cached parent links.
    #[cfg(debug_assertions)]
    pub fn debug_assert_valid_parents(&self) {
        fn node_summary(tree: &NodeTree, node_id: LocalNodeIdAny) -> String {
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
            fn push_node(&mut self, tree: &NodeTree, node_id: LocalNodeIdAny) {
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
                fn $method(&mut self, tree: &NodeTree, id: LocalNodeId<$ty>, node: &$ty) {
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
                crate::walk_expression
            );
            validate_visit!(visit_block, Block, NodeType::Block, crate::walk_block);
            validate_visit!(
                visit_declaration,
                Declaration,
                NodeType::Declaration,
                crate::walk_declaration
            );
            validate_visit!(
                visit_declarator,
                Declarator,
                NodeType::Declarator,
                crate::walk_declarator
            );
            validate_visit!(
                visit_property,
                Property,
                NodeType::Property,
                crate::walk_property
            );
            validate_visit!(visit_member, Member, NodeType::Member, crate::walk_member);
            validate_visit!(
                visit_enum_field,
                EnumField,
                NodeType::EnumField,
                crate::walk_enum_field
            );
            validate_visit!(
                visit_where_clause,
                WhereClause,
                NodeType::WhereClause,
                crate::walk_where_clause
            );
            validate_visit!(
                visit_dependency_item,
                DependencyItem,
                NodeType::DependencyItem,
                crate::walk_dependency_item
            );
            validate_visit!(
                visit_generic_parameter,
                GenericParameter,
                NodeType::GenericParameter,
                crate::walk_generic_parameter
            );
            validate_visit!(
                visit_parameter,
                Parameter,
                NodeType::Parameter,
                crate::walk_parameter
            );
            validate_visit!(
                visit_generic_argument,
                GenericArgument,
                NodeType::GenericArgument,
                crate::walk_generic_argument
            );
            validate_visit!(
                visit_tuple_element,
                TupleElement,
                NodeType::TupleElement,
                crate::walk_tuple_element
            );
            validate_visit!(
                visit_argument,
                Argument,
                NodeType::Argument,
                crate::walk_argument
            );
            validate_visit!(
                visit_type_expression,
                TypeExpression,
                NodeType::TypeExpression,
                crate::walk_type_expression
            );
            validate_visit!(
                visit_match_case,
                MatchCase,
                NodeType::MatchCase,
                crate::walk_match_case
            );
            validate_visit!(
                visit_pattern,
                Pattern,
                NodeType::Pattern,
                crate::walk_pattern
            );
            validate_visit!(
                visit_pattern_field,
                PatternField,
                NodeType::PatternField,
                crate::walk_pattern_field
            );
            validate_visit!(
                visit_decorator,
                Decorator,
                NodeType::Decorator,
                crate::walk_decorator
            );
        }

        let preserved_alias_targets: HashSet<u32> =
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
                        && self.function_body_is_statement_position(declaration.signature.mode)
                })
            }
            Declaration::Global(declaration) => declaration.expressions.contains(&expression_id),
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
    fn function_body_is_statement_position(&self, mode: Option<FunctionMode>) -> bool {
        if matches!(mode, Some(FunctionMode::Constructor | FunctionMode::Setter)) {
            return true;
        }

        true
    }

    /// Get the AST id of a node by its DIR node id.
    #[inline]
    pub fn get_source(&self, node_id: u32) -> u32 {
        let provenance_id = self.provenance.provenance_by_node_id[node_id as usize];
        self.provenance.source_id(provenance_id)
    }

    /// Get the provenance id of one node by its DIR node id.
    #[inline]
    pub fn get_provenance(&self, node_id: u32) -> ProvenanceId {
        self.provenance.provenance_by_node_id[node_id as usize]
    }

    /// Return true when the node id exists in this tree.
    #[inline]
    pub fn has_node_id(&self, node_id: u32) -> bool {
        (node_id as usize) < self.provenance.provenance_by_node_id.len()
    }

    /// Get the DIR node id by its AST id.
    #[inline]
    pub fn get_node_id_by_source_id(&self, ast_id: u32) -> Option<LocalNodeIdAny> {
        self.alias_node_id_by_source_id
            .get(&ast_id)
            .copied()
            .map(|node_id| {
                LocalNodeIdAny::new(node_id, self.node_type_by_node_id[node_id as usize])
            })
    }

    /// Get the scope for a node id.
    #[inline]
    pub fn get_scope<T: Node>(&self, node_id: LocalNodeId<T>) -> (LocalScopeId, LocalScopeMark) {
        self.scopes_by_node_id[node_id.id as usize]
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
        if decorator.id != target_id.id
            && let Some(parent_slot) = self.parent_id_by_node_id.get_mut(decorator.id as usize)
        {
            *parent_slot = Some(target_id.id);
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

// usage
impl_node_tree_stores! {
    Expression => expressions,
    Block => blocks,
    Declaration => declarations,
    Declarator => declarators,
    Property => properties,
    TypeProperty => type_properties,
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
    Decorator => decorators,
}
