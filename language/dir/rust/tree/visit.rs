#![allow(unused_variables)]

use crate::{
    Annotation, Argument, Block, Definition, Expression, MatchCase, NodeId, NodeTree, NodeType,
    Parameter, Pattern, PatternField, Type, UseItem, Variant, VariantField, WhereClause,
    WithClause, walk_annotation, walk_argument, walk_block, walk_definition, walk_expression,
    walk_match_case, walk_parameter, walk_pattern, walk_pattern_field, walk_type, walk_use_item,
    walk_variant, walk_variant_field, walk_where_clause, walk_with_clause,
};

/// A NodeVisitor is a visitor for the DIR.
pub trait NodeVisitor {
    #[inline]
    fn visit_any(&mut self, tree: &NodeTree, ty: NodeType, id: u32) {
        // nothing to do
    }

    // ------------------------------------------------------------
    // Groupings
    // ------------------------------------------------------------

    /// Visit an Expression.
    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Expression>,
        expression: &Expression,
    ) {
        walk_expression(self, tree, id, expression);
    }

    /// Visit a Block.
    fn visit_block(&mut self, tree: &NodeTree, id: NodeId<Block>, block: &Block) {
        walk_block(self, tree, id, block);
    }

    // ------------------------------------------------------------
    // Definitions
    // ------------------------------------------------------------

    /// Visit a Definition.
    fn visit_definition(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Definition>,
        definition: &Definition,
    ) {
        walk_definition(self, tree, id, definition);
    }

    // ------------------------------------------------------------
    // Types
    // ------------------------------------------------------------

    /// Visit a Type.
    fn visit_type(&mut self, tree: &NodeTree, id: NodeId<Type>, ty: &Type) {
        walk_type(self, tree, id, ty);
    }

    /// Visit a Variant.
    fn visit_variant(&mut self, tree: &NodeTree, id: NodeId<Variant>, variant: &Variant) {
        walk_variant(self, tree, id, variant);
    }

    /// Visit a VariantField.
    fn visit_variant_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<VariantField>,
        variant_field: &VariantField,
    ) {
        walk_variant_field(self, tree, id, variant_field);
    }

    /// Visit a WhereClause.
    fn visit_where_clause(
        &mut self,
        tree: &NodeTree,
        id: NodeId<WhereClause>,
        where_clause: &WhereClause,
    ) {
        walk_where_clause(self, tree, id, where_clause);
    }

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------

    /// Visit a WithClause.
    fn visit_with_clause(
        &mut self,
        tree: &NodeTree,
        id: NodeId<WithClause>,
        with_clause: &WithClause,
    ) {
        walk_with_clause(self, tree, id, with_clause);
    }

    /// Visit a UseItem.
    fn visit_use_item(&mut self, tree: &NodeTree, id: NodeId<UseItem>, use_item: &UseItem) {
        walk_use_item(self, tree, id, use_item);
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    /// Visit a Parameter.
    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, parameter: &Parameter) {
        walk_parameter(self, tree, id, parameter);
    }

    /// Visit an Argument.
    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, argument: &Argument) {
        walk_argument(self, tree, id, argument);
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    /// Visit a Pattern.
    fn visit_pattern(&mut self, tree: &NodeTree, id: NodeId<Pattern>, pattern: &Pattern) {
        walk_pattern(self, tree, id, pattern);
    }

    /// Visit a PatternField.
    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        walk_pattern_field(self, tree, id, pattern_field);
    }

    /// Visit a MatchCase.
    fn visit_match_case(&mut self, tree: &NodeTree, id: NodeId<MatchCase>, match_case: &MatchCase) {
        walk_match_case(self, tree, id, match_case);
    }

    // ------------------------------------------------------------
    // Annotations
    // ------------------------------------------------------------

    /// Visit an Annotation.
    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Annotation>,
        annotation: &Annotation,
    ) {
        walk_annotation(self, tree, id, annotation);
    }
}

/// A CapturingNodeVisitor is a visitor that collects the nodes visited (without walking further).
#[derive(Debug, Clone, Default)]
pub struct CapturingNodeVisitor {
    visited: Vec<u32>,
}

impl CapturingNodeVisitor {
    pub fn new() -> Self {
        Self {
            visited: Vec::new(),
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
    fn visit_any(&mut self, _tree: &NodeTree, _ty: NodeType, id: u32) {
        self.visited.push(id);
    }

    // ------------------------------------------------------------
    // Groupings
    // ------------------------------------------------------------

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Expression>,
        expression: &Expression,
    ) {
        self.visit_any(tree, NodeType::Expression, id.id);
    }

    fn visit_block(&mut self, tree: &NodeTree, id: NodeId<Block>, _block: &Block) {
        self.visit_any(tree, NodeType::Block, id.id);
    }

    // ------------------------------------------------------------
    // Definitions
    // ------------------------------------------------------------

    fn visit_definition(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Definition>,
        _definition: &Definition,
    ) {
        self.visit_any(tree, NodeType::Definition, id.id);
    }

    // ------------------------------------------------------------
    // Types
    // ------------------------------------------------------------

    fn visit_type(&mut self, tree: &NodeTree, id: NodeId<Type>, _ty: &Type) {
        self.visit_any(tree, NodeType::Type, id.id);
    }

    fn visit_variant(&mut self, tree: &NodeTree, id: NodeId<Variant>, _variant: &Variant) {
        self.visit_any(tree, NodeType::Variant, id.id);
    }

    fn visit_variant_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<VariantField>,
        _variant_field: &VariantField,
    ) {
        self.visit_any(tree, NodeType::VariantField, id.id);
    }

    fn visit_where_clause(
        &mut self,
        tree: &NodeTree,
        id: NodeId<WhereClause>,
        _where_clause: &WhereClause,
    ) {
        self.visit_any(tree, NodeType::WhereClause, id.id);
    }

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------

    fn visit_with_clause(
        &mut self,
        tree: &NodeTree,
        id: NodeId<WithClause>,
        _with_clause: &WithClause,
    ) {
        self.visit_any(tree, NodeType::WithClause, id.id);
    }

    fn visit_use_item(&mut self, tree: &NodeTree, id: NodeId<UseItem>, _use_item: &UseItem) {
        self.visit_any(tree, NodeType::UseItem, id.id);
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, _parameter: &Parameter) {
        self.visit_any(tree, NodeType::Parameter, id.id);
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, _argument: &Argument) {
        self.visit_any(tree, NodeType::Argument, id.id);
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    fn visit_pattern(&mut self, tree: &NodeTree, id: NodeId<Pattern>, _pattern: &Pattern) {
        self.visit_any(tree, NodeType::Pattern, id.id);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<PatternField>,
        _pattern_field: &PatternField,
    ) {
        self.visit_any(tree, NodeType::PatternField, id.id);
    }

    fn visit_match_case(
        &mut self,
        tree: &NodeTree,
        id: NodeId<MatchCase>,
        _match_case: &MatchCase,
    ) {
        self.visit_any(tree, NodeType::MatchCase, id.id);
    }

    // ------------------------------------------------------------
    // Annotations
    // ------------------------------------------------------------

    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Annotation>,
        _annotation: &Annotation,
    ) {
        self.visit_any(tree, NodeType::Annotation, id.id);
    }
}
