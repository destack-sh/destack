use serde::{Deserialize, Serialize};

use crate::{
    Argument, Block, Declaration, Declarator, Decorator, DependencyItem, EnumField, Expression,
    GenericArgument, GenericParameter, LocalNodeId, MatchCase, Member, Node, NodeType, NodeVisitor,
    NodeVisitorOptions, Parameter, Pattern, PatternField, Property, Tree, TreeStore, TupleElement,
    TypeExpression, TypeMember, WhereClause, walk_any,
};

/// The NodeParentIndex is a side index of parent nodes into the DIR tree.
/// (We maintain this separately since it's more convenient to build bottom up during parsing;
///  having bottom-up ids also makes it simpler to get the "innermost" or "outermost" node unambiguously.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeParentIndex {
    parent_id_by_node_id: Vec<u32>,
}

impl Default for NodeParentIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeParentIndex {
    /// Sentinel used for nodes without a parent.
    const NO_PARENT: u32 = u32::MAX;

    /// Create a new NodeParentIndex.
    pub fn new() -> Self {
        Self {
            parent_id_by_node_id: Vec::new(),
        }
    }

    /// Create a new NodeParentIndex from a Tree.
    pub fn from_tree(tree: &Tree) -> Self {
        // build dense parent lookup directly: no hash map and no captured child list
        let node_count = tree.node_index_by_node_id.len();
        let mut visitor = ParentIndexBuilderVisitor::new(node_count);
        for (parent_id, entry) in tree.node_index_by_node_id.iter().enumerate() {
            visitor.set_current_parent(parent_id as u32);
            walk_any(&mut visitor, tree, entry.node_type(), parent_id as u32);
        }

        Self {
            parent_id_by_node_id: visitor.take_parent_ids(),
        }
    }

    /// Create a new NodeParentIndex from reachable expression roots.
    pub fn from_expression_roots(tree: &Tree, roots: &[LocalNodeId<Expression>]) -> Self {
        let node_count = tree.node_index_by_node_id.len();
        let mut reachable = ReachableNodeVisitor::new(node_count);

        for root_id in roots {
            walk_any(&mut reachable, tree, NodeType::Expression, root_id.id);
        }

        let mut visitor = ParentIndexBuilderVisitor::new(node_count);
        for parent_id in reachable.take_node_ids() {
            let node_type = tree.get_node_type(parent_id);
            visitor.set_current_parent(parent_id);
            walk_any(&mut visitor, tree, node_type, parent_id);
        }

        Self {
            parent_id_by_node_id: visitor.take_parent_ids(),
        }
    }

    /// Get the parent for a node.
    #[inline]
    pub fn get<T>(&self, node_id: LocalNodeId<T>) -> Option<u32>
    where
        T: Node,
    {
        self.get_by_id(node_id.id)
    }

    /// Get the parent for a node by its id.
    #[inline]
    pub fn get_by_id(&self, node_id: u32) -> Option<u32> {
        let parent_id = self.parent_id_by_node_id[node_id as usize];
        (parent_id != Self::NO_PARENT).then_some(parent_id)
    }

    /// Walk all parents to the root.
    #[inline]
    pub fn walk_parents_by_id(&self, node_id: u32) -> Vec<u32> {
        let mut parents: Vec<u32> = Vec::new();
        let mut current_id = node_id;
        while let Some(parent_id) = self.get_by_id(current_id) {
            parents.push(parent_id);
            current_id = parent_id;
        }
        parents
    }

    /// Walk all parents to the root.
    #[inline]
    pub fn get_ancestors<T>(&self, node_id: LocalNodeId<T>) -> Vec<u32>
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.walk_parents_by_id(node_id.id)
    }

    /// Append a root node with no parent.
    pub fn append_root(&mut self) {
        self.parent_id_by_node_id.push(Self::NO_PARENT);
    }
}

/// Internal visitor that records direct child to parent mappings.
#[derive(Debug, Clone)]
struct ParentIndexBuilderVisitor {
    current_parent: u32,
    parent_id_by_node_id: Vec<u32>,
    options: NodeVisitorOptions,
}

/// Internal visitor that collects reachable node ids from one root set.
#[derive(Debug, Clone)]
struct ReachableNodeVisitor {
    seen: Vec<bool>,
    node_ids: Vec<u32>,
    options: NodeVisitorOptions,
}

impl ReachableNodeVisitor {
    /// Create one reachable-node collector with fixed node capacity.
    fn new(node_count: usize) -> Self {
        Self {
            seen: vec![false; node_count],
            node_ids: Vec::with_capacity(node_count),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Consume the visitor and return the reachable node ids.
    fn take_node_ids(self) -> Vec<u32> {
        self.node_ids
    }
}

impl NodeVisitor for ReachableNodeVisitor {
    #[inline]
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    #[inline]
    fn visit_any(&mut self, _tree: &Tree, _ty: NodeType, id: u32) {
        if self.seen[id as usize] {
            return;
        }

        self.seen[id as usize] = true;
        self.node_ids.push(id);
    }
}

impl ParentIndexBuilderVisitor {
    /// Create a parent index builder with fixed node capacity.
    fn new(node_count: usize) -> Self {
        Self {
            current_parent: 0,
            parent_id_by_node_id: vec![NodeParentIndex::NO_PARENT; node_count],
            options: NodeVisitorOptions::default(),
        }
    }

    /// Set the current parent node being walked.
    #[inline]
    fn set_current_parent(&mut self, parent_id: u32) {
        self.current_parent = parent_id;
    }

    /// Record a parent for a node when it is not the parent itself.
    #[inline]
    fn record_parent_for(&mut self, node_id: u32) {
        if node_id == self.current_parent {
            return;
        }

        self.parent_id_by_node_id[node_id as usize] = self.current_parent;
    }

    /// Consume the builder and return the dense parent table.
    fn take_parent_ids(self) -> Vec<u32> {
        self.parent_id_by_node_id
    }
}

impl NodeVisitor for ParentIndexBuilderVisitor {
    #[inline]
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    #[inline]
    fn visit_any(&mut self, _tree: &Tree, _ty: NodeType, id: u32) {
        self.record_parent_for(id);
    }

    #[inline]
    fn visit_block(&mut self, _tree: &Tree, id: LocalNodeId<Block>, _block: &Block) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_expression(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<Expression>,
        _expression: &Expression,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_declaration(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<Declaration>,
        _declaration: &Declaration,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_property(&mut self, _tree: &Tree, id: LocalNodeId<Property>, _property: &Property) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_type_member(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<TypeMember>,
        _type_member: &TypeMember,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_member(&mut self, _tree: &Tree, id: LocalNodeId<Member>, _member: &Member) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_enum_field(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<EnumField>,
        _enum_field: &EnumField,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_where_clause(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<WhereClause>,
        _where_clause: &WhereClause,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_dependency_item(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<DependencyItem>,
        _dependency_item: &DependencyItem,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_generic_parameter(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<GenericParameter>,
        _generic_parameter: &GenericParameter,
    ) {
        self.record_parent_for(id.id);
    }

    fn visit_parameter(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<Parameter>,
        _parameter: &Parameter,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_argument(&mut self, _tree: &Tree, id: LocalNodeId<Argument>, _argument: &Argument) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_generic_argument(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<GenericArgument>,
        _generic_argument: &GenericArgument,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_tuple_element(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<TupleElement>,
        _tuple_element: &TupleElement,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_match_case(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<MatchCase>,
        _match_case: &MatchCase,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_declarator(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<Declarator>,
        _declarator: &Declarator,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_pattern(&mut self, _tree: &Tree, id: LocalNodeId<Pattern>, _pattern: &Pattern) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_pattern_field(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<PatternField>,
        _pattern_field: &PatternField,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_decorator(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<Decorator>,
        _decorator: &Decorator,
    ) {
        self.record_parent_for(id.id);
    }

    #[inline]
    fn visit_type_expression(
        &mut self,
        _tree: &Tree,
        id: LocalNodeId<TypeExpression>,
        _type_expression: &TypeExpression,
    ) {
        self.record_parent_for(id.id);
    }
}
