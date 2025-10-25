#![allow(unused_variables)]

use crate::{
    Annotation, Argument, Blank, Block, Comment, Decorator, Definition, Doc, EnumField, Expression,
    DependencyItem, MatchCase, NodeId, NodeTree, NodeType, Parameter, Pattern, PatternField, Tag,
    UnionField, VariantField, WhereClause, WithClause, walk_annotation, walk_argument, walk_blank,
    walk_block, walk_comment, walk_decorator, walk_definition, walk_doc, walk_enum_field,
    walk_expression, walk_import_item, walk_match_case, walk_parameter, walk_pattern,
    walk_pattern_field, walk_tag, walk_union_field, walk_variant_field, walk_where_clause,
    walk_with_clause,
};

/// A NodeVisitor is a visitor for the AST.
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

    /// Visit a VariantField.
    fn visit_variant_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<VariantField>,
        variant_field: &VariantField,
    ) {
        walk_variant_field(self, tree, id, variant_field);
    }

    /// Visit an EnumField.
    fn visit_enum_field(&mut self, tree: &NodeTree, id: NodeId<EnumField>, enum_field: &EnumField) {
        walk_enum_field(self, tree, id, enum_field);
    }

    /// Visit a UnionField.
    fn visit_union_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<UnionField>,
        union_field: &UnionField,
    ) {
        walk_union_field(self, tree, id, union_field);
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

    /// Visit a WhereClause.
    fn visit_where_clause(
        &mut self,
        tree: &NodeTree,
        id: NodeId<WhereClause>,
        where_clause: &WhereClause,
    ) {
        walk_where_clause(self, tree, id, where_clause);
    }

    /// Visit a UseItem.
    fn visit_import_item(
        &mut self,
        tree: &NodeTree,
        id: NodeId<DependencyItem>,
        import_item: &DependencyItem,
    ) {
        walk_import_item(self, tree, id, import_item);
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

    /// Visit a MatchCase.
    fn visit_match_case(&mut self, tree: &NodeTree, id: NodeId<MatchCase>, match_case: &MatchCase) {
        walk_match_case(self, tree, id, match_case);
    }

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

    /// Visit a Blank.
    fn visit_blank(&mut self, tree: &NodeTree, id: NodeId<Blank>, blank: &Blank) {
        walk_blank(self, tree, id, blank);
    }

    /// Visit a Doc.
    fn visit_doc(&mut self, tree: &NodeTree, id: NodeId<Doc>, doc: &Doc) {
        walk_doc(self, tree, id, doc);
    }

    /// Visit a Comment.
    fn visit_comment(&mut self, tree: &NodeTree, id: NodeId<Comment>, comment: &Comment) {
        walk_comment(self, tree, id, comment);
    }

    /// Visit a Tag.
    fn visit_tag(&mut self, tree: &NodeTree, id: NodeId<Tag>, tag: &Tag) {
        walk_tag(self, tree, id, tag);
    }

    /// Visit a Decorator.
    fn visit_decorator(&mut self, tree: &NodeTree, id: NodeId<Decorator>, decorator: &Decorator) {
        walk_decorator(self, tree, id, decorator);
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
    fn visit_any(&mut self, tree: &NodeTree, ty: NodeType, id: u32) {
        self.visited.push(id);
    }

    // ------------------------------------------------------------
    // Groupings
    // ------------------------------------------------------------

    fn visit_block(&mut self, tree: &NodeTree, id: NodeId<Block>, block: &Block) {
        self.visit_any(tree, NodeType::Block, id.id);
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Expression>,
        expression: &Expression,
    ) {
        self.visit_any(tree, NodeType::Expression, id.id);
    }

    // ------------------------------------------------------------
    // Declarations
    // ------------------------------------------------------------

    fn visit_definition(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Definition>,
        definition: &Definition,
    ) {
        self.visit_any(tree, NodeType::Definition, id.id);
    }

    fn visit_variant_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<VariantField>,
        variant_field: &VariantField,
    ) {
        self.visit_any(tree, NodeType::VariantField, id.id);
    }

    fn visit_enum_field(&mut self, tree: &NodeTree, id: NodeId<EnumField>, enum_field: &EnumField) {
        self.visit_any(tree, NodeType::EnumField, id.id);
    }

    fn visit_union_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<UnionField>,
        union_field: &UnionField,
    ) {
        self.visit_any(tree, NodeType::UnionField, id.id);
    }

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------

    fn visit_with_clause(
        &mut self,
        tree: &NodeTree,
        id: NodeId<WithClause>,
        with_clause: &WithClause,
    ) {
        self.visit_any(tree, NodeType::WithClause, id.id);
    }

    fn visit_import_item(
        &mut self,
        tree: &NodeTree,
        id: NodeId<DependencyItem>,
        import_item: &DependencyItem,
    ) {
        self.visit_any(tree, NodeType::DependencyItem, id.id);
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, parameter: &Parameter) {
        self.visit_any(tree, NodeType::Parameter, id.id);
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, argument: &Argument) {
        self.visit_any(tree, NodeType::Argument, id.id);
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    fn visit_pattern(&mut self, tree: &NodeTree, id: NodeId<Pattern>, pattern: &Pattern) {
        self.visit_any(tree, NodeType::Pattern, id.id);
    }

    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        self.visit_any(tree, NodeType::PatternField, id.id);
    }

    fn visit_match_case(&mut self, tree: &NodeTree, id: NodeId<MatchCase>, match_case: &MatchCase) {
        self.visit_any(tree, NodeType::MatchCase, id.id);
    }

    // ------------------------------------------------------------
    // Annotations
    // ------------------------------------------------------------

    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Annotation>,
        annotation: &Annotation,
    ) {
        self.visit_any(tree, NodeType::Annotation, id.id);
    }

    fn visit_blank(&mut self, tree: &NodeTree, id: NodeId<Blank>, blank: &Blank) {
        self.visit_any(tree, NodeType::Blank, id.id);
    }

    fn visit_doc(&mut self, tree: &NodeTree, id: NodeId<Doc>, doc: &Doc) {
        self.visit_any(tree, NodeType::Doc, id.id);
    }

    fn visit_comment(&mut self, tree: &NodeTree, id: NodeId<Comment>, comment: &Comment) {
        self.visit_any(tree, NodeType::Comment, id.id);
    }

    fn visit_tag(&mut self, tree: &NodeTree, id: NodeId<Tag>, tag: &Tag) {
        self.visit_any(tree, NodeType::Tag, id.id);
    }

    fn visit_decorator(&mut self, tree: &NodeTree, id: NodeId<Decorator>, decorator: &Decorator) {
        self.visit_any(tree, NodeType::Decorator, id.id);
    }
}
