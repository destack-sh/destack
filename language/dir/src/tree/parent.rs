use crate::{
    Argument, AssignPattern, AssignPatternField, Block, Catch, Declaration, Declarator, Decorator,
    DependencyItem, EnumField, Expression, GenericArgument, GenericParameter, LocalNodeId,
    LocalNodeIdAny, MatchCase, Member, NodeVisitor, NodeVisitorOptions, Parameter, Pattern,
    PatternField, Property, Tree, TupleElement, TypeExpression, TypeMappedParameter, TypeMember,
    WhereClause, walk_argument, walk_assign_pattern, walk_assign_pattern_field, walk_block,
    walk_catch, walk_declaration, walk_declarator, walk_decorator, walk_dependency_item,
    walk_enum_field, walk_expression, walk_generic_argument, walk_generic_parameter,
    walk_match_case, walk_member, walk_parameter, walk_pattern, walk_pattern_field, walk_property,
    walk_root, walk_tuple_element, walk_type_expression, walk_type_mapped_parameter,
    walk_type_member, walk_where_clause,
};

/// Reparent reused direct children onto one root.
pub(crate) fn reparent_direct_children(tree: &mut Tree, root_id: LocalNodeIdAny) {
    let children = collect_direct_children(tree, root_id);

    for child_id in children {
        tree.set_parent_id(child_id.id, Some(root_id.id));
    }
}

/// Collect the direct structural children of one root.
fn collect_direct_children(tree: &Tree, root_id: LocalNodeIdAny) -> Vec<LocalNodeIdAny> {
    let mut collector = DirectChildCollector {
        options: NodeVisitorOptions::default(),
        parent_stack: Vec::new(),
        children: Vec::new(),
    };

    walk_root(&mut collector, tree, &root_id);

    collector.children
}

/// Visitor that records immediate children while still walking descendants.
struct DirectChildCollector {
    /// The visitor options.
    options: NodeVisitorOptions,
    /// The structural parent stack.
    parent_stack: Vec<LocalNodeIdAny>,
    /// The collected direct children.
    children: Vec<LocalNodeIdAny>,
}

impl DirectChildCollector {
    /// Push one visited node and record it when it is directly below the root.
    fn push_node(&mut self, node_id: LocalNodeIdAny) {
        if self.parent_stack.len() == 1 {
            self.children.push(node_id);
        }

        self.parent_stack.push(node_id);
    }
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

    fn visit_block(&mut self, tree: &Tree, id: LocalNodeId<Block>, block: &Block) {
        self.push_node(id.into_any());
        walk_block(self, tree, id, block);
        self.parent_stack.pop();
    }

    fn visit_catch(&mut self, tree: &Tree, id: LocalNodeId<Catch>, catch: &Catch) {
        self.push_node(id.into_any());
        walk_catch(self, tree, id, catch);
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

    fn visit_property(&mut self, tree: &Tree, id: LocalNodeId<Property>, property: &Property) {
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

    fn visit_type_mapped_parameter(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeMappedParameter>,
        parameter: &TypeMappedParameter,
    ) {
        self.push_node(id.into_any());
        walk_type_mapped_parameter(self, tree, id, parameter);
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

    fn visit_parameter(&mut self, tree: &Tree, id: LocalNodeId<Parameter>, parameter: &Parameter) {
        self.push_node(id.into_any());
        walk_parameter(self, tree, id, parameter);
        self.parent_stack.pop();
    }

    fn visit_argument(&mut self, tree: &Tree, id: LocalNodeId<Argument>, argument: &Argument) {
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

    fn visit_assign_pattern(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPattern>,
        assign_pattern: &AssignPattern,
    ) {
        self.push_node(id.into_any());
        walk_assign_pattern(self, tree, id, assign_pattern);
        self.parent_stack.pop();
    }

    fn visit_assign_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPatternField>,
        assign_pattern_field: &AssignPatternField,
    ) {
        self.push_node(id.into_any());
        walk_assign_pattern_field(self, tree, id, assign_pattern_field);
        self.parent_stack.pop();
    }

    fn visit_decorator(&mut self, tree: &Tree, id: LocalNodeId<Decorator>, decorator: &Decorator) {
        self.push_node(id.into_any());
        walk_decorator(self, tree, id, decorator);
        self.parent_stack.pop();
    }
}
