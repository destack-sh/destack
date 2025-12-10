#![allow(unused_variables)]

use crate::{
    Annotation, Argument, Block, Declaration, Declarator, DependencyItem, EnumField, Expression,
    LocalNodeId, NodeTree, NodeType, Parameter, Pattern, PatternField, Property, Statement,
    SwitchCase, Type, TypeField, walk_annotation, walk_argument, walk_block, walk_declaration,
    walk_declarator, walk_dependency_item, walk_enum_field, walk_expression, walk_parameter,
    walk_pattern, walk_pattern_field, walk_property, walk_statement, walk_switch_case, walk_type,
    walk_type_field,
};

#[derive(Debug, Clone, Default)]
pub struct NodeVisitorOptions {}

/// A NodeVisitor visits nodes in the JS/TS AST.
pub trait NodeVisitor {
    /// Get the options for the visitor.
    fn options(&self) -> &NodeVisitorOptions;

    #[inline]
    fn visit_any(&mut self, tree: &NodeTree, ty: NodeType, id: u32) {}

    /// Visit a block.
    fn visit_block(&mut self, tree: &NodeTree, id: LocalNodeId<Block>, block: &Block) {
        walk_block(self, tree, id, block);
    }

    /// Visit a statement.
    fn visit_statement(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Statement>,
        statement: &Statement,
    ) {
        walk_statement(self, tree, id, statement);
    }

    /// Visit an expression.
    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        walk_expression(self, tree, id, expression);
    }

    /// Visit a switch case.
    fn visit_switch_case(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<SwitchCase>,
        switch_case: &SwitchCase,
    ) {
        walk_switch_case(self, tree, id, switch_case);
    }

    /// Visit a declaration.
    fn visit_declaration(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        walk_declaration(self, tree, id, declaration);
    }

    /// Visit a declarator.
    fn visit_declarator(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Declarator>,
        declarator: &Declarator,
    ) {
        walk_declarator(self, tree, id, declarator);
    }

    /// Visit a property.
    fn visit_property(&mut self, tree: &NodeTree, id: LocalNodeId<Property>, property: &Property) {
        walk_property(self, tree, id, property);
    }

    /// Visit an enum field.
    fn visit_enum_field(&mut self, tree: &NodeTree, id: LocalNodeId<EnumField>, field: &EnumField) {
        walk_enum_field(self, tree, id, field);
    }

    /// Visit a dependency item.
    fn visit_dependency_item(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<DependencyItem>,
        dependency_item: &DependencyItem,
    ) {
        walk_dependency_item(self, tree, id, dependency_item);
    }

    /// Visit a parameter.
    fn visit_parameter(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Parameter>,
        parameter: &Parameter,
    ) {
        walk_parameter(self, tree, id, parameter);
    }

    /// Visit an argument.
    fn visit_argument(&mut self, tree: &NodeTree, id: LocalNodeId<Argument>, argument: &Argument) {
        walk_argument(self, tree, id, argument);
    }

    /// Visit a pattern.
    fn visit_pattern(&mut self, tree: &NodeTree, id: LocalNodeId<Pattern>, pattern: &Pattern) {
        walk_pattern(self, tree, id, pattern);
    }

    /// Visit a pattern field.
    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<PatternField>,
        field: &PatternField,
    ) {
        walk_pattern_field(self, tree, id, field);
    }

    /// Visit a type.
    fn visit_type(&mut self, tree: &NodeTree, id: LocalNodeId<Type>, ty: &Type) {
        walk_type(self, tree, id, ty);
    }

    /// Visit a type field.
    fn visit_type_field(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<TypeField>,
        attribute: &TypeField,
    ) {
        walk_type_field(self, tree, id, attribute);
    }

    /// Visit an annotation.
    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
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

    fn visit_any(&mut self, _tree: &NodeTree, _ty: NodeType, id: u32) {
        self.visited.push(id);
    }

    fn visit_block(&mut self, tree: &NodeTree, id: LocalNodeId<Block>, _block: &Block) {
        self.visit_any(tree, NodeType::Block, id.id);
    }

    fn visit_statement(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Statement>,
        _statement: &Statement,
    ) {
        self.visit_any(tree, NodeType::Statement, id.id);
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        _expression: &Expression,
    ) {
        self.visit_any(tree, NodeType::Expression, id.id);
    }

    fn visit_switch_case(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<SwitchCase>,
        _switch_case: &SwitchCase,
    ) {
        self.visit_any(tree, NodeType::SwitchCase, id.id);
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

    fn visit_property(&mut self, tree: &NodeTree, id: LocalNodeId<Property>, _field: &Property) {
        self.visit_any(tree, NodeType::Property, id.id);
    }

    fn visit_enum_field(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<EnumField>,
        _field: &EnumField,
    ) {
        self.visit_any(tree, NodeType::EnumField, id.id);
    }

    fn visit_dependency_item(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<DependencyItem>,
        _dependency_item: &DependencyItem,
    ) {
        self.visit_any(tree, NodeType::DependencyItem, id.id);
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

    fn visit_pattern(&mut self, tree: &NodeTree, id: LocalNodeId<Pattern>, _pattern: &Pattern) {
        self.visit_any(tree, NodeType::Pattern, id.id);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<PatternField>,
        _field: &PatternField,
    ) {
        self.visit_any(tree, NodeType::PatternField, id.id);
    }

    fn visit_type(&mut self, tree: &NodeTree, id: LocalNodeId<Type>, _ty: &Type) {
        self.visit_any(tree, NodeType::Type, id.id);
    }

    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Annotation>,
        _annotation: &Annotation,
    ) {
        self.visit_any(tree, NodeType::Annotation, id.id);
    }
}
