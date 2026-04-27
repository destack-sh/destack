#![allow(unused_variables)]

use crate::{
    Annotation, Argument, ArrayElement, AssignPattern, AssignPatternField, Block, CatchClause,
    Declaration, Declarator, DependencyItem, EnumField, Expression, GenericParameter, LocalNodeId,
    Member, NodeType, Parameter, Pattern, PatternField, Property, Statement, SwitchCase, Tree,
    TupleElement, TypeExpression, TypeMember, walk_annotation, walk_argument, walk_array_element,
    walk_assign_pattern, walk_assign_pattern_field, walk_block, walk_catch_clause,
    walk_declaration, walk_declarator, walk_dependency_item, walk_enum_field, walk_expression,
    walk_generic_parameter, walk_member, walk_parameter, walk_pattern, walk_pattern_field,
    walk_property, walk_statement, walk_switch_case, walk_tuple_element, walk_type_expression,
    walk_type_member,
};

#[derive(Debug, Clone, Default)]
pub struct NodeVisitorOptions {}

/// A NodeVisitor visits nodes in the JS/TS AST.
pub trait NodeVisitor {
    /// Get the options for the visitor.
    fn options(&self) -> &NodeVisitorOptions;

    #[inline]
    fn visit_any(&mut self, tree: &Tree, ty: NodeType, id: u32) {}

    /// Visit a block.
    fn visit_block(&mut self, tree: &Tree, id: LocalNodeId<Block>, block: &Block) {
        walk_block(self, tree, id, block);
    }

    /// Visit a catch clause.
    fn visit_catch_clause(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<CatchClause>,
        catch_clause: &CatchClause,
    ) {
        walk_catch_clause(self, tree, id, catch_clause);
    }

    /// Visit a statement.
    fn visit_statement(&mut self, tree: &Tree, id: LocalNodeId<Statement>, statement: &Statement) {
        destack_core::ensure_sufficient_stack(|| walk_statement(self, tree, id, statement));
    }

    /// Visit an expression.
    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        destack_core::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }

    /// Visit one array element.
    fn visit_array_element(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ArrayElement>,
        array_element: &ArrayElement,
    ) {
        walk_array_element(self, tree, id, array_element);
    }

    /// Visit a switch case.
    fn visit_switch_case(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<SwitchCase>,
        switch_case: &SwitchCase,
    ) {
        walk_switch_case(self, tree, id, switch_case);
    }

    /// Visit a declaration.
    fn visit_declaration(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        walk_declaration(self, tree, id, declaration);
    }

    /// Visit a declarator.
    fn visit_declarator(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declarator>,
        declarator: &Declarator,
    ) {
        walk_declarator(self, tree, id, declarator);
    }

    /// Visit a property.
    fn visit_property(&mut self, tree: &Tree, id: LocalNodeId<Property>, property: &Property) {
        walk_property(self, tree, id, property);
    }

    /// Visit a member.
    fn visit_member(&mut self, tree: &Tree, id: LocalNodeId<Member>, member: &Member) {
        walk_member(self, tree, id, member);
    }

    /// Visit an enum field.
    fn visit_enum_field(&mut self, tree: &Tree, id: LocalNodeId<EnumField>, field: &EnumField) {
        walk_enum_field(self, tree, id, field);
    }

    /// Visit a dependency item.
    fn visit_dependency_item(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<DependencyItem>,
        dependency_item: &DependencyItem,
    ) {
        walk_dependency_item(self, tree, id, dependency_item);
    }

    /// Visit one generic parameter.
    fn visit_generic_parameter(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<GenericParameter>,
        parameter: &GenericParameter,
    ) {
        walk_generic_parameter(self, tree, id, parameter);
    }

    /// Visit a parameter.
    fn visit_parameter(&mut self, tree: &Tree, id: LocalNodeId<Parameter>, parameter: &Parameter) {
        walk_parameter(self, tree, id, parameter);
    }

    /// Visit an argument.
    fn visit_argument(&mut self, tree: &Tree, id: LocalNodeId<Argument>, argument: &Argument) {
        walk_argument(self, tree, id, argument);
    }

    /// Visit a pattern.
    fn visit_pattern(&mut self, tree: &Tree, id: LocalNodeId<Pattern>, pattern: &Pattern) {
        walk_pattern(self, tree, id, pattern);
    }

    /// Visit a pattern field.
    fn visit_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PatternField>,
        field: &PatternField,
    ) {
        walk_pattern_field(self, tree, id, field);
    }

    /// Visit an assign pattern.
    fn visit_assign_pattern(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPattern>,
        assign_pattern: &AssignPattern,
    ) {
        walk_assign_pattern(self, tree, id, assign_pattern);
    }

    /// Visit an assign pattern field.
    fn visit_assign_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPatternField>,
        assign_pattern_field: &AssignPatternField,
    ) {
        walk_assign_pattern_field(self, tree, id, assign_pattern_field);
    }

    /// Visit a type expression.
    fn visit_type_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeExpression>,
        type_expression: &TypeExpression,
    ) {
        walk_type_expression(self, tree, id, type_expression);
    }

    /// Visit a tuple element.
    fn visit_tuple_element(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TupleElement>,
        tuple_element: &TupleElement,
    ) {
        walk_tuple_element(self, tree, id, tuple_element);
    }

    /// Visit a type member.
    fn visit_type_member(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeMember>,
        attribute: &TypeMember,
    ) {
        walk_type_member(self, tree, id, attribute);
    }

    /// Visit an annotation.
    fn visit_annotation(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Annotation>,
        annotation: &Annotation,
    ) {
        walk_annotation(self, tree, id, annotation);
    }
}

/// A NodeVisitor that records visited node ids.
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

    fn visit_any(&mut self, _tree: &Tree, _ty: NodeType, id: u32) {
        self.visited.push(id);
    }

    fn visit_block(&mut self, tree: &Tree, id: LocalNodeId<Block>, _block: &Block) {
        self.visit_any(tree, NodeType::Block, id.id);
    }

    fn visit_catch_clause(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<CatchClause>,
        _catch_clause: &CatchClause,
    ) {
        self.visit_any(tree, NodeType::CatchClause, id.id);
    }

    fn visit_statement(&mut self, tree: &Tree, id: LocalNodeId<Statement>, _statement: &Statement) {
        self.visit_any(tree, NodeType::Statement, id.id);
    }

    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        _expression: &Expression,
    ) {
        self.visit_any(tree, NodeType::Expression, id.id);
    }

    fn visit_switch_case(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<SwitchCase>,
        _switch_case: &SwitchCase,
    ) {
        self.visit_any(tree, NodeType::SwitchCase, id.id);
    }

    fn visit_declaration(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declaration>,
        _declaration: &Declaration,
    ) {
        self.visit_any(tree, NodeType::Declaration, id.id);
    }

    fn visit_declarator(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declarator>,
        _declarator: &Declarator,
    ) {
        self.visit_any(tree, NodeType::Declarator, id.id);
    }

    fn visit_property(&mut self, tree: &Tree, id: LocalNodeId<Property>, _field: &Property) {
        self.visit_any(tree, NodeType::Property, id.id);
    }

    fn visit_member(&mut self, tree: &Tree, id: LocalNodeId<Member>, _member: &Member) {
        self.visit_any(tree, NodeType::Member, id.id);
    }

    fn visit_enum_field(&mut self, tree: &Tree, id: LocalNodeId<EnumField>, _field: &EnumField) {
        self.visit_any(tree, NodeType::EnumField, id.id);
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
        _parameter: &GenericParameter,
    ) {
        self.visit_any(tree, NodeType::GenericParameter, id.id);
    }

    fn visit_parameter(&mut self, tree: &Tree, id: LocalNodeId<Parameter>, _parameter: &Parameter) {
        self.visit_any(tree, NodeType::Parameter, id.id);
    }

    fn visit_argument(&mut self, tree: &Tree, id: LocalNodeId<Argument>, _argument: &Argument) {
        self.visit_any(tree, NodeType::Argument, id.id);
    }

    fn visit_array_element(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ArrayElement>,
        _array_element: &ArrayElement,
    ) {
        self.visit_any(tree, NodeType::ArrayElement, id.id);
    }

    fn visit_pattern(&mut self, tree: &Tree, id: LocalNodeId<Pattern>, _pattern: &Pattern) {
        self.visit_any(tree, NodeType::Pattern, id.id);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PatternField>,
        _field: &PatternField,
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

    fn visit_type_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TypeExpression>,
        _type_expression: &TypeExpression,
    ) {
        self.visit_any(tree, NodeType::TypeExpression, id.id);
    }

    fn visit_tuple_element(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<TupleElement>,
        _tuple_element: &TupleElement,
    ) {
        self.visit_any(tree, NodeType::TupleElement, id.id);
    }

    fn visit_annotation(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Annotation>,
        _annotation: &Annotation,
    ) {
        self.visit_any(tree, NodeType::Annotation, id.id);
    }
}
