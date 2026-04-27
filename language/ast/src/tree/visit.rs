#![allow(unused_variables)]

use crate::{
    Argument, AssignPattern, AssignPatternField, Block, Declaration, Declarator, Decorator,
    DependencyItem, EnumField, Expression, GenericArgument, GenericParameter, LocalNodeId,
    MatchCase, Member, NodeType, Parameter, Pattern, PatternField, Property, Tree, TupleElement,
    TypeExpression, TypeMember, WhereClause, walk_argument, walk_assign_pattern,
    walk_assign_pattern_field, walk_block, walk_declaration, walk_declarator, walk_decorator,
    walk_dependency_item, walk_enum_field, walk_expression, walk_generic_argument,
    walk_generic_parameter, walk_match_case, walk_member, walk_parameter, walk_pattern,
    walk_pattern_field, walk_property, walk_tuple_element, walk_type_expression, walk_type_member,
    walk_where_clause,
};

#[derive(Debug, Clone, Default)]
pub struct NodeVisitorOptions {}

/// A NodeVisitor is a visitor for the AST.
pub trait NodeVisitor {
    /// Get the options for the visitor.
    fn options(&self) -> &NodeVisitorOptions;

    #[inline]
    fn visit_any(&mut self, tree: &Tree, ty: NodeType, id: u32) {
        // nothing to do
    }

    /// Visit an Expression.
    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        destack_core::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }

    /// Visit a TypeExpression.
    fn visit_type_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeExpression>,
        type_expression: &TypeExpression,
    ) {
        destack_core::ensure_sufficient_stack(|| {
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

    /// Visit a MatchCase.
    fn visit_match_case(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<MatchCase>,
        match_case: &MatchCase,
    ) {
        walk_match_case(self, tree, id, match_case);
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
}

/// A CapturingNodeVisitor is a visitor that collects the nodes visited (without walking further).
#[derive(Debug, Clone, Default)]
pub struct CapturingNodeVisitor {
    visited: Vec<u32>,
    options: NodeVisitorOptions,
}

impl CapturingNodeVisitor {
    pub fn new(options: NodeVisitorOptions) -> Self {
        Self {
            visited: Vec::new(),
            options,
        }
    }

    pub fn reset(&mut self) {
        self.visited.clear();
    }

    pub fn visited(&self) -> &[u32] {
        &self.visited
    }
}

impl NodeVisitor for CapturingNodeVisitor {
    #[inline]
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_any(&mut self, tree: &Tree, ty: NodeType, id: u32) {
        self.visited.push(id);
    }

    fn visit_block(&mut self, tree: &Tree, id: LocalNodeId<Block>, block: &Block) {
        self.visit_any(tree, NodeType::Block, id.id);
    }

    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
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
        declaration: &Declaration,
    ) {
        self.visit_any(tree, NodeType::Declaration, id.id);
    }

    fn visit_property(&mut self, tree: &Tree, id: LocalNodeId<Property>, property: &Property) {
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

    fn visit_member(&mut self, tree: &Tree, id: LocalNodeId<Member>, member: &Member) {
        self.visit_any(tree, NodeType::Member, id.id);
    }

    fn visit_enum_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<EnumField>,
        enum_field: &EnumField,
    ) {
        self.visit_any(tree, NodeType::EnumField, id.id);
    }

    fn visit_dependency_item(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<DependencyItem>,
        dependency_item: &DependencyItem,
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

    fn visit_parameter(&mut self, tree: &Tree, id: LocalNodeId<Parameter>, parameter: &Parameter) {
        self.visit_any(tree, NodeType::Parameter, id.id);
    }

    fn visit_argument(&mut self, tree: &Tree, id: LocalNodeId<Argument>, argument: &Argument) {
        self.visit_any(tree, NodeType::Argument, id.id);
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

    fn visit_pattern(&mut self, tree: &Tree, id: LocalNodeId<Pattern>, pattern: &Pattern) {
        self.visit_any(tree, NodeType::Pattern, id.id);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        self.visit_any(tree, NodeType::PatternField, id.id);
    }

    fn visit_assign_pattern(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPattern>,
        assign_pattern: &AssignPattern,
    ) {
        self.visit_any(tree, NodeType::AssignPattern, id.id);
    }

    fn visit_assign_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPatternField>,
        assign_pattern_field: &AssignPatternField,
    ) {
        self.visit_any(tree, NodeType::AssignPatternField, id.id);
    }

    fn visit_match_case(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<MatchCase>,
        match_case: &MatchCase,
    ) {
        self.visit_any(tree, NodeType::MatchCase, id.id);
    }

    fn visit_declarator(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declarator>,
        declarator: &Declarator,
    ) {
        self.visit_any(tree, NodeType::Declarator, id.id);
    }

    fn visit_decorator(&mut self, tree: &Tree, id: LocalNodeId<Decorator>, decorator: &Decorator) {
        self.visit_any(tree, NodeType::Decorator, id.id);
    }
}
