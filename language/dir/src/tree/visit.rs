use smallvec::SmallVec;

use crate::{
    Argument, AssignPattern, AssignPatternField, Block, Catch, Declaration, Declarator, Decorator,
    DependencyItem, EnumField, Expression, GenericArgument, GenericParameter, LocalNodeId,
    LocalNodeIdAny, MatchArm, Member, NodeType, Parameter, Pattern, PatternField, Property,
    SwitchCase, Tree, TreeAttribute, TreeChild, TupleElement, TypeExpression, TypeMappedParameter,
    TypeMember, WhereClause, walk_any, walk_argument, walk_assign_pattern,
    walk_assign_pattern_field, walk_block, walk_catch, walk_declaration, walk_declarator,
    walk_decorator, walk_dependency_item, walk_enum_field, walk_expression, walk_generic_argument,
    walk_generic_parameter, walk_match_arm, walk_member, walk_parameter, walk_pattern,
    walk_pattern_field, walk_property, walk_switch_case, walk_tree_attribute, walk_tree_child,
    walk_tuple_element, walk_type_expression, walk_type_mapped_parameter, walk_type_member,
    walk_where_clause,
};

/// A NodeVisitor is a visitor for the parsed DIR.
pub trait NodeVisitor {
    #[inline]
    fn visit_any(&mut self, _tree: &Tree, _ty: NodeType, _id: u32) {
        // nothing to do
    }

    /// Visit an Expression.
    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        tspp_core::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }

    /// Visit a TypeExpression.
    fn visit_type_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeExpression>,
        type_expression: &TypeExpression,
    ) {
        tspp_core::ensure_sufficient_stack(|| {
            walk_type_expression(self, tree, id, type_expression)
        });
    }

    /// Visit a Block.
    fn visit_block(&mut self, tree: &Tree, id: LocalNodeId<Block>, block: &Block) {
        walk_block(self, tree, id, block);
    }

    /// Visit a Declaration.
    fn visit_declaration(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        walk_declaration(self, tree, id, declaration);
    }

    /// Visit a Property.
    fn visit_property(&mut self, tree: &Tree, id: LocalNodeId<Property>, property: &Property) {
        walk_property(self, tree, id, property);
    }

    /// Visit a TypeMember.
    fn visit_type_member(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeMember>,
        type_member: &TypeMember,
    ) {
        walk_type_member(self, tree, id, type_member);
    }

    /// Visit a Member.
    fn visit_member(&mut self, tree: &Tree, id: LocalNodeId<Member>, member: &Member) {
        walk_member(self, tree, id, member);
    }

    /// Visit an EnumField.
    fn visit_enum_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<EnumField>,
        enum_field: &EnumField,
    ) {
        walk_enum_field(self, tree, id, enum_field);
    }

    /// Visit a WhereClause.
    fn visit_where_clause(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<WhereClause>,
        where_clause: &WhereClause,
    ) {
        walk_where_clause(self, tree, id, where_clause);
    }

    /// Visit a DependencyItem.
    fn visit_dependency_item(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<DependencyItem>,
        dependency_item: &DependencyItem,
    ) {
        walk_dependency_item(self, tree, id, dependency_item);
    }

    /// Visit a GenericParameter.
    fn visit_generic_parameter(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<GenericParameter>,
        generic_parameter: &GenericParameter,
    ) {
        walk_generic_parameter(self, tree, id, generic_parameter);
    }

    /// Visit a Parameter.
    fn visit_parameter(&mut self, tree: &Tree, id: LocalNodeId<Parameter>, parameter: &Parameter) {
        walk_parameter(self, tree, id, parameter);
    }

    /// Visit an Argument.
    fn visit_argument(&mut self, tree: &Tree, id: LocalNodeId<Argument>, argument: &Argument) {
        walk_argument(self, tree, id, argument);
    }

    /// Visit a tree attribute.
    fn visit_tree_attribute(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TreeAttribute>,
        attribute: &TreeAttribute,
    ) {
        walk_tree_attribute(self, tree, id, attribute);
    }

    /// Visit a tree child.
    fn visit_tree_child(&mut self, tree: &Tree, id: LocalNodeId<TreeChild>, child: &TreeChild) {
        walk_tree_child(self, tree, id, child);
    }

    /// Visit a GenericArgument.
    fn visit_generic_argument(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<GenericArgument>,
        generic_argument: &GenericArgument,
    ) {
        walk_generic_argument(self, tree, id, generic_argument);
    }

    /// Visit a TupleElement.
    fn visit_tuple_element(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TupleElement>,
        tuple_element: &TupleElement,
    ) {
        walk_tuple_element(self, tree, id, tuple_element);
    }

    /// Visit a match arm.
    fn visit_match_arm(&mut self, tree: &Tree, id: LocalNodeId<MatchArm>, arm: &MatchArm) {
        walk_match_arm(self, tree, id, arm);
    }

    /// Visit a switch case.
    fn visit_switch_case(&mut self, tree: &Tree, id: LocalNodeId<SwitchCase>, case: &SwitchCase) {
        walk_switch_case(self, tree, id, case);
    }

    /// Visit a Declarator.
    fn visit_declarator(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declarator>,
        declarator: &Declarator,
    ) {
        walk_declarator(self, tree, id, declarator);
    }

    /// Visit a Pattern.
    fn visit_pattern(&mut self, tree: &Tree, id: LocalNodeId<Pattern>, pattern: &Pattern) {
        walk_pattern(self, tree, id, pattern);
    }

    /// Visit a PatternField.
    fn visit_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        walk_pattern_field(self, tree, id, pattern_field);
    }

    /// Visit an AssignPattern.
    fn visit_assign_pattern(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPattern>,
        assign_pattern: &AssignPattern,
    ) {
        walk_assign_pattern(self, tree, id, assign_pattern);
    }

    /// Visit an AssignPatternField.
    fn visit_assign_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPatternField>,
        assign_pattern_field: &AssignPatternField,
    ) {
        walk_assign_pattern_field(self, tree, id, assign_pattern_field);
    }

    /// Visit a Decorator.
    fn visit_decorator(&mut self, tree: &Tree, id: LocalNodeId<Decorator>, decorator: &Decorator) {
        walk_decorator(self, tree, id, decorator);
    }

    /// Visit a Catch.
    fn visit_catch(&mut self, tree: &Tree, id: LocalNodeId<Catch>, catch: &Catch) {
        walk_catch(self, tree, id, catch);
    }

    /// Visit a TypeMappedParameter.
    fn visit_type_mapped_parameter(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeMappedParameter>,
        parameter: &TypeMappedParameter,
    ) {
        walk_type_mapped_parameter(self, tree, id, parameter);
    }
}

/// A collector for direct structural child node ids.
#[derive(Default)]
pub(crate) struct DirectChildCollector {
    /// The node ids collected during the current walk.
    node_ids: SmallVec<[u32; 8]>,
}

impl DirectChildCollector {
    /// Collect one node's direct structural children.
    pub(crate) fn collect(&mut self, tree: &Tree, parent: LocalNodeIdAny) -> &[u32] {
        self.node_ids.clear();
        walk_any(self, tree, parent.ty, parent.id);

        // require the parent to be the first visited node
        assert_eq!(
            self.node_ids.first(),
            Some(&parent.id),
            "DIR walk did not begin at parent node {}",
            parent.id
        );
        let children = &self.node_ids[1..];

        // reject structural self-cycles
        assert!(
            !children.contains(&parent.id),
            "DIR node {} is its own structural child",
            parent.id
        );

        children
    }
}

impl NodeVisitor for DirectChildCollector {
    #[inline]
    fn visit_any(&mut self, _tree: &Tree, _ty: NodeType, id: u32) {
        self.node_ids.push(id);
    }

    fn visit_block(&mut self, tree: &Tree, id: LocalNodeId<Block>, _block: &Block) {
        self.visit_any(tree, NodeType::Block, id.id);
    }

    fn visit_catch(&mut self, tree: &Tree, id: LocalNodeId<Catch>, _catch: &Catch) {
        self.visit_any(tree, NodeType::Catch, id.id);
    }

    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        _expression: &Expression,
    ) {
        self.visit_any(tree, NodeType::Expression, id.id);
    }

    fn visit_type_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeExpression>,
        _type_expression: &TypeExpression,
    ) {
        self.visit_any(tree, NodeType::TypeExpression, id.id);
    }

    fn visit_declaration(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declaration>,
        _declaration: &Declaration,
    ) {
        self.visit_any(tree, NodeType::Declaration, id.id);
    }

    fn visit_property(&mut self, tree: &Tree, id: LocalNodeId<Property>, _property: &Property) {
        self.visit_any(tree, NodeType::Property, id.id);
    }

    fn visit_type_member(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeMember>,
        _type_member: &TypeMember,
    ) {
        self.visit_any(tree, NodeType::TypeMember, id.id);
    }

    fn visit_type_mapped_parameter(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeMappedParameter>,
        _parameter: &TypeMappedParameter,
    ) {
        self.visit_any(tree, NodeType::TypeMappedParameter, id.id);
    }

    fn visit_member(&mut self, tree: &Tree, id: LocalNodeId<Member>, _member: &Member) {
        self.visit_any(tree, NodeType::Member, id.id);
    }

    fn visit_enum_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<EnumField>,
        _enum_field: &EnumField,
    ) {
        self.visit_any(tree, NodeType::EnumField, id.id);
    }

    fn visit_where_clause(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<WhereClause>,
        _where_clause: &WhereClause,
    ) {
        self.visit_any(tree, NodeType::WhereClause, id.id);
    }

    fn visit_dependency_item(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<DependencyItem>,
        _dependency_item: &DependencyItem,
    ) {
        self.visit_any(tree, NodeType::DependencyItem, id.id);
    }

    fn visit_generic_parameter(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<GenericParameter>,
        _generic_parameter: &GenericParameter,
    ) {
        self.visit_any(tree, NodeType::GenericParameter, id.id);
    }

    fn visit_parameter(&mut self, tree: &Tree, id: LocalNodeId<Parameter>, _parameter: &Parameter) {
        self.visit_any(tree, NodeType::Parameter, id.id);
    }

    fn visit_argument(&mut self, tree: &Tree, id: LocalNodeId<Argument>, _argument: &Argument) {
        self.visit_any(tree, NodeType::Argument, id.id);
    }

    fn visit_tree_attribute(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TreeAttribute>,
        _attribute: &TreeAttribute,
    ) {
        self.visit_any(tree, NodeType::TreeAttribute, id.id);
    }

    fn visit_tree_child(&mut self, tree: &Tree, id: LocalNodeId<TreeChild>, _child: &TreeChild) {
        self.visit_any(tree, NodeType::TreeChild, id.id);
    }

    fn visit_generic_argument(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<GenericArgument>,
        _generic_argument: &GenericArgument,
    ) {
        self.visit_any(tree, NodeType::GenericArgument, id.id);
    }

    fn visit_tuple_element(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TupleElement>,
        _tuple_element: &TupleElement,
    ) {
        self.visit_any(tree, NodeType::TupleElement, id.id);
    }

    fn visit_pattern(&mut self, tree: &Tree, id: LocalNodeId<Pattern>, _pattern: &Pattern) {
        self.visit_any(tree, NodeType::Pattern, id.id);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PatternField>,
        _pattern_field: &PatternField,
    ) {
        self.visit_any(tree, NodeType::PatternField, id.id);
    }

    fn visit_assign_pattern(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPattern>,
        _assign_pattern: &AssignPattern,
    ) {
        self.visit_any(tree, NodeType::AssignPattern, id.id);
    }

    fn visit_assign_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPatternField>,
        _assign_pattern_field: &AssignPatternField,
    ) {
        self.visit_any(tree, NodeType::AssignPatternField, id.id);
    }

    fn visit_match_arm(&mut self, tree: &Tree, id: LocalNodeId<MatchArm>, _arm: &MatchArm) {
        self.visit_any(tree, NodeType::MatchArm, id.id);
    }

    fn visit_switch_case(&mut self, tree: &Tree, id: LocalNodeId<SwitchCase>, _case: &SwitchCase) {
        self.visit_any(tree, NodeType::SwitchCase, id.id);
    }

    fn visit_declarator(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declarator>,
        _declarator: &Declarator,
    ) {
        self.visit_any(tree, NodeType::Declarator, id.id);
    }

    fn visit_decorator(&mut self, tree: &Tree, id: LocalNodeId<Decorator>, _decorator: &Decorator) {
        self.visit_any(tree, NodeType::Decorator, id.id);
    }
}
