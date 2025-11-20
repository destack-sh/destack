#![allow(unused_variables)]

use crate::{
    Annotation, Argument, Block, Definition, DependencyItem, EnumField, Expression, NodeId,
    NodeTree, NodeType, Parameter, Pattern, PatternField, Property, Statement, SwitchCase, Type,
    TypeField, walk_annotation, walk_argument, walk_block, walk_definition, walk_dependency_item,
    walk_enum_field, walk_expression, walk_parameter, walk_pattern, walk_pattern_field,
    walk_property, walk_statement, walk_switch_case, walk_type, walk_type_field,
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
    fn visit_block(&mut self, tree: &NodeTree, id: NodeId<Block>, block: &Block) {
        walk_block(self, tree, id, block);
    }

    /// Visit a statement.
    fn visit_statement(&mut self, tree: &NodeTree, id: NodeId<Statement>, statement: &Statement) {
        walk_statement(self, tree, id, statement);
    }

    /// Visit an expression.
    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Expression>,
        expression: &Expression,
    ) {
        walk_expression(self, tree, id, expression);
    }

    /// Visit a switch case.
    fn visit_switch_case(
        &mut self,
        tree: &NodeTree,
        id: NodeId<SwitchCase>,
        switch_case: &SwitchCase,
    ) {
        walk_switch_case(self, tree, id, switch_case);
    }

    /// Visit a definition.
    fn visit_definition(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Definition>,
        definition: &Definition,
    ) {
        walk_definition(self, tree, id, definition);
    }

    /// Visit a property.
    fn visit_property(&mut self, tree: &NodeTree, id: NodeId<Property>, property: &Property) {
        walk_property(self, tree, id, property);
    }

    /// Visit an enum field.
    fn visit_enum_field(&mut self, tree: &NodeTree, id: NodeId<EnumField>, field: &EnumField) {
        walk_enum_field(self, tree, id, field);
    }

    /// Visit a dependency item.
    fn visit_dependency_item(
        &mut self,
        tree: &NodeTree,
        id: NodeId<DependencyItem>,
        dependency_item: &DependencyItem,
    ) {
        walk_dependency_item(self, tree, id, dependency_item);
    }

    /// Visit a parameter.
    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, parameter: &Parameter) {
        walk_parameter(self, tree, id, parameter);
    }

    /// Visit an argument.
    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, argument: &Argument) {
        walk_argument(self, tree, id, argument);
    }

    /// Visit a pattern.
    fn visit_pattern(&mut self, tree: &NodeTree, id: NodeId<Pattern>, pattern: &Pattern) {
        walk_pattern(self, tree, id, pattern);
    }

    /// Visit a pattern field.
    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<PatternField>,
        field: &PatternField,
    ) {
        walk_pattern_field(self, tree, id, field);
    }

    /// Visit a type.
    fn visit_type(&mut self, tree: &NodeTree, id: NodeId<Type>, ty: &Type) {
        walk_type(self, tree, id, ty);
    }

    /// Visit a type field.
    fn visit_type_field(&mut self, tree: &NodeTree, id: NodeId<TypeField>, attribute: &TypeField) {
        walk_type_field(self, tree, id, attribute);
    }

    /// Visit an annotation.
    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Annotation>,
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

    fn visit_block(&mut self, tree: &NodeTree, id: NodeId<Block>, _block: &Block) {
        self.visit_any(tree, NodeType::Block, id.id);
    }

    fn visit_statement(&mut self, tree: &NodeTree, id: NodeId<Statement>, _statement: &Statement) {
        self.visit_any(tree, NodeType::Statement, id.id);
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Expression>,
        _expression: &Expression,
    ) {
        self.visit_any(tree, NodeType::Expression, id.id);
    }

    fn visit_switch_case(
        &mut self,
        tree: &NodeTree,
        id: NodeId<SwitchCase>,
        _switch_case: &SwitchCase,
    ) {
        self.visit_any(tree, NodeType::SwitchCase, id.id);
    }

    fn visit_definition(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Definition>,
        _definition: &Definition,
    ) {
        self.visit_any(tree, NodeType::Definition, id.id);
    }

    fn visit_property(&mut self, tree: &NodeTree, id: NodeId<Property>, _field: &Property) {
        self.visit_any(tree, NodeType::Property, id.id);
    }

    fn visit_enum_field(&mut self, tree: &NodeTree, id: NodeId<EnumField>, _field: &EnumField) {
        self.visit_any(tree, NodeType::EnumField, id.id);
    }

    fn visit_dependency_item(
        &mut self,
        tree: &NodeTree,
        id: NodeId<DependencyItem>,
        _dependency_item: &DependencyItem,
    ) {
        self.visit_any(tree, NodeType::DependencyItem, id.id);
    }

    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, _parameter: &Parameter) {
        self.visit_any(tree, NodeType::Parameter, id.id);
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, _argument: &Argument) {
        self.visit_any(tree, NodeType::Argument, id.id);
    }

    fn visit_pattern(&mut self, tree: &NodeTree, id: NodeId<Pattern>, _pattern: &Pattern) {
        self.visit_any(tree, NodeType::Pattern, id.id);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<PatternField>,
        _field: &PatternField,
    ) {
        self.visit_any(tree, NodeType::PatternField, id.id);
    }

    fn visit_type(&mut self, tree: &NodeTree, id: NodeId<Type>, _ty: &Type) {
        self.visit_any(tree, NodeType::Type, id.id);
    }

    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Annotation>,
        _annotation: &Annotation,
    ) {
        self.visit_any(tree, NodeType::Annotation, id.id);
    }
}
