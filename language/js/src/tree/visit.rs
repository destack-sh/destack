use crate::{
    Argument, ArrayElement, AssignPattern, AssignPatternField, Block, CatchClause, Declaration,
    Declarator, ExportSpecifier, Expression, ImportAttribute, ImportSpecifier, LocalNodeId, Member,
    NodeType, Parameter, Pattern, PatternField, Property, ReExportSpecifier, Statement, SwitchCase,
    Tree, walk_argument, walk_array_element, walk_assign_pattern, walk_assign_pattern_field,
    walk_block, walk_catch_clause, walk_declaration, walk_declarator, walk_export_specifier,
    walk_expression, walk_import_attribute, walk_import_specifier, walk_member, walk_parameter,
    walk_pattern, walk_pattern_field, walk_property, walk_re_export_specifier, walk_statement,
    walk_switch_case,
};

/// One visitor over a JavaScript tree.
pub trait NodeVisitor {
    /// Visit one dynamically typed node.
    fn visit_any(&mut self, tree: &Tree, node_type: NodeType, id: u32) {
        let _ = tree;
        let _ = node_type;
        let _ = id;
    }

    /// Visit one block.
    fn visit_block(&mut self, tree: &Tree, id: LocalNodeId<Block>, block: &Block) {
        walk_block(self, tree, id, block);
    }

    /// Visit one catch clause.
    fn visit_catch_clause(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<CatchClause>,
        clause: &CatchClause,
    ) {
        walk_catch_clause(self, tree, id, clause);
    }

    /// Visit one statement.
    fn visit_statement(&mut self, tree: &Tree, id: LocalNodeId<Statement>, statement: &Statement) {
        destack_core::ensure_sufficient_stack(|| walk_statement(self, tree, id, statement));
    }

    /// Visit one expression.
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
        element: &ArrayElement,
    ) {
        walk_array_element(self, tree, id, element);
    }

    /// Visit one declaration.
    fn visit_declaration(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        walk_declaration(self, tree, id, declaration);
    }

    /// Visit one declarator.
    fn visit_declarator(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Declarator>,
        declarator: &Declarator,
    ) {
        walk_declarator(self, tree, id, declarator);
    }

    /// Visit one property.
    fn visit_property(&mut self, tree: &Tree, id: LocalNodeId<Property>, property: &Property) {
        walk_property(self, tree, id, property);
    }

    /// Visit one member.
    fn visit_member(&mut self, tree: &Tree, id: LocalNodeId<Member>, member: &Member) {
        walk_member(self, tree, id, member);
    }

    /// Visit one import specifier.
    fn visit_import_specifier(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ImportSpecifier>,
        specifier: &ImportSpecifier,
    ) {
        walk_import_specifier(self, tree, id, specifier);
    }

    /// Visit one export specifier.
    fn visit_export_specifier(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ExportSpecifier>,
        specifier: &ExportSpecifier,
    ) {
        walk_export_specifier(self, tree, id, specifier);
    }

    /// Visit one re-export specifier.
    fn visit_re_export_specifier(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ReExportSpecifier>,
        specifier: &ReExportSpecifier,
    ) {
        walk_re_export_specifier(self, tree, id, specifier);
    }

    /// Visit one import attribute.
    fn visit_import_attribute(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<ImportAttribute>,
        attribute: &ImportAttribute,
    ) {
        walk_import_attribute(self, tree, id, attribute);
    }

    /// Visit one switch case.
    fn visit_switch_case(&mut self, tree: &Tree, id: LocalNodeId<SwitchCase>, case: &SwitchCase) {
        walk_switch_case(self, tree, id, case);
    }

    /// Visit one parameter.
    fn visit_parameter(&mut self, tree: &Tree, id: LocalNodeId<Parameter>, parameter: &Parameter) {
        walk_parameter(self, tree, id, parameter);
    }

    /// Visit one argument.
    fn visit_argument(&mut self, tree: &Tree, id: LocalNodeId<Argument>, argument: &Argument) {
        walk_argument(self, tree, id, argument);
    }

    /// Visit one binding pattern.
    fn visit_pattern(&mut self, tree: &Tree, id: LocalNodeId<Pattern>, pattern: &Pattern) {
        walk_pattern(self, tree, id, pattern);
    }

    /// Visit one binding pattern field.
    fn visit_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<PatternField>,
        field: &PatternField,
    ) {
        walk_pattern_field(self, tree, id, field);
    }

    /// Visit one assignment pattern.
    fn visit_assign_pattern(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPattern>,
        pattern: &AssignPattern,
    ) {
        walk_assign_pattern(self, tree, id, pattern);
    }

    /// Visit one assignment pattern field.
    fn visit_assign_pattern_field(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<AssignPatternField>,
        field: &AssignPatternField,
    ) {
        walk_assign_pattern_field(self, tree, id, field);
    }
}
