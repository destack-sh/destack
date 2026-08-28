use crate::{
    Argument, ArrayAssignPatternField, ArrayElement, ArrayPatternField, AssignPattern, Block,
    CatchClause, Declaration, Declarator, ExportSpecifier, Expression, Identifier, IdentifierName,
    ImportAttribute, ImportSpecifier, LocalNodeId, Member, NodeType, ObjectAssignPatternField,
    ObjectPatternField, Parameter, Pattern, Place, Property, ReExportSpecifier, Statement,
    SwitchCase, Tree, walk_argument, walk_array_assign_pattern_field, walk_array_element,
    walk_array_pattern_field, walk_assign_pattern, walk_block, walk_catch_clause, walk_declaration,
    walk_declarator, walk_export_specifier, walk_expression, walk_import_attribute,
    walk_import_specifier, walk_member, walk_object_assign_pattern_field,
    walk_object_pattern_field, walk_parameter, walk_pattern, walk_place, walk_property,
    walk_re_export_specifier, walk_statement, walk_switch_case,
};

/// One visitor over a JavaScript tree.
pub trait NodeVisitor {
    /// Visit one lexical identifier occurrence.
    fn visit_identifier(&mut self, identifier: Identifier) {
        let _ = identifier;
    }

    /// Visit one fixed identifier-name occurrence.
    fn visit_identifier_name(&mut self, identifier: IdentifierName) {
        let _ = identifier;
    }

    /// Visit one dynamically typed node.
    fn visit_any(&mut self, tree: &Tree, node_type: NodeType, id: u32) {
        let _ = tree;
        let _ = node_type;
        let _ = id;
    }

    /// Visit one block.
    fn visit_block(&mut self, tree: &Tree, id: LocalNodeId<Block>, node: &Block) {
        walk_block(self, tree, id, node);
    }

    /// Visit one catch clause.
    fn visit_catch_clause(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<CatchClause>,
        node: &CatchClause,
    ) {
        walk_catch_clause(self, tree, id, node);
    }

    /// Visit one statement.
    fn visit_statement(&mut self, tree: &Tree, id: LocalNodeId<Statement>, node: &Statement) {
        destack_core::ensure_sufficient_stack(|| walk_statement(self, tree, id, node));
    }

    /// Visit one expression.
    fn visit_expression(&mut self, tree: &Tree, id: LocalNodeId<Expression>, node: &Expression) {
        destack_core::ensure_sufficient_stack(|| walk_expression(self, tree, id, node));
    }

    /// Visit one array element.
    fn visit_array_element(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ArrayElement>,
        node: &ArrayElement,
    ) {
        walk_array_element(self, tree, id, node);
    }

    /// Visit one declaration.
    fn visit_declaration(&mut self, tree: &Tree, id: LocalNodeId<Declaration>, node: &Declaration) {
        walk_declaration(self, tree, id, node);
    }

    /// Visit one declarator.
    fn visit_declarator(&mut self, tree: &Tree, id: LocalNodeId<Declarator>, node: &Declarator) {
        walk_declarator(self, tree, id, node);
    }

    /// Visit one property.
    fn visit_property(&mut self, tree: &Tree, id: LocalNodeId<Property>, node: &Property) {
        walk_property(self, tree, id, node);
    }

    /// Visit one member.
    fn visit_member(&mut self, tree: &Tree, id: LocalNodeId<Member>, node: &Member) {
        walk_member(self, tree, id, node);
    }

    /// Visit one import specifier.
    fn visit_import_specifier(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ImportSpecifier>,
        node: &ImportSpecifier,
    ) {
        walk_import_specifier(self, tree, id, node);
    }

    /// Visit one export specifier.
    fn visit_export_specifier(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ExportSpecifier>,
        node: &ExportSpecifier,
    ) {
        walk_export_specifier(self, tree, id, node);
    }

    /// Visit one re-export specifier.
    fn visit_re_export_specifier(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ReExportSpecifier>,
        node: &ReExportSpecifier,
    ) {
        walk_re_export_specifier(self, tree, id, node);
    }

    /// Visit one import attribute.
    fn visit_import_attribute(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ImportAttribute>,
        node: &ImportAttribute,
    ) {
        walk_import_attribute(self, tree, id, node);
    }

    /// Visit one switch case.
    fn visit_switch_case(&mut self, tree: &Tree, id: LocalNodeId<SwitchCase>, node: &SwitchCase) {
        walk_switch_case(self, tree, id, node);
    }

    /// Visit one parameter.
    fn visit_parameter(&mut self, tree: &Tree, id: LocalNodeId<Parameter>, node: &Parameter) {
        walk_parameter(self, tree, id, node);
    }

    /// Visit one argument.
    fn visit_argument(&mut self, tree: &Tree, id: LocalNodeId<Argument>, node: &Argument) {
        walk_argument(self, tree, id, node);
    }

    /// Visit one binding pattern.
    fn visit_pattern(&mut self, tree: &Tree, id: LocalNodeId<Pattern>, node: &Pattern) {
        destack_core::ensure_sufficient_stack(|| walk_pattern(self, tree, id, node));
    }

    /// Visit one array binding field.
    fn visit_array_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ArrayPatternField>,
        node: &ArrayPatternField,
    ) {
        walk_array_pattern_field(self, tree, id, node);
    }

    /// Visit one object binding field.
    fn visit_object_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ObjectPatternField>,
        node: &ObjectPatternField,
    ) {
        walk_object_pattern_field(self, tree, id, node);
    }

    /// Visit one writable place.
    fn visit_place(&mut self, tree: &Tree, id: LocalNodeId<Place>, node: &Place) {
        walk_place(self, tree, id, node);
    }

    /// Visit one assignment pattern.
    fn visit_assign_pattern(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPattern>,
        node: &AssignPattern,
    ) {
        destack_core::ensure_sufficient_stack(|| walk_assign_pattern(self, tree, id, node));
    }

    /// Visit one array assignment field.
    fn visit_array_assign_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ArrayAssignPatternField>,
        node: &ArrayAssignPatternField,
    ) {
        walk_array_assign_pattern_field(self, tree, id, node);
    }

    /// Visit one object assignment field.
    fn visit_object_assign_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ObjectAssignPatternField>,
        node: &ObjectAssignPatternField,
    ) {
        walk_object_assign_pattern_field(self, tree, id, node);
    }
}
