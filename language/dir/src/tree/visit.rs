#![allow(unused_variables)]

use crate::{
    Argument, Block, Declaration, Declarator, Decorator, DependencyItem, EnumField, Expression,
    GenericArgument, GenericParameter, LocalNodeId, MatchCase, Member, NodeTree, NodeType,
    Parameter, Pattern, PatternField, Property, TupleElement, TypeExpression, TypeProperty,
    WhereClause, walk_argument, walk_block, walk_declaration, walk_declarator, walk_decorator,
    walk_dependency_item, walk_enum_field, walk_expression, walk_generic_argument,
    walk_generic_parameter, walk_match_case, walk_member, walk_parameter, walk_pattern,
    walk_pattern_field, walk_property, walk_tuple_element, walk_type_expression,
    walk_type_property, walk_where_clause,
};

#[derive(Debug, Clone, Default)]
pub struct NodeVisitorOptions {}

/// A NodeVisitor is a visitor for the DIR.
pub trait NodeVisitor {
    /// Get the options for the visitor.
    fn options(&self) -> &NodeVisitorOptions;

    #[inline]
    fn visit_any(&mut self, tree: &NodeTree, ty: NodeType, id: u32) {
        // nothing to do
    }

    /// Visit an Expression.
    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        destack_core::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }

    /// Visit a TypeExpression.
    fn visit_type_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<TypeExpression>,
        type_expression: &TypeExpression,
    ) {
        destack_core::ensure_sufficient_stack(|| {
            walk_type_expression(self, tree, id, type_expression)
        });
    }

    /// Visit a Block.
    fn visit_block(&mut self, tree: &NodeTree, id: LocalNodeId<Block>, block: &Block) {
        walk_block(self, tree, id, block);
    }

    /// Visit a Declaration.
    fn visit_declaration(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        walk_declaration(self, tree, id, declaration);
    }

    /// Visit a Declarator.
    fn visit_declarator(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Declarator>,
        declarator: &Declarator,
    ) {
        walk_declarator(self, tree, id, declarator);
    }

    /// Visit a Property.
    fn visit_property(&mut self, tree: &NodeTree, id: LocalNodeId<Property>, property: &Property) {
        walk_property(self, tree, id, property);
    }

    /// Visit a TypeProperty.
    fn visit_type_property(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<TypeProperty>,
        type_property: &TypeProperty,
    ) {
        walk_type_property(self, tree, id, type_property);
    }

    /// Visit a Member.
    fn visit_member(&mut self, tree: &NodeTree, id: LocalNodeId<Member>, member: &Member) {
        walk_member(self, tree, id, member);
    }

    /// Visit an EnumField.
    fn visit_enum_field(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<EnumField>,
        enum_field: &EnumField,
    ) {
        walk_enum_field(self, tree, id, enum_field);
    }

    /// Visit a WhereClause.
    fn visit_where_clause(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<WhereClause>,
        where_clause: &WhereClause,
    ) {
        walk_where_clause(self, tree, id, where_clause);
    }

    /// Visit a DependencyItem.
    fn visit_dependency_item(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<DependencyItem>,
        dependency_item: &DependencyItem,
    ) {
        walk_dependency_item(self, tree, id, dependency_item);
    }

    /// Visit a GenericParameter.
    fn visit_generic_parameter(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<GenericParameter>,
        generic_parameter: &GenericParameter,
    ) {
        walk_generic_parameter(self, tree, id, generic_parameter);
    }

    /// Visit a Parameter.
    fn visit_parameter(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Parameter>,
        parameter: &Parameter,
    ) {
        walk_parameter(self, tree, id, parameter);
    }

    /// Visit an Argument.
    fn visit_argument(&mut self, tree: &NodeTree, id: LocalNodeId<Argument>, argument: &Argument) {
        walk_argument(self, tree, id, argument);
    }

    /// Visit a GenericArgument.
    fn visit_generic_argument(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<GenericArgument>,
        generic_argument: &GenericArgument,
    ) {
        walk_generic_argument(self, tree, id, generic_argument);
    }

    /// Visit a TupleElement.
    fn visit_tuple_element(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<TupleElement>,
        tuple_element: &TupleElement,
    ) {
        walk_tuple_element(self, tree, id, tuple_element);
    }

    /// Visit a MatchCase.
    fn visit_match_case(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<MatchCase>,
        match_case: &MatchCase,
    ) {
        walk_match_case(self, tree, id, match_case);
    }

    /// Visit a Pattern.
    fn visit_pattern(&mut self, tree: &NodeTree, id: LocalNodeId<Pattern>, pattern: &Pattern) {
        walk_pattern(self, tree, id, pattern);
    }

    /// Visit a PatternField.
    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        walk_pattern_field(self, tree, id, pattern_field);
    }

    /// Visit a Decorator.
    fn visit_decorator(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Decorator>,
        decorator: &Decorator,
    ) {
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

    fn visit_any(&mut self, _tree: &NodeTree, _ty: NodeType, id: u32) {
        self.visited.push(id);
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        _expression: &Expression,
    ) {
        self.visit_any(tree, NodeType::Expression, id.id);
    }

    fn visit_type_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<TypeExpression>,
        _type_expression: &TypeExpression,
    ) {
        self.visit_any(tree, NodeType::TypeExpression, id.id);
    }

    fn visit_block(&mut self, tree: &NodeTree, id: LocalNodeId<Block>, _block: &Block) {
        self.visit_any(tree, NodeType::Block, id.id);
    }

    fn visit_declaration(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Declaration>,
        _declaration: &Declaration,
    ) {
        self.visit_any(tree, NodeType::Declaration, id.id);
    }

    fn visit_declarator(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Declarator>,
        _declarator: &Declarator,
    ) {
        self.visit_any(tree, NodeType::Declarator, id.id);
    }

    fn visit_property(&mut self, tree: &NodeTree, id: LocalNodeId<Property>, _property: &Property) {
        self.visit_any(tree, NodeType::Property, id.id);
    }

    fn visit_type_property(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<TypeProperty>,
        _type_property: &TypeProperty,
    ) {
        self.visit_any(tree, NodeType::TypeProperty, id.id);
    }

    fn visit_member(&mut self, tree: &NodeTree, id: LocalNodeId<Member>, _member: &Member) {
        self.visit_any(tree, NodeType::Member, id.id);
    }

    fn visit_enum_field(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<EnumField>,
        _enum_field: &EnumField,
    ) {
        self.visit_any(tree, NodeType::EnumField, id.id);
    }

    fn visit_where_clause(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<WhereClause>,
        _where_clause: &WhereClause,
    ) {
        self.visit_any(tree, NodeType::WhereClause, id.id);
    }

    fn visit_dependency_item(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<DependencyItem>,
        _dependency_item: &DependencyItem,
    ) {
        self.visit_any(tree, NodeType::DependencyItem, id.id);
    }

    fn visit_generic_parameter(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<GenericParameter>,
        _generic_parameter: &GenericParameter,
    ) {
        self.visit_any(tree, NodeType::GenericParameter, id.id);
    }

    fn visit_parameter(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Parameter>,
        _parameter: &Parameter,
    ) {
        self.visit_any(tree, NodeType::Parameter, id.id);
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: LocalNodeId<Argument>, _argument: &Argument) {
        self.visit_any(tree, NodeType::Argument, id.id);
    }

    fn visit_generic_argument(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<GenericArgument>,
        _generic_argument: &GenericArgument,
    ) {
        self.visit_any(tree, NodeType::GenericArgument, id.id);
    }

    fn visit_tuple_element(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<TupleElement>,
        _tuple_element: &TupleElement,
    ) {
        self.visit_any(tree, NodeType::TupleElement, id.id);
    }

    fn visit_match_case(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<MatchCase>,
        _match_case: &MatchCase,
    ) {
        self.visit_any(tree, NodeType::MatchCase, id.id);
    }

    fn visit_pattern(&mut self, tree: &NodeTree, id: LocalNodeId<Pattern>, _pattern: &Pattern) {
        self.visit_any(tree, NodeType::Pattern, id.id);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<PatternField>,
        _pattern_field: &PatternField,
    ) {
        self.visit_any(tree, NodeType::PatternField, id.id);
    }

    fn visit_decorator(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Decorator>,
        _decorator: &Decorator,
    ) {
        self.visit_any(tree, NodeType::Decorator, id.id);
    }
}
